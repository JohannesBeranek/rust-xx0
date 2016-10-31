extern crate tokio_service;
extern crate futures;
extern crate env_logger;

extern crate http;

extern crate sha1;
extern crate rustc_serialize as serialize;

use tokio_service::Service;
use futures::{Async, Finished};
use std::io;
use std::io::Read;

// for ws
use sha1::Sha1;
use serialize::base64::ToBase64;



static PATHBASE: &'static str = "htdocs/";
static WS_MAGIC: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Clone)]
struct StaticFiles;

impl StaticFiles {
    fn set_error_response<'a>(&'a self, resp: &'a mut http::Response) -> &mut http::Response {
        resp.status_code(http::StatusCode::InternalServerError);
        resp.content_type("text/html");
        resp.body("Error");
        resp
    }

    fn finish_error_response(&self, mut resp: http::Response) -> Finished<http::Response, io::Error> {
        let _ = self.set_error_response(& mut resp);
        futures::finished(resp)
    }
}

impl Service for StaticFiles {
    type Request = http::Request;
    type Response = http::Response;
    type Error = io::Error;
    type Future = Finished<http::Response, io::Error>;


    fn call(&self, _request: http::Request) -> Self::Future {
        let mut resp = http::Response::new();

        // Only allow GET requests
        if _request.method() != "GET" {
            return self.finish_error_response(resp);
        }


        // version can be one of
        // 0 => http 1.0
        // 1 => http 1.1
        // We need at least 1.1 for websockets
        if _request.version() >= 1 {
            // Now we need the headers
            // Upgrade: websocket
            // Connection: Upgrade
            // Sec-WebSocket-Key: ...
            // Sec-WebSocket-Version: >= 13

            let mut has_hupgrade = false;
            let mut has_hconnection = false;
            let mut hwskey = None;
            let mut has_hversion = false;

            for (name, value) in _request.headers() {
                match name {
                    "Upgrade" => {
                        if std::str::from_utf8(value) == Ok("websocket") {
                            has_hupgrade = true;
                        }
                    },
                    "Connection" => {
                        if std::str::from_utf8(value) == Ok("Upgrade") {
                            has_hconnection = true;
                        }
                    },
                    "Sec-WebSocket-Key" => {
                        hwskey = std::str::from_utf8(value).ok();
                    },
                    "Sec-WebSocket-Version" => {
                        match std::str::from_utf8(value).ok().and_then(|v| v.parse::<u8>().ok()) {
                            Some(n) if n >= 13 => {
                                has_hversion = true;
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
                
            }

            // ws handshake etc
            if has_hupgrade && has_hconnection && has_hversion && hwskey.is_some() {
                let hwskey_concat = hwskey.unwrap_or_default().to_string() + WS_MAGIC;

                let mut hash = Sha1::new();
                hash.update(hwskey_concat.as_bytes());
                let return_key = hash.digest().bytes().to_base64(serialize::base64::STANDARD);

                resp.status_code(http::StatusCode::SwitchingProtocols);
                resp.header("Upgrade", "Websocket");
                resp.header("Connection", "Upgrade");
                resp.header("Sec-WebSocket-Accept", &return_key);
                return futures::finished(resp);
            }
        }
           

        let mut filename = std::path::PathBuf::from(PATHBASE);

        for path_component in _request.path().split("/") {
            if path_component != "" && path_component != ".." {
                filename.push(path_component);
            }
        }

        let out = std::fs::metadata(&filename).and_then(|meta| {
            if meta.is_dir() {
                // TODO: this would break if there was a dir named index.html
                filename.push("index.html");
            }

            if let Some(content_type) = match filename.extension().and_then(|os_str| os_str.to_str()).unwrap_or_default() {
                "html" => Some("text/html"),
                "js" => Some("application/javascript"),
                _ => None
            } {
                resp.content_type(content_type);
            }
             

            std::fs::File::open(&filename).and_then(|mut f| {
                let mut out = String::new();
                let _ = f.read_to_string(&mut out);
                Ok(out)
            })
        }).unwrap_or_else(|_| {
            resp.status_code(http::StatusCode::NotFound);
            resp.content_type("text/html");
            String::from(filename.to_str().unwrap_or("Error"))
        });
                 
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
        .serve(StaticFiles);
}
