//! `vox_kb_status` payload contract (RET-03, D-05): every Phase 2 field is kept unchanged and the
//! Phase 3 `legacy_voxkb` and `inbox` objects are added, in both the not-indexed and the indexed
//! shape. The tool stays off the HTTP read role because it carries absolute host paths, so the
//! added objects must be counts only.

mod common;

use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};
use serial_test::serial;
use vox_db::kb_text::{ContentHash, FindingOrigin, SanitizedText};
use vox_db::{FindingTier, LEGACY_VOXKB_DDL, NewKbFinding, VoxDb};
use vox_orchestrator_mcp::{ServerState, handle_tool_call};

/// Every key the indexed Phase 2 payload carries. Phase 3 must not remove or rename one.
const PHASE2_INDEXED_KEYS: &[&str] = &[
    "indexed",
    "index_dir",
    "corpus_root",
    "corpus_commit",
    "indexed_at",
    "doc_count",
    "chunk_count",
    "card_count",
    "chunker_version",
    "lexical_backend",
    "embed",
    "trust_tier",
    "errors",
    "duration_ms",
    "db_chunk_count",
    "lexical_docs",
    "consistent",
];

fn finding(n: u32) -> NewKbFinding {
    NewKbFinding {
        dedupe_key: format!("status-{n}"),
        tier: FindingTier::Untrusted,
        origin: FindingOrigin::WebSearch,
        title: SanitizedText::new("Title"),
        summary: SanitizedText::new("A summary"),
        quotes: vec![],
        source_url: None,
        fetched_at_ms: 1_700_000_000_000,
        content_hash: ContentHash::from_blake3_hex(&format!("{n:064x}")).unwrap(),
        snapshot_relpath: None,
        provenance: json!({}),
    }
}

/// A memory DB with 3 pending findings and, when `legacy`, 1 KB + 2 entries + 1 rule. The entries
/// are whitespace-only so nothing could ever migrate into the inbox and skew the counts.
async fn seeded_db(legacy: bool) -> Arc<VoxDb> {
    let db = VoxDb::open_memory().await.unwrap();
    if legacy {
        db.connection()
            .execute_batch(format!(
                "{LEGACY_VOXKB_DDL} \
                 INSERT INTO knowledge_bases (id, name, description, created_at_ms, updated_at_ms, entry_count) \
                   VALUES ('kb1', 'research', '', 1, 1, 2); \
                 INSERT INTO kb_entries (id, kb_id, content, source_signal, created_at_ms) \
                   VALUES ('e1', 'kb1', '  ', 'chat', 1); \
                 INSERT INTO kb_entries (id, kb_id, content, source_signal, created_at_ms) \
                   VALUES ('e2', 'kb1', ' ', 'chat', 2); \
                 INSERT INTO kb_routing_rules (id, kb_id, rule_type, pattern, created_at_ms) \
                   VALUES ('r1', 'kb1', 'keyword', 'widget', 1);"
            ))
            .await
            .unwrap();
    }
    for n in 0..3 {
        db.insert_pending_finding(&finding(n))
            .await
            .unwrap()
            .expect("inserted");
    }
    Arc::new(db)
}

/// A state with `db` attached directly (no attach-time migration task racing the counts).
async fn state_with(db: Arc<VoxDb>) -> ServerState {
    let mut state = ServerState::new_test().await;
    state.db = Some(db);
    state
}

async fn status(state: &ServerState) -> Value {
    let raw = handle_tool_call(state, "vox_kb_status", json!({}))
        .await
        .expect("dispatch ok");
    serde_json::from_str(&raw).expect("tool json")
}

/// Point the Library index at `<root>/library`. Callers hold `#[serial]`.
fn set_index_root(root: &Path) {
    // SAFETY: every test that touches this variable is `#[serial]`.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("VOX_SEARCH_TANTIVY_ROOT", root)
    };
    vox_config::snapshot::bump(&["VOX_SEARCH_TANTIVY_ROOT"]);
}

