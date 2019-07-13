#![feature(proc_macro_hygiene, decl_macro)]

use honeybadger;
use honeybadger_rocket;
#[macro_use]
extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/ping")]
fn ping() -> &'static str {
    "pong"
}

#[get("/error")]
fn error() -> &'static str {
    panic!("/error is requested");
}

fn main() {
    honeybadger::setup();
    rocket::ignite()
        .mount("/", routes![index, ping, error])
        .attach(honeybadger_rocket::HoneybadgerHook::new())
        .launch();
}
