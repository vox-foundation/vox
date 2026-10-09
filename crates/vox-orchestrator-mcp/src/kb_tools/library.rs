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
use vox_db::kb_text::SanitizedText;
use vox_db::{KB_LIBRARY_CORPUS, LibraryChunkRow, VoxDb};
use vox_search::library::{
    IndexDeps, IndexReport, LibraryHit, index_library, lexical, search, status,
};

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

/// Sanitize every string leaf of a tool result (TRUST-05 at the injection boundary): curated
/// docs and rows indexed before the sanitizer existed reach a model only through here.
fn clean(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::String(s) => Value::String(SanitizedText::new(&s).into_string()),
        Value::Array(a) => Value::Array(a.into_iter().map(clean).collect()),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, clean(v))).collect()),
        other => other,
    }
}

fn ok_clean(v: serde_json::Value) -> ToolResult<serde_json::Value> {
    ToolResult::ok(clean(v))
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
        Ok(hits) => ok_clean(json!({
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

fn hasher(bytes: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(bytes))
}

/// One reindex at a time per process (AP-20); Tantivy's own writer lockfile guards other
/// processes. A contending caller is refused, never queued.
static KB_REINDEX_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Index `<repo_root>/docs/src` into `db` and `lexical_dir` with the vox-crypto hasher. The
/// trust tier is `tier_for_curated(false)` = trusted (curated, merged to git; AP-12). Refuses with `reindex already
/// running` while another call holds [`KB_REINDEX_LOCK`].
pub(crate) async fn reindex_at(
    db: &VoxDb,
    repo_root: &Path,
    lexical_dir: &Path,
    corpus_commit: Option<String>,
) -> Result<IndexReport, String> {
    let Ok(_guard) = KB_REINDEX_LOCK.try_lock() else {
        return Err("reindex already running".into());
    };
    let deps = IndexDeps {
        hasher: &hasher,
        corpus_commit,
        trust_tier: vox_db::tier_for_curated(false),
    };
    index_library(db, repo_root, lexical_dir, &deps)
        .await
        .map_err(|e| format!("reindex failed: {e:#}"))
}

/// Keys the dispatch layer or a caller may attach to any tool call; ignored here.
const META_KEYS: &[&str] = &[
    "agent_id",
    "session_id",
    "trace_id",
    "correlation_id",
    "task_id",
];

/// `vox_kb_reindex` has no parameters: the corpus root is fixed, so any other key (a `root`,
/// `path`, ...) is rejected before anything runs (T-02-16).
pub fn reject_reindex_args(args: &serde_json::Value) -> Result<(), String> {
    match args {
        serde_json::Value::Null => Ok(()),
        serde_json::Value::Object(m) => match m.keys().find(|k| !META_KEYS.contains(&k.as_str())) {
            None => Ok(()),
            Some(k) => Err(format!("vox_kb_reindex takes no arguments (got `{k}`)")),
        },
        _ => Err("vox_kb_reindex takes no arguments".into()),
    }
}

fn report_json(r: &IndexReport) -> serde_json::Value {
    json!({
        "docs_seen": r.docs_seen,
        "docs_unchanged": r.docs_unchanged,
        "docs_changed": r.docs_changed,
        "docs_pruned": r.docs_pruned,
        "chunks_inserted": r.chunks_inserted,
        "chunks_deleted": r.chunks_deleted,
        "chunks_kept": r.chunks_kept,
        "cards": r.cards,
        "errors": r.errors,
        "lexical_backend": r.lexical_backend,
        "lexical_docs": r.lexical_docs,
        "corpus_commit": r.corpus_commit,
        "duration_ms": r.duration_ms,
    })
}

/// `HEAD` of the repository, or `None` when git is unavailable or the root is not a checkout.
async fn head_commit(root: &Path) -> Option<String> {
    let out = crate::git_exec::GitExec::new(root)
        .run(&["rev-parse", "HEAD"])
        .await
        .ok()?;
    let sha = out.stdout.trim();
    (out.exit_code == 0 && !sha.is_empty()).then(|| sha.to_string())
}

/// `vox_kb_reindex`: rebuild the Library from `<repo root>/docs/src`; returns the index report.
pub async fn kb_reindex(state: &ServerState) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return err_json(e, REM_KB_DB),
    };
    let Some(dir) = lexical::default_index_dir() else {
        return err_json(
            "no Library index directory could be resolved",
            "Set VOX_SEARCH_TANTIVY_ROOT or ensure the Vox data directory is available.",
        );
    };
    let root = &state.repository.root;
    match reindex_at(&db, root, &dir, head_commit(root).await).await {
        Ok(r) => ok_clean(report_json(&r)).to_json(),
        Err(e) => err_json(
            e,
            "If a reindex is already running, wait for it to finish; otherwise fix the error and retry.",
        ),
    }
}

/// `vox_kb_get` arguments: exactly one of `path` or `chunk_hash`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KbGetParams {
    /// Repo-relative Library document path under `docs/src`, e.g. `docs/src/guide.md`.
    #[serde(default)]
    pub path: Option<String>,
    /// 64 lowercase hex chars: the BLAKE3 identity of one indexed chunk.
    #[serde(default)]
    pub chunk_hash: Option<String>,
}

