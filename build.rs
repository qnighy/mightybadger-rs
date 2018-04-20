use std::env;

fn main() {
    println!(
        "cargo:rustc-env=HONEYBADGER_CLIENT_ARCH={}",
        env::var("TARGET").unwrap()
    );
}
