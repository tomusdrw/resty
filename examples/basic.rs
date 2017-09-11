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
