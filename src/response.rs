use hyper::{self, header};
use serde;
use serde_json;

/// Resty response wrapper.
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
