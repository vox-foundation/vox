//! KB Library MCP tools (`vox_kb_search`, `vox_kb_get`, `vox_kb_cite`, `vox_kb_status`,
//! `vox_kb_reindex`) over the curated `docs/src` index built by [`vox_search::library`].
//!
//! Self-contained on purpose: Phase 3 deletes `handlers.rs` / `params.rs`, so nothing here
//! imports from them. Every handler reads only the corpus-filtered vox-db readers and the
//! Tantivy index; none touches the file system with a caller-supplied path (AP-15).

use std::path::Path;
use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use vox_db::VoxDb;
use vox_search::library::{IndexDeps, IndexReport, LibraryHit, index_library, lexical, search};

use crate::params::ToolResult;
use crate::server_state::ServerState;

const REM_KB_DB: &str =
    "Attach VoxDb (VOX_DB_PATH / VOX_DB_URL) to the MCP server for KB operations.";
const REM_KB_REINDEX: &str = "Run vox_kb_reindex to build the Library index.";

/// Longest accepted `vox_kb_search` query; longer input is rejected, never truncated.
pub(crate) const MAX_QUERY_BYTES: usize = 1024;
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 50;

/// `vox_kb_search` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KbSearchParams {
    /// Free-text query (1..=1024 bytes).
    pub query: String,
    /// Maximum hits, default 10, clamped to 1..=50.
    #[serde(default)]
    pub limit: Option<u32>,
}

fn require_db(state: &ServerState) -> Result<Arc<VoxDb>, String> {
    state.db.clone().ok_or_else(|| REM_KB_DB.to_string())
}

fn err_json(msg: impl Into<String>, remediation: &str) -> String {
    ToolResult::<()>::err_with_remediation(msg, remediation).to_json()
}

fn clamp_limit(limit: Option<u32>) -> usize {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize
}

/// Trimmed query, or why it is unacceptable.
fn validate_query(query: &str) -> Result<&str, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("query must not be empty".into());
    }
    if q.len() > MAX_QUERY_BYTES {
        return Err(format!("query exceeds {MAX_QUERY_BYTES} bytes"));
    }
    Ok(q)
}

pub(crate) fn chunk_uri(hash: &str) -> String {
    format!("kb://chunk/{hash}")
}

pub(crate) fn doc_uri(rel_path: &str) -> String {
    format!("kb://doc/{rel_path}")
}

fn hit_json(h: &LibraryHit) -> serde_json::Value {
    json!({
        "chunk_hash": h.chunk_hash,
        "doc_path": h.doc_path,
        "title": h.title,
        "heading_path": h.heading_path,
        "anchor": h.anchor,
        "chunk_kind": h.chunk_kind,
        "snippet": h.snippet,
        "score": h.score,
        "uri": chunk_uri(&h.chunk_hash),
        "doc_uri": doc_uri(&h.doc_path),
    })
}

/// Search the Library index in `dir`.
async fn search_in(db: &VoxDb, dir: &Path, query: &str, limit: usize) -> String {
    match search(db, dir, query, limit).await {
        Ok(hits) => ToolResult::ok(json!({
            "lexical_backend": lexical::LEXICAL_BACKEND,
            "hits": hits.iter().map(hit_json).collect::<Vec<_>>(),
        }))
        .to_json(),
        Err(e) => err_json(format!("library search failed: {e}"), REM_KB_REINDEX),
    }
}

/// `vox_kb_search`: BM25 over the curated Library, ranked hits with citation-ready identity.
pub async fn kb_search(state: &ServerState, params: KbSearchParams) -> String {
    let query = match validate_query(&params.query) {
        Ok(q) => q,
        Err(e) => return err_json(e, "Pass a non-empty query of at most 1024 bytes."),
    };
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return err_json(e, REM_KB_DB),
    };
    let Some(dir) = lexical::default_index_dir() else {
        return err_json(
            "no Library index directory could be resolved",
            REM_KB_REINDEX,
        );
    };
    search_in(&db, &dir, query, clamp_limit(params.limit)).await
}

#[allow(dead_code)] // used by reindex_at; wired to vox_kb_reindex in the last task of 02-06
fn hasher(bytes: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(bytes))
}