/// `vox_kb_cite` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KbCiteParams {
    /// 64 lowercase hex chars: the identity of the chunk to cite.
    pub chunk_hash: String,
}

const MAX_PATH_BYTES: usize = 512;
const REM_PATH: &str = "Pass a repo-relative docs/src/**/*.md path as returned by vox_kb_search.";
const REM_HASH: &str = "Pass a 64-char lowercase hex chunk_hash as returned by vox_kb_search.";

/// A caller-supplied doc path is acceptable only if it is a corpus path by the same rule the
/// walker and the Phase 1 eval snapshot use (`docs/src/**.md`, no traversal, no absolute or
/// Windows-shaped path, no `archive`), and short. Errors start with `invalid path`.
pub(crate) fn validate_doc_path(path: &str) -> Result<&str, String> {
    if path.len() > MAX_PATH_BYTES {
        return Err(format!("invalid path: longer than {MAX_PATH_BYTES} bytes"));
    }
    if !vox_search::kb_ids::is_kb_corpus_path(path) {
        return Err("invalid path: not a Library corpus path (docs/src/**/*.md)".into());
    }
    Ok(path)
}

/// Exactly 64 lowercase hex characters. Errors start with `invalid chunk_hash`.
pub(crate) fn validate_chunk_hash(hash: &str) -> Result<&str, String> {
    if hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        Ok(hash)
    } else {
        Err("invalid chunk_hash: expected 64 lowercase hex characters".into())
    }
}

fn chunk_meta(c: &LibraryChunkRow) -> serde_json::Value {
    json!({
        "chunk_hash": c.chunk_hash,
        "chunk_kind": c.chunk_kind,
        "heading_path": c.heading_path,
        "anchor": c.anchor,
        "token_estimate": c.token_estimate,
        "uri": chunk_uri(&c.chunk_hash),
    })
}

/// Frontmatter is stored as JSON text; surface it as JSON, falling back to the raw string.
fn json_or_string(raw: &str) -> serde_json::Value {
    if raw.trim().is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_str(raw).unwrap_or_else(|_| json!(raw))
}

