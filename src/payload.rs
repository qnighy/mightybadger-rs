use chrono::Utc;
use serde_json;
use std::collections::{BTreeMap, HashMap};

/// Notification payload.
#[derive(Debug, Serialize, Default)]
pub struct Payload {
    pub api_key: String,
    pub notifier: Option<NotifierInfo>,
    pub error: ErrorInfo,
    pub request: Option<RequestInfo>,
    pub server: ServerInfo,
}

/// Information of the app that caused the error.
#[derive(Debug, Serialize, Clone)]
pub struct NotifierInfo {
    pub name: &'static str,
    pub url: &'static str,
    pub version: &'static str,
    pub language: &'static str,
}

#[derive(Debug, Serialize, Default)]
pub struct ErrorInfo {
    pub token: Option<String>,
    pub class: String,
    pub message: String,
    pub tags: Vec<String>,
    pub fingerprint: String,
    pub backtrace: Vec<BacktraceEntry>,
    pub causes: Vec<ErrorCause>,
}

#[derive(Debug, Serialize)]
pub struct BacktraceEntry {
    pub number: String,
    pub file: String,
    pub method: String,
    pub source: BTreeMap<u32, String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorCause {
    pub class: String,
    pub message: String,
    pub backtrace: Vec<BacktraceEntry>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RequestInfo {
    pub url: String,
    pub cgi_data: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub component: String,
    pub action: String,
    pub session: HashMap<String, String>,
    pub context: HashMap<String, serde_json::Value>,
    pub local_variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Default)]
pub struct ServerInfo {
    pub project_root: String,
    pub revision: String,
    pub environment_name: String,
    pub hostname: String,
    pub stats: Stats,
    pub time: String,
    pub pid: u32,
}

impl ServerInfo {
    pub fn generate() -> Self {
        let time = Utc::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();
        let pid = 0; // TODO: wait for stabilization of std::process::id();
        ServerInfo {
            time: time,
            pid: pid,
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct Stats {
    pub mem: MemoryInfo,
    pub load: LoadInfo,
}

#[derive(Debug, Serialize, Default)]
pub struct MemoryInfo {
    pub total: f64,
    pub free: f64,
    pub buffers: f64,
    pub cached: f64,
    pub free_total: f64,
}

#[derive(Debug, Serialize, Default)]
pub struct LoadInfo {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}