/// Index `<repo_root>/docs/src` into `db` and `lexical_dir` with the vox-crypto hasher. The
/// trust tier is `trusted` (curated, merged to git; AP-12).
#[allow(dead_code)] // wired to vox_kb_reindex in the last task of 02-06
pub(crate) async fn reindex_at(
    db: &VoxDb,
    repo_root: &Path,
    lexical_dir: &Path,
    corpus_commit: Option<String>,
) -> Result<IndexReport, String> {
    let deps = IndexDeps {
        hasher: &hasher,
        corpus_commit,
        trust_tier: "trusted",
    };
    index_library(db, repo_root, lexical_dir, &deps)
        .await
        .map_err(|e| format!("reindex failed: {e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn parse(s: &str) -> serde_json::Value {
        serde_json::from_str(s).expect("tool json")
    }

    /// Temp repo with one doc, indexed into `<tmp>/idx/library`; `VOX_SEARCH_TANTIVY_ROOT`
    /// points at `<tmp>/idx` so `default_index_dir()` finds it. Caller holds `#[serial]`.
    async fn indexed_state() -> (TempDir, ServerState) {
        let tmp = tempfile::tempdir().unwrap();
        let doc = tmp.path().join("repo/docs/src/guide.md");
        std::fs::create_dir_all(doc.parent().unwrap()).unwrap();
        std::fs::write(
            &doc,
            "# Guide\n\nIntro text.\n\n## Install\nrun turbofrobnicate to install\n",
        )
        .unwrap();
        let idx_root = tmp.path().join("idx");
        // SAFETY: `#[serial]` — no concurrent env mutation in this crate's tests.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_SEARCH_TANTIVY_ROOT", &idx_root)
        };
        vox_config::snapshot::bump(&["VOX_SEARCH_TANTIVY_ROOT"]);
        let db = Arc::new(VoxDb::open_memory().await.unwrap());
        reindex_at(
            &db,
            &tmp.path().join("repo"),
            &idx_root.join("library"),
            None,
        )
        .await
        .unwrap();
        let state = ServerState::new_test().await.with_db_initialized(db).await;
        (tmp, state)
    }

    #[tokio::test]
    #[serial]
    async fn kb_search_tool_end_to_end() {
        let (_tmp, state) = indexed_state().await;
        let out = crate::dispatch::handle_tool_call(
            &state,
            "vox_kb_search",
            json!({"query": "turbofrobnicate"}),
        )
        .await
        .unwrap();
        let v = parse(&out);
        assert_eq!(v["success"], true, "{out}");
        let first = &v["data"]["hits"][0];
        assert_eq!(first["heading_path"], "Guide › Install", "{out}");
        let hash = first["chunk_hash"].as_str().unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(first["uri"], format!("kb://chunk/{hash}"));
        assert_eq!(first["doc_uri"], "kb://doc/docs/src/guide.md");
        assert_eq!(v["data"]["lexical_backend"], lexical::LEXICAL_BACKEND);
    }

    #[tokio::test]
    #[serial]
    async fn kb_search_rejects_bad_input() {
        let (_tmp, state) = indexed_state().await;
        for q in ["", "   \n\t", &"x".repeat(MAX_QUERY_BYTES + 1)] {
            let out = kb_search(
                &state,
                KbSearchParams {
                    query: q.to_string(),
                    limit: None,
                },
            )
            .await;
            assert_eq!(parse(&out)["success"], false, "query {:?}", q.len());
        }
        // Exactly at the cap is accepted (not truncated, not rejected).
        assert!(validate_query(&"x".repeat(MAX_QUERY_BYTES)).is_ok());
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(500)), 50);
        assert_eq!(clamp_limit(None), 10);
    }

    #[tokio::test]
    async fn kb_search_without_db() {
        let state = ServerState::new_test().await;
        let out = crate::dispatch::handle_tool_call(
            &state,
            "vox_kb_search",
            json!({"query": "anything"}),
        )
        .await
        .expect("must not be Unknown tool");
        let v = parse(&out);
        assert_eq!(v["success"], false);
        assert!(v["remediation"].as_str().is_some_and(|r| !r.is_empty()));
    }

    #[test]
    fn uri_shapes() {
        assert_eq!(chunk_uri("ab"), "kb://chunk/ab");
        assert_eq!(doc_uri("docs/src/a.md"), "kb://doc/docs/src/a.md");
    }
}
