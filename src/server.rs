use std::sync::Arc;
use hyper;
use futures::future;

use error::Error;
use router::{Routes, HandlerResult};

#[derive(Clone)]
pub struct Server {
    pub routes: Arc<Routes>,
}

impl Server {
    pub fn new(routes: Routes) -> Self {
        Server {
            routes: Arc::new(routes),
        }
    }
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
            Some((prefix, ref endpoint)) => {
                if endpoint.method == method {
                    (endpoint.handler)(req, prefix)
                } else {
                    Box::new(future::ok(Error::method_not_allowed(
                                format!("Method {:?} is not allowed.", method),
                                format!("Allowed methods: {:?}", endpoint.method)
                                ).into()))
                }
            },
            None => Box::new(future::ok(Error::not_found(
                        "Requested resource was not found."
                        ).into())),
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
