//! Integration tests for `VoxDb` when `local` feature is enabled (`connect(DbConfig::Local/::Memory)` paths).

use super::*;
use crate::codex_schema::missing_codex_reactivity_tables;
use crate::legacy::codex::{
    LEGACY_EXPORT_SKIP_TABLES, LEGACY_EXPORT_TABLES, export_legacy_jsonl, import_legacy_jsonl,
    list_sqlite_user_tables, verify_legacy_store,
};
use crate::schema::{BASELINE_VERSION, CODEX_CHAT_TABLES};

#[tokio::test]
async fn cas_store_and_load_is_idempotent() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let hash = db.store("test_kind", b"test_data").await.expect("store");
    let data = db.get(&hash).await.expect("get");
    assert_eq!(data, b"test_data");
}

#[tokio::test]
async fn schema_init_v7_is_ok() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let v = db.schema_version().await.expect("version");
    assert_eq!(v, BASELINE_VERSION);
}

#[tokio::test]
async fn append_codex_change_is_ok() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let id = db
        .append_codex_change("test.topic", None, None, "upsert", None)
        .await
        .expect("append");
    assert!(id > 0);
}

#[tokio::test]
async fn test_connect_memory() {
    let db = VoxDb::connect(DbConfig::Memory)
        .await
        .expect("Failed to connect to memory DB");
    let hash = db
        .store("test_kind", b"test_data")
        .await
        .expect("Store failed");
    assert!(!hash.is_empty());
}

#[tokio::test]
async fn codex_reactivity_schema_and_legacy_verify() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let v = db.schema_version().await.expect("version");
    assert_eq!(v, BASELINE_VERSION);
    assert!(
        missing_codex_reactivity_tables(&db)
            .await
            .expect("cap")
            .is_empty()
    );
    let leg = verify_legacy_store(&db).await.expect("verify");
    assert!(leg.has_codex_reactivity);
    assert!(!leg.is_legacy_schema_chain);
    let id = db
        .append_codex_change("test.topic", None, None, "upsert", None)
        .await
        .expect("change log");
    assert!(id > 0);
}

#[tokio::test]
async fn codex_alias_connects() {
    let db: Codex = VoxDb::connect(DbConfig::Memory).await.expect("db");
    assert_eq!(db.schema_version().await.expect("v"), BASELINE_VERSION);
}

#[tokio::test]
async fn baseline_schema_includes_chat_and_search_tables() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    assert_eq!(
        db.schema_version().await.expect("schema_version"),
        BASELINE_VERSION
    );
    for t in CODEX_CHAT_TABLES {
        let rows = db
            .query_all(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                turso::params![t.to_string()],
            )
            .await
            .expect("sqlite_master");
        assert!(!rows.is_empty(), "missing table {t}");
    }
    for t in [
        "search_documents",
        "search_document_chunks",
        "search_indexing_jobs",
    ] {
        let rows = db
            .query_all(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                turso::params![t.to_string()],
            )
            .await
            .expect("search table");
        assert!(!rows.is_empty(), "missing search table {t}");
    }
    for t in ["processing_runs", "processing_run_steps", "audit_log"] {
        let rows = db
            .query_all(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                (t.to_string(),),
            )
            .await
            .expect("sqlite_master");
        assert!(!rows.is_empty(), "missing V16 table {t}");
    }
    for t in [
        "research_sessions",
        "conversation_versions",
        "conversation_edges",
        "topic_evolution_events",
    ] {
        let rows = db
            .query_all(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                (t.to_string(),),
            )
            .await
            .expect("sqlite_master");
        assert!(!rows.is_empty(), "missing V17 table {t}");
    }
}

#[tokio::test]
async fn raw_sqlite_gamify_profiles_integer_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = dir.path().join("raw.db");
    let db = VoxDb::connect(DbConfig::Local {
        path: p.to_string_lossy().into_owned(),
    })
    .await
    .expect("db");
    db.connection()
        .execute(
            "INSERT INTO gamify_profiles (user_id, level, xp) VALUES (?1, ?2, ?3)",
            turso::params!["u1", 3i64, 900i64],
        )
        .await
        .expect("insert");
    let mut q = db
        .connection()
        .query(
            "SELECT xp FROM gamify_profiles WHERE user_id = ?1",
            turso::params!["u1"],
        )
        .await
        .expect("sel");
    let row = q.next().await.expect("r").expect("row");
    assert_eq!(row.get::<i64>(0).expect("xp"), 900);
}

