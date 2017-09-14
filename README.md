# Resty

Resty - a simple JSON REST-API framework for Rust.

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/tomusdrw/resty.svg?branch=master
[travis-url]: https://travis-ci.org/tomusdrw/resty

[Documentation](http://docs.rs/resty)

# Examples
```rust
extern crate futures;
extern crate resty;
#[macro_use]
extern crate serde_derive;

use futures::Future;

#[derive(Deserialize, Serialize)]
struct Call {
    pub test: u64,
}

fn main() {
    let mut server = resty::Router::new();

    server.get("/", |_| {
        Ok("Hello World!") as Result<_, resty::Error>
    });

    server.post("/", |request| {
        // Deserialize payload
        request.json().map(|mut call: Call| {
            call.test += 1;
            // And return the same payload as a response
            call
        })
    });

    // Print out supported routes.
    println!("{}", server.routes());

    let listening = server.bind("localhost:3000").unwrap();
    listening.wait()
}
```

For more see [examples folder](./examples).

# TODO

## General
- [x] `get_*()` for dynamic params.
- [x] Auto handle HEAD requests.
- [ ] CORS support
- [ ] Middlewares
- [ ] Cache Control
- [ ] Auto-derive `Into<Router>` for structs.
- [ ] Query parameters
- [ ] Optional parameters
- [ ] Parameters with /

