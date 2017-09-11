#![warn(missing_docs)]

//! Resty - a simple JSON REST API server.

extern crate arrayvec;
extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

mod error;
mod prefix_tree;
pub mod request;
mod response;
mod router;
mod server;

pub use error::Error;
pub use request::Request;
pub use response::Response;
pub use router::Router;
pub use server::Listening;
pub use hyper::{Uri, StatusCode};


#[cfg(test)]
mod tests {
}