async fn get_by_path(db: &VoxDb, path: &str) -> String {
    let doc = match db.library_document_by_path(path).await {
        Ok(Some(d)) => d,
        Ok(None) => return err_json(format!("not indexed: {path}"), REM_KB_REINDEX),
        Err(e) => return err_json(format!("library lookup failed: {e}"), REM_KB_REINDEX),
    };
    let chunks = match db.library_doc_chunks(KB_LIBRARY_CORPUS, path).await {
        Ok(c) => c,
        Err(e) => return err_json(format!("library lookup failed: {e}"), REM_KB_REINDEX),
    };
    // Card chunks are retrieval summaries, not document text.
    let markdown = chunks
        .iter()
        .filter(|c| c.chunk_kind != "card")
        .map(|c| c.body.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    ok_clean(json!({
        "doc": {
            "rel_path": doc.rel_path,
            "title": doc.title,
            "frontmatter": json_or_string(&doc.frontmatter_json),
            "indexed_commit": doc.indexed_commit,
            "index_error": doc.index_error,
            "uri": doc_uri(&doc.rel_path),
        },
        "chunks": chunks.iter().map(chunk_meta).collect::<Vec<_>>(),
        "markdown": markdown,
    }))
    .to_json()
}

async fn get_by_hash(db: &VoxDb, hash: &str) -> String {
    match db.library_chunk_by_hash(hash).await {
        Ok(Some(c)) => {
            let mut chunk = chunk_meta(&c);
            chunk["body"] = json!(c.body);
            ok_clean(json!({
                "chunk": chunk,
                "doc_path": c.doc_path,
                "uri": chunk_uri(&c.chunk_hash),
                "doc_uri": doc_uri(&c.doc_path),
            }))
            .to_json()
        }
        Ok(None) => err_json(format!("not indexed: chunk {hash}"), REM_KB_REINDEX),
        Err(e) => err_json(format!("library lookup failed: {e}"), REM_KB_REINDEX),
    }
}

/// `vox_kb_get`: one indexed Library document or chunk, reconstructed from the index only.
pub async fn kb_get(state: &ServerState, params: KbGetParams) -> String {
    match (params.path.as_deref(), params.chunk_hash.as_deref()) {
        (Some(p), None) => match validate_doc_path(p) {
            Err(e) => err_json(e, REM_PATH),
            Ok(p) => match require_db(state) {
                Err(e) => err_json(e, REM_KB_DB),
                Ok(db) => get_by_path(&db, p).await,
            },
        },
        (None, Some(h)) => match validate_chunk_hash(h) {
            Err(e) => err_json(e, REM_HASH),
            Ok(h) => match require_db(state) {
                Err(e) => err_json(e, REM_KB_DB),
                Ok(db) => get_by_hash(&db, h).await,
            },
        },
        _ => err_json(
            "exactly one of path or chunk_hash is required",
            "Pass exactly one of `path` or `chunk_hash`.",
        ),
    }
}

/// `vox_kb_cite`: a citation for one indexed chunk (link anchors are doc-relative).
pub async fn kb_cite(state: &ServerState, params: KbCiteParams) -> String {
    let hash = match validate_chunk_hash(&params.chunk_hash) {
        Ok(h) => h,
        Err(e) => return err_json(e, REM_HASH),
    };
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return err_json(e, REM_KB_DB),
    };
    let c = match db.library_chunk_by_hash(hash).await {
        Ok(Some(c)) => c,
        Ok(None) => return err_json(format!("not indexed: chunk {hash}"), REM_KB_REINDEX),
        Err(e) => return err_json(format!("library lookup failed: {e}"), REM_KB_REINDEX),
    };
    let indexed_commit = match db.library_document_by_path(&c.doc_path).await {
        Ok(d) => d.and_then(|d| d.indexed_commit),
        Err(_) => None,
    };
    let link = match c.anchor.as_deref().filter(|a| !a.is_empty()) {
        Some(a) => format!("{}#{a}", c.doc_path),
        None => c.doc_path.clone(),
    };
    ok_clean(json!({
        "chunk_hash": c.chunk_hash,
        "doc_path": c.doc_path,
        "title": c.title,
        "heading_path": c.heading_path,
        "anchor": c.anchor,
        "link": link,
        "citation": format!("[{}]({link})", c.heading_path),
        "uri": chunk_uri(&c.chunk_hash),
        "indexed_commit": indexed_commit,
    }))
    .to_json()
}

