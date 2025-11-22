//! # DX Forge - Production-Ready VCS and Orchestration Engine
//!
//! Forge is the orchestration backbone for the DX tools ecosystem, providing:
//! - Content-addressable storage with SHA-256 blob hashing
//! - Git-compatible versioning with traffic branch safety system
//! - Dual-watcher architecture (LSP + File System monitoring)
//! - Tool orchestration with priority-based execution and dependency resolution
//! - Component injection for zero-bloat dependency management
//! - Semantic versioning with dependency resolution
//! - Pattern detection for dx-tools (dxButton, dxiIcon, dxfRoboto, etc.)
//! - R2 component caching and injection
//! - Production error handling with retry logic
//!
//! ## Architecture Overview
//!
//! Forge eliminates node_modules bloat by detecting code patterns via LSP,
//! injecting only needed components directly into user files, and coordinating
//! DX tool execution with traffic branch safety logic.
//!
//! ### Core Components
//!
//! - **Orchestrator**: Coordinates tool execution with lifecycle hooks, circular dependency detection
//! - **Dual-Watcher**: Monitors LSP + file system changes with pattern detection
//! - **Traffic Branch System**: Green (auto), Yellow (merge), Red (manual) for safe updates
//! - **Storage Layer**: Content-addressable blobs with R2 cloud sync
//! - **Version Manager**: Semantic versioning with compatibility checking
//! - **Pattern Detector**: Identifies dx-tool patterns in source code
//! - **Injection Manager**: Fetches and caches components from R2 storage
//!
//! ## Quick Start - Tool Development
//!
//! ```rust,no_run
//! use dx_forge::{DxTool, ExecutionContext, ToolOutput, Orchestrator};
//! use anyhow::Result;
//!
//! struct MyDxTool;
//!
//! impl DxTool for MyDxTool {
//!     fn name(&self) -> &str { "dx-mytool" }
//!     fn version(&self) -> &str { "1.0.0" }
//!     fn priority(&self) -> u32 { 50 }
//!     
//!     fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
//!         // Your tool logic here
//!         Ok(ToolOutput::success())
//!     }
//! }
//!
//! fn main() -> Result<()> {
//!     let mut orchestrator = Orchestrator::new(".")?;
//!     orchestrator.register_tool(Box::new(MyDxTool))?;
//!     let _outputs = orchestrator.execute_all()?;
//!     Ok(())
//! }
//! ```
//!
//! ## Quick Start - Change Detection
//!
//! ```rust,no_run
//! use dx_forge::{DualWatcher, FileChange};
//! use anyhow::Result;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut watcher = DualWatcher::new()?;
//!     let project_root = PathBuf::from(".");
//! 
//!     // Start watching for changes
//!     watcher.start(&project_root).await?;
//! 
//!     // Subscribe to the unified change stream
//!     let mut rx = watcher.receiver();
//! 
//!     while let Ok(change) = rx.recv().await {
//!         println!("Change detected: {:?} ({:?})", change.path, change.source);
//!     }
//! 
//!     Ok(())
//! }
//! ```

// Core modules
pub mod context;
pub mod crdt;
pub mod server;
pub mod storage;
pub mod sync;

// Core library - NEW unified API
pub mod core;

// ========================================================================
// The 132 Eternal API Functions (v0.1.0)
// ========================================================================
pub mod api;

// Legacy watcher module (for CLI compatibility)
#[path = "watcher_legacy/mod.rs"]
pub mod watcher_legacy;

// Production orchestration modules (v1.0.0)
pub mod orchestrator;
pub mod watcher;

// DX Tools support modules
pub mod version;
pub mod patterns;
pub mod injection;
pub mod error;

// Phase 5 modules
pub mod auto_update;
pub mod profiler;
pub mod cache;

// ========================================================================
// Primary Public API - Forge Unified Interface
// ========================================================================

pub use core::{
    Forge, ForgeConfig,
    LifecycleEvent, ToolId, ToolStatus,
    EditorInfo, EditorType, OutputStrategy,
    GeneratedFileInfo,
};

// ========================================================================
// Re-export orchestration types (public API)
// ========================================================================

pub use orchestrator::{
    Conflict, DxTool, ExecutionContext, Orchestrator, OrchestratorConfig, ToolOutput,
    TrafficAnalyzer, TrafficBranch,
};

pub use watcher::{ChangeKind, ChangeSource, DualWatcher, FileChange, FileWatcher, LspWatcher};

// ========================================================================
// Re-export storage types
// ========================================================================

pub use context::{ComponentStateManager, UpdateResult};
pub use crdt::{Operation, OperationType, Position};
pub use storage::{Database, OperationLog};

// ========================================================================
// Re-export DX tools support types
// ========================================================================

pub use version::{
    ToolInfo, ToolRegistry, ToolSource, Version, VersionReq,
    Snapshot, SnapshotId, SnapshotManager, Branch, ToolState, FileSnapshot, SnapshotDiff,
};
pub use patterns::{DxToolType, PatternDetector, PatternMatch};
pub use injection::{CacheStats, ComponentMetadata, InjectionManager};
pub use error::{categorize_error, EnhancedError, EnhancedResult, ErrorCategory, RetryPolicy, ToEnhanced, with_retry};

