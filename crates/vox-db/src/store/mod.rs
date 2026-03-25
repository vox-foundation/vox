//! Turso-backed Arca [`VoxDb`]: CAS (`objects`), logical `names`, and typed SQL tables.
//!
//! **Cross-crate SQL access:** the connection lives in the public field [`VoxDb::conn`]. For a
//! stable borrowed accessor (used by `vox-db` research paths and other crates), prefer
//! [`VoxDb::connection`](crate::store::VoxDb::connection) (see `store/ops.rs`) instead of
//! reaching for `.conn` ad hoc in new call sites.

pub mod types;

/// Default relative path for the project Arca [`VoxDb`] SQLite file (under the repo/working tree).
pub const DEFAULT_PROJECT_STORE_PATH: &str = ".vox/store.db";

pub use types::{
    A2AMessageRow, AgentDefEntry, AgentEventRow, ArtifactEntry, BehaviorEventEntry,
    BenchmarkEventRow, BuildRunRow, BuilderSessionEntry, CloudDispatchRow, CodexChangeLogEntry,
    CommandFrequencyEntry, ComponentEntry, CrateSampleRow, EmbeddingEntry,
    EndpointReliabilityEntry, ExecutionEntry, KnowledgeNodeSummary, LearnedPatternEntry,
    LocalTrainRow, LogExecutionParams, LogInteractionParams, MemoryEntry, PackageSearchResult,
    PlanNodeRow, PlanSessionRow, PlanVersionRow, PublishArtifactParams, QuestionRow,
    RegisterAgentParams, ReviewEntry, SaveMemoryParams, SaveSnippetParams, ScheduledEntry,
    SessionEventRow, SessionRow, SessionTurnEntry, SkillExecutionParams, SkillExecutionRow,
    SkillManifestEntry, SkillReliabilityReport, SnippetEntry, StoreError, ThroughputProfileRow,
    TrainingPair, TypedStreamEventEntry, UserEntry, WarningRow, WorkflowExecutionRow,
};

pub use ops_build::{BuildHealthSummary, CrateSample, RegressionRow};
pub use ops_mens_cloud::CloudCostSummary;

mod open;
mod ops;
mod ops_agents;
pub mod ops_build;
mod ops_cas;
mod ops_codex;
mod ops_learning;
mod ops_ludus;
mod ops_memory;
mod ops_mens_cloud;
mod ops_news;
mod ops_orchestrator;
mod ops_planning;
