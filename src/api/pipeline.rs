//! Pipeline Execution & Orchestration APIs

use anyhow::Result;
use parking_lot::RwLock;
use std::sync::{Arc, OnceLock};

#[cfg(test)]
use std::sync::Mutex;

/// Pipeline execution state
#[cfg(not(test))]
static PIPELINE_STATE: OnceLock<Arc<RwLock<PipelineState>>> = OnceLock::new();

// In test builds we keep pipeline state thread-local so that different tests
// (which may run on different worker threads) don't interfere with each
// other's view of pipeline suspension / active pipeline.
#[cfg(test)]
thread_local! {
    static TEST_PIPELINE_STATE: Arc<RwLock<PipelineState>> = Arc::new(RwLock::new(PipelineState::default()));
}

// When running tests, multiple test functions may touch the global
// pipeline APIs concurrently. To avoid flaky behaviour due to
// cross-test interference, we serialize pipeline operations behind a
// simple process-wide mutex. In production builds this guard is
// completely omitted.
#[cfg(test)]
static PIPELINE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(test)]
fn pipeline_test_guard() -> std::sync::MutexGuard<'static, ()> {
    PIPELINE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("pipeline test lock poisoned")
}

struct PipelineState {
    active_pipeline: Option<String>,
    execution_order: Vec<String>,
    is_suspended: bool,
    override_order: Option<Vec<String>>,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self {
            active_pipeline: None,
            execution_order: Vec::new(),
            is_suspended: false,
            override_order: None,
        }
    }
}

#[cfg(not(test))]
fn get_pipeline_state() -> Arc<RwLock<PipelineState>> {
    PIPELINE_STATE
        .get_or_init(|| Arc::new(RwLock::new(PipelineState::default())))
        .clone()
}

#[cfg(test)]
fn get_pipeline_state() -> Arc<RwLock<PipelineState>> {
    TEST_PIPELINE_STATE.with(|state| state.clone())
}

/// Executes named pipeline ("default" | "auth" | "deploy" | "ci")
pub fn execute_pipeline(pipeline_name: &str) -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();
    
    if state.is_suspended {
        anyhow::bail!("Pipeline execution is suspended");
    }
    
    tracing::info!("🎼 Executing pipeline: {}", pipeline_name);
    state.active_pipeline = Some(pipeline_name.to_string());
    
    // Get global forge instance
    let forge = unsafe {
        if let Some(forge) = crate::api::lifecycle::FORGE_INSTANCE.as_ref() {
            forge.clone()
        } else {
            anyhow::bail!("Forge not initialized");
        }
    };

    // Execute tools via orchestrator
    let orchestrator = forge.read().orchestrator();
    let mut orchestrator = orchestrator.write();
    
    // TODO: Filter tools based on pipeline name
    // For now, we execute all tools for the "default" pipeline
    if pipeline_name == "default" {
        let results = orchestrator.execute_all()?;
        
        // Update state with execution order
        state.execution_order = results.iter()
            .map(|o| "tool-id-placeholder".to_string()) // TODO: Get tool ID from output
            .collect();
            
        // Check for failures
        let failures: Vec<_> = results.iter().filter(|o| !o.success).collect();
        if !failures.is_empty() {
            anyhow::bail!("Pipeline execution failed: {} tools failed", failures.len());
        }
    } else {
        tracing::warn!("Pipeline '{}' not yet implemented, running default", pipeline_name);
        let _ = orchestrator.execute_all()?;
    }
    
    Ok(())
}

/// Highest priority execution — bypasses queue and debounce
pub fn execute_tool_immediately(tool_id: &str) -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    tracing::info!("⚡ Immediate execution: {}", tool_id);
    
    // TODO: Execute tool directly, bypassing normal queue
    
    Ok(())
}

/// Returns final Vec<ToolId> after topology sort
pub fn get_resolved_execution_order() -> Result<Vec<String>> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let state = state.read();
    
    if let Some(override_order) = &state.override_order {
        Ok(override_order.clone())
    } else {
        Ok(state.execution_order.clone())
    }
}

/// Used by traffic_branching and user experiments
pub fn temporarily_override_pipeline_order(new_order: Vec<String>) -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();
    
    tracing::info!("🔀 Temporarily overriding pipeline order");
    state.override_order = Some(new_order);
    
    Ok(())
}

/// Aborts and restarts active pipeline from scratch
pub fn restart_current_pipeline() -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let state = state.read();
    
    if let Some(pipeline) = &state.active_pipeline {
        let name = pipeline.clone();
        drop(state);
        
        tracing::info!("🔄 Restarting pipeline: {}", name);
        execute_pipeline(&name)?;
    } else {
        anyhow::bail!("No active pipeline to restart");
    }
    
    Ok(())
}

/// Pauses all tool execution until resumed
pub fn suspend_pipeline_execution() -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();
    
    tracing::info!("⏸️  Pipeline execution suspended");
    state.is_suspended = true;
    
    Ok(())
}

/// Continues from suspended state
pub fn resume_pipeline_execution() -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();
    
    tracing::info!("▶️  Pipeline execution resumed");
    state.is_suspended = false;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pipeline_execution() {
        assert!(execute_pipeline("default").is_ok());
    }
    
    #[test]
    fn test_suspend_resume() {
        suspend_pipeline_execution().unwrap();
        assert!(execute_pipeline("test").is_err());
        
        resume_pipeline_execution().unwrap();
        assert!(execute_pipeline("test").is_ok());
    }
}
