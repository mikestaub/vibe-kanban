
use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use workspace_utils::msg_store::MsgStore;

use crate::executors::{ExecutorError, SpawnedChild, StandardCodingAgentExecutor};

use super::{
    config::PluginConfig,
    registry::get_plugin,
    traits::CodingAgentPlugin,
};

#[derive(Clone)]
pub struct DynamicCodingAgent {
    plugin: Arc<dyn CodingAgentPlugin>,
    config: PluginConfig,
}

impl DynamicCodingAgent {
    pub fn new(plugin_id: String, config: JsonValue) -> Result<Self, ExecutorError> {
        let plugin = get_plugin(&plugin_id).map_err(|e| {
            ExecutorError::UnknownExecutorType(format!("Plugin '{}' not found: {}", plugin_id, e))
        })?;

        plugin
            .validate_config(&config)
            .map_err(|e| ExecutorError::UnknownExecutorType(format!("Invalid config: {}", e)))?;

        Ok(Self {
            plugin,
            config: PluginConfig::new(plugin_id, config),
        })
    }

    pub fn with_variant(
        plugin_id: String,
        variant: String,
        config: JsonValue,
    ) -> Result<Self, ExecutorError> {
        let plugin = get_plugin(&plugin_id).map_err(|e| {
            ExecutorError::UnknownExecutorType(format!("Plugin '{}' not found: {}", plugin_id, e))
        })?;

        plugin
            .validate_config(&config)
            .map_err(|e| ExecutorError::UnknownExecutorType(format!("Invalid config: {}", e)))?;

        Ok(Self {
            plugin,
            config: PluginConfig::with_variant(plugin_id, variant, config),
        })
    }

    pub fn plugin_id(&self) -> &str {
        &self.config.plugin_id
    }

    pub fn variant(&self) -> Option<&str> {
        self.config.variant.as_deref()
    }

    pub fn config(&self) -> &JsonValue {
        &self.config.config
    }

    pub fn plugin(&self) -> &Arc<dyn CodingAgentPlugin> {
        &self.plugin
    }
}

impl std::fmt::Debug for DynamicCodingAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicCodingAgent")
            .field("plugin_id", &self.config.plugin_id)
            .field("variant", &self.config.variant)
            .field("config", &self.config.config)
            .finish()
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for DynamicCodingAgent {
    async fn spawn(&self, current_dir: &Path, prompt: &str) -> Result<SpawnedChild, ExecutorError> {
        self.plugin
            .spawn(current_dir, prompt, &self.config.config)
            .await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
    ) -> Result<SpawnedChild, ExecutorError> {
        self.plugin
            .spawn_follow_up(current_dir, prompt, session_id, &self.config.config)
            .await
    }

    fn normalize_logs(&self, msg_store: Arc<MsgStore>, worktree_path: &Path) {
        self.plugin
            .normalize_logs(msg_store, worktree_path, &self.config.config);
    }

    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        self.plugin.default_mcp_config_path()
    }

    async fn check_availability(&self) -> bool {
        self.plugin.check_availability().await
    }
}