// ========================================================================
// Legacy exports (deprecated in favor of new Forge API)
// ========================================================================

#[deprecated(since = "1.0.0", note = "use `Forge` instead")]
pub use watcher::DualWatcher as ForgeWatcher;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ========================================================================
// Re-export The 132 Eternal API Functions
// ========================================================================

// Core Lifecycle & System Orchestration (4 functions)
pub use api::lifecycle::{
    initialize_forge, register_tool, get_tool_context, shutdown_forge,
};

// Version Governance & Package Identity (6 functions)
pub use api::version::{
    declare_tool_version, enforce_exact_version, require_forge_minimum,
    current_forge_version, query_active_package_variant, activate_package_variant,
};

// Pipeline Execution & Orchestration (7 functions)
pub use api::pipeline::{
    execute_pipeline, execute_tool_immediately, get_resolved_execution_order,
    temporarily_override_pipeline_order, restart_current_pipeline,
    suspend_pipeline_execution, resume_pipeline_execution,
};

// Triple-Path Reactivity Engine (5 functions)
pub use api::reactivity::{
    trigger_realtime_event, trigger_debounced_event, trigger_idle_event,
    begin_batch_operation, end_batch_operation,
};

// Safe File Application & Branching Decision Engine (15 functions)
pub use api::branching::{
    apply_changes, apply_changes_with_preapproved_votes, apply_changes_force_unchecked,
    preview_proposed_changes, automatically_accept_green_conflicts,
    prompt_review_for_yellow_conflicts, automatically_reject_red_conflicts,
    revert_most_recent_application, submit_branching_vote,
    register_permanent_branching_voter, query_predicted_branch_color,
    is_change_guaranteed_safe, issue_immediate_veto, reset_branching_engine_state,
    BranchColor, BranchingVote,
};
// Note: FileChange is already exported from watcher module

// Global Event Bus & Observability (9 functions)
pub use api::events::{
    publish_event, subscribe_to_event_stream, emit_tool_started_event,
    emit_tool_completed_event, emit_pipeline_started_event, emit_pipeline_completed_event,
    emit_package_installation_begin, emit_package_installation_success,
    emit_security_violation_detected, emit_magical_config_injection, ForgeEvent,
};

// The One True Configuration System (16 functions)
pub use api::config::{
    get_active_config_file_path, reload_configuration_manifest,
    enable_live_config_watching, inject_full_config_section_at_cursor,
    expand_config_placeholder, jump_to_config_section, validate_config_in_realtime,
    provide_config_completion_suggestions, auto_format_config_file,
    perform_config_schema_migration, inject_style_tooling_config,
    inject_authentication_config, inject_ui_framework_config,
    inject_icon_system_config, inject_font_system_config,
    inject_media_pipeline_config, inject_package_specific_config,
};

// CI/CD & Workspace Orchestration (8 functions)
pub use api::cicd::{
    trigger_ci_cd_pipeline, register_ci_stage, query_current_ci_status,
    abort_running_ci_job, synchronize_monorepo_workspace, detect_workspace_root,
    list_all_workspace_members, broadcast_change_to_workspace,
};

// .dx/ Directory Management (10 functions)
pub use api::dx_directory::{
    get_dx_directory_path, get_dx_binary_storage_path, cache_tool_offline_binary,
    load_tool_offline_binary, commit_current_dx_state, checkout_dx_state,
    list_dx_history, show_dx_state_diff, push_dx_state_to_remote,
    pull_dx_state_from_remote,
};

// Offline-First Architecture (5 functions)
pub use api::offline::{
    detect_offline_mode, force_offline_operation, download_missing_tool_binaries,
    verify_binary_integrity_and_signature, update_tool_binary_atomically,
};

// Cart System (8 functions)
pub use api::cart::{
    stage_item_in_cart, commit_entire_cart, commit_cart_immediately,
    clear_cart_completely, remove_specific_cart_item, get_current_cart_contents,
    export_cart_as_shareable_json, import_cart_from_json, CartItem,
};

// Package Management (8 functions)
pub use api::packages::{
    install_package_with_variant, uninstall_package_safely, update_package_intelligently,
    list_all_installed_packages, search_dx_package_registry, pin_package_to_exact_version,
    fork_existing_variant, publish_your_variant, PackageInfo,
};

// Generated Code Governance (5 functions)
pub use api::codegen::{
    mark_code_region_as_dx_generated, is_region_dx_generated,
    allow_safe_manual_edit_of_generated_code, claim_full_ownership_of_file,
    release_ownership_of_file,
};

// Developer Experience & Editor Integration (26 functions)
pub use api::dx_experience::{
    project_root_directory, path_to_forge_manifest, dx_global_cache_directory,
    create_watcher_ignored_scratch_file, log_structured_tool_action,
    schedule_task_for_idle_time, await_editor_idle_state, request_user_attention_flash,
    open_file_and_reveal_location, display_inline_code_suggestion,
    apply_user_accepted_suggestion, show_onboarding_welcome_tour,
    execute_full_security_audit, generate_comprehensive_project_report,
    display_dx_command_palette, open_embedded_dx_terminal,
    trigger_ai_powered_suggestion, apply_ai_generated_completion,
    open_dx_explorer_sidebar, update_dx_status_bar_indicator,
};

// Testing forge logging
// test logging
// test event
// event2
// test edit
