use std::{io, slice, str};

use futures::{Poll, Async};
use tokio_core::easy::{EasyBuf, Parse};

use httparse;

pub struct Request {
    method: Slice,
    path: Slice,
    version: u8,
    // TODO: use a small vec to avoid this unconditional allocation
    headers: Vec<(Slice, Slice)>,
    data: EasyBuf,
}

type Slice = (usize, usize);

pub struct RequestHeaders<'req> {
    headers: slice::Iter<'req, (Slice, Slice)>,
    req: &'req Request,
}

impl Request {
    pub fn method(&self) -> &str {
        str::from_utf8(self.slice(&self.method)).unwrap()
    }

    pub fn path(&self) -> &str {
        str::from_utf8(self.slice(&self.path)).unwrap()
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn headers(&self) -> RequestHeaders {
        RequestHeaders {
            headers: self.headers.iter(),
            req: self,
        }
    }

    fn slice(&self, slice: &Slice) -> &[u8] {
        &self.data.as_slice()[slice.0..slice.1]
    }
}

pub struct Parser;

impl Parse for Parser {
    type Out = Request;

    fn parse(&mut self, buf: &mut EasyBuf) -> Poll<Request, io::Error> {
        // TODO: we should grow this headers array if parsing fails and asks
        //       for more headers
        let (method, path, version, headers, amt) = {
            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut r = httparse::Request::new(&mut headers);
            let status = try!(r.parse(buf.as_slice()).map_err(|e| {
                let msg = format!("failed to parse http request: {:?}", e);
                io::Error::new(io::ErrorKind::Other, msg)
            }));

            let amt = match status {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => return Ok(Async::NotReady),
            };

            let toslice = |a: &[u8]| {
                let start = a.as_ptr() as usize - buf.as_slice().as_ptr() as usize;
                assert!(start < buf.len());
                (start, start + a.len())
            };

            (toslice(r.method.unwrap().as_bytes()),
             toslice(r.path.unwrap().as_bytes()),
             r.version.unwrap(),
             r.headers
              .iter()
              .map(|h| (toslice(h.name.as_bytes()), toslice(h.value)))
              .collect(),
             amt)
        };

        Ok(Request {
            method: method,
            path: path,
            version: version,
            headers: headers,
            data: buf.drain_to(amt),
        }.into())
    }
}

impl<'req> Iterator for RequestHeaders<'req> {
    type Item = (&'req str, &'req [u8]);

    fn next(&mut self) -> Option<(&'req str, &'req [u8])> {
        self.headers.next().map(|&(ref a, ref b)| {
            let a = self.req.slice(a);
            let b = self.req.slice(b);
            (str::from_utf8(a).unwrap(), b)
        })
    }
}