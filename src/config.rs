//! Honeybadger configuration.
//!
//! This module defines [`Config`][Config] and related functions.
//!
//! [Config]: struct.Config.html
//!
//! Basically you will need [`configure`][configure] for modifying the configuration
//! and [`read_config`][read_config] for reading the configuration.
//!
//! [configure]: fn.configure.html
//! [read_config]: fn.read_config.html

use std::env;
use std::mem;
use std::ops::Deref;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::str::FromStr;
use std::sync::{RwLock, RwLockReadGuard};

use lazy_static::lazy_static;

/// Honeybadger configuration.
///
/// It roughly corresponds with [the Ruby notifier configuration][ruby-config].
///
/// [ruby-config]: https://docs.honeybadger.io/ruby/gem-reference/configuration.html
///
/// To **modify** the global configuration, use [`configure`][configure].
/// To **inspect** the global configuration, use [`read_config`][read_config].
///
/// [configure]: fn.configure.html
/// [read_config]: fn.read_config.html
///
/// ## Examples
///
/// ```
/// mightybadger::configure(|config| {
///     config.api_key = Some("abcd1234".to_string());
///     config.env = Some("production".to_string());
/// });
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    /// The API key for your Honeybadger project.
    pub api_key: Option<String>,
    /// The environment the app is running in e.g. `"development"` and `"production"`.
    pub env: Option<String>,
    /// Enable/disable reporting of data.
    /// Defaults to `false` for `"test"`, `"development"`, and `"cucumber"` environments.
    pub report_data: Option<bool>,
    /// The project's absolute root path.
    pub root: Option<String>,
    /// The project's git revision.
    pub revision: Option<String>,
    /// The hostname of the current box.
    pub hostname: Option<String>,
    /// HTTP connection options.
    pub connection: ConnectionConfig,
    /// Request data filtering options.
    pub request: RequestConfig,
    #[doc(hidden)]
    pub _non_exhaustive: (),
}

/// HTTP connection options.
///
/// This is part of [`Config`][Config] data structure.
///
/// [Config]: struct.Config.html
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectionConfig {
    /// Whether to use TLS when sending data.
    /// Defaults to `true`.
    pub secure: Option<bool>,
    /// The host to use when sending data.
    /// Defaults to `api.honeybadger.io`.
    pub host: Option<String>,
    /// The port to use when sending data.
    /// Defaults to 443.
    pub port: Option<u16>,
    #[doc(hidden)]
    pub _non_exhaustive: (),
}

/// Request data filtering options.
///
/// This is part of [`Config`][Config] data structure.
///
/// [Config]: struct.Config.html
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestConfig {
    /// A list of keys to filter when sending request data.
    /// Defaults to `["password", "HTTP_AUTHORIZATION"]`.
    pub filter_keys: Option<Vec<String>>,
    #[doc(hidden)]
    pub _non_exhaustive: (),
}

impl RequestConfig {
    /// Returns `true` if the key likely contains secrets and
    /// should be filtered out before sending reports.
    pub(crate) fn filter_key(&self, key: &str) -> bool {
        if let Some(ref filter_keys) = self.filter_keys {
            filter_keys.iter().any(|s| key.contains(s))
        } else {
            ["password", "HTTP_AUTHORIZATION"]
                .iter()
                .any(|s| key.contains(s))
        }
    }
}

lazy_static! {
    /// Global Honeybadger configuration.
    static ref CONFIG: RwLock<Config> = RwLock::new(Config::default());
    /// The copy of the global configuration. Used by `configure`.
    static ref CONFIG_PROXY: RwLock<Config> = RwLock::new(Config::default());
}

