use hyper;
use futures::{future, Stream, Future};
use serde;
use serde_json;

/// Request parsing error.
#[derive(Debug)]
pub enum Error {
  /// Deserialization error.
  Serde(serde_json::Error),
  /// Hyper error while reading the body.
  Hyper(hyper::Error),
}

impl From<Error> for ::error::Error {
  fn from(err: Error) -> Self {
    ::error::Error {
      code: 400,
      message: "Unable to parse request".into(),
      details: format!("{:?}", err),
    }
  }
}

/// Resty Request wrapper.
#[derive(Debug)]
pub struct Request {
  request: hyper::Request,
}

impl Request {
  /// Read the body of this request and deserialize it from JSON.
  /// Returns error in case the request body cannot be read or deserialization fails.
  pub fn json<'a, T: serde::de::DeserializeOwned + 'a>(self) -> Box<Future<Item = T, Error = Error> + 'a> where
    Self: 'a
  {
    Box::new(
      self.request.body().concat2().then(|chunk| {
        match chunk {
          Ok(chunk) => match serde_json::from_slice(&*chunk) {
            Ok(res) => future::ok(res),
            Err(err) => future::err(Error::Serde(err)),
          },
          Err(err) => future::err(Error::Hyper(err)),
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
