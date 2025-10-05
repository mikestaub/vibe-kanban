# Plugin Architecture for Coding Agents

This document describes the new plugin-based architecture for coding agents in Vibe Kanban.

## Overview

The plugin architecture allows coding agents to be registered dynamically without requiring code changes to the core system. This enables:

- **Zero-code agent additions**: Add new coding agents by registering plugins
- **Community contributions**: Users can create custom agent plugins
- **Better separation**: Clear API boundary between core system and agents
- **Backward compatibility**: Existing enum-based executors continue to work

## Architecture Components

### 1. Plugin Trait (`plugin/traits.rs`)

The `CodingAgentPlugin` trait defines the interface all plugins must implement:

```rust
#[async_trait]
pub trait CodingAgentPlugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    async fn spawn(&self, current_dir: &Path, prompt: &str, config: &JsonValue) -> Result<SpawnedChild, ExecutorError>;
    async fn spawn_follow_up(&self, current_dir: &Path, prompt: &str, session_id: &str, config: &JsonValue) -> Result<SpawnedChild, ExecutorError>;
    fn normalize_logs(&self, msg_store: Arc<MsgStore>, worktree_path: &Path, config: &JsonValue);
    fn default_mcp_config_path(&self) -> Option<PathBuf>;
    fn get_mcp_config(&self) -> Option<McpConfig>;
    async fn check_availability(&self) -> bool;
    fn validate_config(&self, config: &JsonValue) -> Result<(), String>;
}
```

### 2. Plugin Registry (`plugin/registry.rs`)

The registry manages all registered plugins:

```rust
// Register a plugin
register_plugin(Arc::new(MyPlugin::new()))?;

// Get a plugin
let plugin = get_plugin("my-plugin-id")?;

// List all plugins
let plugins = list_plugins();
```

### 3. Dynamic Executor (`plugin/dynamic_executor.rs`)

The `DynamicCodingAgent` wraps plugins and implements `StandardCodingAgentExecutor`:

```rust
// Create from plugin ID and config
let executor = DynamicCodingAgent::new(
    "claude-code".to_string(),
    serde_json::json!({ "model": "claude-3-5-sonnet" })
)?;

// Use like any other executor
executor.spawn(current_dir, prompt).await?;
```

### 4. Unified Executor (`executors/plugin_bridge.rs`)

The `UnifiedExecutor` enum bridges legacy and plugin systems:

```rust
pub enum UnifiedExecutor {
    Legacy(CodingAgent),
    Plugin {
        plugin_id: String,
        variant: Option<String>,
        config: JsonValue,
    },
}
```

## Built-in Plugins

### Claude Code Plugin (`plugins/claude_code.rs`)

Wraps the existing Claude Code executor:

- **Plugin ID**: `claude-code`
- **Capabilities**: SessionFork, McpSupport, ApprovalWorkflow, PlanMode
- **Config**: Supports all existing Claude Code options

### Codex Plugin (`plugins/codex.rs`)

Wraps the existing Codex executor:

- **Plugin ID**: `codex`
- **Capabilities**: SessionFork, McpSupport
- **Config**: Supports all existing Codex options

## Usage

### Initializing Plugins

Call `initialize_plugins()` at application startup:

```rust
use executors::plugin::initialize_plugins;

fn main() {
    initialize_plugins().expect("Failed to initialize plugins");
    // ... rest of application
}
```

### Using Plugins

```rust
// Get available plugins
let plugins = list_plugins();
for metadata in plugins {
    println!("Plugin: {} ({})", metadata.name, metadata.id);
}

// Create executor from plugin
let executor = DynamicCodingAgent::new(
    "claude-code".to_string(),
    serde_json::json!({
        "model": "claude-3-5-sonnet",
        "plan": true
    })
)?;

// Execute
let child = executor.spawn(current_dir, "Write a hello world program").await?;
```

### Creating Custom Plugins

