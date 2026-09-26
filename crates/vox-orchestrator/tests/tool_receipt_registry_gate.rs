//! Pins `Orchestrator::issue_tool_receipt` failing closed on tool names absent from
//! `vox_mcp_registry::TOOL_REGISTRY` (REQ-dead-crate-wire-up SC#2, CONTEXT.md D-02/D-04).
//! Covers ledger admission only; live MCP dispatch wiring is Phase 5 (TRUST-01).

use vox_mcp_registry::TOOL_REGISTRY;
use vox_orchestrator::{AgentId, Orchestrator, OrchestratorConfig, ToolReceiptError};

fn orch() -> Orchestrator {
    Orchestrator::new(OrchestratorConfig::for_testing())
}

fn registered(name: &str) -> bool {
    TOOL_REGISTRY.iter().any(|e| e.name == name)
}

#[test]
fn issue_tool_receipt_rejects_unknown_tool_fail_closed() {
    let name = "definitely_not_a_registered_tool";
    assert!(
        !registered(name),
        "test input must stay absent from the registry"
    );
    let orch = orch();

    let err = orch
        .issue_tool_receipt(AgentId(1), name, "{}")
        .expect_err("unknown tool must be rejected");

    assert_eq!(
        err,
        ToolReceiptError::UnknownTool {
            tool_name: name.to_string()
        }
    );
    assert!(orch.tool_ledger_handle().read().unwrap().is_empty());
}

#[test]
fn issue_tool_receipt_rejects_near_miss_names() {
    let orch = orch();
    for name in [
        "",
        "VOX_SUBMIT_TASK",
        "vox_submit_task ",
        " vox_git_status",
        "vox_git_status\n",
    ] {
        assert!(
            !registered(name),
            "near-miss {name:?} must stay absent from the registry"
        );
        let res = orch.issue_tool_receipt(AgentId(1), name, "{}");
        assert!(
            matches!(res, Err(ToolReceiptError::UnknownTool { .. })),
            "near-miss name {name:?} must be rejected, got {res:?}"
        );
    }
    assert!(orch.tool_ledger_handle().read().unwrap().is_empty());
}

#[test]
fn issue_tool_receipt_accepts_every_registry_entry() {
    assert!(!TOOL_REGISTRY.is_empty(), "registry must not be empty");
    let orch = orch();
    for entry in TOOL_REGISTRY {
        let id = orch
            .issue_tool_receipt(AgentId(1), entry.name, "{}")
            .unwrap_or_else(|e| panic!("registered tool {:?} rejected: {e}", entry.name));
        assert!(
            orch.verify_tool_receipt(&id),
            "receipt for {:?} must verify",
            entry.name
        );
    }
    assert_eq!(
        orch.tool_ledger_handle().read().unwrap().len(),
        TOOL_REGISTRY.len()
    );
}
