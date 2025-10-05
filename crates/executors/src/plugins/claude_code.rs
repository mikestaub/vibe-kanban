
use std::{path::{Path, PathBuf}, sync::Arc};

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use workspace_utils::msg_store::MsgStore;

use crate::{
    executors::{ExecutorError, SpawnedChild, claude::{ClaudeCode, HistoryStrategy, ClaudeLogProcessor}},
    logs::stderr_processor::normalize_stderr_logs,
    logs::utils::EntryIndexProvider,
    mcp_config::McpConfig,
    plugin::traits::{CodingAgentPlugin, PluginCapability, PluginMetadata},
};

pub struct ClaudeCodePlugin;

impl ClaudeCodePlugin {
    pub fn new() -> Self {
        Self
    }

    fn parse_config(&self, config: &JsonValue) -> Result<ClaudeCode, ExecutorError> {
        serde_json::from_value(config.clone()).map_err(ExecutorError::Json)
    }

    fn config_schema() -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "append_prompt": {
                    "type": ["object", "null"],
                    "title": "Append Prompt",
                    "description": "Extra text appended to the prompt"
                },
                "claude_code_router": {
                    "type": ["boolean", "null"],
                    "description": "Use claude-code-router instead of standard claude-code"
                },
                "plan": {
                    "type": ["boolean", "null"],
                    "description": "Enable plan mode"
                },
                "approvals": {
                    "type": ["boolean", "null"],
                    "description": "Enable approval workflow"
                },
                "model": {
                    "type": ["string", "null"],
                    "description": "Model to use (e.g., claude-3-5-sonnet-20241022)"
                },
                "dangerously_skip_permissions": {
                    "type": ["boolean", "null"],
                    "description": "Skip permission checks (dangerous!)"
                },
                "base_command_override": {
                    "type": ["string", "null"],
                    "title": "Base Command Override",
                    "description": "Override the base command with a custom command"
                },
                "additional_params": {
                    "type": ["array", "null"],
                    "title": "Additional Parameters",
                    "description": "Additional parameters to append to the base command",
                    "items": {
                        "type": "string"
                    }
                }
            }
        })
    }
}

#[async_trait]
impl CodingAgentPlugin for ClaudeCodePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Anthropic's Claude Code coding assistant".to_string()),
            capabilities: vec![
                PluginCapability::SessionFork,
                PluginCapability::McpSupport,
                PluginCapability::ApprovalWorkflow,
                PluginCapability::PlanMode,
            ],
            config_schema: Self::config_schema(),
        }
    }

    async fn spawn(
        &self,
        current_dir: &Path,
        prompt: &str,
        config: &JsonValue,
    ) -> Result<SpawnedChild, ExecutorError> {
        let executor = self.parse_config(config)?;
        
        use crate::executors::StandardCodingAgentExecutor;
        executor.spawn(current_dir, prompt).await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        config: &JsonValue,
    ) -> Result<SpawnedChild, ExecutorError> {
        let executor = self.parse_config(config)?;
        
        use crate::executors::StandardCodingAgentExecutor;
        executor.spawn_follow_up(current_dir, prompt, session_id).await
    }

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        worktree_path: &Path,
        _config: &JsonValue,
    ) {
        let entry_index_provider = EntryIndexProvider::start_from(&msg_store);

        // Process stdout logs (Claude's JSON output)
        ClaudeLogProcessor::process_logs(
            msg_store.clone(),
            worktree_path,
            entry_index_provider.clone(),
            HistoryStrategy::Default,
        );

        // Process stderr logs using the standard stderr processor
        normalize_stderr_logs(msg_store, entry_index_provider);
    }

    fn default_mcp_config_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".claude.json"))
    }

    fn get_mcp_config(&self) -> Option<McpConfig> {
        Some(McpConfig::new(
            vec!["mcpServers".to_string()],
            serde_json::json!({
                "mcpServers": {}
            }),
            crate::mcp_config::PRECONFIGURED_MCP_SERVERS.clone(),
            false,
        ))
    }

    fn validate_config(&self, config: &JsonValue) -> Result<(), String> {
        self.parse_config(config)
            .map(|_| ())
            .map_err(|e| format!("Invalid configuration: {}", e))
    }
}
