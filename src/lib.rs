//! Honeybadger notifier for Rust.

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate hyper;
extern crate reqwest;

extern crate backtrace;
extern crate rustc_version_runtime;

#[cfg(feature = "rocket_hook")]
extern crate rocket;

#[cfg(feature = "rocket_hook")]
pub mod rocket_hook;

use backtrace::Backtrace;
use reqwest::header::{qitem, Accept, ContentType, UserAgent};
use reqwest::{mime, StatusCode};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;
use std::panic::{set_hook, take_hook, PanicInfo};
use std::sync::RwLock;
use HoneybadgerError::*;

/// Error occurred during Honeybadger reporting.
#[derive(Debug)]
pub enum HoneybadgerError {
    ReportUnable(String),
    ReportFailed(String),
}

/// Notification payload.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HoneybadgerPayload {
    notifier: Option<NotifierInfo>,
    error: Error,
    request: Option<RequestInfo>,
    server: Server,
}

/// Information of the app that caused the error.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct NotifierInfo {
    /// name of the app (e.g. `My Awesome To-Do App`)
    pub name: String,
    /// URL pointing to the app (e.g. `https://github.com/github/my-awesome-to-do-app`)
    pub url: String,
    /// App version (e.g. `1.0.0`)
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Error {
    class: String,
    message: String,
    tags: Vec<String>,
    fingerprint: String,
    backtrace: Vec<BacktraceEntry>,
    causes: Vec<ErrorCause>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BacktraceEntry {
    number: String,
    file: String,
    method: String,
    source: BTreeMap<u32, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorCause {
    class: String,
    message: String,
    backtrace: Vec<BacktraceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestInfo {
    url: String,
    cgi_data: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Server {}

header! {
    (XApiKey, "X-API-Key") => [String]
}

#[macro_export]
macro_rules! initialize_honeybadger {
    () => {{
        $crate::set_notifier_info($crate::NotifierInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            url: env!("CARGO_PKG_HOMEPAGE").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        });
        $crate::install_hook();
    }};
}

lazy_static! {
    static ref GLOBAL_NOTIFIER_INFO: RwLock<Option<NotifierInfo>> = RwLock::new(None);
}

/// Sets the notifier info. Typically used through `initialize_honeybadger!`.
pub fn set_notifier_info(notifier_info: NotifierInfo) {
    let mut global_notifier_info = GLOBAL_NOTIFIER_INFO.write().unwrap();
    *global_notifier_info = Some(notifier_info);
}

pub fn report(payload: &HoneybadgerPayload) -> Result<(), HoneybadgerError> {
    let api_key = env::var("HONEYBADGER_API_KEY").map_err(|e| match e {
        env::VarError::NotPresent => ReportUnable(format!("API key is missing")),
        env::VarError::NotUnicode(_) => {
            ReportUnable(format!("API key is an invalid Unicode string"))
        }
    })?;
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
            let method = symbol.name().and_then(|name| name.as_str()).unwrap_or("");
            Some(BacktraceEntry {
                number: number,
                file: file.to_string(),
                method: method.to_string(),
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
    let notifier_info = if let Ok(global_notifier_info) = GLOBAL_NOTIFIER_INFO.read() {
        global_notifier_info.clone()
    } else {
        None
    };
    let error = Error {
        class: "std::panic".to_string(),
        message: message,
        tags: vec![],
        fingerprint: "".to_string(),
        backtrace: backtrace,
        causes: vec![],
    };
    #[cfg(not(feature = "rocket_hook"))]
    let request_info = None;
    #[cfg(feature = "rocket_hook")]
    let request_info = rocket_hook::try_get();
    let server = Server {};
    let payload = HoneybadgerPayload {
        notifier: notifier_info,
        error: error,
        request: request_info,
        server: server,
    };
    match report(&payload) {
        Err(ReportUnable(msg)) => {
            eprintln!("** [Honeybadger] Unable to send error report: {}", msg);
        }
        Err(ReportFailed(msg)) => {
            eprintln!("** [Honeybadger] Error report failed: {}", msg);
        }
        Ok(()) => {
            eprintln!("** [Honeybadger] Success ⚡");
        }
    }
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
