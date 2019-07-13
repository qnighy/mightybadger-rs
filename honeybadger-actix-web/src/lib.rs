extern crate actix_web;
extern crate honeybadger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate serde_urlencoded;

use futures::prelude::*;

use std::collections::HashMap;

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::{HeaderMap, StatusCode, Uri};
use honeybadger::payload::RequestInfo;

use futures::future::{self, FutureResult};

#[derive(Debug)]
pub struct HoneybadgerMiddleware(());

impl HoneybadgerMiddleware {
    pub fn new() -> Self {
        HoneybadgerMiddleware(())
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Unknown Error Response: {}", _0)]
pub struct ErrorStatus(StatusCode);

impl<S> Transform<S> for HoneybadgerMiddleware
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = actix_web::Error>,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = actix_web::Error;
    type Transform = HoneybadgerHandler<S>;
    type InitError = ();
    type Future = FutureResult<HoneybadgerHandler<S>, ()>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(HoneybadgerHandler(service))
    }
}

#[derive(Debug)]
pub struct HoneybadgerHandler<S>(S);

impl<S> Service for HoneybadgerHandler<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = actix_web::Error>,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = actix_web::Error;
    type Future = HoneybadgerHandlerFuture<S::Future>;

    fn poll_ready(&mut self) -> Result<Async<()>, Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        let uri = req.head().uri.clone();
        let headers = {
            let mut headers = HeaderMap::with_capacity(req.head().headers().len());
            for (name, value) in req.head().headers() {
                headers.append(name.clone(), value.clone());
            }
            headers
        };
        HoneybadgerHandlerFuture {
            inner: self.0.call(req),
            uri,
            headers,
        }
    }
}

#[derive(Debug)]
pub struct HoneybadgerHandlerFuture<F> {
    inner: F,
    uri: Uri,
    headers: HeaderMap,
}

impl<F> Future for HoneybadgerHandlerFuture<F>
where
    F: Future<Item = ServiceResponse, Error = actix_web::Error>,
{
    type Item = ServiceResponse;
    type Error = actix_web::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.inner.poll() {
            Ok(Async::Ready(resp)) => {
                self.report(Ok(&resp));
                Ok(Async::Ready(resp))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => {
                self.report(Err(&e));
                Err(e)
            }
        }
    }
}

impl<F> HoneybadgerHandlerFuture<F>
where
    F: Future<Item = ServiceResponse, Error = actix_web::Error>,
{
    fn report(&self, resp: Result<&ServiceResponse, &F::Error>) {
        let status = match resp {
            Ok(resp) => resp.status(),
            Err(e) => e.as_response_error().error_response().status(),
        };
        if !(status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()) {
            return;
        }
        let request_info = {
            let mut cgi_data: HashMap<String, String> = HashMap::new();
            for (name, value) in self.headers.iter() {
                let name = "HTTP_"
                    .chars()
                    .chain(name.as_str().chars())
                    .map(|ch| {
                        if ch == '-' {
                            '_'
                        } else {
                            ch.to_ascii_uppercase()
                        }
                    })
                    .collect::<String>();
                cgi_data.insert(name, String::from_utf8_lossy(value.as_bytes()).to_string());
            }
            let url = format!("http://localhost/{}", self.uri.path());
            let params: HashMap<String, String> = self
                .uri
                .query()
                .and_then(|query| serde_urlencoded::from_str(query).ok())
                .unwrap_or_else(HashMap::new);
            RequestInfo {
                url: url,
                cgi_data: cgi_data,
                params: params,
                ..Default::default()
            }
        };
        honeybadger::context::with(&request_info, || {
            if let Err(error) = resp {
                honeybadger::notify_std_error(error);
            } else {
                let error = ErrorStatus(status);
                honeybadger::notify(&error);
            }
        });
    }
}
