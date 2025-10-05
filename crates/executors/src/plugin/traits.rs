
use std::{path::{Path, PathBuf}, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ts_rs::TS;
use workspace_utils::msg_store::MsgStore;

use crate::executors::{ExecutorError, SpawnedChild};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluginCapability {
    SessionFork,
    McpSupport,
    ApprovalWorkflow,
    PlanMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub capabilities: Vec<PluginCapability>,
    pub config_schema: JsonValue,
}

#[async_trait]
pub trait CodingAgentPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    async fn spawn(
        &self,
        current_dir: &Path,
        prompt: &str,
        config: &JsonValue,
    ) -> Result<SpawnedChild, ExecutorError>;

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
        config: &JsonValue,
    ) -> Result<SpawnedChild, ExecutorError>;

    fn normalize_logs(
        &self,
        msg_store: Arc<MsgStore>,
        worktree_path: &Path,
        config: &JsonValue,
    );

    fn default_mcp_config_path(&self) -> Option<PathBuf>;

    fn get_mcp_config(&self) -> Option<crate::mcp_config::McpConfig> {
        None
    }

    async fn check_availability(&self) -> bool {
        self.default_mcp_config_path()
            .map(|path| path.exists())
            .unwrap_or(false)
    }

    fn validate_config(&self, config: &JsonValue) -> Result<(), String> {
        let _ = config;
        Ok(())
    }
}