```rust
use executors::plugin::{CodingAgentPlugin, PluginMetadata, PluginCapability};

pub struct MyCustomPlugin;

#[async_trait]
impl CodingAgentPlugin for MyCustomPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "my-custom-agent".to_string(),
            name: "My Custom Agent".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Custom coding agent".to_string()),
            capabilities: vec![PluginCapability::SessionFork],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string" }
                }
            }),
        }
    }

    async fn spawn(&self, current_dir: &Path, prompt: &str, config: &JsonValue) -> Result<SpawnedChild, ExecutorError> {
        // Implementation
    }

    // ... implement other trait methods
}

// Register the plugin
register_plugin(Arc::new(MyCustomPlugin))?;
```

## Migration Path

### Phase 1: Parallel Systems (Current)

Both legacy enum-based and plugin-based systems coexist:

- Legacy executors (Amp, Gemini, QwenCode, etc.) continue as enums
- Claude Code and Codex available as both enum and plugin
- New agents can be added as plugins only

### Phase 2: Gradual Migration (Future)

Migrate remaining executors to plugins:

1. Convert each executor to a plugin
2. Update default profiles to use plugin format
3. Maintain backward compatibility via `UnifiedExecutor`

### Phase 3: Plugin-Only (Future)

Remove legacy enum system once all executors are migrated:

1. Remove `CodingAgent` enum
2. Use `DynamicCodingAgent` everywhere
3. Update database schema

## Benefits

### For Developers

- **No code changes needed**: Add agents without modifying core code
- **Clear API**: Well-defined plugin interface
- **Type safety**: Rust's type system ensures correctness
- **Testing**: Plugins can be tested independently

### For Users

- **Extensibility**: Create custom agents for specific workflows
- **Community plugins**: Share and use community-created agents
- **Faster iterations**: New agents don't require app recompilation

### For Maintainers

- **Reduced complexity**: No more enum sprawl
- **Better organization**: Each plugin is self-contained
- **Easier reviews**: Plugin code is isolated and testable

## Technical Details

### Plugin Configuration

Plugins receive configuration as JSON:

```json
{
  "model": "claude-3-5-sonnet",
  "plan": true,
  "dangerously_skip_permissions": false
}
```

The JSON schema for configuration is provided by the plugin via `metadata().config_schema`.

### Session Management

Plugins that support `SessionFork` capability can:

1. Start new sessions via `spawn()`
2. Resume/fork sessions via `spawn_follow_up()`
3. Store session state in plugin-specific namespaces

### Log Normalization

Each plugin normalizes its own log format into structured `NormalizedEntry` objects:

```rust
fn normalize_logs(&self, msg_store: Arc<MsgStore>, worktree_path: &Path, config: &JsonValue) {
    // Parse agent-specific output
    // Convert to NormalizedEntry
    // Push patches to msg_store
}
```

### MCP Configuration

Plugins declare MCP support via:

1. `default_mcp_config_path()`: Where to find MCP config file
2. `get_mcp_config()`: Structure of MCP configuration

## Future Enhancements

### External Plugin Loading

Load plugins from filesystem:

```rust
let plugin = load_plugin_from_file("~/.vibe-kanban/plugins/my-agent.so")?;
register_plugin(plugin)?;
```

### WASM Plugins

Support WebAssembly plugins for sandboxed execution:

```rust
let wasm_plugin = WasmPlugin::load("custom-agent.wasm")?;
register_plugin(Arc::new(wasm_plugin))?;
```

### Plugin Marketplace

Browse and install community plugins:

```bash
vibe-kanban plugin install custom-agent
vibe-kanban plugin list
vibe-kanban plugin remove custom-agent
```

### Hot Reloading

Reload plugins without restarting:

```rust
reload_plugin("claude-code")?;
```

## Testing

Test plugins independently:

```rust
#[tokio::test]
async fn test_claude_code_plugin() {
    let plugin = ClaudeCodePlugin::new();
    let metadata = plugin.metadata();
    assert_eq!(metadata.id, "claude-code");
    
    let config = serde_json::json!({ "model": "claude-3-5-sonnet" });
    assert!(plugin.validate_config(&config).is_ok());
}
```

## Conclusion

The plugin architecture provides a solid foundation for extensible, maintainable coding agent support in Vibe Kanban. It enables community contributions while maintaining backward compatibility with existing code.
