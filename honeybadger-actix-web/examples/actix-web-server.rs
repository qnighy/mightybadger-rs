extern crate actix_web;

extern crate honeybadger;
extern crate honeybadger_actix_web;

#[macro_use]
extern crate failure;

use actix_web::error::ResponseError;
use actix_web::http::Method;
use actix_web::{server, App, Path, Responder};
use failure::Backtrace;
use honeybadger_actix_web::HoneybadgerMiddleware;

fn index(_: Path<()>) -> impl Responder {
    "Hello, world!"
}

fn ping(_: Path<()>) -> impl Responder {
    "pong"
}

#[derive(Debug, Fail)]
#[fail(display = "MyError")]
struct MyError(#[cause] std::io::Error, Backtrace);

impl ResponseError for MyError {}

fn error(_: Path<()>) -> actix_web::error::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open("quux.quux").map_err(|e| MyError(e, Backtrace::new()))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .map_err(|e| MyError(e, Backtrace::new()))?;
    Ok(contents)
}

fn error_panic(_: Path<()>) -> &'static str {
    panic!("/error_panic is requested");
}

fn main() {
    honeybadger::setup();

    server::new(|| {
        App::new()
            .middleware(HoneybadgerMiddleware::new())
            .route("/", Method::GET, index)
            .route("/ping", Method::GET, ping)
            .route("/error", Method::GET, error)
            .route("/error_panic", Method::GET, error_panic)
    }).bind("localhost:7878")
    .expect("bind failed")
    .run();
}
