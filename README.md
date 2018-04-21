# Honeybadger Notifier for Rust

## Specifying API key

Currently, it only supports API key via the `HONEYBADGER_API_KEY` environment variable.

## With Rocket

```toml
[dependencies]
honeybadger = "0.1.0"
honeybadger-rocket = "0.1.0"
```

```rust
extern crate honeybadger;
extern crate honeybadger_rocket;

...

fn main() {
    honeybadger::install_hook();
    honeybadger_rocket::install();
    rocket::ignite()
        ...
        .attach(honeybadger_rocket::HoneybadgerHook::new())
        .launch();
}
```

## With Gotham

```toml
[dependencies]
honeybadger = "0.1.0"
honeybadger-gotham = "0.1.0"
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
    honeybadger::install_hook();
    honeybadger_gotham::install();
    gotham::start(..., router())
}
```

## Status

honeybadger-rs is in its early stage. All APIs are subject to change. You may want to specify the exact version, like `=0.1.0`.

## License

MIT License
