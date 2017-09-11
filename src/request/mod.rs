//! Resty request wrapper.

use hyper;
use futures::{self, Stream, Future};
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

  /// Read the body of this request and deserialize it from JSON.
  /// Returns error in case the request body cannot be read or deserialization fails.
  pub fn json<T>(self) -> JsonResult<T> where
    T: for<'a> serde::de::Deserialize<'a>,
  {
    self.request.body().concat2().then(deserialize)
  }
}

fn deserialize<T: for<'a> serde::de::Deserialize<'a>>(chunk: Result<hyper::Chunk, hyper::Error>) -> Result<T, Error> {
  match chunk {
    Ok(chunk) => match serde_json::from_slice(&*chunk) {
      Ok(res) => Ok(res),
      Err(err) => Err(Error::Serde(err)),
    },
    Err(err) => Err(Error::Hyper(err)),
  }
}

type JsonResult<T> = futures::Then<
  futures::stream::Concat2<hyper::Body>,
  Result<T, Error>,
  fn(Result<hyper::Chunk, hyper::Error>) -> Result<T, Error>,
>;
