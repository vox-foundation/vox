//! Arca SQL schema — **baseline V1** + ordered fragments (SSOT).
//!
//! Canonical DDL is defined by `SCHEMA_FRAGMENTS` and applied once as `baseline_sql()`.
//! Fresh databases store a single `schema_version` row with version **1**.
//! Legacy databases that already ran the historical multi-row `schema_version` chain (any
//! `MAX(version) > 1`) are rejected at open; use export → new file → import (see
//! `vox codex export-legacy` / ADR 004).
//!
//! SQL in each fragment is run through Turso `execute_batch` (see `store/open.rs`), which must not
//! include row-returning statements such as bare `SELECT`.

mod manifest;
mod pragmas;
mod v1;
mod v10;
mod v11;
mod v12;
mod v13;
mod v14;
mod v15;
mod v16;
mod v17;
pub(crate) mod v18;
mod v19;
mod v2;
mod v3;
mod v4;
mod v5;
mod v6;
mod v7;
mod v8;
mod v9;

pub mod domains;

pub use manifest::{
    BASELINE_VERSION, CODEX_API_REQUIRED_TABLES, CODEX_CHAT_TABLES, CODEX_REACTIVITY_TABLES,
    SCHEMA_FRAGMENTS, SchemaFragment, baseline_sql, schema_baseline_digest_hex,
};
pub use domains::coordination::SCHEMA_COORDINATION;

#[cfg(test)]
mod migration_chain_tests {
    use super::SCHEMA_FRAGMENTS;

    #[test]
    fn fragments_strictly_ordered_and_nonempty_except_v7() {
        assert_eq!(SCHEMA_FRAGMENTS.len(), 19);
        for (i, f) in SCHEMA_FRAGMENTS.iter().enumerate() {
            let n = i + 1;
            assert_eq!(f.name, format!("v{n}"));
            let sql = f.sql.trim();
            if n == 7 {
                assert!(sql.is_empty(), "only v7 may be empty");
            } else {
                assert!(!sql.is_empty(), "{0} must be non-empty", f.name);
            }
        }
    }

    #[test]
    fn chat_search_and_codex_in_fragments() {
        let v11 = SCHEMA_FRAGMENTS
            .iter()
            .find(|f| f.name == "v11")
            .map(|f| f.sql)
            .expect("v11");
        assert!(
            v11.contains("conversations") && v11.contains("conversation_messages"),
            "v11 must define chat tables"
        );
        let v12 = SCHEMA_FRAGMENTS
            .iter()
            .find(|f| f.name == "v12")
            .map(|f| f.sql)
            .expect("v12");
        assert!(v12.contains("conversation_tool_calls"));
        let v14 = SCHEMA_FRAGMENTS
            .iter()
            .find(|f| f.name == "v14")
            .map(|f| f.sql)
            .expect("v14");
        assert!(v14.contains("topics") && v14.contains("conversation_topics"));
        let v15 = SCHEMA_FRAGMENTS
            .iter()
            .find(|f| f.name == "v15")
            .map(|f| f.sql)
            .expect("v15");
        assert!(
            v15.contains("search_documents") && v15.contains("search_indexing_jobs"),
            "v15 must define search tables"
        );
        let v16 = SCHEMA_FRAGMENTS
            .iter()
            .find(|f| f.name == "v16")
            .map(|f| f.sql)
            .expect("v16");
        assert!(
            v16.contains("processing_runs")
                && v16.contains("processing_run_steps")
                && v16.contains("audit_log"),
            "v16 must define processing run + audit DDL"
        );
        let v17 = SCHEMA_FRAGMENTS
            .iter()
            .find(|f| f.name == "v17")
            .map(|f| f.sql)
            .expect("v17");
        assert!(
            v17.contains("research_sessions")
                && v17.contains("conversation_versions")
                && v17.contains("conversation_edges")
                && v17.contains("topic_evolution_events"),
            "v17 must define graph/research/topic-evolution DDL"
        );
        let v18 = SCHEMA_FRAGMENTS
            .iter()
            .find(|f| f.name == "v18")
            .map(|f| f.sql)
            .expect("v18");
        assert!(
            v18.contains("corpus_snapshots"),
            "v18 must define corpus_snapshots DDL"
        );
    }
}
