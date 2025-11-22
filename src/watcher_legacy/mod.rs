pub mod cache_warmer;
pub mod detector;
pub mod lsp_detector;

use anyhow::Result;
use colored::Colorize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;

use crate::crdt::Operation;
use crate::storage::{Database, OperationLog};
use crate::sync::{remote::connect_peer, SyncManager};

/// Rapid change notification - ultra-fast (<35µs typical, 1-2µs best case)
#[derive(Debug, Clone)]
pub struct RapidChange {
    /// File path that changed
    pub path: String,
    /// Detection time in microseconds (typically 1-2µs, max 35µs)
    pub time_us: u64,
    /// Sequence number for ordering
    pub sequence: u64,
}

/// Quality change notification - detailed analysis (<60µs typical)
#[derive(Debug, Clone)]
pub struct QualityChange {
    /// File path that changed
    pub path: String,
    /// Detected operations
    pub operations: Vec<Operation>,
    /// Detection time in microseconds (typically <60µs)
    pub time_us: u64,
    /// Total processing time (rapid + quality)
    pub total_us: u64,
}

/// Forge event types emitted by the watcher
#[derive(Debug, Clone)]
pub enum ForgeEvent {
    /// Rapid notification - immediate feedback (<35µs)
    Rapid {
        path: String,
        time_us: u64,
        sequence: u64,
    },
    /// Quality notification - full details (<60µs after rapid)
    Quality {
        path: String,
        operations: Vec<Operation>,
        time_us: u64,
        total_us: u64,
    },
}

/// Forge watcher - monitors file changes and emits rapid + quality events
pub struct ForgeWatcher {
    pub repo_root: PathBuf,
    pub oplog: Arc<OperationLog>,
    pub actor_id: String,
    pub repo_id: String,
    pub sync_mgr: Option<Arc<SyncManager>>,
}

impl ForgeWatcher {
    /// Create a new forge watcher
    pub async fn new<P: Into<PathBuf>>(
        path: P,
        enable_sync: bool,
        peers: Vec<String>,
    ) -> Result<Self> {
        let path_buf = path.into();
        let repo_root = path_buf.canonicalize().unwrap_or(path_buf);
        let forge_dir = repo_root.join(".dx/forge");

        let db = Database::new(&forge_dir)?;
        db.initialize()?;
        let oplog = Arc::new(OperationLog::new(Arc::new(db)));

        // Load config
        let config_raw = tokio::fs::read_to_string(forge_dir.join("config.json")).await?;
        let config: serde_json::Value = serde_json::from_str(&config_raw)?;
        let actor_id = config["actor_id"].as_str().unwrap().to_string();
        let repo_id = config["repo_id"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let mut hasher = Sha256::new();
                let path_string = repo_root.to_string_lossy().into_owned();
                hasher.update(path_string.as_bytes());
                format!("local-{:x}", hasher.finalize())
            });

        let sync_mgr = if enable_sync {
            Some(Arc::new(SyncManager::new()))
        } else {
            None
        };

        // Connect to remote peers if provided
        if let (Some(mgr), true) = (&sync_mgr, !peers.is_empty()) {
            for url in peers {
                let _ = connect_peer(
                    &url,
                    actor_id.clone(),
                    repo_id.clone(),
                    mgr.as_ref().clone(),
                    oplog.clone(),
                )
                .await;
            }
        }

        // Warm OS page cache
        let _ = tokio::task::spawn_blocking({
            let repo_root_clone = repo_root.clone();
            move || cache_warmer::warm_cache(&repo_root_clone)
        })
        .await??;

        Ok(Self {
            repo_root,
            oplog,
            actor_id,
            repo_id,
            sync_mgr,
        })
    }

    /// Run the watcher (blocking)
    pub async fn run(self) -> Result<()> {
        // Check if LSP support is available
        let lsp_available = lsp_detector::detect_lsp_support().await?;

        if lsp_available {
            // Use LSP-based detection
            lsp_detector::start_lsp_monitoring(
                self.repo_root,
                self.oplog,
                self.actor_id,
                self.sync_mgr,
            )
            .await
        } else {
                // Fall back to file system watching
                tracing::info!(
                    "{} {} mode (no LSP extension detected)",
                    "👁️".bright_yellow(),
                    "File watching".bright_cyan().bold()
                );
            detector::start_watching(
                self.repo_root,
                self.oplog,
                self.actor_id,
                self.repo_id,
                self.sync_mgr,
            )
            .await
        }
    }
}

// Legacy function for backward compatibility
pub async fn watch(path: PathBuf, enable_sync: bool, peers: Vec<String>) -> Result<()> {
    let watcher = ForgeWatcher::new(path, enable_sync, peers).await?;
    watcher.run().await
}
