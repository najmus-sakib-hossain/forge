//! Version control and tool registry
//!
//! Provides semantic versioning, version requirements, tool registry management,
//! and Git-like version control with snapshots and branching.

pub mod types;
pub mod registry;
pub mod snapshot;

pub use types::{Version, VersionReq};
pub use registry::{ToolInfo, ToolRegistry, ToolSource};
pub use snapshot::{
    Snapshot, SnapshotId, SnapshotManager, Branch, ToolState, FileSnapshot,
    SnapshotDiff,
};
