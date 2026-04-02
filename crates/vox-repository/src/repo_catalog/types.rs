use serde::{Deserialize, Serialize};

#[cfg(feature = "json-schema")]
use schemars::JsonSchema;

use super::{
    MAX_FILE_BYTES_DEFAULT, MAX_FILE_READ_BYTES_DEFAULT, MAX_FILES_PER_REPO_DEFAULT,
    MAX_HISTORY_COMMITS_DEFAULT, MAX_TEXT_MATCHES_PER_REPO_DEFAULT,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RepoAccessMode {
    Local,
    RemoteMcp,
    RemoteGitHost,
    RemoteSearchService,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RepoCapability {
    ReadFile,
    ListFiles,
    TextSearch,
    SemanticSearch,
    HistorySearch,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(default)]
pub struct RemoteAdapterHints {
    pub mcp_url: Option<String>,
    pub git_host_api_base: Option<String>,
    pub git_host_owner: Option<String>,
    pub git_host_repo: Option<String>,
    pub search_service_url: Option<String>,
    pub search_namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(default)]
pub struct RepositoryDescriptor {
    pub repository_id: Option<String>,
    pub display_name: String,
    pub root_path: Option<String>,
    pub origin_url: Option<String>,
    pub provider: Option<String>,
    pub default_ref: Option<String>,
    pub access_mode: RepoAccessMode,
    pub capabilities: Vec<RepoCapability>,
    pub remote: Option<RemoteAdapterHints>,
    pub metadata: Option<serde_json::Value>,
}

impl Default for RepositoryDescriptor {
    fn default() -> Self {
        Self {
            repository_id: None,
            display_name: String::new(),
            root_path: None,
            origin_url: None,
            provider: None,
            default_ref: None,
            access_mode: RepoAccessMode::Local,
            capabilities: Vec::new(),
            remote: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct RepoCatalog {
    pub schema_version: u32,
    pub repositories: Vec<RepositoryDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedRepositoryDescriptor {
    pub repository_id: Option<String>,
    pub declared_repository_id: Option<String>,
    pub display_name: String,
    pub root_path: Option<String>,
    pub resolved_root: Option<String>,
    pub git_root: Option<String>,
    pub origin_url: Option<String>,
    pub provider: Option<String>,
    pub default_ref: Option<String>,
    pub access_mode: RepoAccessMode,
    pub capabilities: Vec<RepoCapability>,
    pub remote: Option<RemoteAdapterHints>,
    pub metadata: Option<serde_json::Value>,
    pub resolution_status: String,
    pub resolution_error: Option<String>,
    pub repository_id_mismatch: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedRepoCatalog {
    pub schema_version: u32,
    pub manifest_path: String,
    pub repositories: Vec<ResolvedRepositoryDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoCatalogRefreshResult {
    pub manifest_path: String,
    pub snapshot_path: String,
    pub catalog: ResolvedRepoCatalog,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossRepoQueryTrace {
    pub trace_id: String,
    pub correlation_id: String,
    pub conversation_id: Option<String>,
    pub workspace_repository_id: String,
    pub target_repository_ids: Vec<String>,
    pub source_plane: String,
    pub query_backend: String,
    pub query_kind: String,
    pub started_at_ms: i64,
    pub completed_at_ms: i64,
    pub latency_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoQuerySkippedRepository {
    pub display_name: String,
    pub repository_id: Option<String>,
    pub access_mode: RepoAccessMode,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoTextMatch {
    pub repository_id: String,
    pub display_name: String,
    pub root: String,
    pub path: String,
    pub line_number: usize,
    pub line_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoTextSearchResponse {
    pub trace: CrossRepoQueryTrace,
    pub repositories_considered: usize,
    pub repositories_queried: usize,
    pub result_count: usize,
    pub skipped: Vec<RepoQuerySkippedRepository>,
    pub hits: Vec<RepoTextMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoFileRead {
    pub repository_id: String,
    pub display_name: String,
    pub root: String,
    pub path: String,
    pub bytes_read: usize,
    pub truncated: bool,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoFileReadResponse {
    pub trace: CrossRepoQueryTrace,
    pub repositories_considered: usize,
    pub repositories_queried: usize,
    pub result_count: usize,
    pub skipped: Vec<RepoQuerySkippedRepository>,
    pub files: Vec<RepoFileRead>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoHistoryEntry {
    pub repository_id: String,
    pub display_name: String,
    pub root: String,
    pub commit: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoHistoryResponse {
    pub trace: CrossRepoQueryTrace,
    pub repositories_considered: usize,
    pub repositories_queried: usize,
    pub result_count: usize,
    pub skipped: Vec<RepoQuerySkippedRepository>,
    pub commits: Vec<RepoHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(default)]
pub struct QueryTextParams {
    pub query: String,
    pub repository_ids: Option<Vec<String>>,
    pub case_insensitive: bool,
    pub regex: bool,
    pub max_matches_per_repo: usize,
    pub max_files_per_repo: usize,
    pub max_file_bytes: usize,
    pub conversation_id: Option<String>,
}

impl Default for QueryTextParams {
    fn default() -> Self {
        Self {
            query: String::new(),
            repository_ids: None,
            case_insensitive: true,
            regex: false,
            max_matches_per_repo: MAX_TEXT_MATCHES_PER_REPO_DEFAULT,
            max_files_per_repo: MAX_FILES_PER_REPO_DEFAULT,
            max_file_bytes: MAX_FILE_BYTES_DEFAULT,
            conversation_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(default)]
pub struct QueryFileParams {
    pub path: String,
    pub repository_ids: Option<Vec<String>>,
    pub max_bytes: usize,
    pub conversation_id: Option<String>,
}

impl Default for QueryFileParams {
    fn default() -> Self {
        Self {
            path: String::new(),
            repository_ids: None,
            max_bytes: MAX_FILE_READ_BYTES_DEFAULT,
            conversation_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(default)]
pub struct QueryHistoryParams {
    pub repository_ids: Option<Vec<String>>,
    pub path: Option<String>,
    pub contains: Option<String>,
    pub max_commits: usize,
    pub conversation_id: Option<String>,
}

impl Default for QueryHistoryParams {
    fn default() -> Self {
        Self {
            repository_ids: None,
            path: None,
            contains: None,
            max_commits: MAX_HISTORY_COMMITS_DEFAULT,
            conversation_id: None,
        }
    }
}
