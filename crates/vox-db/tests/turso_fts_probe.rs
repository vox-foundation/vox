//! Characterization test of turso 0.6.1 FTS behaviour (spike S1, 2026-10).
//!
//! These assertions pin what the workspace's turso build can and cannot do today, so the
//! IDX-07 lexical-leg decision rests on an executed fact. If any assertion flips after a turso
//! bump, re-open the IDX-07 lexical-leg decision (Phase 2) and update the spike section of
//! `docs/src/architecture/knowledge-base-practices-research-2026.md`.

use vox_db::{DbConfig, VoxDb};

const DRIFT: &str = "turso FTS behaviour changed; revisit IDX-07";

/// Display + Debug of an error, so a substring match survives either rendering.
fn err_text(e: &turso::Error) -> String {
    format!("{e} | {e:?}")
}

const CREATE_PROBE: &str = "CREATE TABLE kb_probe (id INTEGER PRIMARY KEY, body TEXT)";
const CREATE_FTS_INDEX: &str = "CREATE INDEX kb_probe_fts ON kb_probe USING fts (body)";

#[tokio::test]
async fn compile_options_pragma_is_rejected() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    let err = match db.connection().query("PRAGMA compile_options", ()).await {
        Ok(_) => panic!("PRAGMA compile_options now succeeds; {DRIFT}"),
        Err(e) => err_text(&e),
    };
    assert!(err.contains("Not a valid pragma name"), "{DRIFT}: {err}");
}

#[tokio::test]
async fn fts5_virtual_table_module_is_absent() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    let res = db
        .connection()
        .execute_batch("CREATE VIRTUAL TABLE kb_probe_v USING fts5(body)")
        .await;
    let err = res.expect_err(DRIFT);
    assert!(
        err_text(&err).contains("no such module: fts5"),
        "{DRIFT}: {err:?}"
    );
}

#[tokio::test]
async fn native_fts_index_requires_experimental_flag() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    db.connection().execute_batch(CREATE_PROBE).await.unwrap();
    let err = db
        .connection()
        .execute_batch(CREATE_FTS_INDEX)
        .await
        .expect_err(DRIFT);
    assert!(err_text(&err).contains("experimental"), "{DRIFT}: {err:?}");
}

#[tokio::test]
async fn native_fts_index_needs_turso_fts_feature() {
    // Flag on, but the workspace turso build lacks the `fts` cargo feature.
    let tdb = turso::Builder::new_local(":memory:")
        .experimental_index_method(true)
        .build()
        .await
        .unwrap();
    let conn = tdb.connect().unwrap();
    conn.execute_batch(CREATE_PROBE).await.unwrap();
    let err = conn.execute_batch(CREATE_FTS_INDEX).await.expect_err(DRIFT);
    assert!(
        err_text(&err).contains("unknown module name 'fts'"),
        "{DRIFT}: {err:?}"
    );
}

#[tokio::test]
async fn capability_probe_reports_no_fts5() {
    let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
    let snap = db.probe_sqlite_capabilities().await.unwrap();
    assert!(!snap.fts5_reported, "{DRIFT}");

    let mut rows = db
        .connection()
        .query(
            "SELECT name FROM sqlite_master WHERE name = 'search_document_chunks_fts'",
            (),
        )
        .await
        .unwrap();
    assert!(rows.next().await.unwrap().is_none(), "{DRIFT}");
}
