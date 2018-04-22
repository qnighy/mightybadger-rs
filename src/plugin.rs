use std::sync::RwLock;
use Payload;

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

pub(crate) fn decorate_with_plugins(payload: &mut Payload) -> Result<(), PluginError> {
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
    fn decorate(&self, payload: &mut Payload) -> Result<bool, PluginError>;
}
