//! Request parameters and row shapes for [`crate::VoxDb`] (Arca on Turso).
//!
//! **SSOT:** Schema versions live under [`crate::schema`]. Public Codex naming and env vars are
//! defined in ADR 004 and the repo-root `AGENTS.md`—do not restate deployment semantics on
//! every struct; link those docs when adding new persisted fields.
//!
//! **Pattern:** Parameter structs that mirror a `VoxDb::…` method include a rustdoc link to that
//! method so call sites stay discoverable.

/// Parameters for logging an execution (workflow / activity).
pub struct LogExecutionParams<'a> {
    /// Workflow identifier.
    pub workflow_id: &'a str,
    /// Owning agent id.
    pub agent_id: Option<&'a str>,
    /// Skill identifier.
    pub skill_id: Option<&'a str>,
    /// Activity or step name.
    pub activity_name: &'a str,
    /// Outcome label (e.g. `ok`, `error`).
    pub status: &'a str,
    /// Attempt number (1-based).
    pub attempt: u32,
    /// Wall-clock latency in milliseconds.
    pub duration_ms: i64,
    /// Output payload size in bytes.
    pub output_size: i64,
    /// Serialized input payload, if any.
    pub input: Option<&'a [u8]>,
    /// Serialized output payload, if any.
    pub output: Option<&'a [u8]>,
    /// Error message, if failed.
    pub error: Option<&'a str>,
    /// Optional JSON or opaque options string.
    pub options: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::save_memory`].
#[derive(Debug, Clone)]
pub struct SaveMemoryParams<'a> {
    /// Owning agent id.
    pub agent_id: &'a str,
    /// Conversation or session id.
    pub session_id: &'a str,
    /// Memory category / type label.
    pub memory_type: &'a str,
    /// Stored text content.
    pub content: &'a str,
    /// Optional JSON metadata.
    pub metadata: Option<&'a str>,
    /// Relative importance score.
    pub importance: f64,
    /// Optional VCS snapshot correlation id.
    pub vcs_snapshot_id: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::log_interaction`].
#[derive(Debug, Clone)]
pub struct LogInteractionParams<'a> {
    /// Session id for the interaction.
    pub session_id: &'a str,
    /// End-user id when known.
    pub user_id: Option<&'a str>,
    /// User prompt text.
    pub prompt: &'a str,
    /// Model response text.
    pub response: &'a str,
    /// Model identifier or version string.
    pub model_version: &'a str,
    /// Wall-clock latency in milliseconds, if recorded.
    pub latency_ms: Option<i64>,
    /// Token usage, if recorded.
    pub token_count: Option<i64>,
}

/// Parameters for [`crate::VoxDb::save_snippet`].
#[derive(Debug, Clone)]
pub struct SaveSnippetParams<'a> {
    /// Programming language tag.
    pub language: &'a str,
    /// Short title.
    pub title: &'a str,
    /// Source code body.
    pub code: &'a str,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Optional comma-separated or JSON tags.
    pub tags: Option<&'a str>,
    /// Author user id, if any.
    pub author_id: Option<&'a str>,
    /// External reference (URL, path, etc.).
    pub source_ref: Option<&'a str>,
    /// Reserved for future embedding linkage.
    pub embedding_ref: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::publish_artifact`].
#[derive(Debug, Clone)]
pub struct PublishArtifactParams<'a> {
    /// Stable artifact id.
    pub id: &'a str,
    /// Type discriminator (e.g. model, dataset).
    pub artifact_type: &'a str,
    /// Human-readable name.
    pub name: &'a str,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Publishing user id.
    pub author_id: &'a str,
    /// CAS hash of artifact bytes.
    pub content_hash: &'a str,
    /// Semantic version string.
    pub version: &'a str,
    /// Optional tags string.
    pub tags: Option<&'a str>,
    /// Lifecycle status (e.g. draft, published).
    pub status: &'a str,
}

/// Parameters for [`crate::VoxDb::upsert_publication_manifest`].
#[derive(Debug, Clone)]
pub struct PublicationManifestParams<'a> {
    /// Stable publication id (e.g. news id or scientia artifact id).
    pub publication_id: &'a str,
    /// Logical content type (`news`, `scientia`, `paper`, ...).
    pub content_type: &'a str,
    /// Optional upstream source reference (URL, repo path, DOI, etc.).
    pub source_ref: Option<&'a str>,
    /// Human title.
    pub title: &'a str,
    /// Author name/identifier string.
    pub author: &'a str,
    /// Optional abstract.
    pub abstract_text: Option<&'a str>,
    /// Canonical markdown body.
    pub body_markdown: &'a str,
    /// Optional serialized citations payload.
    pub citations_json: Option<&'a str>,
    /// Optional serialized metadata payload.
    pub metadata_json: Option<&'a str>,
    /// Immutable digest for approval and publication checks.
    pub content_sha3_256: &'a str,
    /// Lifecycle status (`draft`, `approved`, `submitted`, `published`, ...).
    pub state: &'a str,
}

/// Parameters for [`crate::VoxDb::upsert_publication_media_asset`].
#[derive(Debug, Clone)]
pub struct PublicationMediaAssetParams<'a> {
    /// Stable publication id owning this asset.
    pub publication_id: &'a str,
    /// Repository/local asset reference key.
    pub asset_ref: &'a str,
    /// Media type (`video`, `image`, `dataset`, ...).
    pub media_type: &'a str,
    /// Optional storage URI (local path, object store URL, external id).
    pub storage_uri: Option<&'a str>,
    /// Lifecycle status (`pending`, `uploaded`, `failed`, ...).
    pub status: &'a str,
    /// Optional JSON metadata blob.
    pub metadata_json: Option<&'a str>,
}

/// Parameters for inserting or updating one [`external_submission_jobs`] row by idempotency key.
#[derive(Debug, Clone)]
pub struct ExternalSubmissionJobUpsertParams<'a> {
    pub publication_id: &'a str,
    pub content_sha3_256: &'a str,
    pub adapter: &'a str,
    pub operation: &'a str,
    pub idempotency_key: &'a str,
    pub status: &'a str,
    pub lock_owner: Option<&'a str>,
    pub lock_expires_at_ms: Option<i64>,
    pub next_retry_at_ms: Option<i64>,
    pub attempt_count: i64,
    pub last_error_class: Option<&'a str>,
    pub last_error_message: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::record_external_submission_attempt`].
