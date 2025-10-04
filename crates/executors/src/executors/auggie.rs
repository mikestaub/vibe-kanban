use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use workspace_utils::msg_store::MsgStore;

use crate::{
    command::{CmdOverrides, CommandBuilder, apply_overrides},
    executors::{
        AppendPrompt, ExecutorError, SpawnedChild, StandardCodingAgentExecutor,
        gemini::AcpAgentHarness,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS, JsonSchema)]
pub struct Auggie {
    #[serde(default)]
    pub append_prompt: AppendPrompt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(flatten)]
    pub cmd: CmdOverrides,
}

impl Auggie {
    fn build_command_builder(&self, is_follow_up: bool) -> CommandBuilder {
        let mut builder = CommandBuilder::new("auggie");

        if is_follow_up {
            builder = builder.extend_params(["--continue"]);
        }

        if let Some(ref model) = self.model {
            builder = builder.extend_params(["--model", model]);
        }

        apply_overrides(builder, &self.cmd)
    }
}

#[async_trait]
impl StandardCodingAgentExecutor for Auggie {
    async fn spawn(&self, current_dir: &Path, prompt: &str) -> Result<SpawnedChild, ExecutorError> {
        let auggie_command = self.build_command_builder(false);
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        
        let mut cmd = auggie_command.build_initial();
        cmd.push_str(&format!(
            " --instruction \"{}\" --workspace-root \"{}\"",
            combined_prompt.replace('"', "\\\""),
            current_dir.display()
        ));

        let harness = AcpAgentHarness::with_session_namespace("auggie_sessions");
        harness
            .spawn_with_command(current_dir, combined_prompt, cmd)
            .await
    }

    async fn spawn_follow_up(
        &self,
        current_dir: &Path,
        prompt: &str,
        session_id: &str,
    ) -> Result<SpawnedChild, ExecutorError> {
        let auggie_command = self.build_command_builder(true);
        let combined_prompt = self.append_prompt.combine_prompt(prompt);
        
        let mut cmd = auggie_command.build_follow_up(&[]);
        cmd.push_str(&format!(
            " --instruction \"{}\" --workspace-root \"{}\"",
            combined_prompt.replace('"', "\\\""),
            current_dir.display()
        ));

        let harness = AcpAgentHarness::with_session_namespace("auggie_sessions");
        harness
            .spawn_follow_up_with_command(current_dir, combined_prompt, session_id, cmd)
            .await
    }

    fn normalize_logs(&self, msg_store: Arc<MsgStore>, worktree_path: &Path) {
        crate::executors::acp::normalize_logs(msg_store, worktree_path);
    }

    // MCP configuration methods
    fn default_mcp_config_path(&self) -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|home| home.join(".augment").join("settings.json"))
    }
}
