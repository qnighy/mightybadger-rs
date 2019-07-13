//! Honeybadger notifier for Rust.

mod btparse;
pub mod config;
pub mod context;
pub mod payload;
mod stats;

use crate::payload::*;
use crate::HoneybadgerError::*;
use failure::{Backtrace, Fail};
use rand::RngCore;
use reqwest::header::{ACCEPT, CONTENT_TYPE, USER_AGENT};
use reqwest::StatusCode;
use serde_derive::Deserialize;
use std::fmt;
use std::panic::{set_hook, take_hook, PanicInfo};
use uuid::Uuid;

pub use crate::config::configure;
pub use crate::config::configure_from_env;
pub use crate::payload::Payload;

#[derive(Debug, Fail)]
#[fail(display = "{}", message)]
pub struct Panic {
    message: String,
    backtrace: Backtrace,
}

impl Panic {
    fn new(panic_info: &PanicInfo<'_>) -> Self {
        let message = if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            message.to_string()
        } else if let Some(&message) = panic_info.payload().downcast_ref::<&'static str>() {
            message.to_string()
        } else {
            "Box<Any>".to_string()
        };
        let backtrace = Backtrace::new();
        Panic { message, backtrace }
    }
}

/// Error occurred during Honeybadger reporting.
#[derive(Debug, Fail)]
pub enum HoneybadgerError {
    #[fail(display = "Configured not to send reports")]
    NoReportData(Backtrace),
    #[fail(display = "API key is missing")]
    NoApiKey(Backtrace),
    #[fail(display = "could not assemble payload")]
    CouldNotAssemblePayload(#[cause] serde_json::Error, Backtrace),
    #[fail(display = "HTTP request failed")]
    HttpRequestFailed(#[cause] reqwest::Error, Backtrace),
    #[fail(display = "project is sending too many errors")]
    TooManyRequests(Backtrace),
    #[fail(display = "payment is required")]
    PaymentRequired(Backtrace),
    #[fail(display = "API key is invalid")]
    Forbidden(Backtrace),
    #[fail(display = "unknown response from server")]
    UnknownResponse(Backtrace),
    #[fail(display = "failed to decode response body")]
    ResponseDecodeFailed(#[cause] reqwest::Error, Backtrace),
}

#[derive(Deserialize)]
struct HoneybadgerResponse {
    id: Uuid,
}

fn report(payload: &Payload) -> Result<HoneybadgerResponse, HoneybadgerError> {
    let api_key = payload.api_key.clone();
    let payload =
        serde_json::to_string(payload).map_err(|e| CouldNotAssemblePayload(e, Backtrace::new()))?;
    // eprintln!("Payload = {}", payload);
    let client = reqwest::Client::new();
    let client_version = format!(
        "HB-Rust {}; {}; {}",
        env!("CARGO_PKG_VERSION"),
        rustc_version_runtime::version(),
        env!("HONEYBADGER_CLIENT_ARCH"),
    );
    let resp = client
        .post("https://api.honeybadger.io/v1/notices")
        .body(payload)
        .header("X-API-Key", api_key)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, client_version)
        .send();
    let mut resp = resp.map_err(|e| HttpRequestFailed(e, Backtrace::new()))?;
    match resp.status() {
        StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => {
            return Err(TooManyRequests(Backtrace::new()));
        }
        StatusCode::PAYMENT_REQUIRED => return Err(PaymentRequired(Backtrace::new())),
        StatusCode::FORBIDDEN => return Err(Forbidden(Backtrace::new())),
        StatusCode::CREATED => {}
        _ => return Err(UnknownResponse(Backtrace::new())),
    }
    resp.json()
        .map_err(|e| ResponseDecodeFailed(e, Backtrace::new()))
}

fn honeybadger_panic_hook(panic_info: &PanicInfo<'_>) {
    notify(&Panic::new(panic_info));
}

pub fn notify(error: &dyn Fail) {
    notify_either(FailOrError::Fail(error))
}

pub fn notify_std_error(error: &(dyn std::error::Error + 'static)) {
    notify_either(FailOrError::StdError(error))
}

#[derive(Debug, Clone, Copy)]
enum FailOrError<'a> {
    Fail(&'a dyn Fail),
    StdError(&'a (dyn std::error::Error + 'static)),
}

impl<'a> FailOrError<'a> {
    fn cause(self) -> Option<FailOrError<'a>> {
        match self {
            FailOrError::Fail(error) => error.cause().map(FailOrError::Fail),
            FailOrError::StdError(error) => error.source().map(FailOrError::StdError),
        }
    }
    fn backtrace(self) -> Option<&'a Backtrace> {
        if let FailOrError::Fail(error) = self {
            error.backtrace()
        } else {
            None
        }
    }
}
impl<'a> fmt::Display for FailOrError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FailOrError::Fail(error) => fmt::Display::fmt(error, f),
            FailOrError::StdError(error) => fmt::Display::fmt(error, f),
        }
    }
}

fn notify_either<'a>(error: FailOrError<'a>) {
    let id = random_uuid();
    let iddisp = id
        .as_ref()
        .map(|u| u.to_string())
        .unwrap_or_else(|| "nil".to_string());
    let resp = match notify_internal(error, &id) {
        Err(NoReportData(_)) => {
            eprintln!(
                "** [Honeybadger] Configured not to send reports, id={}",
                iddisp
            );
            return;
        }
        Err(e) => {
            eprintln!("** [Honeybadger] Error report failed: {}, id={}", e, iddisp);
            return;
        }
        Ok(resp) => resp,
    };
    let id = resp.id;
    eprintln!(
        "** [Honeybadger] Success ⚡ https://app.honeybadger.io/notice/{} id={}",
        id, id
    );
}

