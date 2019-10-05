use futures::{Future, Poll};
use gotham::handler::HandlerFuture;
use gotham::middleware::Middleware;
use gotham::state::{FromState, State};
use gotham_derive::NewMiddleware;
use hyper::HeaderMap;
use std::collections::HashMap;

use mightybadger::payload::RequestInfo;

#[derive(Clone, NewMiddleware)]
pub struct HoneybadgerMiddleware;

struct WithRequestContext<F> {
    inner: F,
    context: RequestInfo,
}

impl<F> WithRequestContext<F> {
    fn new(inner: F, context: RequestInfo) -> Self {
        Self { inner, context }
    }
}

impl<F: Future> Future for WithRequestContext<F> {
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<F::Item, F::Error> {
        let inner = &mut self.inner;
        mightybadger::context::with(&self.context, || inner.poll())
    }
}

impl Middleware for HoneybadgerMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let request_info = {
            let mut cgi_data = HashMap::new();
            let headers = HeaderMap::borrow_from(&state);
            for (name, value) in headers.iter() {
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
                cgi_data.insert(name, String::from_utf8_lossy(value.as_bytes()).into_owned());
            }
            RequestInfo {
                cgi_data: cgi_data,
                ..Default::default()
            }
        };
        let f = mightybadger::context::with(&request_info, || chain(state));
        let f = WithRequestContext::new(f, request_info);
        Box::new(f)
    }
}
