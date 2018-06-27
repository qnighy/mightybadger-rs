use std::env;
use std::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ConfigInner {
    pub(crate) api_key: Option<String>,
    pub(crate) env: Option<String>,
    pub(crate) report_data: Option<bool>,
    pub(crate) root: Option<String>,
    pub(crate) revision: Option<String>,
    pub(crate) hostname: Option<String>,
}

#[derive(Debug)]
pub struct Config {
    inner: &'static RwLock<ConfigInner>,
}

macro_rules! impl_config_writer {
    ($(($field:ident : $ty:ty, $setter:ident, $opt_setter:ident),)*) => {
        impl Config {
            $(
            pub fn $field(&self) -> Option<$ty> {
                self.inner.read().unwrap().$field.clone()
            }
            pub fn $setter(&mut self, val: Option<$ty>) {
                self.inner.write().unwrap().$field = val;
            }
            pub fn $opt_setter(&mut self, val: Option<$ty>) {
                let mut guard = self.inner.write().unwrap();
                if guard.$field.is_none() {
                    guard.$field = val;
                }
            }
            )*
        }
    }
}

impl_config_writer! {
    (api_key: String, set_api_key, opt_set_api_key),
    (env: String, set_env, opt_set_env),
    (report_data: bool, set_report_data, opt_set_report_data),
    (root: String, set_root, opt_set_root),
    (revision: String, set_revision, opt_set_revision),
    (hostname: String, set_hostname, opt_set_hostname),
}

lazy_static! {
    static ref CONFIG: RwLock<ConfigInner> = RwLock::new(ConfigInner::default());
}

pub fn configure_from_env() {
    fn set_string(entry: &mut Option<String>, env_name: &str) {
        if entry.is_none() {
            *entry = env::var_os(env_name).map(|s| s.to_string_lossy().to_string());
        }
    }

    fn set_bool(entry: &mut Option<bool>, env_name: &str) {
        if entry.is_none() {
            *entry = env::var_os(env_name).map(|s| {
                let s = s.to_string_lossy().to_string();
                ["true", "t", "1"].iter().any(|t| s.eq_ignore_ascii_case(t))
            });
        }
    }

    configure_locked(|config| {
        set_string(&mut config.api_key, "HONEYBADGER_API_KEY");
        set_string(&mut config.env, "HONEYBADGER_ENV");
        set_bool(&mut config.report_data, "HONEYBADGER_REPORT_DATA");
        set_string(&mut config.root, "HONEYBADGER_ROOT");
        set_string(&mut config.revision, "HONEYBADGER_REVISION");
        set_string(&mut config.hostname, "HONEYBADGER_HOSTNAME");
    })
}

pub(crate) fn configure_locked<F>(f: F)
where
    F: FnOnce(&mut ConfigInner),
{
    let mut config = CONFIG.write().unwrap();
    f(&mut config);
}

pub fn configure<F>(f: F)
where
    F: FnOnce(&mut Config),
{
    let mut config = Config { inner: &CONFIG };
    f(&mut config);
}

pub(crate) fn read_config() -> RwLockReadGuard<'static, ConfigInner> {
    CONFIG.read().unwrap()
}
