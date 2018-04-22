//! Honeybadger notifier for Rust.

extern crate chrono;
#[macro_use]
extern crate lazy_static;
extern crate rand;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate hyper;
extern crate reqwest;

extern crate backtrace;
extern crate rustc_version_runtime;

pub mod payload;
pub mod plugin;

use backtrace::Backtrace;
use reqwest::header::{qitem, Accept, ContentType, UserAgent};
use reqwest::{mime, StatusCode};
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;
use std::panic::{set_hook, take_hook, PanicInfo};
use rand::Rng;
use HoneybadgerError::*;
use payload::*;

pub use payload::Payload;
pub use plugin::add_plugin;

/// Error occurred during Honeybadger reporting.
#[derive(Debug)]
pub enum HoneybadgerError {
    ReportUnable(String),
    ReportFailed(String),
}

header! {
    (XApiKey, "X-API-Key") => [String]
}

pub fn report(payload: &Payload) -> Result<(), HoneybadgerError> {
    let api_key = payload.api_key.clone();
    let payload = serde_json::to_string(payload)
        .map_err(|e| ReportFailed(format!("could not assemble payload: {}", e)))?;
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
        .header(XApiKey(api_key))
        .header(ContentType(mime::APPLICATION_JSON))
        .header(Accept(vec![qitem(mime::APPLICATION_JSON)]))
        .header(UserAgent::new(client_version))
        .send();
    let resp = resp.map_err(|e| ReportFailed(format!("HTTP request failed: {}", e)))?;
    match resp.status() {
        StatusCode::TooManyRequests | StatusCode::ServiceUnavailable => {
            return Err(ReportFailed(format!("project is sending too many errors.")))
        }
        StatusCode::PaymentRequired => return Err(ReportFailed(format!("payment is required."))),
        StatusCode::Forbidden => return Err(ReportFailed(format!("API key is invalid."))),
        StatusCode::Created => {}
        _ => return Err(ReportFailed(format!("unknown response from server."))),
    }
    Ok(())
}

fn honeybadger_panic_hook(panic_info: &PanicInfo) {
    let id = random_uuid();
    let iddisp = id.as_ref().map(|x| x.as_str()).unwrap_or("nil");
    match honeybadger_panic_hook_internal(panic_info, &id) {
        Err(ReportUnable(msg)) => {
            eprintln!(
                "** [Honeybadger] Unable to send error report: {}, id={}",
                msg, iddisp
            );
        }
        Err(ReportFailed(msg)) => {
            eprintln!(
                "** [Honeybadger] Error report failed: {}, id={}",
                msg, iddisp
            );
        }
        Ok(()) => {
            eprintln!(
                "** [Honeybadger] Success ⚡ https://app.honeybadger.io/notice/{} id={}",
                iddisp, iddisp
            );
        }
    }
}

fn honeybadger_panic_hook_internal(
    panic_info: &PanicInfo,
    id: &Option<String>,
) -> Result<(), HoneybadgerError> {
    let api_key = env::var("HONEYBADGER_API_KEY").map_err(|e| match e {
        env::VarError::NotPresent => ReportUnable(format!("API key is missing")),
        env::VarError::NotUnicode(_) => {
            ReportUnable(format!("API key is an invalid Unicode string"))
        }
    })?;
    let message = if let Some(message) = panic_info.payload().downcast_ref::<String>() {
        message.to_string()
    } else if let Some(message) = panic_info.payload().downcast_ref::<&'static str>() {
        message.to_string()
    } else {
        "Box<Any>".to_string()
    };
    let mut backtrace = Backtrace::new()
        .frames()
        .iter()
        .filter_map(|frame| {
            let symbol = if let Some(symbol) = frame.symbols().first() {
                symbol
            } else {
                return None;
            };
            let number = if let Some(lineno) = symbol.lineno() {
                lineno.to_string()
            } else {
                "".to_string()
            };
            let file = symbol
                .filename()
                .and_then(|filename| filename.to_str())
                .unwrap_or("");
            let method = symbol
                .name()
                .map(|name| name.to_string())
                .unwrap_or_else(|| "".to_string());
            Some(BacktraceEntry {
                number: number,
                file: file.to_string(),
                method: method,
                source: BTreeMap::new(),
            })
        })
        .collect::<Vec<_>>();
    let backtrace_trim_index = backtrace
        .iter()
        .position(|bt| bt.method.starts_with("std::panicking::begin_panic"))
        .map(|x| x.saturating_add(1))
        .unwrap_or(0);
    backtrace.drain(..backtrace_trim_index);
    for entry in &mut backtrace {
        let number = if let Ok(number) = entry.number.parse::<u32>() {
            number
        } else {
            continue;
        };
        let number = number.saturating_sub(1);
        let skip = number.saturating_sub(2);
        let upto = number.saturating_add(3);

        let file = if let Ok(file) = File::open(&entry.file) {
            file
        } else {
            continue;
        };
        let mut file = BufReader::new(file);
        let mut line = String::new();
        for lineno in 0..upto {
            line.clear();
            if let Ok(num_read) = file.read_line(&mut line) {
                if num_read == 0 {
                    break;
                }
            } else {
                break;
            }
            if lineno >= skip {
                let lineno = lineno.saturating_add(1);
                let line = mem::replace(&mut line, String::new());
                entry.source.insert(lineno, line);
            }
        }
    }
    let notifier_info = Some(NotifierInfo {
        name: "honeybadger-rust",
        url: "https://github.com/qnighy/honeybadger-rs",
        version: env!("CARGO_PKG_VERSION"),
        language: "rust",
    });
    let error_info = ErrorInfo {
        token: id.clone(),
        class: "std::panic".to_string(),
        message: message,
        tags: vec![],
        fingerprint: "".to_string(),
        backtrace: backtrace,
        causes: vec![],
    };
    let server_info = ServerInfo::generate();
    let mut payload = Payload {
        api_key: api_key,
        notifier: notifier_info,
        error: error_info,
        request: None,
        server: server_info,
    };
    match plugin::decorate_with_plugins(&mut payload) {
        Err(plugin::PluginError::Other(msg)) => {
            eprintln!("** [Honeybadger] Plugin error: {}", msg);
        }
        Ok(()) => {}
    }
    report(&payload)?;
    Ok(())
}

pub fn install_hook() {
    use std::sync::{Once, ONCE_INIT};

    static INSTALL_ONCE: Once = ONCE_INIT;

    INSTALL_ONCE.call_once(|| {
        let old_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            old_hook(panic_info);
            honeybadger_panic_hook(panic_info);
        }));
    });
}

fn random_uuid() -> Option<String> {
    let mut rng = if let Ok(rng) = rand::os::OsRng::new() {
        rng
    } else {
        return None;
    };
    let dw0 = rng.next_u64();
    let dw1 = rng.next_u64();
    Some(format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        dw0 >> 32,
        (dw0 >> 16) & 0xFFFF,
        (dw0 & 0x0FFF) | 0x4000,
        ((dw1 >> 48) & 0x3FFF) | 0x8000,
        dw1 & 0xFFFFFFFFFFFF
    ))
}
