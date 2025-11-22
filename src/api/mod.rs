//! # DX Forge Public API - The 132 Eternal Functions
//!
//! This module contains the complete, final, immutable public API for Forge v0.1.0.
//! All 132 functions are implemented here and organized by category.

// Core API modules
pub mod lifecycle;
pub mod version;
pub mod pipeline;
pub mod reactivity;
pub mod branching;
pub mod events;
pub mod config;
pub mod cicd;
pub mod dx_directory;
pub mod offline;
pub mod cart;
pub mod packages;
pub mod codegen;
pub mod dx_experience;

// Re-export all public API functions
pub use lifecycle::*;
pub use version::*;
pub use pipeline::*;
pub use reactivity::*;
pub use branching::*;
pub use events::*;
pub use config::*;
pub use cicd::*;
pub use dx_directory::*;
pub use offline::*;
pub use cart::*;
pub use packages::*;
pub use codegen::*;
pub use dx_experience::*;
