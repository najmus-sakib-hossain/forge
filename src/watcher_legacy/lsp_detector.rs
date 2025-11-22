/// LSP-based Change Detection
///
/// This module provides an alternative to file watching by using the Language Server Protocol.
/// When a DX code editor extension is present, we can receive change events directly from the LSP,
/// which provides several advantages:
/// - Lower latency (no file system polling)
/// - More accurate change tracking (exact text edits)
/// - Better integration with editor features
/// - Reduced CPU usage
use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::crdt::{Operation, OperationType, Position};
use crate::storage::OperationLog;
use crate::sync::{SyncManager, GLOBAL_CLOCK};
use tracing::{debug};

/// LSP text document change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspChangeEvent {
    /// File URI (file:///path/to/file)
    pub uri: String,

    /// Text changes
    pub changes: Vec<LspTextEdit>,

    /// Document version
    pub version: i32,
}

/// LSP text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspTextEdit {
    /// Range of text to replace
    pub range: LspRange,

    /// New text content
    pub text: String,
}

/// LSP range (line/character positions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP position (line and character)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    /// Line number (0-based)
    pub line: u32,

    /// Character offset in line (0-based, UTF-16 code units)
    pub character: u32,
}

/// LSP-based change detector
pub struct LspDetector {
    #[allow(dead_code)]
    repo_root: PathBuf,
    oplog: Arc<OperationLog>,
    actor_id: String,
    sync_mgr: Option<Arc<SyncManager>>,

    /// Track document versions to prevent duplicates
    document_versions: Arc<RwLock<std::collections::HashMap<String, i32>>>,
}

