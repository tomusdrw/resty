extern crate futures;
extern crate resty;
#[macro_use]
extern crate serde_derive;

use futures::Future;

#[derive(Default)]
struct Products;

impl Products {
  pub fn list(&self, _request: resty::Request) -> Result<Vec<Call>, resty::Error> {
    Ok(vec![Call { test: 1 }, Call { test: 2}])
  }
}

impl Into<resty::Router> for Products {
  fn into(self) -> resty::Router {
    let mut router = resty::Router::new();
    let self_ = ::std::sync::Arc::new(self);
    let a = self_.clone();
    router.get("/", move |request| {
      a.list(request)
    });
    router
  }
}

#[derive(Deserialize, Serialize)]
struct Call {
  pub test: u64,
}

fn main() {
  let mut v1 = resty::Router::new();
  v1.add("/products", Products::default().into());

  let mut server = resty::Router::new();
  server.add("/v1", v1);
  server.get("/test", |request| {
    request.json().map(|mut call: Call| {
      call.test += 1;
      call
    })
  });

  let listening = server.bind("localhost:3000").unwrap();
  listening.wait()
}
