#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate honeybadger;
extern crate honeybadger_rocket;
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
    honeybadger::install_hook();
    honeybadger_rocket::install();
    rocket::ignite()
        .mount("/", routes![index, ping, error])
        .attach(honeybadger_rocket::HoneybadgerHook::new())
        .launch();
}
