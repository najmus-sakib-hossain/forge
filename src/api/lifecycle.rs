//! Core Lifecycle & System Orchestration APIs

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::sync::{Arc, Once};
use std::path::PathBuf;
use std::collections::HashMap;

use crate::orchestrator::{DxTool, ExecutionContext};
use crate::core::Forge;

// Global forge instance
static INIT: Once = Once::new();
pub(crate) static mut FORGE_INSTANCE: Option<Arc<RwLock<Forge>>> = None;
pub(crate) static mut TOOL_REGISTRY: Option<Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn DxTool>>>>>>> = None;
static mut CURRENT_CONTEXT: Option<Arc<RwLock<ExecutionContext>>> = None;

/// Global one-time initialization (dx binary, LSP, editor extension, daemon)
///
/// This must be called exactly once at application startup before using any other forge APIs.
/// It initializes the global forge instance, LSP server, file watchers, and all core systems.
///
/// # Example
/// ```no_run
/// use dx_forge::initialize_forge;
///
/// fn main() -> anyhow::Result<()> {
///     initialize_forge()?;
///     // Now forge is ready to use
///     Ok(())
/// }
/// ```
pub fn initialize_forge() -> Result<()> {
    let mut init_result = Ok(());
    
    INIT.call_once(|| {
        tracing::info!("🚀 Initializing Forge v{}", crate::VERSION);
        
        // Detect project root (walk up to find .dx or .git)
        let project_root = detect_workspace_root().unwrap_or_else(|_| {
            std::env::current_dir().expect("Failed to get current directory")
        });
        
        tracing::info!("📁 Project root: {:?}", project_root);
        
        // Create forge instance
        match Forge::new(&project_root) {
            Ok(forge) => {
                unsafe {
                    FORGE_INSTANCE = Some(Arc::new(RwLock::new(forge)));
                    TOOL_REGISTRY = Some(Arc::new(RwLock::new(HashMap::new())));
                    
                    // Create initial execution context
                    let forge_path = project_root.join(".dx/forge");
                    let context = ExecutionContext::new(project_root.clone(), forge_path);
                    CURRENT_CONTEXT = Some(Arc::new(RwLock::new(context)));
                }
                
                tracing::info!("✅ Forge initialization complete");
            }
            Err(e) => {
                init_result = Err(e).context("Failed to initialize forge");
            }
        }
    });
    
    init_result
}

/// Every dx-tool must call this exactly once during startup
///
/// Registers a tool with the forge orchestrator. Tools are indexed by name and
/// version for dependency resolution and execution ordering.
///
/// # Arguments
/// * `tool` - The tool implementation to register
///
/// # Returns
/// A unique tool ID for subsequent operations
///
/// # Example
/// ```no_run
/// use dx_forge::{register_tool, DxTool};
///
/// struct MyTool;
/// impl DxTool for MyTool {
///     fn name(&self) -> &str { "my-tool" }
///     fn version(&self) -> &str { "1.0.0" }
///     fn priority(&self) -> u32 { 50 }
///     fn execute(&mut self, _ctx: &dx_forge::ExecutionContext) -> anyhow::Result<dx_forge::ToolOutput> {
///         Ok(dx_forge::ToolOutput::success())
///     }
/// }
///
/// fn main() -> anyhow::Result<()> {
///     dx_forge::initialize_forge()?;
///     register_tool(Box::new(MyTool))?;
///     Ok(())
/// }
/// ```
pub fn register_tool(tool: Box<dyn DxTool>) -> Result<String> {
    ensure_initialized()?;
    
    let tool_name = tool.name().to_string();
    let tool_version = tool.version().to_string();
    let tool_id = format!("{}@{}", tool_name, tool_version);
    
    tracing::info!("📦 Registering tool: {}", tool_id);
    
    unsafe {
        if let Some(registry) = (*std::ptr::addr_of!(TOOL_REGISTRY)).as_ref() {
            let tool_arc = Arc::new(RwLock::new(tool));
            registry.write().insert(tool_id.clone(), tool_arc);
        }
    }
    
    Ok(tool_id)
}

/// Returns the live, immutable ToolContext for the current operation
///
/// Provides access to the execution context including repository state,
/// changed files, and shared data between tools.
///
/// # Returns
/// A clone of the current execution context
///
/// # Example
/// ```no_run
/// use dx_forge::get_tool_context;
///
/// fn my_operation() -> anyhow::Result<()> {
///     let ctx = get_tool_context()?;
///     println!("Working in: {:?}", ctx.repo_root);
///     Ok(())
/// }
/// ```
pub fn get_tool_context() -> Result<ExecutionContext> {
    ensure_initialized()?;
    
    unsafe {
        if let Some(context) = (*std::ptr::addr_of!(CURRENT_CONTEXT)).as_ref() {
            Ok(context.read().clone())
        } else {
            anyhow::bail!("Tool context not available")
        }
    }
}

/// Full graceful shutdown with progress reporting and cleanup
///
/// Shuts down all running tools, flushes caches, closes file watchers,
/// and performs cleanup. Should be called before application exit.
///
/// # Example
/// ```no_run
/// use dx_forge::shutdown_forge;
///
/// fn main() -> anyhow::Result<()> {
///     dx_forge::initialize_forge()?;
///     // ... do work ...
///     shutdown_forge()?;
///     Ok(())
/// }
/// ```
pub fn shutdown_forge() -> Result<()> {
    tracing::info!("🛑 Shutting down Forge...");
    
    unsafe {
        // Clear tool registry
        if let Some(registry) = (*std::ptr::addr_of_mut!(TOOL_REGISTRY)).take() {
            let count = registry.read().len();
            tracing::info!("📦 Unregistering {} tools", count);
            drop(registry);
        }
        
        // Drop forge instance (triggers Drop impl cleanup)
        if let Some(forge) = (*std::ptr::addr_of_mut!(FORGE_INSTANCE)).take() {
            tracing::info!("🧹 Cleaning up forge instance");
            drop(forge);
        }
        
        // Clear context
        CURRENT_CONTEXT = None;
    }
    
    tracing::info!("✅ Forge shutdown complete");
    Ok(())
}

// Helper functions

fn ensure_initialized() -> Result<()> {
    unsafe {
        if (*std::ptr::addr_of!(FORGE_INSTANCE)).is_none() {
            anyhow::bail!("Forge not initialized. Call initialize_forge() first.");
        }
    }
    Ok(())
}

fn detect_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    
    loop {
        // Check for .dx directory
        if current.join(".dx").exists() {
            return Ok(current);
        }
        
        // Check for .git directory
        if current.join(".git").exists() {
            return Ok(current);
        }
        
        // Move up one directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            // Reached filesystem root
            break;
        }
    }
    
    // Default to current directory
    Ok(std::env::current_dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::ToolOutput;
    
    struct TestTool;
    
    impl DxTool for TestTool {
        fn name(&self) -> &str { "test-tool" }
        fn version(&self) -> &str { "1.0.0" }
        fn priority(&self) -> u32 { 50 }
        fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
            Ok(ToolOutput::success())
        }
    }
    
    #[test]
    fn test_lifecycle() {
        // Note: Can only test once per process due to Once
        initialize_forge().ok();
        
        let result = register_tool(Box::new(TestTool));
        assert!(result.is_ok());
        
        let ctx = get_tool_context();
        assert!(ctx.is_ok());
    }
}