#[derive(Debug, Clone)]
pub struct ExternalSubmissionAttemptParams<'a> {
    pub job_id: i64,
    pub http_status: Option<i32>,
    pub error_class: Option<&'a str>,
    pub retryable: bool,
    pub request_fingerprint: Option<&'a str>,
    pub response_fingerprint: Option<&'a str>,
    pub detail_json: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::insert_external_status_snapshot`].
#[derive(Debug, Clone)]
pub struct ExternalStatusSnapshotParams<'a> {
    pub adapter: &'a str,
    pub external_submission_id: &'a str,
    pub publication_id: &'a str,
    pub content_sha3_256: &'a str,
    pub snapshot_json: &'a str,
}

/// Parameters for [`crate::VoxDb::upsert_publication_external_link`].
#[derive(Debug, Clone)]
pub struct PublicationExternalLinkUpsertParams<'a> {
    pub publication_id: &'a str,
    pub content_sha3_256: &'a str,
    pub adapter: &'a str,
    pub link_kind: &'a str,
    pub link_value: &'a str,
    pub metadata_json: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::upsert_publication_external_revision`].
#[derive(Debug, Clone)]
pub struct PublicationExternalRevisionUpsertParams<'a> {
    pub publication_id: &'a str,
    pub content_sha3_256: &'a str,
    pub adapter: &'a str,
    pub external_revision: &'a str,
    pub metadata_json: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::register_agent`].
#[derive(Debug, Clone)]
pub struct RegisterAgentParams<'a> {
    /// Agent id (primary key).
    pub id: &'a str,
    /// Display name.
    pub name: &'a str,
    /// Optional description.
    pub description: Option<&'a str>,
    /// System prompt text, if any.
    pub system_prompt: Option<&'a str>,
    /// JSON tool manifest or config, if any.
    pub tools: Option<&'a str>,
    /// JSON model configuration, if any.
    pub model_config: Option<&'a str>,
    /// Owning user id, if any.
    pub owner_id: Option<&'a str>,
    /// Agent definition version.
    pub version: &'a str,
    /// Whether the agent is visible to other users.
    pub is_public: bool,
}
/// Parameters for `record_skill_execution`.
#[derive(Debug, Clone)]
pub struct SkillExecutionParams<'a> {
    pub skill_id: &'a str,
    pub version: &'a str,
    pub session_id: Option<&'a str>,
    pub workflow_id: Option<&'a str>,
    pub agent_id: Option<&'a str>,
    pub status: &'a str,
    pub duration_ms: i64,
    pub input_hash: Option<&'a str>,
    pub output_size: i64,
    pub error_kind: Option<&'a str>,
    pub reflection_score: Option<f64>,
}
