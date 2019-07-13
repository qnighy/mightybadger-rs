use actix_web;

use honeybadger;


#[macro_use]
extern crate failure;

use actix_web::error::ResponseError;
use actix_web::{web, App, HttpServer, Responder};
use failure::Backtrace;
use honeybadger_actix_web::HoneybadgerMiddleware;

fn index(_: web::Path<()>) -> impl Responder {
    "Hello, world!"
}

fn ping(_: web::Path<()>) -> impl Responder {
    "pong"
}

#[derive(Debug, Fail)]
#[fail(display = "MyError")]
struct MyError(#[cause] std::io::Error, Backtrace);

impl ResponseError for MyError {}

fn error(_: web::Path<()>) -> actix_web::error::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open("quux.quux").map_err(|e| MyError(e, Backtrace::new()))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .map_err(|e| MyError(e, Backtrace::new()))?;
    Ok(contents)
}

fn error_panic(_: web::Path<()>) -> &'static str {
    panic!("/error_panic is requested");
}

fn main() {
    honeybadger::setup();

    HttpServer::new(|| {
        App::new()
            .wrap(HoneybadgerMiddleware::new())
            .route("/", web::get().to(index))
            .route("/ping", web::get().to(ping))
            .route("/error", web::get().to(error))
            .route("/error_panic", web::get().to(error_panic))
    })
    .bind("localhost:7878")
    .expect("bind failed")
    .run()
    .unwrap();
}
