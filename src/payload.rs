use std::collections::{BTreeMap, HashMap};
use std::process;

use chrono::Utc;
use serde_json;
use uuid::Uuid;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<Uuid>,
    pub class: String,
    pub message: String,
    pub tags: Vec<String>,
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<Vec<BacktraceEntry>>,
    pub causes: Vec<ErrorCause>,
}

#[derive(Debug, Serialize)]
pub struct BacktraceEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<BTreeMap<u32, String>>,
}

#[derive(Debug, Serialize)]
pub struct ErrorCause {
    pub class: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtrace: Option<Vec<BacktraceEntry>>,
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
        let pid = process::id();
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
