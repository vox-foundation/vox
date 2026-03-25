//! Single-source Arca/Codex schema: ordered fragments, baseline SQL, digest, and capability metadata.
//!
//! **SSOT:** This manifest defines the current global Arca schema collapsed into logical domains.
//! Baseline version is [`BASELINE_VERSION`] (see monolithic DDL in `baseline_sql()`).

use super::domains;
use sha3::{Digest, Keccak256};
use std::sync::OnceLock;

/// Latest unified schema baseline version for new and existing databases.
pub const BASELINE_VERSION: i64 = 38;

/// One ordered SQL slice (domain-scoped DDL); empty bodies are skipped in [`baseline_sql`].
#[derive(Debug, Clone, Copy)]
pub struct SchemaFragment {
    /// Stable fragment name (e.g., "agents").
    pub name: &'static str,
    /// DDL/DML run through Turso `execute_batch` (no row-returning statements).
    pub sql: &'static str,
}

/// Baseline tables required for Codex HTTP API (ready check).
pub const CODEX_API_REQUIRED_TABLES: &[&str] = &[
    "conversations",
    "conversation_messages",
    "search_documents",
    "agent_sessions",
    "ludus_events",
    "distributed_locks",
];

/// Subset of CORE chat tables for cleanup and diagnostics.
pub const CODEX_CHAT_TABLES: &[&str] = &["conversations", "conversation_messages"];

/// Tables that trigger reactivity/SSE broadcast on mutate.
pub const CODEX_REACTIVITY_TABLES: &[&str] = &[
    "conversation_messages",
    "agent_sessions",
    "behavior_events",
    "distributed_locks",
];

/// All non-empty SQL fragments to be applied as the monolithic baseline DDL.
pub const SCHEMA_FRAGMENTS: &[SchemaFragment] = &[
    SchemaFragment {
        name: "identity",
        sql: domains::identity::SCHEMA_IDENTITY,
    },
    SchemaFragment {
        name: "billing",
        sql: domains::billing::SCHEMA_BILLING,
    },
    SchemaFragment {
        name: "cas",
        sql: domains::cas::SCHEMA_CAS,
    },
    SchemaFragment {
        name: "codex",
        sql: domains::codex::SCHEMA_CODEX,
    },
    SchemaFragment {
        name: "conversations",
        sql: domains::conversations::SCHEMA_CONVERSATIONS,
    },
    SchemaFragment {
        name: "knowledge",
        sql: domains::knowledge::SCHEMA_KNOWLEDGE,
    },
    SchemaFragment {
        name: "execution",
        sql: domains::execution::SCHEMA_EXECUTION,
    },
    SchemaFragment {
        name: "agents",
        sql: domains::agents::SCHEMA_AGENTS,
    },
    SchemaFragment {
        name: "packages",
        sql: domains::packages::SCHEMA_PACKAGES,
    },
    SchemaFragment {
        name: "gamification",
        sql: domains::gamification::SCHEMA_GAMIFICATION,
    },
    SchemaFragment {
        name: "coordination",
        sql: domains::coordination::SCHEMA_COORDINATION,
    },
    SchemaFragment {
        name: "toestub",
        sql: domains::toestub::SCHEMA_TOESTUB,
    },
    SchemaFragment {
        name: "build_observability",
        sql: domains::build_observability::SCHEMA_BUILD_OBSERVABILITY,
    },
    SchemaFragment {
        name: "mens_cloud",
        sql: domains::mens_cloud::SCHEMA_POPULI_CLOUD,
    },
    SchemaFragment {
        name: "news",
        sql: domains::news::SCHEMA_NEWS,
    },
    SchemaFragment {
        name: "publication",
        sql: domains::publication::SCHEMA_PUBLICATION,
    },
];

/// Hex encoded Keccak-256 digest of [`baseline_sql`].
pub fn schema_baseline_digest_hex() -> String {
    let mut hasher = Keccak256::new();
    hasher.update(baseline_sql());
    format!("0x{:x}", hasher.finalize())
}

/// Monolithic SQL string containing all active fragments joined by double-newlines.
pub fn baseline_sql() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut full = String::new();
        for fragment in SCHEMA_FRAGMENTS {
            let sql = fragment.sql.trim();
            if !sql.is_empty() {
                full.push_str(sql);
                full.push_str("\n\n");
            }
        }
        full
    })
}
