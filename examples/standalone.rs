extern crate honeybadger;

fn main() {
    honeybadger::setup();

    panic!("test panic");
}
