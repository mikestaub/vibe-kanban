
use std::sync::Once;

static INIT: Once = Once::new();

pub fn initialize_plugins() -> Result<(), Box<dyn std::error::Error>> {
    let mut result = Ok(());
    
    INIT.call_once(|| {
        tracing::info!("Initializing plugin system...");
        
        if let Err(e) = crate::plugins::register_builtin_plugins() {
            tracing::error!("Failed to register built-in plugins: {}", e);
            result = Err(e);
            return;
        }
        
        tracing::info!("Plugin system initialized successfully");
    });
    
    result
}

pub fn is_initialized() -> bool {
    INIT.is_completed()
}
