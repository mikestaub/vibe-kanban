
pub mod claude_code;
pub mod codex;

use std::sync::Arc;

use crate::plugin::registry::register_plugin;

pub fn register_builtin_plugins() -> Result<(), Box<dyn std::error::Error>> {
    let claude_plugin = Arc::new(claude_code::ClaudeCodePlugin::new());
    register_plugin(claude_plugin)?;
    
    let codex_plugin = Arc::new(codex::CodexPlugin::new());
    register_plugin(codex_plugin)?;
    
    tracing::info!("Successfully registered {} built-in plugins", 2);
    Ok(())
}
