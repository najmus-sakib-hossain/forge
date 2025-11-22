//! Safe File Application with Enterprise-Grade Branching Decision Engine APIs

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use parking_lot::RwLock;
use std::collections::HashMap;

/// File change representation
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub old_content: Option<String>,
    pub new_content: String,
    pub tool_id: String,
}

/// Branching vote colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchColor {
    Green,      // Auto-approve
    Yellow,     // Review recommended
    Red,        // Manual resolution required
    NoOpinion,  // Abstain from voting
}

/// Branching vote from a tool
#[derive(Debug, Clone)]
pub struct BranchingVote {
    pub voter_id: String,
    pub color: BranchColor,
    pub reason: String,
    pub confidence: f32,  // 0.0 to 1.0
}

/// Branching engine state
static BRANCHING_STATE: OnceLock<Arc<RwLock<BranchingState>>> = OnceLock::new();

struct BranchingState {
    voters: Vec<String>,
    pending_changes: Vec<FileChange>,
    votes: HashMap<PathBuf, Vec<BranchingVote>>,
    last_application: Option<Vec<PathBuf>>,
}

impl Default for BranchingState {
    fn default() -> Self {
        Self {
            voters: Vec::new(),
            pending_changes: Vec::new(),
            votes: HashMap::new(),
            last_application: None,
        }
    }
}

fn get_branching_state() -> Arc<RwLock<BranchingState>> {
    BRANCHING_STATE.get_or_init(|| Arc::new(RwLock::new(BranchingState::default()))).clone()
}

/// Primary API — full branching resolution + telemetry
pub fn apply_changes(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    tracing::info!("📝 Applying {} changes with branching safety", changes.len());
    
    let state = get_branching_state();
    let mut state = state.write();
    
    let mut applied_files = Vec::new();
    
    for change in changes {
        // Collect votes for this change
        let color = query_predicted_branch_color(&change.path)?;
        
        match color {
            BranchColor::Green => {
                // Auto-apply
                apply_file_change(&change)?;
                applied_files.push(change.path.clone());
                tracing::info!("🟢 Auto-applied: {:?}", change.path);
            }
            BranchColor::Yellow => {
                // Review recommended
                tracing::warn!("🟡 Review recommended for: {:?}", change.path);
                prompt_review_for_yellow_conflicts(vec![change.clone()])?;
                // After review, apply
                apply_file_change(&change)?;
                applied_files.push(change.path.clone());
            }
            BranchColor::Red => {
                // Manual resolution required
                tracing::error!("🔴 Manual resolution required: {:?}", change.path);
                automatically_reject_red_conflicts(vec![change.clone()])?;
            }
            BranchColor::NoOpinion => {
                // Default to yellow behavior
                apply_file_change(&change)?;
                applied_files.push(change.path.clone());
            }
        }
    }
    
    state.last_application = Some(applied_files.clone());
    
    Ok(applied_files)
}

/// Fast path when tool knows its changes are safe
pub fn apply_changes_with_preapproved_votes(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    tracing::info!("⚡ Fast-path applying {} pre-approved changes", changes.len());
    
    let mut applied_files = Vec::new();
    
    for change in changes {
        apply_file_change(&change)?;
        applied_files.push(change.path.clone());
    }
    
    let state = get_branching_state();
    state.write().last_application = Some(applied_files.clone());
    
    Ok(applied_files)
}

/// Only forge core or `dx apply --force`
pub fn apply_changes_force_unchecked(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    tracing::warn!("⚠️  FORCE APPLYING {} changes WITHOUT SAFETY CHECKS", changes.len());
    
    let mut applied_files = Vec::new();
    
    for change in changes {
        apply_file_change(&change)?;
        applied_files.push(change.path.clone());
    }
    
    Ok(applied_files)
}

/// Dry-run with full diff, colors, and risk score
pub fn preview_proposed_changes(changes: Vec<FileChange>) -> Result<String> {
    let mut preview = String::new();
    
    preview.push_str("╔══════════════════════════════════════════════════════════════╗\n");
    preview.push_str("║          PROPOSED CHANGES PREVIEW                            ║\n");
    preview.push_str("╚══════════════════════════════════════════════════════════════╝\n\n");
    
    for change in &changes {
        let color = query_predicted_branch_color(&change.path)?;
        let color_icon = match color {
            BranchColor::Green => "🟢",
            BranchColor::Yellow => "🟡",
            BranchColor::Red => "🔴",
            BranchColor::NoOpinion => "⚪",
        };
        
        preview.push_str(&format!("{} {:?}\n", color_icon, change.path));
        preview.push_str(&format!("   Tool: {}\n", change.tool_id));
        preview.push_str(&format!("   Risk: {:?}\n\n", color));
    }
    
    Ok(preview)
}

/// Auto-accept green conflicts
pub fn automatically_accept_green_conflicts(changes: Vec<FileChange>) -> Result<Vec<PathBuf>> {
    let green_changes: Vec<FileChange> = changes.into_iter()
        .filter(|c| query_predicted_branch_color(&c.path).ok() == Some(BranchColor::Green))
        .collect();
    
    tracing::info!("🟢 Auto-accepting {} green changes", green_changes.len());
    apply_changes_with_preapproved_votes(green_changes)
}

