//! Web hit -> pending Untrusted finding end to end, plus capture hardening (TRUST-02/04/05).

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use vox_db::kb_text::{FindingOrigin, FindingStatus};
use vox_db::{ContextTrustTier, KbFindingRow, VoxDb, heuristic_search_plan};
use vox_orchestrator_mcp::ServerState;
use vox_orchestrator_mcp::kb_tools::findings::{
    CaptureInput, MAX_SNAPSHOT_BYTES, McpFindingCapture, capture_finding, snapshot_relpath,
    web_capture_for_state,
};
use vox_search::context::SearchRuntimeContext;
use vox_search::execute_search_plan;
use vox_search::policy::SearchPolicy;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const QUERY: &str = "what is the latest gemini flash model and when was it released";

fn input(text: &str, url: &str) -> CaptureInput {
    CaptureInput {
        origin: FindingOrigin::WebSearch,
        title: "A title".into(),
        text: text.into(),
        full_text: None,
        source_url: Some(url.into()),
        dedupe_scope: None,
        provenance: serde_json::json!({"engine": "test"}),
    }
}

async fn rows(db: &VoxDb) -> Vec<KbFindingRow> {
    db.kb_findings_list(None, 100).await.unwrap()
}

fn snapshot_files(root: &Path) -> Vec<String> {
    match std::fs::read_dir(root.join(".vox/cache/kb-findings")) {
        Ok(d) => d
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => vec![],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn capture_web_leg_end_to_end() {
    let mock = MockServer::start().await;
    let hit_url = format!("{}/gemini-3-8-flash", mock.uri());
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"url": hit_url, "title": "Gemini 3.8 Flash",
                "content": "Gemini 3.8 Flash release notes", "engine": "test"}],
        })))
        .mount(&mock)
        .await;
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let tmp = tempfile::tempdir().unwrap();
    let ctx = SearchRuntimeContext::new(
        tmp.path().to_path_buf(),
        Some(db.clone()),
        tmp.path().join("log"),
        tmp.path().join("lt.md"),
    )
    .with_web_capture(Some(Arc::new(McpFindingCapture {
        db: db.clone(),
        repo_root: tmp.path().to_path_buf(),
    })));
    let policy = SearchPolicy {
        searxng_url: Some(mock.uri()),
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        duckduckgo_fallback_enabled: false,
        persist_web_hits: true,
        deep_timeout_ms: 10_000,
        fast_timeout_ms: 10_000,
        ..Default::default()
    };
    let plan = heuristic_search_plan(QUERY, false, None);
    execute_search_plan(&ctx, QUERY, &plan, 5, &policy, None)
        .await
        .unwrap();

    let mut found = vec![];
    for _ in 0..50 {
        found = rows(&db).await;
        if !found.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(found.len(), 1, "{found:?}");
    let r = &found[0];
    assert_eq!(r.status, FindingStatus::Pending);
    assert_eq!(r.trust_tier, ContextTrustTier::Untrusted);
    assert_eq!(r.origin, FindingOrigin::WebSearch);
    assert!(!r.summary.is_empty());
    // The loopback mock URL carries an IPv4 literal, which capture redacts by design.
    assert_eq!(r.source_url, Some(vox_redact::redact_owned(&hit_url)));
    assert!(r.fetched_at_ms > 0);
    let hex = r
        .content_hash
        .strip_prefix("blake3:")
        .expect("blake3: prefix");
    assert_eq!(hex.len(), 64);
    assert!(
        tmp.path()
            .join(format!(".vox/cache/kb-findings/{hex}.txt"))
            .exists()
    );
    assert_eq!(db.search_corpus_row_counts().await.unwrap(), (0, 0));
}

#[tokio::test]
async fn capture_web_capture_for_state_needs_db() {
    let none = ServerState::new_test().await;
    assert!(web_capture_for_state(&none).is_none());
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let some = ServerState::new_test().await.with_db_initialized(db).await;
    assert!(web_capture_for_state(&some).is_some());
}

#[tokio::test]
async fn capture_empty_input_is_rejected_without_writes() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let err = capture_finding(&db, tmp.path(), input("  \n\t", "https://e.x/a"))
        .await
        .unwrap_err();
    assert!(err.contains("empty"), "{err}");
    assert!(rows(&db).await.is_empty());
    assert!(!tmp.path().join(".vox").exists());
    assert!(snapshot_files(tmp.path()).is_empty());

    let mut full_only = input("", "https://e.x/a");
    full_only.full_text = Some("Body sentence one. Two.".into());
    let out = capture_finding(&db, tmp.path(), full_only).await.unwrap();
    assert!(out.finding_id.is_some());
    assert_eq!(snapshot_files(tmp.path()).len(), 1);
    let row = &rows(&db).await[0];
    assert!(row.summary.starts_with("Body sentence one."));
    assert_eq!(row.source_url.as_deref(), Some("https://e.x/a"));
}

const REDTEAM: &str = include_str!("../../../contracts/kb/redteam/cases.v1.jsonl");

