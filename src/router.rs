use hyper;
use futures::{future, Future, IntoFuture};

use error::Error;
use request::Request;
use response::Response;
use server::{Server, Listening};
use prefix_tree;


pub type HandlerResult = Box<Future<Item = hyper::Response, Error = hyper::Error>>;
pub type BoxHandler = Box<Fn(Request) -> HandlerResult + Sync + Send>;

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

/// Resty router.
/// TODO [ToDr] More docs
#[derive(Default)]
pub struct Router {
  routes: prefix_tree::Tree<(Method, BoxHandler)>,
}

// TODO [ToDr] impl Debug

impl Router {
  /// Creates a new instance of router.
  pub fn new() -> Self {
    Router::default()
  }

  /// Declare GET endpoint.
  pub fn get<'a, F, I, R, E>(&mut self, prefix: &str, fun: F) where
    F: Fn(Request) -> I + Sync + Send + 'static,
    I: IntoFuture<Item = R, Error = E>,
    R: Into<Response>,
    E: Into<Error>,
    I::Future: 'static,
  {
    self.routes.insert(prefix, (Method::Get, Box::new(move |request| {
      Box::new(fun(request).into_future().then(|result| {
        future::ok(match result {
          Ok(res) => res.into(),
          Err(err) => err.into().into(),
        }.into())
      }))
    })));
  }

  /// Compose with some other router under given prefix.
  pub fn add(&mut self, prefix: &str, router: Router) {
    self.routes.merge(prefix, router.routes);
  }

  /// Consume the router and start HTTP server on given address.
  pub fn bind<T: ::std::net::ToSocketAddrs>(self, address: T) -> Result<Listening, hyper::Error> {
    let address = address.to_socket_addrs().unwrap().next().unwrap();
    let routes = ::std::sync::Arc::new(self.routes);
    let server = hyper::server::Http::new().bind(&address, move || Ok(Server { routes: routes.clone() })).unwrap();
    server.run().unwrap();
    unimplemented!()
  }
}
