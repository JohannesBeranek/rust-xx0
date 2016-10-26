extern crate tokio_service;
extern crate tokio_minihttp as http;
extern crate futures;
extern crate env_logger;

use tokio_service::Service;
use futures::{Async, Finished};
use std::io;
use std::io::Read;


static PATHBASE: &'static str = "./htdocs/";

#[derive(Clone)]
struct HelloWorld;

impl Service for HelloWorld {
    type Request = http::Request;
    type Response = http::Response;
    type Error = io::Error;
    type Future = Finished<http::Response, io::Error>;

    fn call(&self, _request: http::Request) -> Self::Future {
        let mut resp = http::Response::new();

        if _request.path() == "/exit" {
            panic!("Exit");
        }

        let mut filename = std::path::PathBuf::from(PATHBASE);

        for path_component in _request.path().split("/") {
            if path_component != "" && path_component != ".." {
                filename.push(path_component);
            }
        }

        let out = std::fs::File::open(&filename).and_then(|mut f| {
            let mut out = String::new();
            let _ = f.read_to_string(&mut out);
            Ok(out)
        }).unwrap_or_else(|_| 
            String::from(filename.to_str().unwrap_or("Error"))
        );
                 
        resp.body(&out);
        futures::finished(resp)
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}

fn main() {
    drop(env_logger::init());
    let addr = "0.0.0.0:8080".parse().unwrap();
    http::Server::new(addr)
        .serve(HelloWorld);
}
