
use std::{path::{Path, PathBuf}, sync::Arc};

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use workspace_utils::msg_store::MsgStore;

use crate::{
    executors::{
        ExecutorError, SpawnedChild,
        codex::{Codex, CodexJson},
        codex::session::SessionHandler,
    },
    logs::utils::EntryIndexProvider,
    mcp_config::McpConfig,
    plugin::traits::{CodingAgentPlugin, PluginCapability, PluginMetadata},
};

pub struct CodexPlugin;

impl CodexPlugin {
    pub fn new() -> Self {
        Self
    }

    fn parse_config(&self, config: &JsonValue) -> Result<Codex, ExecutorError> {
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
                "sandbox": {
                    "type": ["string", "null"],
                    "enum": ["auto", "read-only", "workspace-write", "danger-full-access"],
                    "description": "Sandbox policy mode"
                },
                "oss": {
                    "type": ["boolean", "null"],
                    "description": "Use OSS mode"
                },
                "model": {
                    "type": ["string", "null"],
                    "description": "Model to use (e.g., gpt-5-codex, gpt-5)"
                },
                "model_reasoning_effort": {
                    "type": ["string", "null"],
                    "enum": ["low", "medium", "high"],
                    "description": "Reasoning effort for the model"
                },
                "model_reasoning_summary": {
                    "type": ["string", "null"],
                    "enum": ["auto", "concise", "detailed", "none"],
                    "description": "Model reasoning summary style"
                },
                "model_reasoning_summary_format": {
                    "type": ["string", "null"],
                    "enum": ["none", "experimental"],
                    "description": "Format for model reasoning summaries"
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
impl CodingAgentPlugin for CodexPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "codex".to_string(),
            name: "Codex".to_string(),
            version: "1.0.0".to_string(),
            description: Some("OpenAI's Codex coding assistant".to_string()),
            capabilities: vec![
                PluginCapability::SessionFork,
                PluginCapability::McpSupport,
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

        // Process stderr logs for session extraction only
        SessionHandler::start_session_id_extraction(msg_store.clone());

        // Process stdout logs (Codex's JSONL output)
        let current_dir = worktree_path.to_path_buf();
        tokio::spawn(async move {
            let mut stream = msg_store.stdout_lines_stream();
            use futures::StreamExt;
            use std::collections::HashMap;
            use crate::logs::{ActionType, NormalizedEntry, NormalizedEntryType, ToolStatus};
            use crate::logs::utils::patch::ConversationPatch;
            
            // Track exec call ids to entry index, tool_name, content, and command
            let mut exec_info_map: HashMap<String, (usize, String, String, String)> =
                HashMap::new();
            // Track MCP calls to index, tool_name, args, and initial content
            let mut mcp_info_map: HashMap<
                String,
                (usize, String, Option<serde_json::Value>, String),
            > = HashMap::new();

            while let Some(Ok(line)) = stream.next().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Ok(cj) = serde_json::from_str::<CodexJson>(trimmed) {
                    // Handle result-carrying events that require replacement
                    match &cj {
                        CodexJson::StructuredMessage { msg, .. } => match msg {
                            crate::executors::codex::CodexMsgContent::ExecCommandBegin {
                                call_id, command, ..
                            } => {
                                let command_str = command.join(" ");
                                let entry = NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::ToolUse {
                                        tool_name: if command_str.contains("bash") {
                                            "bash".to_string()
                                        } else {
                                            "shell".to_string()
                                        },
                                        action_type: ActionType::CommandRun {
                                            command: command_str.clone(),
                                            result: None,
                                        },
                                        status: ToolStatus::Created,
                                    },
                                    content: format!("`{command_str}`"),
                                    metadata: None,
                                };
                                let id = entry_index_provider.next();
                                if let Some(cid) = call_id.as_ref() {
                                    let tool_name = if command_str.contains("bash") {
                                        "bash".to_string()
                                    } else {
                                        "shell".to_string()
                                    };
                                    exec_info_map.insert(
                                        cid.clone(),
                                        (id, tool_name, entry.content.clone(), command_str.clone()),
                                    );
                                }
                                msg_store
                                    .push_patch(ConversationPatch::add_normalized_entry(id, entry));
                            }
                            crate::executors::codex::CodexMsgContent::ExecCommandEnd {
                                call_id,
                                stdout,
                                stderr,
                                success,
                                exit_code,
                            } => {
                                if let Some(cid) = call_id.as_ref()
                                    && let Some((idx, tool_name, prev_content, prev_command)) =
                                        exec_info_map.get(cid).cloned()
                                {
                                    // Merge stdout and stderr for richer context
                                    let output = match (stdout.as_ref(), stderr.as_ref()) {
                                        (Some(sout), Some(serr)) => {
                                            let sout_trim = sout.trim();
                                            let serr_trim = serr.trim();
                                            if sout_trim.is_empty() && serr_trim.is_empty() {
                                                None
                                            } else if sout_trim.is_empty() {
                                                Some(serr.clone())
                                            } else if serr_trim.is_empty() {
                                                Some(sout.clone())
                                            } else {
                                                Some(format!(
                                                    "STDOUT:\n{sout_trim}\n\nSTDERR:\n{serr_trim}"
                                                ))
                                            }
                                        }
                                        (Some(sout), None) => {
                                            if sout.trim().is_empty() {
                                                None
                                            } else {
                                                Some(sout.clone())
                                            }
                                        }
                                        (None, Some(serr)) => {
                                            if serr.trim().is_empty() {
                                                None
                                            } else {
                                                Some(serr.clone())
                                            }
                                        }
                                        (None, None) => None,
                                    };
                                    let exit_status = if let Some(s) = success {
                                        Some(crate::logs::CommandExitStatus::Success {
                                            success: *s,
                                        })
                                    } else {
                                        exit_code.as_ref().map(|code| {
                                            crate::logs::CommandExitStatus::ExitCode { code: *code }
                                        })
                                    };

                                    let status = if let Some(s) = success {
                                        if *s {
                                            ToolStatus::Success
                                        } else {
                                            ToolStatus::Failed
                                        }
                                    } else if let Some(code) = exit_code {
                                        if *code == 0 {
                                            ToolStatus::Success
                                        } else {
                                            ToolStatus::Failed
                                        }
                                    } else {
                                        ToolStatus::Failed
                                    };

                                    let entry = NormalizedEntry {
                                        timestamp: None,
                                        entry_type: NormalizedEntryType::ToolUse {
                                            tool_name,
                                            action_type: ActionType::CommandRun {
                                                command: prev_command,
                                                result: Some(crate::logs::CommandRunResult {
                                                    exit_status,
                                                    output,
                                                }),
                                            },
                                            status,
                                        },
                                        content: prev_content,
                                        metadata: None,
                                    };
                                    msg_store.push_patch(ConversationPatch::replace(idx, entry));
                                }
                            }
                            crate::executors::codex::CodexMsgContent::McpToolCallBegin {
                                call_id,
                                invocation,
                            } => {
                                let tool_name =
                                    format!("mcp:{}:{}", invocation.server, invocation.tool);
                                let content_str = invocation.tool.clone();
                                let entry = NormalizedEntry {
                                    timestamp: None,
                                    entry_type: NormalizedEntryType::ToolUse {
                                        tool_name: tool_name.clone(),
                                        action_type: ActionType::Tool {
                                            tool_name: tool_name.clone(),
                                            arguments: invocation.arguments.clone(),
                                            result: None,
                                        },
                                        status: ToolStatus::Created,
                                    },
                                    content: content_str.clone(),
                                    metadata: None,
                                };
                                let id = entry_index_provider.next();
                                mcp_info_map.insert(
                                    call_id.clone(),
                                    (
                                        id,
                                        tool_name.clone(),
                                        invocation.arguments.clone(),
                                        content_str,
                                    ),
                                );
                                msg_store
                                    .push_patch(ConversationPatch::add_normalized_entry(id, entry));
                            }
                            crate::executors::codex::CodexMsgContent::McpToolCallEnd {
                                call_id, result, ..
                            } => {
                                if let Some((idx, tool_name, args, prev_content)) =
                                    mcp_info_map.remove(call_id)
                                {
                                    let entry = NormalizedEntry {
                                        timestamp: None,
                                        entry_type: NormalizedEntryType::ToolUse {
                                            tool_name: tool_name.clone(),
                                            action_type: ActionType::Tool {
                                                tool_name,
                                                arguments: args,
                                                result: Some(crate::logs::ToolResult {
                                                    r#type: crate::logs::ToolResultValueType::Json,
                                                    value: result.clone(),
                                                }),
                                            },
                                            status: ToolStatus::Success,
                                        },
                                        content: prev_content,
                                        metadata: None,
                                    };
                                    msg_store.push_patch(ConversationPatch::replace(idx, entry));
                                }
                            }
                            _ => {
                                if let Some(entries) = cj.to_normalized_entries(&current_dir) {
                                    for entry in entries {
                                        let new_id = entry_index_provider.next();
                                        let patch =
                                            ConversationPatch::add_normalized_entry(new_id, entry);
                                        msg_store.push_patch(patch);
                                    }
                                }
                            }
                        },
                        _ => {
                            if let Some(entries) = cj.to_normalized_entries(&current_dir) {
                                for entry in entries {
                                    let new_id = entry_index_provider.next();
                                    let patch =
                                        ConversationPatch::add_normalized_entry(new_id, entry);
                                    msg_store.push_patch(patch);
                                }
                            }
                        }
                    }
                } else {
                    // Handle malformed JSON as raw output
                    let entry = NormalizedEntry {
                        timestamp: None,
                        entry_type: NormalizedEntryType::SystemMessage,
                        content: trimmed.to_string(),
                        metadata: None,
                    };

                    let new_id = entry_index_provider.next();
                    let patch = ConversationPatch::add_normalized_entry(new_id, entry);
                    msg_store.push_patch(patch);
                }
            }
        });
    }

    fn default_mcp_config_path(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".codex").join("config.toml"))
    }

    fn get_mcp_config(&self) -> Option<McpConfig> {
        Some(McpConfig::new(
            vec!["mcp_servers".to_string()],
            serde_json::json!({
                "mcp_servers": {}
            }),
            crate::mcp_config::PRECONFIGURED_MCP_SERVERS.clone(),
            true,  // Codex uses TOML
        ))
    }

    fn validate_config(&self, config: &JsonValue) -> Result<(), String> {
        self.parse_config(config)
            .map(|_| ())
            .map_err(|e| format!("Invalid configuration: {}", e))
    }
}
