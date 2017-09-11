//! Resty request wrapper.

use hyper;
use futures::{future, Stream, Future};
use serde;
use serde_json;

use error;

pub mod params;
pub mod url_parser;

pub use self::params::Params;

/// Request parsing error.
#[derive(Debug)]
pub enum Error {
  /// Deserialization error.
  Serde(serde_json::Error),
  /// Hyper error while reading the body.
  Hyper(hyper::Error),
}

impl From<Error> for error::Error {
  fn from(err: Error) -> Self {
    error::Error::bad_request(
      "Unable to parse request as JSON.",
      format!("{:?}", err),
    )
  }
}

/// Resty Request wrapper.
#[derive(Debug)]
pub struct Request<P = ()> {
  request: hyper::Request,
  params: Option<P>,
}

impl<P> Request<P> {
  /// Creates new instance of request
  pub fn new(request: hyper::Request, params: P) -> Self {
    Request { request, params: Some(params) }
  }

  /// Returns params reference.
  pub fn params(&self) -> &P {
    self.params.as_ref().unwrap()
  }

  /// Consumes params.
  pub fn take_params(&mut self) -> P {
    self.params.take().unwrap()
  }

  // TODO Don't require DeserializeOwned here!
  /// Read the body of this request and deserialize it from JSON.
  /// Returns error in case the request body cannot be read or deserialization fails.
  pub fn json<'b, T: serde::de::DeserializeOwned + 'b>(self) -> Box<Future<Item = T, Error = Error> + 'b> {
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
