use std::collections::{BTreeMap, HashMap};
use std::process;

use chrono::Utc;
use serde_json;
use uuid::Uuid;

use config;
use stats;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    pub stats: Stats,
    pub time: String,
    pub pid: u32,
}

impl ServerInfo {
    pub fn generate() -> Self {
        let config = config::read_config();
        let time = Utc::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();
        let pid = process::id();
        let stats = Stats::generate();
        ServerInfo {
            project_root: config.root.clone(),
            revision: config.revision.clone(),
            environment_name: config.env.clone(),
            hostname: config.hostname.clone(),
            time: time,
            pid: pid,
            stats: stats,
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct Stats {
    pub mem: Option<MemoryInfo>,
    pub load: Option<LoadInfo>,
}

impl Stats {
    pub fn generate() -> Self {
        stats::get_stats()
    }
}

#[derive(Debug, Serialize, Default)]
pub struct MemoryInfo {
    pub total: Option<f64>,
    pub free: Option<f64>,
    pub buffers: Option<f64>,
    pub cached: Option<f64>,
    pub free_total: Option<f64>,
}

#[derive(Debug, Serialize, Default)]
pub struct LoadInfo {
    pub one: Option<f64>,
    pub five: Option<f64>,
    pub fifteen: Option<f64>,
}
