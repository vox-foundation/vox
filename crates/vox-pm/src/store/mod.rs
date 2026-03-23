//! Turso-backed Arca [`CodeStore`]: CAS (`objects`), logical `names`, and typed SQL tables.
//!
//! **Cross-crate SQL access:** the connection lives in the public field [`CodeStore::conn`]. For a
//! stable borrowed accessor (used by `vox-db` research paths and other crates), prefer
//! [`CodeStore::connection`](crate::store::CodeStore::connection) (see `store/ops.rs`) instead of
//! reaching for `.conn` ad hoc in new call sites.

pub mod types;

/// Default relative path for the project Arca [`CodeStore`] SQLite file (under the repo/working tree).
pub const DEFAULT_PROJECT_STORE_PATH: &str = ".vox/store.db";

pub use types::{
    AgentDefEntry, ArtifactEntry, BehaviorEventEntry, BuilderSessionEntry, CodexChangeLogEntry,
    CommandFrequencyEntry, ComponentEntry, EmbeddingEntry, EndpointReliabilityEntry, ExecutionEntry, KnowledgeNodeSummary,
    LearnedPatternEntry, LogExecutionParams, LogInteractionParams, MemoryEntry,
    PackageSearchResult, PublishArtifactParams, RegisterAgentParams, ReviewEntry, SaveMemoryParams,
    SaveSnippetParams, ScheduledEntry, SessionTurnEntry, SkillExecutionParams, SkillManifestEntry, SkillReliabilityReport, SnippetEntry,
    StoreError, TrainingPair, TypedStreamEventEntry, UserEntry,
};

/// Arca internal content-addressed store (libSQL / Turso).
pub struct CodeStore {
    /// libSQL connection; exposed for `vox-db` and other in-tree crates that run SQL on Arca.
    pub conn: turso::Connection,
    pub(crate) sync_db: Option<std::sync::Arc<turso::sync::Database>>,
}

mod open;
mod ops;
