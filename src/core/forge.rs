//! Main Forge struct - unified API for DX tools

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use crate::orchestrator::{Orchestrator, OrchestratorConfig};
use crate::watcher::{DualWatcher, FileChange};
use crate::version::{ToolRegistry, Version};
use crate::injection::InjectionManager;
use super::lifecycle::{LifecycleManager, ToolId, ToolStatus};
use super::tracking::GeneratedCodeTracker;
use super::editor_integration::EditorIntegration;


/// Main Forge instance - provides unified API for DX tools
pub struct Forge {
    config: ForgeConfig,
    orchestrator: Arc<RwLock<Orchestrator>>,
    watcher: Option<Arc<RwLock<DualWatcher>>>,
    registry: Arc<RwLock<ToolRegistry>>,
    _injection_manager: Arc<RwLock<InjectionManager>>,
    lifecycle_manager: Arc<RwLock<LifecycleManager>>,
    code_tracker: Arc<RwLock<GeneratedCodeTracker>>,
    _editor_integration: Arc<RwLock<EditorIntegration>>,
}

/// Configuration for Forge instance
#[derive(Clone, Debug)]
pub struct ForgeConfig {
    /// Root directory of the project
    pub project_root: PathBuf,
    
    /// Forge data directory (.dx/forge)
    pub forge_dir: PathBuf,
    
    /// Automatically start file watching
    pub auto_watch: bool,
    
    /// Enable LSP integration
    pub enable_lsp: bool,
    
    /// Enable version control features
    pub enable_versioning: bool,
    
    /// Number of worker threads for orchestration
    pub worker_threads: usize,
}

impl ForgeConfig {
    /// Create default configuration for a project
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        let forge_dir = project_root.join(".dx").join("forge");
        
        Self {
            project_root,
            forge_dir,
            auto_watch: true,
            enable_lsp: true,
            enable_versioning: true,
            worker_threads: num_cpus::get(),
        }
    }
    
    /// Disable automatic file watching
    pub fn without_auto_watch(mut self) -> Self {
        self.auto_watch = false;
        self
    }
    
    /// Disable LSP integration
    pub fn without_lsp(mut self) -> Self {
        self.enable_lsp = false;
        self
    }
    
    /// Set custom forge directory
    pub fn with_forge_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.forge_dir = dir.as_ref().to_path_buf();
        self
    }
}

