use std::sync::Arc;
use hyper;
use futures::future;

use router::{Method, BoxHandler, HandlerResult};
use prefix_tree;

pub struct Server {
  pub routes: Arc<prefix_tree::Tree<(Method, BoxHandler)>>,
}

impl hyper::server::Service for Server {
  type Request = hyper::Request;
  type Response = hyper::Response;
  type Error = hyper::Error;
  type Future = HandlerResult;

  fn call(&self, req: Self::Request) -> Self::Future {
    let path = req.uri().path().to_owned();
    let method = req.method().into();
    match self.routes.find(path) {
      Some(&(ref m, ref router)) => {
        if *m == method {
          router(req.into())
        } else {
          Box::new(future::ok(hyper::Response::new()))
        }
      },
      // TODO 404 / method not allowed?
      None => Box::new(future::ok(hyper::Response::new()))
    }
  }
}

/// Resty Server Handle
#[derive(Debug)]
pub struct Listening {

}

impl Listening {
  /// Block the thread waiting for the server to finish.
  pub fn wait(self) {
    unimplemented!()
  }
}
