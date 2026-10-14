//! Tools whose handlers need a db / session manager must declare it in `requires`, so the
//! profile gate answers `subsystem_not_booted` instead of the handler's own fallback.
//! The tool lists come from the plan-03 degraded-mode audit.

use vox_mcp_registry::{Subsystem, registry_entry, subsystem_closure};

const DB_TOOLS: &[&str] = &[
    "vox_benchmark_list",
    "vox_benchmark_record",
    "vox_questioning_sync_ssot",
    "vox_questioning_pending",
    "vox_questioning_submit_answer",
    "vox_research_start",
    "vox_research_status",
    "vox_research_get",
    "vox_kb_search",
    "vox_kb_get",
    "vox_kb_cite",
    "vox_kb_status",
    "vox_kb_reindex",
    "vox_db_research_session_upsert",
    "vox_db_trust_rollups",
    "vox_db_trust_summary",
    "vox_db_trust_drift",
    "vox_db_trust_propagate",
];

const SESSION_TOOLS: &[&str] = &[
    "vox_session_create",
    "vox_session_list",
    "vox_session_reset",
    "vox_session_compact",
    "vox_session_info",
    "vox_session_cleanup",
];

fn assert_requires(tools: &[&str], needed: Subsystem) {
    let mut missing = Vec::new();
    for name in tools {
        let entry =
            registry_entry(name).unwrap_or_else(|| panic!("{name} is not in the MCP registry"));
        if !subsystem_closure(entry.requires).contains(&needed) {
            missing.push(*name);
        }
    }
    assert!(
        missing.is_empty(),
        "these tools need `{}` in catalog requires (contracts/operations/catalog.v1.yaml), then `vox ci operations-sync --target mcp --write`: {missing:?}",
        needed.as_str()
    );
}

#[test]
fn db_backed_tools_require_db() {
    assert_requires(DB_TOOLS, Subsystem::Db);
}

#[test]
fn session_tools_require_sessions() {
    assert_requires(SESSION_TOOLS, Subsystem::Sessions);
}
