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

    pub fn bind<T: ::std::net::ToSocketAddrs>(self, address: T) -> Result<Listening, hyper::Error> {
        // TODO handle errors
        let address = address.to_socket_addrs()?.next().unwrap();
        let server = hyper::server::Http::new().bind(&address, move || Ok(self.clone()))?;
        server.run()?;
        unimplemented!()
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
                endpoint.handle(method, req, prefix)
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
