extern crate futures;
#[macro_use]
extern crate resty;
#[macro_use]
extern crate serde_derive;

use futures::Future;

#[derive(Default)]
struct Products {
  calls: Vec<Call>,
}

impl Products {
  pub fn list(&self) -> Result<Vec<Call>, resty::Error> {
    Ok(self.calls.clone())
  }

  pub fn single(&self, id: usize) -> Result<Call, resty::Error> {
    if id < self.calls.len() {
      Ok(self.calls[id].clone())
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
    router.get("/", move |_request| {
      a.list()
    });

    let a = self_.clone();
    router.get(url!(/test/{id:usize}), move |request| {
      a.single(request.params().id)
    });

    let a = self_.clone();
    // dynamic params implementation
    router.get("/dyn/{id}", move |request| {
      a.single(request.params().get_usize("id")?)
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
    calls: vec![Call { test: 1 }, Call { test: 2}],
  }.into());

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