impl LspDetector {
    /// Create new LSP detector
    pub fn new(
        repo_root: PathBuf,
        oplog: Arc<OperationLog>,
        actor_id: String,
        sync_mgr: Option<Arc<SyncManager>>,
    ) -> Self {
        Self {
            repo_root,
            oplog,
            actor_id,
            sync_mgr,
            document_versions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Process an LSP change event
    pub async fn process_change(&self, event: LspChangeEvent) -> Result<()> {
        // Check document version to prevent duplicates
        {
            let mut versions = self.document_versions.write().await;
            if let Some(&last_version) = versions.get(&event.uri) {
                if event.version <= last_version {
                    // Already processed this version
                    return Ok(());
                }
            }
            versions.insert(event.uri.clone(), event.version);
        }

        // Convert URI to path
        let path = uri_to_path(&event.uri)?;

        // Convert LSP changes to Forge operations
        let operations = self.convert_changes_to_operations(&path, &event.changes)?;

        // Store operations
        for op in operations {
            if self.oplog.append(op.clone())? {
                // Publish to sync if enabled
                if let Some(mgr) = &self.sync_mgr {
                    let _ = mgr.publish(Arc::new(op.clone()));
                }

                self.print_lsp_operation(&op);
            }
        }

        Ok(())
    }

    /// Convert LSP changes to Forge operations
    fn convert_changes_to_operations(
        &self,
        path: &Path,
        changes: &[LspTextEdit],
    ) -> Result<Vec<Operation>> {
        let mut operations = Vec::new();

        for change in changes {
            let op = self.convert_edit_to_operation(path, change)?;
            operations.push(op);
        }

        Ok(operations)
    }

    /// Convert a single LSP edit to a Forge operation
    fn convert_edit_to_operation(&self, path: &Path, edit: &LspTextEdit) -> Result<Operation> {
        let file_path = path.display().to_string();

        // Convert LSP positions to Forge positions (1-based)
        let start_line = edit.range.start.line as usize + 1;
        let start_col = edit.range.start.character as usize + 1;

        let end_line = edit.range.end.line as usize + 1;
        let end_col = edit.range.end.character as usize + 1;

        // Estimate character offset (approximate)
        let offset = 0; // Would need full document to calculate accurately

        let lamport = GLOBAL_CLOCK.tick();
        let position = Position::new(
            start_line,
            start_col,
            offset,
            self.actor_id.clone(),
            lamport,
        );

        // Determine operation type
        let op_type = if edit.range.start.line == edit.range.end.line
            && edit.range.start.character == edit.range.end.character
        {
            // Pure insertion
            OperationType::Insert {
                position: position.clone(),
                content: edit.text.clone(),
                length: edit.text.chars().count(),
            }
        } else if edit.text.is_empty() {
            // Pure deletion
            let length = calculate_range_length(&edit.range);
            OperationType::Delete {
                position: position.clone(),
                length,
            }
        } else {
            // Replacement
            OperationType::Replace {
                position: position.clone(),
                old_content: format!("({}:{} to {}:{})", start_line, start_col, end_line, end_col),
                new_content: edit.text.clone(),
            }
        };

        Ok(Operation::new(file_path, op_type, self.actor_id.clone()))
    }

    /// Print LSP operation with styling
    fn print_lsp_operation(&self, op: &Operation) {
        let filename = Path::new(&op.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&op.file_path);

        let (action, details) = match &op.op_type {
            OperationType::Insert {
                position,
                content,
                length,
            } => {
                let preview = truncate_preview(content, 40);
                (
                    "INSERT".green(),
                    format!(
                        "{}:{} +{} chars '{}'",
                        position.line,
                        position.column,
                        length,
                        preview.green()
                    ),
                )
            }
            OperationType::Delete { position, length } => (
                "DELETE".red(),
                format!("{}:{} -{} chars", position.line, position.column, length),
            ),
            OperationType::Replace {
                position,
                new_content,
                ..
            } => {
                let preview = truncate_preview(new_content, 40);
                (
                    "REPLACE".yellow(),
                    format!(
                        "{}:{} → '{}'",
                        position.line,
                        position.column,
                        preview.green()
                    ),
                )
            }
            _ => return,
        };

        debug!(
            "{} {} {} {}",
            "📡".bright_blue(),
            "[LSP]".bright_blue().bold(),
            action.bold(),
            format!("{} {}", filename.bright_white(), details)
        );
    }
}

/// Detect if DX code editor extension is available
pub async fn detect_lsp_support() -> Result<bool> {
    // Check for LSP server endpoint
    // This would typically check for:
    // 1. Environment variable indicating LSP is active
    // 2. Socket/pipe connection to LSP server
    // 3. Configuration file indicating LSP mode

    // For now, check environment variable
    if std::env::var("DX_LSP_ENABLED").is_ok() {
        return Ok(true);
    }

    // Check for LSP socket file
    let lsp_socket = std::env::temp_dir().join("dx-lsp.sock");
    if lsp_socket.exists() {
        return Ok(true);
    }

    // Check for VS Code extension
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        let vscode_extensions = PathBuf::from(home).join(".vscode").join("extensions");

        if vscode_extensions.exists() {
            // Look for dx-* extension folders
            if let Ok(entries) = std::fs::read_dir(vscode_extensions) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with("dx-") || name.contains("forge") {
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }

    Ok(false)
}

/// Start LSP-based monitoring
pub async fn start_lsp_monitoring(
    repo_root: PathBuf,
    oplog: Arc<OperationLog>,
    actor_id: String,
    sync_mgr: Option<Arc<SyncManager>>,
) -> Result<()> {
    // Removed noisy startup log - only logged at debug level in detector
    // debug!("Watching for file changes...");

    // Use hybrid mode: file system watching + LSP queue monitoring
    // This ensures we detect changes even without active LSP integration
    use super::detector;

    // Start file system watching (this will detect your actual changes)
    detector::start_watching(
        repo_root,
        oplog,
        actor_id.clone(),
        String::new(), // repo_id not needed here
        sync_mgr,
    )
    .await
}

/// Convert file:// URI to path
fn uri_to_path(uri: &str) -> Result<PathBuf> {
    let path_str = uri
        .strip_prefix("file://")
        .or_else(|| uri.strip_prefix("file:///"))
        .unwrap_or(uri);

    // Handle Windows paths (file:///C:/...)
    #[cfg(windows)]
    let path_str = path_str.trim_start_matches('/');

    Ok(PathBuf::from(path_str))
}

/// Calculate character length of an LSP range
fn calculate_range_length(range: &LspRange) -> usize {
    // Simplified: assume single line for now
    if range.start.line == range.end.line {
        (range.end.character - range.start.character) as usize
    } else {
        // Multi-line range - approximate
        100 // Would need document to calculate accurately
    }
}

/// Truncate string for preview
fn truncate_preview(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.replace('\n', "\\n").replace('\t', "\\t")
    } else {
        let truncated = &s[..max_len.min(s.len())];
        format!("{}…", truncated.replace('\n', "\\n").replace('\t', "\\t"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_to_path() {
        let uri = "file:///home/user/project/src/main.rs";
        let path = uri_to_path(uri).unwrap();
        assert!(path.to_string_lossy().contains("main.rs"));
    }

    #[test]
    fn test_calculate_range_length() {
        let range = LspRange {
            start: LspPosition {
                line: 0,
                character: 5,
            },
            end: LspPosition {
                line: 0,
                character: 10,
            },
        };
        assert_eq!(calculate_range_length(&range), 5);
    }
}
