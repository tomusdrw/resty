use std::fmt;
use hyper;
use futures::{future, Future, IntoFuture};

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
    pub handlers: [EndpointHandler; MAX_NUMBER_OF_ENDPOINTS],
}

impl fmt::Display for Endpoint {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..MAX_NUMBER_OF_ENDPOINTS {
            match self.handlers[i] {
                EndpointHandler::None if i == 0 => {
                    return write!(fmt, "empty handler");
                },
                EndpointHandler::None => {
                    return Ok(());
                },
                EndpointHandler::Some { ref method, .. } => {
                    write!(fmt, "{},", method)?;
                }
            }
        }
        Ok(())
    }
}

impl Endpoint {
    pub fn new() -> Self {
        use self::EndpointHandler::None;
        Endpoint {
            handlers: [None, None, None, None, None, None],
        }
    }

    pub fn add(&mut self, method: Method, handler: BoxHandler) -> bool {
        for i in 0..MAX_NUMBER_OF_ENDPOINTS {
            match self.handlers[i] {
                EndpointHandler::Some { .. } => continue,
                EndpointHandler::None => {},
            }
            self.handlers[i] = EndpointHandler::Some {
                method,
                handler,
            };
            return true;
        }

        false
    }

    pub fn handle(&self, m: Method, req: hyper::Request, prefix: usize) -> HandlerResult {
        for i in 0..MAX_NUMBER_OF_ENDPOINTS {
            match self.handlers[i] {
                EndpointHandler::None => break,
                EndpointHandler::Some { ref method, ref handler } if method == &m => {
                    // TODO [ToDr] Handle validation failures separately and fallback to other methods.
                    return handler(req, prefix);
                },
                _ => {},
            }
        }

        Box::new(future::ok(Error::method_not_allowed(
            format!("Method {} is not allowed.", m),
            format!("Allowed methods: {:?}", self.allowed_methods())
        ).into()))
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
}

impl Router {
    /// Creates a new instance of router.
    pub fn new() -> Self {
        Router::default()
    }

    /// Pretty-prints the endpoints handled by given router.
    pub fn routes(&self) -> String {
        let mut s = String::new();
        let mut it = self.routes.iter();
        while let Some((prefix, route)) = it.next() {
            let prefix = ::std::str::from_utf8(&prefix).expect("Storing only strings in tree; qed");
            s.push_str(&format!("{} -> {}\n", prefix, route));
        }
        s
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
        let mut endpoint = self.routes.remove(params.prefix).unwrap_or_else(Endpoint::new);
        let added = endpoint.add(method, Box::new(move |request, prefix_len| {
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

    /// Compose with some other router under given prefix.
    pub fn add(&mut self, prefix: &str, router: Router) {
        self.routes.merge(prefix, router.routes);
    }

    /// Consume the router and start HTTP server on given address.
    pub fn bind<T: ::std::net::ToSocketAddrs>(self, address: T) -> Result<Listening, hyper::Error> {
        let server = Server::new(self.routes);
        server.bind(address)
    }
}
