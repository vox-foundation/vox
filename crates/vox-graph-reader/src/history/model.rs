//! Row types for the repo-history store (spec §4.1).
use serde::{Deserialize, Serialize};

/// Bump when a row shape changes. Each version gets its own store directory.
pub const SCHEMA_VERSION: u32 = 1;

/// Version-namespaced store directory name. A binary only reads and writes rows its own
/// code produced, so builds from different branches never wipe each other's store.
pub fn store_dir_name() -> String {
    format!("s{SCHEMA_VERSION}-e{}", crate::ast::EXTRACTOR_VERSION)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommitRec {
    pub sha: String,
    /// First parent; `load` walks this chain to decide which rows are valid.
    pub parent: Option<String>,
    /// Committer time, unix seconds.
    pub ts: i64,
    pub author: String,
    /// First `Co-Authored-By` name, e.g. `Claude Opus 5`.
    pub agent: Option<String>,
    pub subject: String,
    pub body: String,
    pub is_merge: bool,
    /// Subjects of `merge^1..merge^2` (empty for non-merges). Built pending owner confirmation.
    pub inner_subjects: Vec<String>,
    /// True when every changed file is mechanical.
    pub mechanical: bool,
    /// What the subject claims (`fmt`, `lint`, `regen`, `hakari`); never sets `mechanical`.
    pub subject_hint: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Generated,
    Whitespace,
    SymbolNeutral,
}

/// Symbol ids with the file path stripped (`Type::method`, `free_fn`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct SymbolDelta {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChangeRec {
    pub sha: String,
    pub path: String,
    pub old_path: Option<String>,
    /// git name-status letter: `A`, `M`, `D`, `R`, `C`, `T`.
    pub status: String,
    pub added: u32,
    pub deleted: u32,
    /// `None` when the file type is not parsed by the extractor.
    pub symbols: Option<SymbolDelta>,
    pub mechanical: bool,
    pub reasons: Vec<Reason>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageKind {
    Rename,
    Split,
    Merge,
    Move,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LineageMethod {
    GitRename,
    Symbol,
    Line,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LineageRec {
    pub sha: String,
    pub from: String,
    pub to: String,
    pub kind: LineageKind,
    pub method: LineageMethod,
    /// Share of `from` that went to `to` (0..=1).
    pub weight: f32,
    /// Symbols moved or lines matched.
    pub evidence: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    pub schema_version: u32,
    pub extractor_version: String,
    pub last_sha: Option<String>,
}

impl State {
    pub fn fresh() -> Self {
        State {
            schema_version: SCHEMA_VERSION,
            extractor_version: crate::ast::EXTRACTOR_VERSION.to_string(),
            last_sha: None,
        }
    }

    /// True when rows on disk were produced by the current code. In a version-namespaced
    /// directory a mismatch only means corruption.
    pub fn is_current(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
            && self.extractor_version == crate::ast::EXTRACTOR_VERSION
    }
}

#[derive(Clone, Debug, Default)]
pub struct HistoryData {
    pub commits: Vec<CommitRec>,
    pub changes: Vec<ChangeRec>,
    pub lineage: Vec<LineageRec>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_is_current_and_stale_extractor_is_not() {
        assert!(State::fresh().is_current());
        let stale = State {
            extractor_version: "3".into(),
            ..State::fresh()
        };
        assert!(!stale.is_current());
    }

    #[test]
    fn enums_serialize_snake_case_and_dir_name_carries_versions() {
        assert_eq!(
            serde_json::to_string(&Reason::SymbolNeutral).unwrap(),
            "\"symbol_neutral\""
        );
        assert_eq!(
            serde_json::to_string(&LineageMethod::GitRename).unwrap(),
            "\"git_rename\""
        );
        assert_eq!(
            store_dir_name(),
            format!("s{SCHEMA_VERSION}-e{}", crate::ast::EXTRACTOR_VERSION)
        );
    }
}
