extern crate actix_web;
extern crate honeybadger;
#[macro_use]
extern crate failure;

use std::collections::HashMap;

use actix_web::http::StatusCode;
use actix_web::middleware::{Middleware, Response};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use honeybadger::payload::RequestInfo;

pub struct HoneybadgerMiddleware(());

impl HoneybadgerMiddleware {
    pub fn new() -> Self {
        HoneybadgerMiddleware(())
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Unknown Error Response: {}", _0)]
pub struct ErrorStatus(StatusCode);

impl<S> Middleware<S> for HoneybadgerMiddleware {
    fn response(
        &self,
        req: &HttpRequest<S>,
        resp: HttpResponse,
    ) -> actix_web::error::Result<Response> {
        let status = resp.status();
        if !(status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()) {
            return Ok(Response::Done(resp));
        }
        let request_info = {
            let mut cgi_data: HashMap<String, String> = HashMap::new();
            for (name, value) in req.headers().iter() {
                let name = "HTTP_"
                    .chars()
                    .chain(name.as_str().chars())
                    .map(|ch| {
                        if ch == '-' {
                            '_'
                        } else {
                            ch.to_ascii_uppercase()
                        }
                    }).collect::<String>();
                cgi_data.insert(name, String::from_utf8_lossy(value.as_bytes()).to_string());
            }
            let url = format!("http://localhost/{}", req.path());
            let params: HashMap<String, String> = req
                .query()
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<HashMap<String, String>>();
            RequestInfo {
                url: url,
                cgi_data: cgi_data,
                params: params,
                ..Default::default()
            }
        };
        honeybadger::context::with(&request_info, || {
            if let Some(error) = resp.error() {
                honeybadger::notify(error.as_fail());
            } else {
                let error = ErrorStatus(status);
                honeybadger::notify(&error);
            }
        });
        Ok(Response::Done(resp))
    }
}
