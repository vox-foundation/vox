//! Turso-backed Arca [`CodeStore`]: CAS (`objects`), logical `names`, and typed SQL tables.
//!
//! **Cross-crate SQL access:** the connection lives in the public field [`CodeStore::conn`]. For a
//! stable borrowed accessor (used by `vox-db` research paths and other crates), prefer
//! [`CodeStore::connection`](crate::arca_store::CodeStore::connection) (see `store/ops.rs`) instead of
//! reaching for `.conn` ad hoc in new call sites.

pub mod types;

/// Default relative path for the project Arca [`CodeStore`] SQLite file (under the repo/working tree).
pub const DEFAULT_PROJECT_STORE_PATH: &str = ".vox/store.db";

pub use types::{
    AgentDefEntry, ArtifactEntry, BehaviorEventEntry, BuilderSessionEntry, CodexChangeLogEntry,
    CommandFrequencyEntry, ComponentEntry, EmbeddingEntry, EndpointReliabilityEntry, ExecutionEntry, KnowledgeNodeSummary,
    LearnedPatternEntry, LogExecutionParams, LogInteractionParams, MemoryEntry,
    PackageSearchResult, PublishArtifactParams, RegisterAgentParams, ReviewEntry, SaveMemoryParams,
    SaveSnippetParams, ScheduledEntry, SessionTurnEntry, SkillExecutionParams, SkillExecutionRow, SkillManifestEntry, SkillReliabilityReport, SnippetEntry,
    StoreError, TrainingPair, TypedStreamEventEntry, UserEntry, WorkflowExecutionRow,
};


mod open;
mod ops;
mod ops_agents;
mod ops_cas;
mod ops_codex;
mod ops_learning;
mod ops_ludus;
mod ops_memory;
mod ops_orchestrator;
