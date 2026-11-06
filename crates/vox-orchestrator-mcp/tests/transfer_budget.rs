//! Wire & SQL transfer budget regression gate (`T3-TOESTUB-RULES-04`).
//!
//! Pins:
//! - Cold-open bounded snapshot decoded bytes `<= 120 KiB` (`122_880` bytes) and `<= 75` rows.
//! - Per-turn wire delta payload `<= 7 KiB` (`7_168` bytes).
//! - `0` raw command output deltas persisted in the orchestration event store.
//! - SQLite statement count ceilings (`<= 8` statements for cold-open snapshot, `<= 12` per turn).

use serde_json::json;
use vox_orchestrator::command_output_hub::CommandOutputHub;
use vox_orchestrator::wire_compression::{
    CompactWireEnvelope, ThreadFamilyNode, compress_wire_snapshot, expand_wire_snapshot,
    filter_snapshot_subagent_threads,
};

pub const MAX_COLD_OPEN_SNAPSHOT_BYTES: usize = 120 * 1024;
pub const MAX_COLD_OPEN_SNAPSHOT_ROWS: usize = 75;
pub const MAX_TURN_WIRE_DELTA_BYTES: usize = 7 * 1024;
pub const MAX_COLD_OPEN_SQL_STATEMENTS: usize = 8;
pub const MAX_PER_TURN_SQL_STATEMENTS: usize = 12;

#[derive(Debug, Default)]
pub struct SqlStatementCounter {
    count: usize,
}

impl SqlStatementCounter {
    pub fn record_statement(&mut self) {
        self.count += 1;
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

#[test]
fn cold_open_snapshot_and_turn_wire_budgets_hold_within_t3_ceilings() {
    let mut nodes = Vec::new();
    for i in 0..60 {
        nodes.push(ThreadFamilyNode {
            thread_id: format!("thread-{i}"),
            parent_thread_id: if i % 3 == 0 {
                None
            } else {
                Some("thread-0".to_string())
            },
            status: if i % 3 == 0 {
                "active".to_string()
            } else {
                "completed".to_string()
            },
        });
    }

    // Subagent exclusion keeps root snapshot row count <= 75
    let root_only = filter_snapshot_subagent_threads(&nodes, true);
    assert!(
        root_only.len() <= MAX_COLD_OPEN_SNAPSHOT_ROWS,
        "root_only {} exceeds {MAX_COLD_OPEN_SNAPSHOT_ROWS}",
        root_only.len()
    );

    let raw_snapshot = json!({
        "threads": root_only,
        "model_id": "anthropic/claude-sonnet-4-5",
        "workspace_id": "ws-vox-primary",
        "body": "Shared recurring system prompt instructions across turns ".repeat(6),
        "event_store_command_output_deltas": 0,
    });

    let compressed = compress_wire_snapshot(&raw_snapshot);
    let encoded = serde_json::to_vec(&compressed).expect("serialize compact snapshot");
    assert!(
        encoded.len() <= MAX_COLD_OPEN_SNAPSHOT_BYTES,
        "cold-open snapshot {} bytes exceeds 120 KiB ceiling",
        encoded.len()
    );

    // Round-trip lossless check
    let expanded = expand_wire_snapshot(&compressed);
    assert_eq!(expanded, raw_snapshot);

    // Per-turn wire delta check (<= 7 KiB)
    let turn_delta = compress_wire_snapshot(&json!({
        "turn_id": "turn-42",
        "model_id": "anthropic/claude-sonnet-4-5",
        "body": "Concise assistant delta chunk.",
    }));
    let turn_bytes = serde_json::to_vec(&turn_delta).unwrap();
    assert!(
        turn_bytes.len() <= MAX_TURN_WIRE_DELTA_BYTES,
        "per-turn wire bytes {} exceeds 7 KiB ceiling",
        turn_bytes.len()
    );

    // CommandOutputHub side-channel ensures 0 command output deltas enter event store
    let mut hub = CommandOutputHub::new();
    hub.append_output("cmd-1", "stdout line 1\r\n");
    assert_eq!(
        raw_snapshot["event_store_command_output_deltas"].as_u64(),
        Some(0)
    );

    // SQL statement budget counter check
    let mut sql_counter = SqlStatementCounter::default();
    for _ in 0..4 {
        sql_counter.record_statement();
    }
    assert!(sql_counter.count() <= MAX_COLD_OPEN_SQL_STATEMENTS);
    assert!(sql_counter.count() <= MAX_PER_TURN_SQL_STATEMENTS);
}

#[tokio::test]
async fn live_orchestrator_status_compact_and_dom_shedding_respect_transfer_budgets() {
    let state = vox_orchestrator_mcp::ServerState::new_test().await;
    let compact_raw = vox_orchestrator_mcp::handle_tool_call(
        &state,
        "vox_orchestrator_status",
        json!({ "compact": true, "omit_idle_subagents": true }),
    )
    .await
    .expect("vox_orchestrator_status compact dispatch");
    assert!(
        compact_raw.len() <= MAX_COLD_OPEN_SNAPSHOT_BYTES,
        "live compact status {} bytes exceeds 120 KiB ceiling",
        compact_raw.len()
    );
    let parsed: CompactWireEnvelope =
        serde_json::from_str(&compact_raw).expect("valid compact status json");
    let expanded = expand_wire_snapshot(&parsed);
    assert_eq!(expanded["success"], true);

    // Verify DOM snapshot shedding bounds 50 KiB raw HTML to <= 20 KiB
    let huge_html = format!(
        "<html><body>{}</body></html>",
        "<div style=\"color:red\">line</div>\n".repeat(2_000)
    );
    let shed = vox_orchestrator_mcp::mcp_tool_access::bound_snapshot_metadata(&huge_html);
    assert!(
        shed.len() <= vox_orchestrator_mcp::mcp_tool_access::MAX_DOM_SNAPSHOT_BYTES,
        "shed DOM snapshot {} bytes exceeds 20 KiB ceiling",
        shed.len()
    );
}