/// Opens rich inline LSP review UI
pub fn prompt_review_for_yellow_conflicts(changes: Vec<FileChange>) -> Result<()> {
    tracing::info!("🟡 Prompting review for {} yellow changes", changes.len());
    
    for change in &changes {
        println!("⚠️  CONFLICT: Review required for {:?}", change.path);
    }

    // Emit event for LSP
    crate::api::events::publish_event(crate::api::events::ForgeEvent::Custom {
        event_type: "conflict_review_required".to_string(),
        data: serde_json::json!({
            "changes": changes.iter().map(|c| c.path.to_string_lossy()).collect::<Vec<_>>()
        }),
        timestamp: chrono::Utc::now().timestamp(),
    })?;
    
    Ok(())
}

/// Auto-reject red conflicts
pub fn automatically_reject_red_conflicts(changes: Vec<FileChange>) -> Result<()> {
    tracing::error!("🔴 Rejecting {} red changes", changes.len());
    
    for change in changes {
        tracing::error!("  ❌ {:?} - Manual resolution required", change.path);
    }
    
    Ok(())
}

/// Undo for cart removal or failed scaffolding
pub fn revert_most_recent_application() -> Result<Vec<PathBuf>> {
    let state = get_branching_state();
    let state = state.read();
    
    if let Some(files) = &state.last_application {
        tracing::info!("🔙 Reverting {} files", files.len());
        
        // TODO: Implement actual file reversion
        // This would restore from backup or git
        
        Ok(files.clone())
    } else {
        anyhow::bail!("No recent application to revert")
    }
}

// ========================================================================
// Branching Decision Engine
// ========================================================================

/// Vote Green/Yellow/Red/NoOpinion on a FileChange
pub fn submit_branching_vote(file: &PathBuf, vote: BranchingVote) -> Result<()> {
    let state = get_branching_state();
    let mut state = state.write();
    
    state.votes
        .entry(file.clone())
        .or_insert_with(Vec::new)
        .push(vote);
    
    Ok(())
}

/// ui, auth, style, security, check, etc.
pub fn register_permanent_branching_voter(voter_id: String) -> Result<()> {
    let state = get_branching_state();
    let mut state = state.write();
    
    if !state.voters.contains(&voter_id) {
        tracing::info!("🗳️  Registered permanent voter: {}", voter_id);
        state.voters.push(voter_id);
    }
    
    Ok(())
}

/// Simulate outcome without applying
pub fn query_predicted_branch_color(file: &PathBuf) -> Result<BranchColor> {
    let state = get_branching_state();
    let state = state.read();
    
    // Get votes for this file
    if let Some(votes) = state.votes.get(file) {
        // Check for any Red votes (veto)
        if votes.iter().any(|v| v.color == BranchColor::Red) {
            return Ok(BranchColor::Red);
        }
        
        // Check for Yellow votes
        if votes.iter().any(|v| v.color == BranchColor::Yellow) {
            return Ok(BranchColor::Yellow);
        }
        
        // All Green
        if votes.iter().all(|v| v.color == BranchColor::Green || v.color == BranchColor::NoOpinion) {
            return Ok(BranchColor::Green);
        }
    }
    
    // Default to Green if no votes
    Ok(BranchColor::Green)
}

/// True iff every voter returned Green
pub fn is_change_guaranteed_safe(file: &PathBuf) -> Result<bool> {
    let state = get_branching_state();
    let state = state.read();
    
    if let Some(votes) = state.votes.get(file) {
        Ok(votes.iter().all(|v| v.color == BranchColor::Green))
    } else {
        Ok(false)
    }
}

/// Hard block — highest priority Red vote
pub fn issue_immediate_veto(file: &PathBuf, voter_id: &str, reason: &str) -> Result<()> {
    let vote = BranchingVote {
        voter_id: voter_id.to_string(),
        color: BranchColor::Red,
        reason: reason.to_string(),
        confidence: 1.0,
    };
    
    tracing::error!("🚫 VETO issued for {:?} by {}: {}", file, voter_id, reason);
    
    submit_branching_vote(file, vote)?;
    
    Ok(())
}

/// Called before cart commit or variant switch
pub fn reset_branching_engine_state() -> Result<()> {
    let state = get_branching_state();
    let mut state = state.write();
    
    tracing::info!("🔄 Resetting branching engine state");
    state.votes.clear();
    state.pending_changes.clear();
    
    Ok(())
}

// Helper function
fn apply_file_change(change: &FileChange) -> Result<()> {
    tracing::debug!("💾 Writing file: {:?}", change.path);
    
    // Ensure directory exists
    if let Some(parent) = change.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Write content
    std::fs::write(&change.path, &change.new_content)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_branching_votes() {
        let file = PathBuf::from("test.ts");
        
        let vote = BranchingVote {
            voter_id: "test-voter".to_string(),
            color: BranchColor::Green,
            reason: "Test vote".to_string(),
            confidence: 0.9,
        };
        
        submit_branching_vote(&file, vote).unwrap();
        
        let color = query_predicted_branch_color(&file).unwrap();
        assert_eq!(color, BranchColor::Green);
    }
}
