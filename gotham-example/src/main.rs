extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate tokio;

extern crate honeybadger;
extern crate honeybadger_gotham;

use std::time::{Duration, Instant};
use hyper::{Response, StatusCode};
use gotham::router::Router;
use gotham::router::builder::*;
use gotham::http::response::create_response;
use gotham::state::State;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::handler::HandlerFuture;
use tokio::timer::Delay;
use tokio::prelude::*;

fn router() -> Router {
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(honeybadger_gotham::HoneybadgerMiddleware)
            .build(),
    );
    build_router(chain, pipelines, |route| {
        route.get("/").to(index);
        route.get("/ping").to(ping);
        route.get("/error").to(error);
        route.get("/error_wait").to(error_wait);
    })
}

fn index(state: State) -> (State, Response) {
    let bytes = "Hello, world!".to_string().into_bytes();
    let res = create_response(&state, StatusCode::Ok, Some((bytes, mime::TEXT_PLAIN)));

    (state, res)
}

fn ping(state: State) -> (State, Response) {
    let bytes = "pong".to_string().into_bytes();
    let res = create_response(&state, StatusCode::Ok, Some((bytes, mime::TEXT_PLAIN)));

    (state, res)
}

fn error(_state: State) -> (State, Response) {
    panic!("/error is requested");
}

fn error_wait(_state: State) -> Box<HandlerFuture> {
    let at = Instant::now() + Duration::from_millis(1000);
    let f = Delay::new(at)
        .map_err(|_| panic!("Timer error"))
        .map(|_| panic!("/error_wait is requested"));
    Box::new(f)
}

fn main() {
    honeybadger::install_hook();
    honeybadger_gotham::install();
    let addr = "127.0.0.1:7878";
    gotham::start(addr, router())
}