#[tokio::test]
async fn legacy_export_covers_all_baseline_tables() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let mut live = list_sqlite_user_tables(db.connection())
        .await
        .expect("list tables");
    live.retain(|n| !LEGACY_EXPORT_SKIP_TABLES.contains(&n.as_str()));
    live.sort();
    let mut expected: Vec<&str> = LEGACY_EXPORT_TABLES.to_vec();
    expected.sort();
    assert_eq!(
        live,
        expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "LEGACY_EXPORT_TABLES must match sqlite_master after migrate (minus skip list)"
    );
}

/// Gamification + coordination rows survive JSONL export/import on baseline DBs.
#[tokio::test]
async fn legacy_jsonl_roundtrips_gamification_and_coordination() {
    use std::io::Cursor;
    use tempfile::tempdir;

    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    db.connection()
        .execute(
            "INSERT INTO gamify_profiles (user_id, level, xp) VALUES ('u1', 3, 900)",
            (),
        )
        .await
        .expect("insert profile");
    db.connection()
        .execute(
            "INSERT INTO gamify_companions (id, user_id, name, language) VALUES ('c1', 'u1', 'Ada', 'vox')",
            (),
        )
        .await
        .expect("insert companion");
    db.connection()
        .execute(
            "INSERT INTO distributed_locks (lock_key, holder_node, holder_agent, fence_token, expires_at) VALUES ('lk', 'node-a', 'owner', 1, '2099-01-01')",
            (),
        )
        .await
        .expect("insert lock");

    let mut jsonl = Vec::<u8>::new();
    let n = export_legacy_jsonl(&db, &mut jsonl).await.expect("export");
    assert!(n >= 3, "expected ≥3 rows, got {n}");
    let profile_lines = String::from_utf8_lossy(&jsonl)
        .lines()
        .filter(|l| l.contains("\"table\":\"gamify_profiles\""))
        .count();
    assert_eq!(
        profile_lines, 1,
        "export must emit exactly one gamify_profiles row"
    );
    let prof_json: serde_json::Value = String::from_utf8_lossy(&jsonl)
        .lines()
        .find(|l| l.contains("\"table\":\"gamify_profiles\""))
        .and_then(|l| serde_json::from_str(l).ok())
        .expect("parse profile jsonl");
    assert_eq!(
        prof_json["row"]["xp"].as_i64(),
        Some(900),
        "exported JSON must preserve xp: {}",
        prof_json["row"]
    );

    let dir = tempdir().expect("tempdir");
    let fresh_path = dir.path().join("roundtrip.db");
    let fresh_str = fresh_path.to_string_lossy().to_string();
    let db2 = VoxDb::connect(DbConfig::Local {
        path: fresh_str.clone(),
    })
    .await
    .expect("fresh file db");
    let imported = import_legacy_jsonl(&db2, Cursor::new(&jsonl))
        .await
        .expect("import");
    assert!(imported >= 3);

    let mut q = db2
        .connection()
        .query(
            "SELECT xp, level FROM gamify_profiles WHERE user_id = ?1",
            turso::params!["u1"],
        )
        .await
        .expect("q");
    let row = q.next().await.expect("row").expect("has row");
    assert_eq!(row.get::<i64>(0).expect("xp"), 900);
    assert_eq!(row.get::<i64>(1).expect("level"), 3);

    let mut q2 = db2
        .connection()
        .query(
            "SELECT name FROM gamify_companions WHERE id = ?1",
            turso::params!["c1"],
        )
        .await
        .expect("q2");
    let row2 = q2.next().await.expect("row").expect("r2");
    assert_eq!(row2.get::<String>(0).expect("name"), "Ada");

    let mut q3 = db2
        .connection()
        .query(
            "SELECT holder_agent FROM distributed_locks WHERE lock_key = ?1",
            turso::params!["lk"],
        )
        .await
        .expect("q3");
    let row3 = q3.next().await.expect("row").expect("r3");
    assert_eq!(row3.get::<String>(0).expect("holder"), "owner");
}

