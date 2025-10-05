
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct PluginConfig {
    pub plugin_id: String,
    /// Optional variant name (e.g., "PLAN", "APPROVALS")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub config: JsonValue,
}

impl PluginConfig {
    pub fn new(plugin_id: String, config: JsonValue) -> Self {
        Self {
            plugin_id,
            variant: None,
            config,
        }
    }

    pub fn with_variant(plugin_id: String, variant: String, config: JsonValue) -> Self {
        Self {
            plugin_id,
            variant: Some(variant),
            config,
        }
    }

    pub fn cache_key(&self) -> String {
        match &self.variant {
            Some(variant) => format!("{}:{}", self.plugin_id, variant),
            None => self.plugin_id.clone(),
        }
    }
}

pub type PluginConfigValue = JsonValue;
