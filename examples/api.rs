extern crate futures;
#[macro_use]
extern crate resty;
#[macro_use]
extern crate serde_derive;

use std::sync::RwLock;
use futures::Future;

#[derive(Default)]
struct Products {
    calls: RwLock<Vec<Call>>,
}

impl Products {
    pub fn list(&self) -> Result<Vec<Call>, resty::Error> {
        Ok(self.calls.read().unwrap().clone())
    }

    pub fn single(&self, id: usize) -> Result<Call, resty::Error> {
        let calls = self.calls.read().unwrap();
        if id < calls.len() {
            Ok(calls[id].clone())
        } else {
            Err(resty::Error::not_found(""))
        }
    }

    pub fn add(&self, call: Call) -> Result<Call, resty::Error> {
        self.calls.write().unwrap().push(call.clone());
        Ok(call)
    }

    pub fn update(&self, id: usize, call: Call) -> Result<Call, resty::Error> {
        let mut calls = self.calls.write().unwrap();
        if id < calls.len() {
            calls[id] = call.clone();
            Ok(call)
        } else {
            Err(resty::Error::not_found(""))
        }
    }
}

// TODO [ToDr] Derive this implementatio
impl Into<resty::Router> for Products {
    fn into(self) -> resty::Router {
        let mut router = resty::Router::new();
        let self_ = ::std::sync::Arc::new(self);
        let a = self_.clone();
        // no params
        router.get("/", move |_request| {
            a.list()
        });

        let a = self_.clone();
        // dynamic params
        router.get("/{id}", move |request| {
            a.single(request.params().get_usize("id")?)
        });

        let a = self_.clone();
        // static params
        router.get(url!(/test/{id:usize}), move |request| {
            a.single(request.params().id)
        });

        let a = self_.clone();
        router.put(url!(/{id:usize}), move |request| {
            let a = a.clone();
            let id = request.params().id;
            request.json().map_err(Into::into).and_then(move |call: Call| {
                a.update(id, call)
            })
        });

        let a = self_.clone();
        // post request
        router.post("/", move |request| {
            let a = a.clone();
            request.json().map_err(Into::into).and_then(move |call: Call| {
                a.add(call)
            })
        });

        router
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct Call {
    pub test: u64,
}

fn main() {
    let mut v1 = resty::Router::new();
    v1.add("/products", Products {
        calls: RwLock::new(vec![Call { test: 1 }, Call { test: 2}]),
    }.into());

    let mut server = resty::Router::new();
    server.add("/v1", v1);
    server.post("/test", |request| {
        request.json().map(|mut call: Call| {
            call.test += 1;
            call
        })
    });

    println!("{}", server.routes());
    let listening = server.bind("localhost:3000").unwrap();
    listening.wait()
}