/// Simulates `vox codex export-legacy` → new file → `vox codex import-legacy` without the CLI.
#[tokio::test]
async fn legacy_chain_db_export_then_import_into_baseline_roundtrips_objects() {
    use crate::schema::BASELINE_VERSION;
    use std::io::Cursor;
    use tempfile::tempdir;
    use turso::Builder;

    let dir = tempdir().expect("tempdir");
    let legacy_path = dir.path().join("legacy.db");
    let legacy_str = legacy_path.to_string_lossy().to_string();
    let fresh_path = dir.path().join("fresh.db");
    let fresh_str = fresh_path.to_string_lossy().to_string();

    let built = Builder::new_local(&legacy_str)
        .build()
        .await
        .expect("legacy build");
    let conn = built.connect().expect("legacy conn");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
    )
    .await
    .expect("schema_version ddl");
    conn.execute("INSERT INTO schema_version (version) VALUES (17)", ())
        .await
        .expect("insert v17");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS objects (
                hash TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                data BLOB NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
    )
    .await
    .expect("objects ddl");
    conn.execute(
        "INSERT INTO objects (hash, kind, data) VALUES ('legacy_row_h', 'legacy_kind', X'01020304')",
        (),
    )
    .await
    .expect("insert object");
    drop(conn);

    let err = match VoxDb::connect(DbConfig::local(&legacy_str)).await {
        Ok(_) => panic!("normal open must reject legacy chain"),
        Err(e) => e,
    };
    assert!(
        matches!(err, StoreError::LegacySchemaChain { max_version: 17 }),
        "expected LegacySchemaChain {{ max_version: 17 }}, got {err:?}"
    );

    let export_db = VoxDb::connect_legacy_export_only(DbConfig::local(&legacy_str))
        .await
        .expect("legacy export open");
    let mut jsonl = Vec::<u8>::new();
    let n = export_legacy_jsonl(&export_db, &mut jsonl)
        .await
        .expect("export");
    assert!(n >= 1, "expected at least one exported row");

    let fresh = VoxDb::connect(DbConfig::local(&fresh_str))
        .await
        .expect("fresh baseline");
    assert_eq!(fresh.schema_version().await.expect("sv"), BASELINE_VERSION);
    let imported = import_legacy_jsonl(&fresh, Cursor::new(&jsonl))
        .await
        .expect("import");
    assert!(imported >= 1);

    let imported_twice = import_legacy_jsonl(&fresh, Cursor::new(&jsonl))
        .await
        .expect("re-import");
    assert_eq!(
        imported_twice, imported,
        "second import should replace, not append duplicate rows"
    );

    let mut q = fresh
        .conn
        .query(
            "SELECT kind, hex(data) FROM objects WHERE hash = ?1",
            turso::params!["legacy_row_h"],
        )
        .await
        .expect("select");
    let row = q.next().await.expect("row").expect("has row");
    let kind: String = row.get(0).expect("kind");
    let hex_data: String = row.get(1).expect("hex");
    assert_eq!(kind, "legacy_kind");
    assert_eq!(hex_data.to_uppercase(), "01020304");

    let leg = verify_legacy_store(&fresh).await.expect("verify");
    assert_eq!(leg.schema_version, BASELINE_VERSION);
    assert!(!leg.is_legacy_schema_chain);
}

async fn seed_legacy_schema_version_only(path: &std::path::Path, version: i64) {
    let s = path.to_string_lossy().to_string();
    let built = turso::Builder::new_local(&s)
        .build()
        .await
        .expect("legacy build");
    let conn = built.connect().expect("conn");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .await
    .expect("schema_version ddl");
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        turso::params![version],
    )
    .await
    .expect("insert version");
}

/// `connect_default_with_training_fallback` must recover when the telemetry sidecar is also legacy.
#[allow(unsafe_code)] // Rust 2024: `set_var` / `remove_var` are `unsafe`; mutex serializes this test.
#[tokio::test]
async fn connect_default_with_training_fallback_resets_stale_sidecar() {
    use std::sync::{Mutex, OnceLock};

    static DATA_DIR_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _g = DATA_DIR_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

    let dir = tempfile::tempdir().expect("tempdir");
    seed_legacy_schema_version_only(&dir.path().join("vox.db"), 38).await;
    seed_legacy_schema_version_only(&dir.path().join("vox_training_telemetry.db"), 38).await;

    let old = std::env::var("VOX_DATA_DIR").ok();
    // SAFETY: `DATA_DIR_LOCK` serializes tests that touch `VOX_DATA_DIR` for this module.
    unsafe {
        unsafe { std::env::set_var("VOX_DATA_DIR", dir.path()) };
    }

    let db = VoxDb::connect_default_with_training_fallback()
        .await
        .expect("fallback connects after resetting sidecar");
    assert_eq!(
        db.schema_version().await.expect("schema_version"),
        BASELINE_VERSION
    );
    drop(db);

    unsafe {
        match &old {
            Some(s) => unsafe { std::env::set_var("VOX_DATA_DIR", s) },
            None => std::env::remove_var("VOX_DATA_DIR"),
        }
    }

    let sidecar = dir.path().join("vox_training_telemetry.db");
    let check = VoxDb::connect(DbConfig::local(sidecar.to_string_lossy().to_string()))
        .await
        .expect("reopen sidecar");
    assert_eq!(
        check.schema_version().await.expect("sidecar version"),
        BASELINE_VERSION
    );
}

#[test]
fn resolve_canonical_matches_resolve_standalone() {
    let a = DbConfig::resolve_canonical().expect("canonical");
    let b = DbConfig::resolve_standalone().expect("standalone");
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}
