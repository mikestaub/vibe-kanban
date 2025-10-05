use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::msg_store::MsgStore;

pub use super::acp::AcpAgentHarness;
use crate::{
    command::{apply_overrides, CmdOverrides, CommandBuilder},
    executors::{AppendPrompt, ExecutorError, SpawnedChild, StandardCodingAgentExecutor},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, JsonSchema)]
pub struct FactoryDroid {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(flatten)]
    pub cmd: CmdOverrides,
}

impl FactoryDroid {
    fn build_command_builder(&self) -> CommandBuilder {
        let mut builder = CommandBuilder::new("npx @factory/cli@latest");

        if let Some(model) = &self.model {
            builder = builder.extend_params(["--model", model]);
        }

        builder = builder.extend_params(["--json"]);

        apply_overrides(builder, &self.cmd)
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for FactoryDroid {
    async fn spawn(&self, current_dir: &Path, prompt: &str) -> Result<SpawnedChild, ExecutorError> {
        let harness =
            AcpAgentHarness::with_session_namespace("factory_droid_sessions".to_string());
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let factory_command = self
            .build_command_builder()
            .extend_params(["--instruction", &combined_prompt])
            .build_initial();

        harness
            .spawn_with_command(current_dir, "".to_string(), factory_command)
            .await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
    ) -> Result<SpawnedChild, ExecutorError> {
        let harness =
            AcpAgentHarness::with_session_namespace("factory_droid_sessions".to_string());
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        let factory_command = self
            .build_command_builder()
            .build_follow_up(&[
                "--continue".to_string(),
                session_id.to_string(),
                "--instruction".to_string(),
                combined_prompt.clone(),
            ]);
        harness
            .spawn_follow_up_with_command(current_dir, "".to_string(), session_id, factory_command)
            .await
    }

    fn normalize_logs(&self, msg_store: Arc<MsgStore>, worktree_path: &Path) {
        super::acp::normalize_logs(msg_store, worktree_path);
    }

    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".factory").join("settings.json"))
    }
}