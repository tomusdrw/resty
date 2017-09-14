extern crate futures;
#[macro_use]
extern crate resty;
#[macro_use]
extern crate serde_derive;

use std::sync::RwLock;
use futures::Future;

fn main() {
    let mut v1 = resty::Router::new();
    v1.add("/products", Products {
        products: RwLock::new(vec![
            Product { id: 0, name: "Bread".into() },
            Product { id: 1, name: "Butter".into() },
        ]),
    }.into());

    let mut server = resty::Router::new();
    // Compose routers to form the API
    server.add("/v1", v1);
    server.post("/test", |request| {
        request.json().map(|mut product: Product| {
            product.id += 1;
            product
        })
    });

    println!("{}", server.routes());

    let listening = server.bind("localhost:3000").unwrap();
    listening.wait()
}

#[derive(Deserialize, Serialize, Clone)]
struct Product {
    pub id: usize,
    pub name: String,
}

#[derive(Default)]
struct Products {
    products: RwLock<Vec<Product>>,
}

impl Products {
    pub fn list(&self) -> Result<Vec<Product>, resty::Error> {
        Ok(self.products.read().unwrap().clone())
    }

    pub fn single(&self, id: usize) -> Result<Product, resty::Error> {
        let products = self.products.read().unwrap();
        if id < products.len() {
            Ok(products[id].clone())
        } else {
            Err(resty::Error::not_found(""))
        }
    }

    pub fn add(&self, product: Product) -> Result<Product, resty::Error> {
        self.products.write().unwrap().push(product.clone());
        Ok(product)
    }

    pub fn update(&self, id: usize, product: Product) -> Result<Product, resty::Error> {
        let mut products = self.products.write().unwrap();
        if id < products.len() {
            products[id] = product.clone();
            Ok(product)
        } else {
            Err(resty::Error::not_found(""))
        }
    }
}

// TODO [ToDr] Derive this implementation
impl Into<resty::Router> for Products {
    fn into(self) -> resty::Router {
        let self_ = ::std::sync::Arc::new(self);
        let mut router = resty::Router::new();

        // no params
        let a = self_.clone();
        router.get("/", move |_request| {
            a.list()
        });

        // dynamic params
        let a = self_.clone();
        router.get("/{id}", move |request| {
            a.single(request.params().get("id")?)
        });

        // static params
        let a = self_.clone();
        router.get(url!(/test/{id:usize}), move |request| {
            a.single(request.params().id)
        });

        let a = self_.clone();
        router.put(url!(/{id:usize}), move |request| {
            let a = a.clone();
            let id = request.params().id;
            request.json().map_err(Into::into).and_then(move |product| {
                a.update(id, product)
            })
        });

        // post request
        let a = self_.clone();
        router.post("/", move |request| {
            let a = a.clone();
            request.json().map_err(Into::into).and_then(move |product| {
                a.add(product)
            })
        });

        router
    }
}
