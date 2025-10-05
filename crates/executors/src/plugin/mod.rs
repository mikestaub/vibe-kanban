
pub mod config;
pub mod dynamic_executor;
pub mod init;
pub mod registry;
pub mod traits;

pub use config::{PluginConfig, PluginConfigValue};
pub use dynamic_executor::DynamicCodingAgent;
pub use init::{initialize_plugins, is_initialized};
pub use registry::{PluginRegistry, PluginRegistryError, get_plugin, list_plugins, register_plugin};
pub use traits::{CodingAgentPlugin, PluginCapability, PluginMetadata};
