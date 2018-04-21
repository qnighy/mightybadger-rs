//! Honeybadger notifier for Rust.

#[macro_use]
extern crate lazy_static;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate hyper;
extern crate reqwest;

extern crate backtrace;
extern crate rustc_version_runtime;

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
#[derive(Debug, Serialize, Default)]
pub struct HoneybadgerPayload {
    notifier: Option<NotifierInfo>,
    error: Error,
    pub request: Option<RequestInfo>,
    server: Server,
}

/// Information of the app that caused the error.
#[derive(Debug, Serialize, Clone)]
struct NotifierInfo {
    name: &'static str,
    url: &'static str,
    version: &'static str,
    language: &'static str,
}

#[derive(Debug, Serialize, Default)]
struct Error {
    class: String,
    message: String,
    tags: Vec<String>,
    fingerprint: String,
    backtrace: Vec<BacktraceEntry>,
    causes: Vec<ErrorCause>,
}

#[derive(Debug, Serialize)]
struct BacktraceEntry {
    number: String,
    file: String,
    method: String,
    source: BTreeMap<u32, String>,
}

#[derive(Debug, Serialize)]
struct ErrorCause {
    class: String,
    message: String,
    backtrace: Vec<BacktraceEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestInfo {
    pub url: String,
    pub cgi_data: HashMap<String, String>,
}

#[derive(Debug, Serialize, Default)]
struct Server {}

header! {
    (XApiKey, "X-API-Key") => [String]
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
    let error = Error {
        class: "std::panic".to_string(),
        message: message,
        tags: vec![],
        fingerprint: "".to_string(),
        backtrace: backtrace,
        causes: vec![],
    };
    let server = Server {};
    let mut payload = HoneybadgerPayload {
        notifier: notifier_info,
        error: error,
        request: None,
        server: server,
    };
    match decorate_with_plugins(&mut payload) {
        Err(PluginError::Other(msg)) => {
            eprintln!("** [Honeybadger] Plugin error: {}", msg);
        }
        Ok(()) => {}
    }
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

lazy_static! {
    static ref PLUGINS: RwLock<Vec<Box<Plugin + Send + Sync>>> = RwLock::new(vec![]);
}

pub fn add_plugin<P: Plugin + Send + Sync + 'static>(plugin: P) {
    let plugin: Box<Plugin + Send + Sync> = Box::new(plugin);
    let mut plugins = PLUGINS.write().unwrap();
    plugins.push(plugin);
}

/// Error occurred during Honeybadger plugin processing.
#[derive(Debug)]
pub enum PluginError {
    Other(String),
}

fn decorate_with_plugins(payload: &mut HoneybadgerPayload) -> Result<(), PluginError> {
    let plugins = PLUGINS
        .read()
        .map_err(|e| PluginError::Other(format!("Failed to read plugins: {}", e)))?;
    for plugin in plugins.iter() {
        if plugin.decorate(payload)? {
            return Ok(());
        }
    }
    Ok(())
}

pub trait Plugin {
    fn decorate(&self, payload: &mut HoneybadgerPayload) -> Result<bool, PluginError>;
}
