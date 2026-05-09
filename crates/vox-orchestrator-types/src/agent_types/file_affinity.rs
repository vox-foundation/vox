//! File affinity types extracted to `vox-orchestrator-types` (2026-05-08).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Kind of access an agent requires on a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessKind {
    /// Read-only access (multiple agents can hold simultaneously).
    Read,
    /// Exclusive write access (only one agent at a time).
    Write,
}

/// A file path paired with the access kind required for a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileAffinity {
    /// Path the task touches.
    pub path: PathBuf,
    /// Required lock / sharing mode.
    pub access: AccessKind,
}

impl FileAffinity {
    pub fn read(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), access: AccessKind::Read }
    }

    pub fn write(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), access: AccessKind::Write }
    }
}