/// Reads configuration from the `HONEYBADGER_*` environment variables.
///
/// Replaces the config only if the field is `None`.
///
/// It is called as a part of [`mightybadger::setup`][::setup].
///
/// [::setup]: ../fn.setup.html
pub fn configure_from_env() {
    fn set_string(entry: &mut Option<String>, env_name: &str) {
        if entry.is_none() {
            *entry = env::var_os(env_name).map(|s| s.to_string_lossy().to_string());
        }
    }

    fn set_parseable<T: FromStr>(entry: &mut Option<T>, env_name: &str) {
        if entry.is_none() {
            *entry =
                env::var_os(env_name).and_then(|s| s.to_string_lossy().to_string().parse().ok());
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

    fn set_string_array(entry: &mut Option<Vec<String>>, env_name: &str) {
        if entry.is_none() {
            *entry = env::var_os(env_name).map(|s| {
                let s = s.to_string_lossy().to_string();
                s.split(",")
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>()
            });
        }
    }

    configure(|config| {
        set_string(&mut config.api_key, "HONEYBADGER_API_KEY");
        set_string(&mut config.env, "HONEYBADGER_ENV");
        set_bool(&mut config.report_data, "HONEYBADGER_REPORT_DATA");
        set_string(&mut config.root, "HONEYBADGER_ROOT");
        set_string(&mut config.revision, "HONEYBADGER_REVISION");
        set_string(&mut config.hostname, "HONEYBADGER_HOSTNAME");
        set_bool(
            &mut config.connection.secure,
            "HONEYBADGER_CONNECTION_SECURE",
        );
        set_string(&mut config.connection.host, "HONEYBADGER_CONNECTION_HOST");
        set_parseable(&mut config.connection.port, "HONEYBADGER_CONNECTION_PORT");
        set_string_array(
            &mut config.request.filter_keys,
            "HONEYBADGER_REQUEST_FILTER_KEYS",
        );
    })
}

/// Modifies Honeybadger configuration.
///
/// ## Example
///
/// ```
/// mightybadger::configure(|config| {
///     config.env = Some("staging".to_string());
/// });
/// ```
///
/// ## Panics
///
/// It may (but not necessarily) panic if:
///
/// - the thread tries a nested call to `configure`, or
/// - the thread tries to finish `configure` while holding a lock acquired by `read_config`.
///
/// In addition to those, a panic from the callback is also propagated.
///
/// ## Notes on multithreading
///
/// To make extra guarantee about panic-safety, it does more than a simple `RwLock`.
/// Therefore you may observe a different behavior.
///
/// `configure` does the following:
///
/// 1. Acquires write-lock for `CONFIG_PROXY`, which is **the copy of** the configuration.
/// 2. Calls back the given closure.
/// 3. Acquires write-lock for `CONFIG`, which is the actual configuration.
/// 4. Copies `CONFIG_PROXY` into `CONFIG`.
/// 5. If a panic occurs during 2-4, then rolls back `CONFIG_PROXY`, and resumes panicking.
///
/// Therefore [`read_config`][read_config] always succeeds, even in `configure` itself.
///
/// [read_config]: fn.read_config.html
pub fn configure<F>(f: F)
where
    F: FnOnce(&mut Config),
{
    let mut config_proxy = CONFIG_PROXY
        .write()
        .expect("Could not acquire write-lock for mightybadger::config::CONFIG_PROXY.");
    let result = {
        let f = AssertUnwindSafe(f);
        let config_proxy = AssertUnwindSafe(&mut config_proxy as &mut Config);
        catch_unwind(move || {
            (f.0)(config_proxy.0);
            replace_config(config_proxy.clone());
        })
    };
    if let Err(e) = result {
        let config = read_config();
        config_proxy.clone_from(&config);
        mem::drop(config_proxy);
        resume_unwind(e);
    }
}

/// The part of `configure` that actually touches `CONFIG`.
///
/// Since we only do `mem::replace` after lock acquisition (even without dropping),
/// it is guaranteed not to poison `CONFIG`.
fn replace_config(new_config: Config) -> Config {
    let mut config = CONFIG
        .write()
        .expect("Could not acquire write-lock for mightybadger::config::CONFIG.");
    mem::replace(&mut config, new_config)
}

/// Read-lock to the global configuration.
///
/// Returned by [`read_config`][read_config].
///
/// [read_config]: fn.read_config.html
#[derive(Debug)]
pub struct ConfigReadGuard(RwLockReadGuard<'static, Config>);

impl Deref for ConfigReadGuard {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Acquires a read-only lock for the global configuration. This is panic-safe.
///
/// The acquired lock blocks the end of [`configure`][configure].
///
/// [configure]: fn.configure.html
///
/// ## Example
///
/// ```
/// let config = mightybadger::config::read_config();
/// println!("config.env = {:?}", config.env);
/// ```
pub fn read_config() -> ConfigReadGuard {
    ConfigReadGuard(
        CONFIG
            .read()
            .expect("Could not acquire read-lock for mightybadger::config::CONFIG"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    lazy_static! {
        static ref CONFIG_TEST_GUARD: Mutex<()> = Mutex::new(());
    }

    fn reset() -> MutexGuard<'static, ()> {
        let guard = match CONFIG_TEST_GUARD.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        configure(|config| {
            *config = Default::default();
        });
        guard
    }

    #[test]
    fn test_read_config() {
        let _guard = reset();
        let config = read_config();
        assert_eq!(config.env, None);
    }

    #[test]
    fn test_configure() {
        let _guard = reset();
        configure(|config| {
            assert_eq!(config.env, None);
            config.env = Some("foo".to_string());
        });
        configure(|config| {
            assert_eq!(config.env, Some("foo".to_string()));
        });
        let config = read_config();
        assert_eq!(config.env, Some("foo".to_string()));
    }

    #[test]
    fn test_configure_panic_recovery() {
        let _guard = reset();
        let r = catch_unwind(|| {
            configure(|config| {
                config.env = Some("foo".to_string());
                panic!();
            });
        });
        assert!(r.is_err());
        configure(|config| {
            assert_eq!(config.env, None);
        });
    }

    #[test]
    fn test_read_config_in_configure() {
        let _guard = reset();
        let config2 = read_config();
        configure(move |config| {
            config.env = Some("foo".to_string());
            let config3 = read_config();
            assert_eq!(config2.env, None);
            assert_eq!(config3.env, None);
        });
    }
}
