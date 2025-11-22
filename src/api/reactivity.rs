//! Triple-Path Reactivity Engine APIs

use anyhow::Result;
use std::sync::{Arc, OnceLock};
use parking_lot::RwLock;
use tokio::time::{Duration, sleep};
use std::path::PathBuf;

/// Reactivity state management
static REACTIVITY_STATE: OnceLock<Arc<RwLock<ReactivityState>>> = OnceLock::new();

struct ReactivityState {
    in_batch: bool,
    batch_start: Option<std::time::Instant>,
}

impl Default for ReactivityState {
    fn default() -> Self {
        Self {
            in_batch: false,
            batch_start: None,
        }
    }
}

fn get_reactivity_state() -> Arc<RwLock<ReactivityState>> {
    REACTIVITY_STATE.get_or_init(|| Arc::new(RwLock::new(ReactivityState::default()))).clone()
}

/// Instant path — called on every DidChangeTextDocument
///
/// Triggers immediate tool execution for realtime feedback (e.g., syntax highlighting, diagnostics).
pub fn trigger_realtime_event(file: PathBuf, _content: String) -> Result<()> {
    tracing::debug!("⚡ Realtime event: {:?}", file);
    
    // Update execution context with changed file
    if let Some(forge) = unsafe { crate::api::lifecycle::FORGE_INSTANCE.as_ref() } {
        let forge = forge.read();
        let orchestrator = forge.orchestrator();
        let mut orchestrator = orchestrator.write();
        
        // Add to changed files if not already present
        if !orchestrator.context().changed_files.contains(&file) {
            orchestrator.context_mut().changed_files.push(file.clone());
        }
        
        // Execute realtime pipeline (if we had one, for now just log)
        tracing::info!("⚡ Triggering realtime analysis for {:?}", file);
        
        // In a real production system, we might run a subset of fast tools here
        // orchestrator.execute_subset(&["dx-lint", "dx-format"])?;
    }
    
    Ok(())
}

/// 300ms debounce — safe default for style, lint, format
///
/// Triggers tool execution after a 300ms debounce period to avoid excessive runs.
pub async fn trigger_debounced_event(file: PathBuf, _content: String) -> Result<()> {
    tracing::debug!("⏱️  Debounced event: {:?} (300ms)", file);
    
    // Wait for debounce period
    sleep(Duration::from_millis(300)).await;
    
    // TODO: Execute debounced tools
    
    Ok(())
}

/// Only when user idle ≥2s — i18n, security, bundle analysis
///
/// Triggers tool execution only when the user has been idle for at least 2 seconds.
pub async fn trigger_idle_event(file: PathBuf) -> Result<()> {
    tracing::debug!("😴 Idle event: {:?} (≥2s idle)", file);
    
    // Wait for idle period
    sleep(Duration::from_secs(2)).await;
    
    // TODO: Execute idle-tier tools
    
    Ok(())
}

/// Marks start of atomic multi-file operation
///
/// Batches multiple file changes together to avoid redundant tool executions.
pub fn begin_batch_operation() -> Result<()> {
    let state = get_reactivity_state();
    let mut state = state.write();
    
    tracing::info!("📦 Beginning batch operation");
    state.in_batch = true;
    state.batch_start = Some(std::time::Instant::now());
    
    Ok(())
}

/// Marks completion — triggers idle queue + resets branching
///
/// Ends the batch operation and triggers all queued events.
pub fn end_batch_operation() -> Result<()> {
    let state = get_reactivity_state();
    let mut state = state.write();
    
    if let Some(start) = state.batch_start {
        let duration = start.elapsed();
        tracing::info!("✅ Batch operation completed in {:.2}s", duration.as_secs_f64());
    }
    
    state.in_batch = false;
    state.batch_start = None;
    
    // TODO: Flush all queued events
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_batch_operation() {
        begin_batch_operation().unwrap();
        end_batch_operation().unwrap();
    }
    
    #[tokio::test]
    async fn test_debounced_event() {
        let file = PathBuf::from("test.ts");
        let result = trigger_debounced_event(file, "content".to_string()).await;
        assert!(result.is_ok());
    }
}
