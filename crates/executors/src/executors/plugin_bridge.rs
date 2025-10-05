
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;

use crate::{
    executors::{ExecutorError, CodingAgent},
    mcp_config::McpConfig,
    plugin::{DynamicCodingAgent, get_plugin},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(untagged)]
pub enum UnifiedExecutor {
    Legacy(CodingAgent),
    Plugin {
        plugin_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        variant: Option<String>,
        config: JsonValue,
    },
}

impl UnifiedExecutor {
    pub fn as_legacy(&self) -> Option<&CodingAgent> {
        match self {
            Self::Legacy(agent) => Some(agent),
            Self::Plugin { .. } => None,
        }
    }
    
    pub fn to_dynamic_agent(&self) -> Result<DynamicCodingAgent, ExecutorError> {
        match self {
            Self::Legacy(_) => {
                Err(ExecutorError::UnknownExecutorType(
                    "Cannot convert legacy executor to dynamic agent".to_string()
                ))
            }
            Self::Plugin {
                plugin_id,
                variant,
                config,
            } => {
                let executor = if let Some(v) = variant {
                    DynamicCodingAgent::with_variant(plugin_id.clone(), v.clone(), config.clone())?
                } else {
                    DynamicCodingAgent::new(plugin_id.clone(), config.clone())?
                };
                Ok(executor)
            }
        }
    }
    
    pub fn is_plugin(&self) -> bool {
        matches!(self, Self::Plugin { .. })
    }

    pub fn get_mcp_config(&self) -> McpConfig {
        match self {
            Self::Legacy(coding_agent) => coding_agent.get_mcp_config(),
            Self::Plugin { plugin_id, .. } => {
                if let Ok(plugin) = get_plugin(plugin_id) {
                    plugin.get_mcp_config().unwrap_or_else(|| {
                        McpConfig::new(
                            vec!["mcpServers".to_string()],
                            serde_json::json!({
                                "mcpServers": {}
                            }),
                            crate::mcp_config::PRECONFIGURED_MCP_SERVERS.clone(),
                            false,
                        )
                    })
                } else {
                    McpConfig::new(
                        vec!["mcpServers".to_string()],
                        serde_json::json!({
                            "mcpServers": {}
                        }),
                        crate::mcp_config::PRECONFIGURED_MCP_SERVERS.clone(),
                        false,
                    )
                }
            }
        }
    }

    pub async fn supports_mcp(&self) -> bool {
        match self {
            Self::Legacy(coding_agent) => coding_agent.supports_mcp(),
            Self::Plugin { plugin_id, .. } => {
                if let Ok(plugin) = get_plugin(plugin_id) {
                    plugin.default_mcp_config_path().is_some()
                } else {
                    false
                }
            }
        }
    }

    pub fn capabilities(&self) -> Vec<crate::executors::BaseAgentCapability> {
        match self {
            Self::Legacy(coding_agent) => coding_agent.capabilities(),
            Self::Plugin { plugin_id, .. } => {
                if let Ok(plugin) = get_plugin(plugin_id) {
                    let metadata = plugin.metadata();
                    metadata
                        .capabilities
                        .iter()
                        .filter_map(|cap| match cap {
                            crate::plugin::PluginCapability::SessionFork => {
                                Some(crate::executors::BaseAgentCapability::SessionFork)
                            }
                            _ => None,
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
        }
    }
}

impl From<CodingAgent> for UnifiedExecutor {
    fn from(agent: CodingAgent) -> Self {
        Self::Legacy(agent)
    }
}
