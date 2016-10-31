use std::fmt::{self, Write};

use tokio_core::easy::Serialize;

pub enum StatusCode {
    SwitchingProtocols = 101,
    Ok = 200,
    NotFound = 404,
    InternalServerError = 500,
}

impl StatusCode {
    fn msg(&self) -> &'static str {
        match *self {
            StatusCode::SwitchingProtocols => "101 Switching Protocols",
            StatusCode::Ok => "200 OK",
            StatusCode::NotFound => "404 Not Found",
            StatusCode::InternalServerError => "500 Internal Server Error",
        }
    }
}

pub struct Response {
    headers: Vec<(String, String)>,
    response: String,
    status: String,
}

pub struct Serializer;

impl Response {
    pub fn new() -> Response {
        Response {
            headers: Vec::new(),
            response: String::new(),
            status: String::from("200 OK"),
        }
    }

    pub fn header(&mut self, name: &str, val: &str) -> &mut Response {
        self.headers.push((name.to_string(), val.to_string()));
        self
    }

    pub fn header_unique(&mut self, name: &str, val: &str) -> &mut Response {
        let name_str = name.to_string();

        let mut found = false;


        for &mut (ref x, ref mut y) in &mut self.headers {
            if *x == name_str {
                *y = val.to_string();

                found = true;
            }
        }

        if !found {
            return self.header(name, val);
        }

        self
    }

    pub fn body(&mut self, s: &str) -> &mut Response {
        self.response = s.to_string();
        self
    }

    pub fn status(&mut self, s: &str) -> &mut Response {
        self.status = s.to_string();
        self
    }

    pub fn status_code(&mut self, s: StatusCode) -> &mut Response {
        self.status = s.msg().to_string();
        self
    }

    pub fn content_type(&mut self, s: &str) -> &mut Response {
        self.header_unique("Content-Type", s)
    }
}

impl Serialize for Serializer {
    type In = Response;

    fn serialize(&mut self, msg: Response, buf: &mut Vec<u8>) {
        write!(FastWrite(buf), "\
            HTTP/1.1 {}\r\n\
            Server: xx0\r\n\
            Content-Length: {}\r\n\
            Date: {}\r\n\
        ", msg.status, msg.response.len(), ::date::now()).unwrap();

        for &(ref k, ref v) in &msg.headers {
            buf.extend_from_slice(k.as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(v.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(msg.response.as_bytes());
    }
}

// TODO: impl fmt::Write for Vec<u8>
//
// Right now `write!` on `Vec<u8>` goes through io::Write and is not super
// speedy, so inline a less-crufty implementation here which doesn't go through
// io::Error.
struct FastWrite<'a>(&'a mut Vec<u8>);

impl<'a> fmt::Write for FastWrite<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        fmt::write(self, args)
    }
}