#[tokio::test]
async fn capture_redteam_neutralized_at_ingest() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let cases: Vec<serde_json::Value> = REDTEAM
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(cases.len() >= 10);
    for (i, c) in cases.iter().enumerate() {
        let url = format!("https://e.x/{i}");
        capture_finding(&db, tmp.path(), input(c["input"].as_str().unwrap(), &url))
            .await
            .unwrap_or_else(|e| panic!("case {}: {e}", c["id"]));
    }
    let stored = rows(&db).await;
    assert_eq!(stored.len(), cases.len());
    for r in &stored {
        let i: usize = r
            .source_url
            .as_deref()
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        for bad in cases[i]["must_not_contain"].as_array().unwrap() {
            let bad = bad.as_str().unwrap();
            assert!(
                !r.summary.contains(bad),
                "case {}: summary has {bad:?}",
                cases[i]["id"]
            );
            assert!(!r.title.contains(bad));
            for q in &r.quotes {
                assert!(
                    !q.contains(bad),
                    "case {}: quote has {bad:?}",
                    cases[i]["id"]
                );
            }
        }
    }
}

#[tokio::test]
async fn capture_secrets_are_redacted_in_summary_and_quotes() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let secret = "sk-abcdefghijklmnopqrstuvwxyz0123456789";
    let mut i = input(
        &format!("Use key {secret} to log in. Second sentence."),
        "https://e.x/s",
    );
    i.title = format!("Title {secret}");
    capture_finding(&db, tmp.path(), i).await.unwrap();
    let r = &rows(&db).await[0];
    assert!(r.summary.contains("[REDACTED_KEY]"), "{}", r.summary);
    assert!(!r.summary.contains(secret) && !r.title.contains(secret));
    assert!(r.quotes.iter().all(|q| !q.contains(secret)));
    assert!(r.quotes[0].contains("[REDACTED_KEY]"));
}

#[tokio::test]
async fn capture_caps_count_chars_not_bytes() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    capture_finding(&db, tmp.path(), input(&"€".repeat(1500), "https://e.x/m"))
        .await
        .unwrap();
    let r = &rows(&db).await[0];
    assert_eq!(r.summary.chars().count(), 1000);
    assert_eq!(r.quotes.len(), 1);
    assert_eq!(r.quotes[0].chars().count(), 300);
}

#[tokio::test]
async fn capture_snapshot_truncates_at_char_boundary() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let mut i = input("short", "https://e.x/big");
    i.full_text = Some("€".repeat(800_000)); // 2.4 MB, 3 bytes per char
    let out = capture_finding(&db, tmp.path(), i).await.unwrap();
    let body = std::fs::read_to_string(tmp.path().join(out.snapshot_relpath.unwrap())).unwrap();
    assert_eq!(body.len(), MAX_SNAPSHOT_BYTES - MAX_SNAPSHOT_BYTES % 3);
}

#[tokio::test]
async fn capture_snapshot_filename_is_hash_only() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let out = capture_finding(
        &db,
        tmp.path(),
        input("hello world", "https://e.x/../../../etc/passwd?a=b"),
    )
    .await
    .unwrap();
    let hex = rows(&db).await[0]
        .content_hash
        .strip_prefix("blake3:")
        .unwrap()
        .to_string();
    assert_eq!(snapshot_files(tmp.path()), vec![format!("{hex}.txt")]);
    assert_eq!(out.snapshot_relpath.unwrap(), snapshot_relpath(&hex));
}

#[tokio::test]
async fn capture_snapshot_path_is_gitignored() {
    let rel = snapshot_relpath(&"0".repeat(64));
    assert!(rel.starts_with(".vox/cache/kb-findings/"));
    assert!(
        include_str!("../../../.gitignore")
            .lines()
            .any(|l| l.trim() == ".vox/cache/")
    );
}

#[tokio::test]
async fn capture_same_hit_twice_one_row() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let a = capture_finding(&db, tmp.path(), input("same body", "https://e.x/a"))
        .await
        .unwrap();
    let b = capture_finding(&db, tmp.path(), input("same body", "https://e.x/a"))
        .await
        .unwrap();
    assert!(a.finding_id.is_some());
    assert!(b.finding_id.is_none());
    assert_eq!(a.dedupe_key, b.dedupe_key);
    assert_eq!(rows(&db).await.len(), 1);
}

#[tokio::test]
async fn capture_same_content_two_urls_two_rows_one_snapshot() {
    let db = VoxDb::open_memory().await.unwrap();
    let tmp = tempfile::tempdir().unwrap();
    capture_finding(&db, tmp.path(), input("same body", "https://e.x/a"))
        .await
        .unwrap();
    capture_finding(&db, tmp.path(), input("same body", "https://e.x/b"))
        .await
        .unwrap();
    assert_eq!(rows(&db).await.len(), 2);
    assert_eq!(snapshot_files(tmp.path()).len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn capture_concurrent_same_hit_one_row() {
    let db = Arc::new(VoxDb::open_memory().await.unwrap());
    let tmp = tempfile::tempdir().unwrap();
    let go = || {
        let db = db.clone();
        let root = tmp.path().to_path_buf();
        tokio::spawn(async move {
            capture_finding(&db, &root, input("racy body", "https://e.x/r")).await
        })
    };
    let (a, b) = tokio::join!(go(), go());
    let (a, b) = (a.unwrap().unwrap(), b.unwrap().unwrap());
    assert_eq!(
        a.finding_id.is_some() as u8 + b.finding_id.is_some() as u8,
        1
    );
    assert_eq!(rows(&db).await.len(), 1);
    assert_eq!(snapshot_files(tmp.path()).len(), 1);
}
