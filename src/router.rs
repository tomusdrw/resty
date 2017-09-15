use std::fmt;
use hyper;
use futures::{future, Future, IntoFuture};

use config::{Config, MaterializedConfig};
use error::Error;
use request::{params, Params, Request};
use response::Response;
use server::{Server, Listening};
use prefix_tree;


pub type HandlerResult = Box<Future<Item = hyper::Response, Error = hyper::Error>>;
pub type BoxHandler = Box<Fn(hyper::Request, usize) -> HandlerResult + Sync + Send>;
pub type Routes = prefix_tree::Tree<Endpoint>;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Method {
    Head,
    Get,
    Post,
    Patch,
    Delete,
    Put,
    Options,
}

impl fmt::Display for Method {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::Method::*;

        write!(fmt, "{}", match *self {
            Head => "HEAD",
            Get => "GET",
            Post => "POST",
            Delete => "DELETE",
            Put => "PUT",
            Patch => "PATCH",
            Options => "OPTIONS",
        })
    }
}

impl<'a> From<&'a hyper::Method> for Method {
    fn from(method: &'a hyper::Method) -> Self {
        use self::Method::*;
        match *method {
            hyper::Method::Head => Head,
            hyper::Method::Get => Get,
            hyper::Method::Post => Post,
            hyper::Method::Delete => Delete,
            hyper::Method::Put => Put,
            hyper::Method::Patch => Patch,
            hyper::Method::Options => Options,
            _ => Get,
        }
    }
}

const MAX_NUMBER_OF_ENDPOINTS: usize = 6;

pub enum EndpointHandler {
    None,
    Some {
        // TODO [ToDr] Many methods with single handler?
        method: Method,
        params: (usize, String),
        handler: BoxHandler,
    }
}

impl fmt::Debug for EndpointHandler {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EndpointHandler::None => write!(fmt, "none"),
            EndpointHandler::Some { ref method, .. } => write!(fmt, "{} handler", method),
        }
    }
}

#[derive(Debug)]
pub struct Endpoint {
    handlers: [EndpointHandler; MAX_NUMBER_OF_ENDPOINTS],
    base_config: Config,
    config: MaterializedConfig,
}

impl fmt::Display for Endpoint {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..MAX_NUMBER_OF_ENDPOINTS {
            match self.handlers[i] {
                EndpointHandler::None if i == 0 => {
                    return writeln!(fmt, "  ?empty handler?");
                },
                EndpointHandler::None => {
                    return Ok(());
                },
                EndpointHandler::Some { ref method, ref params, .. } => {
                    writeln!(fmt, "  {} {}", method, if params.0 == 0 { "/" } else { &params.1 })?;
                }
            }
        }
        Ok(())
    }
}

impl Endpoint {
    pub fn with_config(base_config: Config) -> Self {
        use self::EndpointHandler::None;
        let config = base_config.materialize();
        Endpoint {
            handlers: [None, None, None, None, None, None],
            base_config,
            config,
        }
    }

    /// Adds another config to the list of configs.
    /// All options that have not been set by previous configs
    /// will be applied.
    fn add_config(&mut self, config: &Config) {
        self.base_config.add(config);
        self.config = self.base_config.materialize();
    }

    pub fn add(&mut self, method: Method, params: (usize, String), handler: BoxHandler) -> bool {
        for i in 0..MAX_NUMBER_OF_ENDPOINTS {
            match self.handlers[i] {
                EndpointHandler::Some { .. } => continue,
                EndpointHandler::None => {},
            }
            self.handlers[i] = EndpointHandler::Some {
                method,
                params,
                handler,
            };
            return true;
        }

        false
    }

    pub fn handle(&self, m: Method, req: hyper::Request, prefix: usize) -> HandlerResult {
        if self.config.extra_headers.is_empty() {
            Box::new(self.handle_internal(m, req, prefix))
        } else {
            let extra_headers = self.config.extra_headers.clone();
            Box::new(self.handle_internal(m, req, prefix).map(move |mut response| {
                {
                    let mut headers = response.headers_mut();
                    for (name, val) in extra_headers {
                        // Don't override headers that were provided with the response.
                        if headers.get_raw(name).is_none() {
                            headers.set_raw(name, val);
                        }
                    }
                }
                response
            }))
        }
    }

    fn handle_internal(&self, m: Method, req: hyper::Request, prefix: usize) -> future::Either<
        HandlerResult,
        future::FutureResult<hyper::Response, hyper::Error>,
    > {
        use self::future::Either;

        let expected = {
            let path = &req.path()[prefix..];
            if path.is_empty() {
                0
            } else {
                path.split('/').count()
            }
        };
        let mut method_found = false;

        for i in 0..MAX_NUMBER_OF_ENDPOINTS {
            match self.handlers[i] {
                EndpointHandler::None => break,
                EndpointHandler::Some { ref method, ref params, ref handler } => {
                    if method != &m {
                        continue;
                    }
                    method_found = true;

                    if params.0 != expected {
                        continue;
                    }

                    return Either::A(handler(req, prefix));
                },
            }
        }

        if method_found {
            Either::B(future::ok(Error::not_found("Unable to find a handler.").into()))
        } else if m == Method::Head && self.config.handle_head {
            Either::A(Box::new(self.handle_internal(Method::Get, req, prefix).map(|mut response| {
                response.set_body(vec![]);
                response
            })))
        } else {
            Either::B(future::ok(Error::method_not_allowed(
                format!("Method {} is not allowed.", m),
                format!("Allowed methods: {}", self.allowed_methods())
            ).into()))
        }
    }

