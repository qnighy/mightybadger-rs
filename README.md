# Honeybadger Notifier for Rust

[Honeybadger](https://www.honeybadger.io/) is an error-tracking service run by Honeybadger Industries LLC.

`honeybadger-rs` is an unofficial Honeybadger notifier for Rust, which hooks into panics and error responses, collects related information, and sends reports to the Honeybadger API server.

In addition to standalone configuration, it provides middlewares for [Rocket](https://rocket.rs/), [Gotham](https://gotham.rs/), and [Actix Web](https://actix.rs/).

## Standalone

```toml
[dependencies]
honeybadger = { git = "https://github.com/qnighy/honeybadger-rs.git", rev = "da98547" }
```

```rust
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
```

```
HONEYBADGER_API_KEY=your_own_api_key cargo run
```

## With Rocket

```toml
[dependencies]
honeybadger = { git = "https://github.com/qnighy/honeybadger-rs.git", rev = "da98547" }
honeybadger-rocket = { git = "https://github.com/qnighy/honeybadger-rs.git", rev = "da98547" }
```

```rust
extern crate honeybadger;
extern crate honeybadger_rocket;

...

fn main() {
    honeybadger::setup();
    rocket::ignite()
        ...
        .attach(honeybadger_rocket::HoneybadgerHook::new())
        .launch();
}
```

## With Gotham

```toml
[dependencies]
honeybadger = { git = "https://github.com/qnighy/honeybadger-rs.git", rev = "da98547" }
honeybadger-gotham = { git = "https://github.com/qnighy/honeybadger-rs.git", rev = "da98547" }
```

```rust
extern crate honeybadger;
extern crate honeybadger_gotham;

...

fn router() -> Router {
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(honeybadger_gotham::HoneybadgerMiddleware)
            .build(),
    );
    build_router(chain, pipelines, |route| { ... })
}

...

fn main() {
    honeybadger::setup();
    gotham::start(..., router())
}
```


## With Actix Web

```toml
[dependencies]
honeybadger = { git = "https://github.com/qnighy/honeybadger-rs.git", rev = "da98547" }
honeybadger-actix-web = { git = "https://github.com/qnighy/honeybadger-rs.git", rev = "da98547" }
```

```rust
extern crate honeybadger;
extern crate honeybadger_actix_web;

...

fn main() {
    honeybadger::setup();

    server::new(|| {
        App::new()
            .middleware(honeybadger_actiX_web::HoneybadgerMiddleware::new())
            ..
    }).bind(..)
        .unwrap()
        .run();
}
```

## Configuration

It automatically reads the following environment variables at `honeybadger::setup()`:

- `HONEYBADGER_API_KEY`
- `HONEYBADGER_ENV`
- `HONEYBADGER_REPORT_DATA`
- `HONEYBADGER_ROOT`
- `HONEYBADGER_REVISION`
- `HONEYBADGER_HOSTNAME`

Moreover, you can programmatically configure Honeybadger as follows:

```rust
fn main() {
    honeybadger::setup();
    honeybadger::configure(|config| {
        if config.api_key.is_none() {
            config.api_key = Some("abcd1234".to_string());
        }
        config.env = Some("production".to_string());
        config.report_data = Some(true);
        config.root = Some("/home/ubuntu/app".to_string());
        config.revision = Some("0123456789abcdef0123456789abcdef01234567".to_string());
        config.hostname = Some("api.example.com".to_string());
        config.request.filter_keys = Some(vec![
            "password".to_string(),
            "HTTP_AUTHORIZATION".to_string(),
            "passcode".to_string(),
        ]);
    });
}
```

## Development Status

**Note**: it's still in its early stage and the Rust API is subject to change. I strongly recommend you to insert `rev = ".."` attribute in the dependencies to prevent breakage.

- [x] Assemble notification payload
  - [x] Notifier information
  - [x] Error messages
  - [x] Backtraces
  - [x] Error classes
    - [ ] Custom error classes
  - [x] Error chain
  - [ ] Server information from global configuration
  - [x] Stats from `/proc`
- [x] Send the payload to the Honeybadger API server
- [x] Panic hook
- [x] Notify custom errors with [failure](https://github.com/rust-lang-nursery/failure)
- [x] Pluggable RequestInfo injection
  - [ ] Built-in support for futures/tokio
- [ ] Context injection
- Framework supports
  - [x] Rocket: RequestInfo injection
    - [x] CGI Data
    - [ ] URL
    - [ ] Query Params
    - [ ] Rails-like component
    - [ ] Rails-like action
    - [ ] Session
  - [ ] Rocket: error response hook
  - [x] Gotham: RequestInfo injection
    - [x] CGI Data
    - [ ] URL
    - [ ] Query Params
    - [ ] Rails-like component
    - [ ] Rails-like action
    - [ ] Session
  - [ ] Gotham: error response hook
  - [x] Actix Web: RequestInfo injection (error response only)
    - [x] CGI Data
    - [x] URL (except for hostname)
    - [x] Query Params
    - [ ] Rails-like component
    - [ ] Rails-like action
    - [ ] Session
  - [x] Actix Web: error response hook
  - [ ] Iron
  - [ ] Nickel
  - [ ] Rouille
- [x] Password filtering
- [x] Global configuration via environment variables
- [ ] Global configuration via YAML
- [x] Global configuration via Rust functions
- [x] Travis
- [ ] Docs
- [ ] Rust API stabilization

## License

MIT License
