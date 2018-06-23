extern crate honeybadger;

use std::fs::File;

fn main() {
    honeybadger::setup();

    match File::open("quux.quux") {
        Ok(_) => eprintln!("quux.quux exists."),
        Err(e) => honeybadger::notify(&e),
    };

    panic!("test panic");
}