    fn allowed_methods(&self) -> String {
        let mut string = self.handlers.iter()
            .filter_map(|h| match *h {
                EndpointHandler::None => None,
                EndpointHandler::Some { ref method, .. } => Some(format!("{}", method)),
            })
            .fold(String::new(), |acc, s| acc + &s + ",");
        string.pop();
        string
    }
}

/// Resty router.
/// TODO [ToDr] More docs
#[derive(Default, Debug)]
pub struct Router {
    routes: Routes,
    config: Config,
}

impl Router {
    /// Creates a new instance of router.
    pub fn new() -> Self {
        Router::default()
    }

    /// Creates a new instance of router with given config.
    pub fn with_config(config: Config) -> Self {
        let mut r = Router::new();
        r.config = config;
        r
    }

    /// Pretty-prints the endpoints handled by given router.
    pub fn routes(&self) -> String {
        let mut s = String::new();
        let mut it = self.routes.iter();
        while let Some((prefix, route)) = it.next() {
            let prefix = ::std::str::from_utf8(&prefix).expect("Storing only strings in tree; qed");
            s.push_str(&format!("{}\n{}\n", prefix, route));
        }
        s
    }

    /// Compose with some other router under given prefix.
    pub fn add(&mut self, prefix: &str, mut router: Router) {
        let config = self.config.clone();
        let f = move |endpoint: &mut Endpoint| endpoint.add_config(&config);
        router.routes.for_each(&f);

        self.routes.merge(prefix, router.routes);
    }

    /// Consume the router and start HTTP server on given address.
    pub fn bind<T: ::std::net::ToSocketAddrs>(self, address: T) -> Result<Listening, hyper::Error> {
        let server = Server::new(self.routes);
        server.bind(address)
    }

    /// Declare endpoint.
    pub fn on<'a, F, I, R, E, D, P>(&mut self, method: Method, params: D, fun: F) where
        F: Fn(Request<P::Params>) -> I + Sync + Send + 'static,
        I: IntoFuture<Item = R, Error = E>,
        R: Into<Response>,
        E: Into<Error>,
        D: Into<Params<'a, P>>,
        P: params::Parser,
        I::Future: 'static,
    {
        let params = params.into();
        let parser = params.parser;
        let mut endpoint = self.routes.remove(params.prefix).unwrap_or_else(|| Endpoint::with_config(self.config.clone()));
        let added = endpoint.add(method, parser.expected_params(), Box::new(move |request, prefix_len| {
            let params = match parser.parse(request.uri(), prefix_len) {
                Ok(params) => params,
                Err(err) => return Box::new(future::ok(Error::from(err).into())),
            };
            let req = Request::new(request, params);
            Box::new(fun(req).into_future().then(|result| {
                future::ok(match result {
                    Ok(res) => res.into(),
                    Err(err) => err.into().into(),
                }.into())
            }))
        }));
        assert!(added, "The server does not support more than {} handlers for single prefix.", MAX_NUMBER_OF_ENDPOINTS);
        self.routes.insert(params.prefix, endpoint);
    }


    /// Declare GET endpoint.
    pub fn get<'a, F, I, R, E, D, P>(&mut self, prefix: D, fun: F) where
        F: Fn(Request<P::Params>) -> I + Sync + Send + 'static,
        I: IntoFuture<Item = R, Error = E>,
        R: Into<Response>,
        E: Into<Error>,
        D: Into<Params<'a, P>>,
        P: params::Parser,
        I::Future: 'static,
    {
        self.on(Method::Get, prefix, fun)
    }

    /// Declare POST endpoint.
    pub fn post<'a, F, I, R, E, D, P>(&mut self, prefix: D, fun: F) where
        F: Fn(Request<P::Params>) -> I + Sync + Send + 'static,
        I: IntoFuture<Item = R, Error = E>,
        R: Into<Response>,
        E: Into<Error>,
        D: Into<Params<'a, P>>,
        P: params::Parser,
        I::Future: 'static,
    {
        self.on(Method::Post, prefix, fun)
    }

    /// Declare PUT endpoint.
    pub fn put<'a, F, I, R, E, D, P>(&mut self, prefix: D, fun: F) where
        F: Fn(Request<P::Params>) -> I + Sync + Send + 'static,
        I: IntoFuture<Item = R, Error = E>,
        R: Into<Response>,
        E: Into<Error>,
        D: Into<Params<'a, P>>,
        P: params::Parser,
        I::Future: 'static,
    {
        self.on(Method::Put, prefix, fun)
    }

    /// Declare PATCH endpoint.
    pub fn patch<'a, F, I, R, E, D, P>(&mut self, prefix: D, fun: F) where
        F: Fn(Request<P::Params>) -> I + Sync + Send + 'static,
        I: IntoFuture<Item = R, Error = E>,
        R: Into<Response>,
        E: Into<Error>,
        D: Into<Params<'a, P>>,
        P: params::Parser,
        I::Future: 'static,
    {
        self.on(Method::Patch, prefix, fun)
    }

    /// Declare DELETE endpoint.
    pub fn delete<'a, F, I, R, E, D, P>(&mut self, prefix: D, fun: F) where
        F: Fn(Request<P::Params>) -> I + Sync + Send + 'static,
        I: IntoFuture<Item = R, Error = E>,
        R: Into<Response>,
        E: Into<Error>,
        D: Into<Params<'a, P>>,
        P: params::Parser,
        I::Future: 'static,
    {
        self.on(Method::Delete, prefix, fun)
    }
}
