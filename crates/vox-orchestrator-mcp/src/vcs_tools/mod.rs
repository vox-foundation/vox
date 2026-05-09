//! JJ-inspired VCS tool handlers for the Vox MCP server.
//!
//! Covers: snapshots, operation log (oplog), conflicts, workspaces, change tracking,
//! and pre-commit secret scanning.

mod change;
mod conflicts;
pub mod branch_tools;
pub mod commit_tools;
mod oplog;
mod parse;
pub mod secret_scan;
mod snapshots;
mod workspaces;

pub use change::*;
pub use conflicts::*;
pub use oplog::*;
pub use secret_scan::scan_for_secrets;
pub use secret_scan::{SecretKind, SecretMatch};
pub use snapshots::*;
pub use workspaces::*;