fn assert_counts_only(v: &Value) {
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(m) => m.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    for key in ["legacy_voxkb", "inbox"] {
        let obj = v[key]
            .as_object()
            .unwrap_or_else(|| panic!("{key} missing: {v}"));
        assert!(
            obj.values().all(Value::is_number),
            "{key} must carry counts only: {v}"
        );
        let mut strings = Vec::new();
        walk(&v[key], &mut strings);
        assert!(
            strings
                .iter()
                .all(|s| !s.contains('/') && !s.contains('\\')),
            "{key} leaks a path: {strings:?}"
        );
    }
}

#[tokio::test]
#[serial]
async fn kb_status_not_indexed_carries_legacy_and_inbox_counts() {
    let tmp = tempfile::tempdir().unwrap();
    set_index_root(&tmp.path().join("idx"));
    let state = state_with(seeded_db(true).await).await;
    let v = status(&state).await;
    assert_eq!(v["success"], true, "{v}");
    let d = &v["data"];
    assert_eq!(d["indexed"], false);
    assert_eq!(d["consistent"], false);
    assert!(
        d["index_dir"].as_str().is_some_and(|s| !s.is_empty()),
        "{v}"
    );
    assert_eq!(
        d["legacy_voxkb"],
        json!({"knowledge_bases": 1, "kb_entries": 2, "kb_routing_rules": 1}),
        "{v}"
    );
    assert_eq!(d["inbox"], json!({"total": 3, "pending": 3}), "{v}");
    assert_counts_only(d);
}

#[tokio::test]
#[serial]
async fn kb_status_legacy_tables_absent_report_zeros() {
    let tmp = tempfile::tempdir().unwrap();
    set_index_root(&tmp.path().join("idx"));
    let state = state_with(seeded_db(false).await).await;
    let d = status(&state).await["data"].clone();
    assert_eq!(
        d["legacy_voxkb"],
        json!({"knowledge_bases": 0, "kb_entries": 0, "kb_routing_rules": 0}),
        "{d}"
    );
    assert_eq!(d["inbox"], json!({"total": 3, "pending": 3}), "{d}");
}

#[tokio::test]
#[serial]
async fn kb_status_indexed_keeps_every_phase2_field_and_adds_phase3_objects() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    tokio::fs::create_dir_all(repo.join("docs/src"))
        .await
        .unwrap();
    tokio::fs::write(
        repo.join("docs/src/guide.md"),
        "# Guide\n\nIntro text.\n\n## Install\nrun turbofrobnicate to install\n",
    )
    .await
    .unwrap();
    set_index_root(&tmp.path().join("idx"));

    let mut state = state_with(seeded_db(true).await).await;
    state.repository.root = repo;
    let r: Value = serde_json::from_str(
        &handle_tool_call(&state, "vox_kb_reindex", json!({}))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(r["success"], true, "reindex failed: {r}");

    let v = status(&state).await;
    assert_eq!(v["success"], true, "{v}");
    let d = &v["data"];
    for k in PHASE2_INDEXED_KEYS {
        assert!(d.get(k).is_some(), "Phase 2 key {k} missing: {v}");
    }
    // Phase 2 values are unchanged by the added objects.
    assert_eq!(d["indexed"], true, "{v}");
    assert_eq!(d["doc_count"], 1);
    assert_eq!(d["card_count"], 1);
    assert_eq!(d["lexical_backend"], "tantivy-0.22");
    assert_eq!(d["embed"], "none (lexical-only)");
    assert_eq!(d["trust_tier"], "trusted");
    assert_eq!(d["consistent"], true, "{v}");
    // Phase 3 additions.
    assert_eq!(
        d["legacy_voxkb"],
        json!({"knowledge_bases": 1, "kb_entries": 2, "kb_routing_rules": 1}),
        "{v}"
    );
    assert_eq!(d["inbox"], json!({"total": 3, "pending": 3}), "{v}");
    assert_counts_only(d);
}
