use std::fs::File;

fn main() {
    mightybadger::setup();

    match File::open("quux.quux") {
        Ok(_) => eprintln!("quux.quux exists."),
        Err(e) => mightybadger::notify(&e),
    };

    panic!("test panic");
}