/// The Phase 3 additions to the status payload, read live from the database: legacy VoxKB row
/// counts (zero for a table that no longer exists) and findings-inbox counts. Counts only, no
/// paths: the rest of the payload carries host paths, which is why the tool stays off the HTTP
/// read role. A failed read is an error, never a silent zero.
async fn status_counts(db: &VoxDb) -> Result<(serde_json::Value, serde_json::Value), String> {
    let legacy = db
        .legacy_kb_counts()
        .await
        .map_err(|e| format!("legacy VoxKB count failed: {e}"))?;
    let inbox = db
        .kb_findings_counts()
        .await
        .map_err(|e| format!("findings inbox count failed: {e}"))?;
    Ok((
        json!({
            "knowledge_bases": legacy.knowledge_bases,
            "kb_entries": legacy.kb_entries,
            "kb_routing_rules": legacy.kb_routing_rules,
        }),
        json!({"total": inbox.total, "pending": inbox.pending}),
    ))
}

/// `vox_kb_status`: the last index manifest plus a live DB-vs-Tantivy consistency check, the
/// legacy VoxKB row counts and the findings-inbox counts.
pub async fn kb_status(state: &ServerState) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return err_json(e, REM_KB_DB),
    };
    let (legacy_voxkb, inbox) = match status_counts(&db).await {
        Ok(c) => c,
        Err(e) => return err_json(e, REM_KB_REINDEX),
    };
    let Some(dir) = lexical::default_index_dir() else {
        return err_json(
            "no Library index directory could be resolved",
            REM_KB_REINDEX,
        );
    };
    let st = match status(&db, &dir).await {
        Ok(st) => st,
        Err(e) => return err_json(format!("library status failed: {e}"), REM_KB_REINDEX),
    };
    let index_dir = dir.to_string_lossy().into_owned();
    let Some(s) = st.state else {
        return ok_clean(json!({
            "indexed": false,
            "index_dir": index_dir,
            "consistent": false,
            "legacy_voxkb": legacy_voxkb,
            "inbox": inbox,
        }))
        .to_json();
    };
    let embed = s
        .embed_model_id
        .clone()
        .unwrap_or_else(|| "none (lexical-only)".into());
    ok_clean(json!({
        "indexed": true,
        "index_dir": index_dir,
        "corpus_root": s.corpus_root,
        "corpus_commit": s.corpus_commit,
        "indexed_at": s.indexed_at,
        "doc_count": s.doc_count,
        "chunk_count": s.chunk_count,
        "card_count": s.card_count,
        "chunker_version": s.chunker_version,
        "lexical_backend": s.lexical_backend,
        "embed": embed,
        "trust_tier": s.trust_tier.as_db_str(),
        "errors": json_or_string(&s.errors_json),
        "duration_ms": s.duration_ms,
        "db_chunk_count": st.db_chunk_count,
        "lexical_docs": st.lexical_docs,
        "consistent": st.consistent,
        "legacy_voxkb": legacy_voxkb,
        "inbox": inbox,
    }))
    .to_json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn parse(s: &str) -> serde_json::Value {
        serde_json::from_str(s).expect("tool json")
    }

    /// Point `default_index_dir()` at `<root>/library`. Caller holds `#[serial]`.
    fn set_index_root(root: &Path) {
        // SAFETY: `#[serial]` — no concurrent env mutation in this crate's tests.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_SEARCH_TANTIVY_ROOT", root)
        };
        vox_config::snapshot::bump(&["VOX_SEARCH_TANTIVY_ROOT"]);
    }

    /// Temp repo with one doc, indexed into `<tmp>/idx/library` and served by a state whose
    /// repository root is that temp repo. Caller holds `#[serial]`.
    async fn indexed_state() -> (TempDir, ServerState) {
        indexed_state_with("# Guide\n\nIntro text.\n\n## Install\nrun turbofrobnicate to install\n")
            .await
    }

    async fn indexed_state_with(guide_md: &str) -> (TempDir, ServerState) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        tokio::fs::create_dir_all(repo.join("docs/src"))
            .await
            .unwrap();
        tokio::fs::write(repo.join("docs/src/guide.md"), guide_md)
            .await
            .unwrap();
        let idx_root = tmp.path().join("idx");
        set_index_root(&idx_root);
        let db = Arc::new(VoxDb::open_memory().await.unwrap());
        reindex_at(&db, &repo, &idx_root.join("library"), None)
            .await
            .unwrap();
        let mut state = ServerState::new_test().await.with_db_initialized(db).await;
        state.repository.root = repo;
        (tmp, state)
    }

    /// Red-team `input` of the case `id`.
    fn redteam_input(id: &str) -> String {
        include_str!("../../../../contracts/kb/redteam/cases.v1.jsonl")
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .find(|v| v["id"] == id)
            .and_then(|v| v["input"].as_str().map(str::to_string))
            .unwrap_or_else(|| panic!("no red-team case {id}"))
    }

    /// TRUST-05 at the injection boundary: a curated doc that carries an image-exfil link and a
    /// tag-block payload (the indexer does not sanitize) reaches no model through the KB tools.
    #[tokio::test]
    #[serial]
    async fn kb_injection_kb_tools_neutralize_exfil_and_tag_block() {
        let exfil = redteam_input("inline-image-exfil-url");
        let tags = redteam_input("tag-block-ascii-smuggling");
        let md = format!("# Guide\n\n## Install\nturbofrobnicate {exfil} {tags}\n");
        let (_tmp, state) = indexed_state_with(&md).await;
        let hits = tool(&state, "vox_kb_search", json!({"query": "turbofrobnicate"})).await;
        let hash = hits["data"]["hits"][0]["chunk_hash"]
            .as_str()
            .unwrap()
            .to_string();
        let outs = [
            hits.to_string(),
            tool(&state, "vox_kb_get", json!({"path": "docs/src/guide.md"}))
                .await
                .to_string(),
            tool(&state, "vox_kb_get", json!({"chunk_hash": &hash}))
                .await
                .to_string(),
            tool(&state, "vox_kb_cite", json!({"chunk_hash": &hash}))
                .await
                .to_string(),
        ];
        for (i, out) in outs.iter().enumerate() {
            assert!(
                !out.contains("evil.example") && !out.contains("!["),
                "tool {i}: {out}"
            );
            assert!(
                !out.contains('\u{E0069}') && !out.contains('\u{E0067}'),
                "tool {i}"
            );
        }
        // Non-vacuous: the stored chunk still holds the payload; only the tool output is clean.
        let stored = state
            .db
            .as_ref()
            .unwrap()
            .library_chunk_by_hash(&hash)
            .await
            .unwrap()
            .unwrap();
        assert!(stored.body.contains("evil.example"), "{}", stored.body);
        assert!(outs[2].contains("turbofrobnicate"), "{}", outs[2]);
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

    fn hex64(c: char) -> String {
        std::iter::repeat_n(c, 64).collect()
    }

    #[test]
    fn validate_doc_path_accepts_and_rejects() {
        for ok in ["docs/src/guide.md", "docs/src/library/index.md"] {
            assert_eq!(validate_doc_path(ok), Ok(ok));
        }
        let long = format!("docs/src/{}.md", "a".repeat(MAX_PATH_BYTES));
        for bad in [
            "",
            "/etc/passwd",
            "docs/src/../../etc/passwd",
            // Traversal that still ends in `.md`: only the `..` component rule can stop these.
            "docs/src/../../etc/shadow.md",
            "docs/src/../Cargo.md",
            "docs/src/./x.md",
            "docs\\src\\x.md",
            "C:/x.md",
            "docs/src/archive/old.md",
            "docs/src/research/archive/x.md",
            "docs/src/x.mdx",
            "README.md",
            long.as_str(),
        ] {
            let e = validate_doc_path(bad).expect_err(bad);
            assert!(e.starts_with("invalid path"), "{bad:?}: {e}");
        }
    }

    #[test]
    fn validate_chunk_hash_shape() {
        assert!(validate_chunk_hash(&hex64('a')).is_ok());
        assert!(validate_chunk_hash(&hex64('0')).is_ok());
        for bad in [
            hex64('A'),
            "a".repeat(63),
            "a".repeat(65),
            hex64('g'),
            String::new(),
        ] {
            assert!(validate_chunk_hash(&bad).is_err(), "{bad}");
        }
    }

    async fn tool(state: &ServerState, name: &str, args: serde_json::Value) -> serde_json::Value {
        parse(
            &crate::dispatch::handle_tool_call(state, name, args)
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    #[serial]
    async fn kb_get_by_path_and_hash() {
        let (_tmp, state) = indexed_state().await;
        let v = tool(&state, "vox_kb_get", json!({"path": "docs/src/guide.md"})).await;
        assert_eq!(v["success"], true, "{v}");
        assert_eq!(v["data"]["doc"]["rel_path"], "docs/src/guide.md");
        assert_eq!(v["data"]["doc"]["uri"], "kb://doc/docs/src/guide.md");
        let md = v["data"]["markdown"].as_str().unwrap();
        assert!(
            md.contains("Intro text.") && md.contains("turbofrobnicate"),
            "{md}"
        );
        assert!(
            md.find("Intro text.") < md.find("turbofrobnicate"),
            "document order"
        );
        let chunks = v["data"]["chunks"].as_array().unwrap();
        assert!(chunks.iter().any(|c| c["chunk_kind"] == "card"));
        let section = chunks
            .iter()
            .find(|c| c["heading_path"] == "Guide › Install")
            .expect("install section");
        let hash = section["chunk_hash"].as_str().unwrap();

        let v = tool(&state, "vox_kb_get", json!({"chunk_hash": hash})).await;
        assert_eq!(v["success"], true, "{v}");
        assert!(
            v["data"]["chunk"]["body"]
                .as_str()
                .unwrap()
                .contains("turbofrobnicate")
        );
        assert_eq!(v["data"]["doc_path"], "docs/src/guide.md");
        assert_eq!(v["data"]["uri"], format!("kb://chunk/{hash}"));

        // Both or neither is an error; so is a valid-but-unindexed key.
        for args in [
            json!({}),
            json!({"path": "docs/src/guide.md", "chunk_hash": hash}),
        ] {
            assert_eq!(tool(&state, "vox_kb_get", args).await["success"], false);
        }
        let v = tool(&state, "vox_kb_get", json!({"path": "docs/src/nope.md"})).await;
        assert!(
            v["error"].as_str().unwrap().starts_with("not indexed"),
            "{v}"
        );
        let v = tool(&state, "vox_kb_get", json!({"path": "docs/src/../x.md"})).await;
        assert!(
            v["error"].as_str().unwrap().starts_with("invalid path"),
            "{v}"
        );
        let v = tool(&state, "vox_kb_get", json!({"chunk_hash": hex64('0')})).await;
        assert!(
            v["error"].as_str().unwrap().starts_with("not indexed"),
            "{v}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn kb_get_never_reads_files() {
        let (tmp, state) = indexed_state().await;
        // A valid corpus file on disk under the repo root, created AFTER indexing: the tool
        // must answer from the index only, so it stays "not indexed".
        tokio::fs::write(
            tmp.path().join("repo/docs/src/late.md"),
            "# Late\nonly on disk\n",
        )
        .await
        .unwrap();
        let v = tool(&state, "vox_kb_get", json!({"path": "docs/src/late.md"})).await;
        assert!(
            v["error"].as_str().unwrap().starts_with("not indexed"),
            "{v}"
        );
        // And the handler source carries no file-system access at all (AP-15).
        let src = include_str!("library.rs");
        let prod = src.split("#[cfg(test)]").next().unwrap();
        for banned in [
            concat!("std", "::fs"),
            concat!("tokio", "::fs"),
            "File::open",
            "read_to_string",
        ] {
            assert!(
                !prod.contains(banned),
                "library handlers must not use {banned}"
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn kb_cite_shape() {
        let (_tmp, state) = indexed_state().await;
        let hits = tool(&state, "vox_kb_search", json!({"query": "turbofrobnicate"})).await;
        let hit = &hits["data"]["hits"][0];
        let hash = hit["chunk_hash"].as_str().unwrap();
        let v = tool(&state, "vox_kb_cite", json!({"chunk_hash": hash})).await;
        assert_eq!(v["success"], true, "{v}");
        let d = &v["data"];
        assert_eq!(d["heading_path"], "Guide › Install");
        assert_eq!(d["doc_path"], "docs/src/guide.md");
        let link = d["link"].as_str().unwrap();
        assert!(link.starts_with("docs/src/guide.md"), "{link}");
        match d["anchor"].as_str() {
            Some(a) => assert_eq!(link, format!("docs/src/guide.md#{a}")),
            None => assert_eq!(link, "docs/src/guide.md"),
        }
        assert_eq!(d["citation"], format!("[Guide › Install]({link})"));
        assert_eq!(d["uri"], format!("kb://chunk/{hash}"));
        assert!(d.get("indexed_commit").is_some());
        let bad = tool(&state, "vox_kb_cite", json!({"chunk_hash": "XYZ"})).await;
        assert_eq!(bad["success"], false);
    }

    #[tokio::test]
    #[serial]
    async fn kb_status_before_and_after_index() {
        let tmp = tempfile::tempdir().unwrap();
        set_index_root(&tmp.path().join("idx"));
        let db = Arc::new(VoxDb::open_memory().await.unwrap());
        let state = ServerState::new_test().await.with_db_initialized(db).await;
        let v = tool(&state, "vox_kb_status", json!({})).await;
        assert_eq!(v["success"], true, "{v}");
        assert_eq!(v["data"]["indexed"], false);

        let (_tmp2, state) = indexed_state().await;
        let v = tool(&state, "vox_kb_status", json!({})).await;
        assert_eq!(v["success"], true, "{v}");
        let d = &v["data"];
        assert_eq!(d["indexed"], true, "{v}");
        assert_eq!(d["doc_count"], 1);
        assert!(d["chunk_count"].as_i64().unwrap() >= 2);
        assert_eq!(d["card_count"], 1);
        assert_eq!(d["lexical_backend"], "tantivy-0.22");
        assert_eq!(d["embed"], "none (lexical-only)");
        assert_eq!(d["trust_tier"], "trusted");
        assert_eq!(d["consistent"], true, "{v}");
        assert!(d["errors"].as_array().unwrap().is_empty());
        for k in [
            "corpus_root",
            "corpus_commit",
            "chunker_version",
            "index_dir",
        ] {
            assert!(d.get(k).is_some(), "missing {k}");
        }
        assert!(!d["chunker_version"].as_str().unwrap().is_empty());
    }

    /// Temp repo with `docs/src/a.md` containing `body`, plus its tempdir.
    async fn one_doc_repo(body: &str) -> (TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        tokio::fs::create_dir_all(repo.join("docs/src"))
            .await
            .unwrap();
        tokio::fs::write(repo.join("docs/src/a.md"), body)
            .await
            .unwrap();
        (tmp, repo)
    }

    #[test]
    fn kb_reindex_rejects_args() {
        assert!(reject_reindex_args(&json!({})).is_ok());
        assert!(reject_reindex_args(&serde_json::Value::Null).is_ok());
        assert!(reject_reindex_args(&json!({"agent_id": "1"})).is_ok());
        for bad in [
            json!({"root": "/etc"}),
            json!({"path": "docs/src"}),
            json!({"agent_id": "1", "root": "/"}),
            json!(["x"]),
        ] {
            assert!(reject_reindex_args(&bad).is_err(), "{bad}");
        }
    }

    #[tokio::test]
    #[serial]
    async fn kb_reindex_dispatch_rejects_args() {
        let (_tmp, state) = indexed_state().await;
        let r =
            crate::dispatch::handle_tool_call(&state, "vox_kb_reindex", json!({"root": "/"})).await;
        assert!(r.is_err(), "unexpected args must fail before any indexing");
    }

    #[tokio::test]
    #[serial]
    async fn kb_reindex_single_flight() {
        let (tmp, repo) = one_doc_repo("# A\n## S\nbody\n").await;
        let db = VoxDb::open_memory().await.unwrap();
        let lex = tmp.path().join("lex");
        {
            let _held = KB_REINDEX_LOCK.lock().await;
            let e = reindex_at(&db, &repo, &lex, None).await.unwrap_err();
            assert!(e.contains("already running"), "{e}");
        }
        assert!(reindex_at(&db, &repo, &lex, None).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn kb_reindex_uses_vox_crypto_hasher() {
        let (tmp, repo) = one_doc_repo("# T\n## S\nsome body text\n").await;
        let db = VoxDb::open_memory().await.unwrap();
        reindex_at(&db, &repo, &tmp.path().join("lex"), None)
            .await
            .unwrap();
        let chunks = db
            .library_doc_chunks(KB_LIBRARY_CORPUS, "docs/src/a.md")
            .await
            .unwrap();
        let sec = chunks
            .iter()
            .find(|c| c.heading_path == "T › S")
            .expect("section chunk");
        let input = format!("docs/src/a.md\0{}\0{}", sec.heading_path, sec.body);
        let expected = vox_crypto::hex_encode(&vox_crypto::secure_hash(input.as_bytes()));
        assert_eq!(sec.chunk_hash, expected);
    }

    #[tokio::test]
    #[serial]
    async fn kb_reindex_report() {
        let (tmp, repo) = one_doc_repo("# A\n## S\nquokkaflux body\n").await;
        set_index_root(&tmp.path().join("idx"));
        let db = Arc::new(VoxDb::open_memory().await.unwrap());
        let mut state = ServerState::new_test().await.with_db_initialized(db).await;
        state.repository.root = repo;

        let v = tool(&state, "vox_kb_reindex", json!({})).await;
        assert_eq!(v["success"], true, "{v}");
        let d = &v["data"];
        for k in [
            "docs_seen",
            "docs_unchanged",
            "docs_changed",
            "docs_pruned",
            "chunks_inserted",
            "chunks_deleted",
            "chunks_kept",
            "cards",
            "errors",
            "lexical_backend",
            "lexical_docs",
            "corpus_commit",
            "duration_ms",
        ] {
            assert!(d.get(k).is_some(), "report missing {k}: {v}");
        }
        assert_eq!(d["docs_seen"], 1);
        assert_eq!(d["docs_changed"], 1);
        assert_eq!(d["lexical_backend"], "tantivy-0.22");
        assert!(
            d["corpus_commit"].is_null(),
            "temp dir is not a git checkout"
        );

        // The tool-built index is immediately searchable, and a second run is a no-op.
        let hits = tool(&state, "vox_kb_search", json!({"query": "quokkaflux"})).await;
        assert_eq!(hits["data"]["hits"][0]["heading_path"], "A › S", "{hits}");
        let v = tool(&state, "vox_kb_reindex", json!({})).await;
        assert_eq!(v["data"]["docs_unchanged"], 1, "{v}");
    }
}
