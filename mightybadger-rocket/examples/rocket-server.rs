#![feature(proc_macro_hygiene, decl_macro)]

use rocket::{get, routes};

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
    mightybadger::setup();
    rocket::ignite()
        .mount("/", routes![index, ping, error])
        .attach(mightybadger_rocket::HoneybadgerHook::new())
        .launch();
}
