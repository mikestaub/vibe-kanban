
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use lazy_static::lazy_static;
use thiserror::Error;

use super::traits::{CodingAgentPlugin, PluginMetadata};

#[derive(Debug, Error)]
pub enum PluginRegistryError {
    #[error("Plugin '{0}' not found")]
    PluginNotFound(String),
    #[error("Plugin '{0}' already registered")]
    PluginAlreadyRegistered(String),
    #[error("Failed to load plugin: {0}")]
    LoadError(String),
}

pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn CodingAgentPlugin>>>,
}

impl PluginRegistry {
    fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(
        &self,
        plugin: Arc<dyn CodingAgentPlugin>,
    ) -> Result<(), PluginRegistryError> {
        let metadata = plugin.metadata();
        let mut plugins = self.plugins.write().unwrap();
        
        if plugins.contains_key(&metadata.id) {
            return Err(PluginRegistryError::PluginAlreadyRegistered(metadata.id));
        }
        
        plugins.insert(metadata.id.clone(), plugin);
        tracing::info!("Registered plugin: {} ({})", metadata.name, metadata.id);
        Ok(())
    }

    pub fn get(&self, plugin_id: &str) -> Result<Arc<dyn CodingAgentPlugin>, PluginRegistryError> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginRegistryError::PluginNotFound(plugin_id.to_string()))
    }

    pub fn has(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().unwrap();
        plugins.contains_key(plugin_id)
    }

    pub fn list(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .values()
            .map(|plugin| plugin.metadata())
            .collect()
    }

    pub fn count(&self) -> usize {
        let plugins = self.plugins.read().unwrap();
        plugins.len()
    }

    #[cfg(test)]
    pub fn clear(&self) {
        let mut plugins = self.plugins.write().unwrap();
        plugins.clear();
    }
}

lazy_static! {
    static ref GLOBAL_PLUGIN_REGISTRY: PluginRegistry = PluginRegistry::new();
}

pub fn global_registry() -> &'static PluginRegistry {
    &GLOBAL_PLUGIN_REGISTRY
}

pub fn register_plugin(plugin: Arc<dyn CodingAgentPlugin>) -> Result<(), PluginRegistryError> {
    global_registry().register(plugin)
}

pub fn get_plugin(plugin_id: &str) -> Result<Arc<dyn CodingAgentPlugin>, PluginRegistryError> {
    global_registry().get(plugin_id)
}

pub fn list_plugins() -> Vec<PluginMetadata> {
    global_registry().list()
}
