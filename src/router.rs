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

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Method {
  Head,
  Get,
  Post,
  Delete,
  Put,
  Options,
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

pub struct Endpoint {
  pub method: Method,
  pub handler: BoxHandler,
}

/// Resty router.
/// TODO [ToDr] More docs
#[derive(Default)]
pub struct Router {
  routes: Routes,
}

// TODO [ToDr] impl Debug

impl Router {
  /// Creates a new instance of router.
  pub fn new() -> Self {
    Router::default()
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
    let prefix = params.prefix;
    let parser = params.parser;
    self.routes.insert(prefix, Endpoint {
      method,
      handler: Box::new(move |request, prefix_len| {
        // TODO [ToDr] Error handling
        let params = parser.parse(request.uri(), prefix_len).unwrap();
        let req = Request::new(request, params);
        Box::new(fun(req).into_future().then(|result| {
          future::ok(match result {
            Ok(res) => res.into(),
            Err(err) => err.into().into(),
          }.into())
        }))
      }),
    });
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
    let address = address.to_socket_addrs().unwrap().next().unwrap();
    let server = Server::new(self.routes);
    let server = hyper::server::Http::new().bind(&address, move || Ok(server.clone())).unwrap();
    server.run().unwrap();
    unimplemented!()
  }
}
