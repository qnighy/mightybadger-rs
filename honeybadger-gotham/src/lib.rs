extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
#[macro_use]
extern crate scoped_tls;

extern crate honeybadger;

use std::collections::HashMap;
use hyper::header::Headers;
use gotham::middleware::Middleware;
use gotham::state::{FromState, State};
use gotham::handler::HandlerFuture;
use futures::{Future, Poll};

use honeybadger::{HoneybadgerPayload, Plugin, PluginError, RequestInfo};

#[derive(Clone, NewMiddleware)]
pub struct HoneybadgerMiddleware;

scoped_thread_local!(static CURRENT_REQUEST: RequestInfo);

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
        CURRENT_REQUEST.set(&self.context, || inner.poll())
    }
}

impl Middleware for HoneybadgerMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let request_info = {
            let mut cgi_data = HashMap::new();
            let headers = Headers::borrow_from(&state);
            for header in headers.iter() {
                let name = "HTTP_"
                    .chars()
                    .chain(header.name().chars())
                    .map(|ch| {
                        if ch == '-' {
                            '_'
                        } else {
                            ch.to_ascii_uppercase()
                        }
                    })
                    .collect::<String>();
                cgi_data.insert(name, header.value_string());
            }
            RequestInfo {
                cgi_data: cgi_data,
                ..Default::default()
            }
        };
        let f = CURRENT_REQUEST.set(&request_info, || chain(state));
        let f = WithRequestContext::new(f, request_info);
        Box::new(f)
    }
}

pub fn install() {
    use std::sync::{Once, ONCE_INIT};

    static INSTALL_ONCE: Once = ONCE_INIT;

    INSTALL_ONCE.call_once(|| {
        honeybadger::install_hook();

        honeybadger::add_plugin(GothamPlugin);
    });
}

struct GothamPlugin;

impl Plugin for GothamPlugin {
    fn decorate(&self, payload: &mut HoneybadgerPayload) -> Result<bool, PluginError> {
        if !CURRENT_REQUEST.is_set() {
            return Ok(false);
        }
        CURRENT_REQUEST.with(|current_request| {
            payload.request = Some(current_request.clone());
        });
        Ok(true)
    }
}