fn notify_internal<'a>(
    error: FailOrError<'a>,
    id: &Option<Uuid>,
) -> Result<HoneybadgerResponse, HoneybadgerError> {
    let config = config::read_config();
    let report_data = config.report_data.unwrap_or_else(|| {
        let env = config.env.as_ref().map(|s| s.as_str()).unwrap_or("");
        ["test", "development", "cucumber"]
            .iter()
            .all(|&s| env != s)
    });
    if !report_data {
        return Err(NoReportData(Backtrace::new()));
    }
    let api_key = config
        .api_key
        .clone()
        .ok_or_else(|| NoApiKey(Backtrace::new()))?;
    let backtrace = if let Some(bt) = error.backtrace() {
        btparse::parse_and_decorate(bt)
    } else {
        btparse::parse_and_decorate(&Backtrace::new())
    };
    let notifier_info = Some(NotifierInfo {
        name: "honeybadger-rust",
        url: "https://github.com/qnighy/honeybadger-rs",
        version: env!("CARGO_PKG_VERSION"),
        language: "rust",
    });
    let causes = {
        let mut causes = Vec::new();
        let mut opterror = error.cause();
        while let Some(error) = opterror {
            let backtrace = error.backtrace().map(|bt| btparse::parse_and_decorate(bt));
            causes.push(ErrorCause {
                class: error_class(error),
                message: error.to_string(),
                backtrace: backtrace,
            });
            opterror = error.cause();
        }
        causes
    };
    let error_info = ErrorInfo {
        token: id.clone(),
        class: error_class(error),
        message: error.to_string(),
        tags: vec![],
        fingerprint: "".to_string(),
        backtrace: Some(backtrace),
        causes: causes,
    };
    let server_info = ServerInfo::generate();
    let request_info = context::get();
    let mut payload = Payload {
        api_key: api_key,
        notifier: notifier_info,
        error: error_info,
        request: request_info,
        server: server_info,
    };
    payload.sanitize();
    report(&payload)
}

fn error_class<'a>(error: FailOrError<'a>) -> String {
    macro_rules! error_classes {
        ($($class:ty,)*) => {
            $(
                if let FailOrError::Fail(error) = error {
                    if Fail::downcast_ref::<$class>(error).is_some() {
                        return stringify!($class).to_string();
                    }
                    if Fail::downcast_ref::<failure::Context<$class>>(error).is_some() {
                        return stringify!(failure::Context<$class>).to_string();
                    }
                } else if let FailOrError::StdError(error) = error {
                    if std::error::Error::downcast_ref::<$class>(error).is_some() {
                        return stringify!($class).to_string();
                    }
                }
            )*
        };
    }
    macro_rules! fail_classes {
        ($($class:ty,)*) => {
            $(
                if let FailOrError::Fail(error) = error {
                    if Fail::downcast_ref::<$class>(error).is_some() {
                        return stringify!($class).to_string();
                    }
                    if Fail::downcast_ref::<failure::Context<$class>>(error).is_some() {
                        return stringify!(failure::Context<$class>).to_string();
                    }
                }
            )*
        };
    }
    error_classes!(
        // std::boxed::Box<T>,
        std::cell::BorrowError,
        std::cell::BorrowMutError,
        // std::char::CharTryFromError,
        std::char::DecodeUtf16Error,
        std::char::ParseCharError,
        std::env::JoinPathsError,
        std::env::VarError,
        std::ffi::FromBytesWithNulError,
        std::ffi::IntoStringError,
        std::ffi::NulError,
        std::fmt::Error,
        // std::io::CharsError,
        std::io::Error,
        // std::io::IntoInnerError<W>,
        std::net::AddrParseError,
        std::num::ParseFloatError,
        std::num::ParseIntError,
        // std::num::TryFromIntError,
        std::path::StripPrefixError,
        std::str::ParseBoolError,
        std::str::Utf8Error,
        std::string::FromUtf16Error,
        std::string::FromUtf8Error,
        std::string::ParseError,
        // std::sync::PoisonError<T>,
        // std::sync::TryLockError<T>,
        std::sync::mpsc::RecvError,
        std::sync::mpsc::RecvTimeoutError,
        // std::sync::mpsc::SendError<T>,
        std::sync::mpsc::TryRecvError,
        // std::sync::mpsc::TrySendError<T>,
        std::time::SystemTimeError,
    );
    fail_classes!(honeybadger::Panic,);
    // hack for stringify
    mod honeybadger {
        pub use crate::Panic;
    }
    return "Fail".to_string();
}

pub fn install_hook() {
    use std::sync::Once;

    static INSTALL_ONCE: Once = Once::new();

    INSTALL_ONCE.call_once(|| {
        let old_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            old_hook(panic_info);
            honeybadger_panic_hook(panic_info);
        }));
    });
}

pub fn enable_backtrace() {
    use std::env;

    env::set_var("RUST_FAILURE_BACKTRACE", "1");
}

pub fn setup() {
    configure_from_env();
    install_hook();
    enable_backtrace();
}

fn random_uuid() -> Option<Uuid> {
    let mut rng = rand::rngs::OsRng::new().ok()?;

    let mut bytes = [0; 16];
    rng.fill_bytes(&mut bytes);

    let uuid = uuid::Builder::from_bytes(bytes)
        .set_variant(uuid::Variant::RFC4122)
        .set_version(uuid::Version::Random)
        .build();
    Some(uuid)
}
