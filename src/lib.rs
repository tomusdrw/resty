extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use hyper::{header};
use futures::{future, IntoFuture, Future, Stream};

#[derive(Debug)]
pub enum RequestError {
  Serde(serde_json::Error),
  Hyper(hyper::Error),
}

#[derive(Debug)]
pub struct Request {
  request: hyper::Request,
}

impl Request {
  pub fn json<'a, T: serde::de::DeserializeOwned + 'a>(self) -> Box<Future<Item = T, Error = RequestError> + 'a> where
    Self: 'a
  {
    Box::new(
      self.request.body().concat2().then(|chunk| {
        match chunk {
          Ok(chunk) => match serde_json::from_slice(&*chunk) {
            Ok(res) => future::ok(res),
            Err(err) => future::err(RequestError::Serde(err)),
          },
          Err(err) => future::err(RequestError::Hyper(err)),
        }
      })
    )
  }
}

impl From<hyper::Request> for Request {
  fn from(request: hyper::Request) -> Self {
    Request { request }
  }
}

#[derive(Debug, Default)]
pub struct Response {
  response: hyper::Response,
}

impl Into<hyper::Response> for Response {
  fn into(self) -> hyper::Response {
    self.response
  }
}

impl<T: serde::Serialize> From<T> for Response {
  fn from(val: T) -> Self {
    let serialized = serde_json::to_vec(&val);
    let response = hyper::Response::new()
      .with_status(hyper::StatusCode::Ok)
      .with_header(header::ContentType::json())
      // TODO [ToDr] Add some runtime error
      .with_body(serialized.unwrap_or_else(|_| unimplemented!()));
    Response { response }
  }
}

#[derive(Debug, Default, Serialize)]
pub struct Error {
  pub code: u32,
  pub message: String,
  pub details: String,
}

impl From<RequestError> for Error {
  fn from(err: RequestError) -> Self {
    Error {
      code: 400,
      message: "Unable to parse request".into(),
      details: format!("{:?}", err),
    }
  }
}


type ServerResult = Box<Future<Item = hyper::Response, Error = hyper::Error>>;
type BoxHandler = Box<Fn(Request) -> ServerResult + Sync + Send>;

#[derive(Debug, PartialEq, Eq, Hash)]
enum Method {
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

#[derive(Default)]
pub struct Router {
  routes: HashMap<(Method, String), BoxHandler>,
}

// TODO [ToDr] impl Debug

impl Router {
  pub fn new() -> Self {
    Router::default()
  }

  pub fn get<'a, F, I, R, E>(&mut self, prefix: &str, fun: F) where
    F: Fn(Request) -> I + Sync + Send + 'static,
    I: IntoFuture<Item = R, Error = E>,
    R: Into<Response>,
    E: Into<Error>,
    I::Future: 'static,
  {
    self.routes.insert((Method::Get, prefix.to_owned()), Box::new(move |request| {
      Box::new(fun(request).into_future().then(|result| {
        future::ok(match result {
          Ok(res) => res.into(),
          Err(err) => err.into().into(),
        }.into())
      }))
    }));
  }

  pub fn add(&mut self, prefix: &str, router: Router) {
    for (k, v) in router.routes {
      // TODO parser prefixes
      self.routes.insert((k.0, format!("{}{}", prefix, k.1)), v);
    }
  }

  pub fn bind<T: ::std::net::ToSocketAddrs>(self, address: T) -> Result<Listening, hyper::Error> {
    let address = address.to_socket_addrs().unwrap().next().unwrap();
    let routes = ::std::sync::Arc::new(self.routes);
    let server = hyper::server::Http::new().bind(&address, move || Ok(Server { routes: routes.clone() })).unwrap();
    server.run().unwrap();
    unimplemented!()
  }
}

struct Server {
  routes: ::std::sync::Arc<HashMap<(Method, String), BoxHandler>>,
}
impl hyper::server::Service for Server {
  type Request = hyper::Request;
  type Response = hyper::Response;
  type Error = hyper::Error;
  type Future = ServerResult;

  fn call(&self, req: Self::Request) -> Self::Future {
    let path = req.uri().path().to_owned();
    let method = req.method().into();
    match self.routes.get(&(method, path)) {
      Some(router) => router(req.into()),
      // TODO 404 / method not allowed?
      None => return Box::new(future::ok(hyper::Response::new()))
    }
  }
}

#[derive(Debug)]
pub struct Listening {

}

impl Listening {
  pub fn wait(self) {
    unimplemented!()
  }
}

#[cfg(test)]
mod tests {
}
