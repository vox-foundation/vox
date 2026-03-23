//! Single-source Arca/Codex schema: ordered fragments, baseline SQL, digest, and capability metadata.
//!
//! The database records only **baseline version 1** in `schema_version`. Legacy chains (any row
//! with version greater than 1) are rejected at open; operators use export → fresh DB → import.

use std::sync::OnceLock;

use sha3::{Digest, Keccak256};

use super::{v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15, v16, v17, v18};

/// Row written to `schema_version` for a database on the baseline-V1 model.
pub const BASELINE_VERSION: i64 = 1;

/// One ordered SQL slice (historical `vN.rs` bodies); empty bodies are skipped in [`baseline_sql`].
#[derive(Debug, Clone, Copy)]
pub struct SchemaFragment {
    /// Stable fragment id (matches source file stem).
    pub name: &'static str,
    /// DDL/DML run through Turso `execute_batch` (no row-returning statements).
    pub sql: &'static str,
}

/// Ordered schema fragments — canonical source order for [`baseline_sql`].
pub const SCHEMA_FRAGMENTS: &[SchemaFragment] = &[
    SchemaFragment {
        name: "v1",
        sql: v1::SCHEMA_V1,
    },
    SchemaFragment {
        name: "v2",
        sql: v2::SCHEMA_V2,
    },
    SchemaFragment {
        name: "v3",
        sql: v3::SCHEMA_V3,
    },
    SchemaFragment {
        name: "v4",
        sql: v4::SCHEMA_V4,
    },
    SchemaFragment {
        name: "v5",
        sql: v5::SCHEMA_V5,
    },
    SchemaFragment {
        name: "v6",
        sql: v6::SCHEMA_V6,
    },
    SchemaFragment {
        name: "v7",
        sql: v7::SCHEMA_V7,
    },
    SchemaFragment {
        name: "v8",
        sql: v8::SCHEMA_V8,
    },
    SchemaFragment {
        name: "v9",
        sql: v9::SCHEMA_V9,
    },
    SchemaFragment {
        name: "v10",
        sql: v10::SCHEMA_V10,
    },
    SchemaFragment {
        name: "v11",
        sql: v11::SCHEMA_V11,
    },
    SchemaFragment {
        name: "v12",
        sql: v12::SCHEMA_V12,
    },
    SchemaFragment {
        name: "v13",
        sql: v13::SCHEMA_V13,
    },
    SchemaFragment {
        name: "v14",
        sql: v14::SCHEMA_V14,
    },
    SchemaFragment {
        name: "v15",
        sql: v15::SCHEMA_V15,
    },
    SchemaFragment {
        name: "v16",
        sql: v16::SCHEMA_V16,
    },
    SchemaFragment {
        name: "v17",
        sql: v17::SCHEMA_V17,
    },
    SchemaFragment {
        name: "v18",
        sql: v18::SCHEMA_V18,
    },
];

static BASELINE_SQL_CACHE: OnceLock<String> = OnceLock::new();

/// Full baseline DDL: concatenation of non-empty [`SCHEMA_FRAGMENTS`] (idempotent `IF NOT EXISTS` / `IF NOT EXISTS` indexes).
pub fn baseline_sql() -> &'static str {
    BASELINE_SQL_CACHE
        .get_or_init(|| {
            SCHEMA_FRAGMENTS
                .iter()
                .map(|f| f.sql.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .as_str()
}

/// Keccak-256 over [`baseline_sql`] bytes, lowercase hex with `0x` prefix (manifest identity).
pub fn schema_baseline_digest_hex() -> String {
    let mut h = Keccak256::new();
    h.update(baseline_sql().as_bytes());
    let d = h.finalize();
    format!(
        "0x{}",
        d.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

/// Tables required for Codex reactivity (`append_codex_change`, SSE, lineage helpers).
pub const CODEX_REACTIVITY_TABLES: &[&str] = &[
    "codex_change_log",
    "codex_subscriptions",
    "codex_schema_lineage",
    "codex_query_snapshots",
    "codex_projection_versions",
];

/// Tables required for `vox-codex-api` routes that exercise chat/search/codex together (former “≥ V15” gate).
pub const CODEX_API_REQUIRED_TABLES: &[&str] = &[
    "codex_change_log",
    "codex_query_snapshots",
    "search_documents",
    "search_document_chunks",
    "search_indexing_jobs",
];

/// Tables required for `vox_db::codex_chat` helpers.
pub const CODEX_CHAT_TABLES: &[&str] = &[
    "conversations",
    "conversation_messages",
    "conversation_tool_calls",
    "usage_limit_definitions",
    "usage_counter_snapshots",
    "topics",
    "conversation_topics",
    "conversation_message_topics",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_nonempty_and_idempotent_fragments_cover_chat_and_search() {
        let b = baseline_sql();
        assert!(!b.trim().is_empty());
        assert!(
            b.contains("conversations") && b.contains("conversation_messages"),
            "baseline must include chat DDL"
        );
        assert!(
            b.contains("search_documents") && b.contains("search_indexing_jobs"),
            "baseline must include search DDL"
        );
        assert!(b.contains("codex_change_log"));
        assert!(
            b.contains("processing_runs")
                && b.contains("processing_run_steps")
                && b.contains("audit_log"),
            "baseline must include V16 processing/audit DDL"
        );
        let d = schema_baseline_digest_hex();
        assert!(d.starts_with("0x") && d.len() > 8);
    }

    #[test]
    fn fragments_match_historical_v_labels() {
        assert_eq!(SCHEMA_FRAGMENTS.len(), 18);
        for (i, f) in SCHEMA_FRAGMENTS.iter().enumerate() {
            assert_eq!(f.name, format!("v{}", i + 1));
        }
    }
}