impl Forge {
    /// Create a new Forge instance for a project
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use dx_forge::Forge;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let forge = Forge::new(".")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let config = ForgeConfig::new(project_root);
        Self::with_config(config)
    }
    
    /// Create Forge instance with custom configuration
    pub fn with_config(config: ForgeConfig) -> Result<Self> {
        // Ensure forge directory exists
        std::fs::create_dir_all(&config.forge_dir)
            .context("Failed to create forge directory")?;
        
        // Initialize components
        let orchestrator_config = OrchestratorConfig {
            parallel: false,
            fail_fast: true,
            max_concurrent: config.worker_threads,
            traffic_branch_enabled: true,
        };
        
        let orchestrator = Arc::new(RwLock::new(
            Orchestrator::with_config(config.project_root.clone(), orchestrator_config)
                .context("Failed to initialize orchestrator")?,
        ));
        
        let registry = Arc::new(RwLock::new(
            ToolRegistry::new(&config.forge_dir)
                .context("Failed to initialize tool registry")?,
        ));
        
        let injection_manager = Arc::new(RwLock::new(
            InjectionManager::new(&config.forge_dir)
                .context("Failed to initialize injection manager")?,
        ));
        
        let lifecycle_manager = Arc::new(RwLock::new(LifecycleManager::new()));
        
        let code_tracker = Arc::new(RwLock::new(
            GeneratedCodeTracker::new(&config.forge_dir)
                .context("Failed to initialize code tracker")?,
        ));
        
        let editor_integration = Arc::new(RwLock::new(EditorIntegration::new()));
        
        // Initialize watcher if auto_watch is enabled
        let watcher = if config.auto_watch {
            let dual_watcher = DualWatcher::new()
                .context("Failed to initialize file watcher")?;
            Some(Arc::new(RwLock::new(dual_watcher)))
        } else {
            None
        };
        
        Ok(Self {
            config,
            orchestrator,
            watcher,
            registry,
            _injection_manager: injection_manager,
            lifecycle_manager,
            code_tracker,
            _editor_integration: editor_integration,
        })
    }
    
    /// Get the project root directory
    pub fn project_root(&self) -> &Path {
        &self.config.project_root
    }

    /// Get the orchestrator instance
    pub fn orchestrator(&self) -> Arc<RwLock<Orchestrator>> {
        self.orchestrator.clone()
    }
    
    /// Get the forge data directory
    pub fn forge_dir(&self) -> &Path {
        &self.config.forge_dir
    }
    
    // ========================================================================
    // Tool Lifecycle Management
    // ========================================================================
    
    /// Get the current status of a tool
    pub fn get_tool_status(&self, id: ToolId) -> Option<ToolStatus> {
        self.lifecycle_manager.read().get_status(id)
    }
    
    /// Subscribe to lifecycle events
    pub fn subscribe_lifecycle_events(&self) -> broadcast::Receiver<super::lifecycle::LifecycleEvent> {
        self.lifecycle_manager.read().subscribe()
    }
    
    // ========================================================================
    // File Watching & Change Detection
    // ========================================================================
    
    /// Start watching a directory for changes
    pub async fn watch_directory(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            let path_ref = path.as_ref();
            watcher.write().start(path_ref).await?;
            tracing::info!("Started watching directory: {:?}", path_ref);
            Ok(())
        } else {
            anyhow::bail!("File watching is disabled in configuration")
        }
    }
    
    /// Subscribe to file change events
    pub fn subscribe_changes(&self) -> Result<broadcast::Receiver<FileChange>> {
        if let Some(watcher) = &self.watcher {
            Ok(watcher.read().receiver())
        } else {
            anyhow::bail!("File watching is disabled in configuration")
        }
    }
    
    /// Stop file watching
    pub async fn stop_watching(&mut self) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.write().stop().await?;
            tracing::info!("Stopped file watching");
            Ok(())
        } else {
            Ok(())
        }
    }
    
    // ========================================================================
    // Generated Code Tracking
    // ========================================================================
    
    /// Track a file as being generated by a tool
    pub fn track_generated_file(
        &mut self,
        file: PathBuf,
        tool: &str,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<()> {
        self.code_tracker.write().track_file(file, tool, metadata)
    }
    
    /// Get all files generated by a specific tool
    pub fn get_generated_files(&self, tool: &str) -> Vec<PathBuf> {
        self.code_tracker.read().get_files_by_tool(tool)
    }
    
    /// Remove all files generated by a tool
    pub async fn cleanup_generated(&mut self, tool: &str) -> Result<Vec<PathBuf>> {
        self.code_tracker.write().cleanup_tool_files(tool).await
    }
    
    // ========================================================================
    // Tool Registry & Versioning
    // ========================================================================
    
    /// Check if a tool is registered
    pub fn is_tool_registered(&self, name: &str) -> bool {
        self.registry.read().is_registered(name)
    }
    
    /// Get version of a registered tool
    pub fn get_tool_version(&self, name: &str) -> Option<Version> {
        self.registry.read().version(name).cloned()
    }
    
    /// List all registered tools
    pub fn list_tools(&self) -> Vec<String> {
        self.registry
            .read()
            .list()
            .iter()
            .map(|info| info.name.clone())
            .collect()
    }
}

impl Drop for Forge {
    fn drop(&mut self) {
        // Cleanup: stop all running tools
        if let Some(mut lifecycle) = self.lifecycle_manager.try_write() {
            if let Err(e) = lifecycle.stop_all() {
                tracing::error!("Failed to stop all tools during cleanup: {}", e);
            }
        }
        
        tracing::debug!("Forge instance dropped");
    }
}
