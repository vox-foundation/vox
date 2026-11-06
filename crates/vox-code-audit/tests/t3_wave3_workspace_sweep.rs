//! Workspace-wide dogfood sweep (`Phases 253–254`, `T3-AUDIT-SWEEP-01..04`, `T3-MCP-UI-SWEEP-01..04`):
//! Verifies that all Wave 2/3 Rust and TypeScript modules across `vox-orchestrator`,
//! `vox-orchestrator-mcp`, `vox-llm-egress`, `vox-search`, `vox-code-audit`, and `vox-gui/ui`
//! pass the 4 T3 static enforcement detectors (`UnreasonedSuppressionDetector`,
//! `TestClockAndSkipDetector`, `McpAccessAndErrorSafetyDetector`, `UiPerfHazardsDetector`)
//! with `0` violations.

use std::path::PathBuf;
use vox_code_audit::detectors::{
    mcp_access_and_error_safety::McpAccessAndErrorSafetyDetector,
    test_clock_and_skip::TestClockAndSkipDetector, ui_perf_hazards::UiPerfHazardsDetector,
    unreasoned_suppression::UnreasonedSuppressionDetector,
};
use vox_code_audit::rules::{DetectionRule, SourceFile};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const TARGET_FILES: &[&str] = &[
    // Phase 242
    "crates/vox-orchestrator/src/id_allocator.rs",
    "crates/vox-orchestrator/src/wire_compat.rs",
    "crates/vox-orchestrator/src/wire_compression.rs",
    "crates/vox-orchestrator-mcp/src/dei_tools/orchestrator_snapshot.rs",
    // Phase 243
    "crates/vox-orchestrator/src/command_output_hub.rs",
    "crates/vox-orchestrator/src/guest_share.rs",
    "crates/vox-orchestrator/src/thread_run_queue.rs",
    // Phase 244
    "crates/vox-llm-egress/src/repetition_guard.rs",
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/tool_refusal_detector.rs",
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs",
    "crates/vox-orchestrator-mcp/src/external_mcp.rs",
    // Phase 245
    "crates/vox-orchestrator/src/budget/tracker.rs",
    "crates/vox-orchestrator/src/models/bench_evidence.rs",
    "crates/vox-orchestrator/src/compaction.rs",
    "crates/vox-orchestrator/src/context_sanitizer.rs",
    // Phase 246
    "crates/vox-orchestrator/src/models/consensus.rs",
    "crates/vox-orchestrator/src/models/quality_analyzer.rs",
    "crates/vox-gui/ui/src/components/surfaces/Chat/ConsensusGridView.tsx",
    "crates/vox-gui/ui/src/components/surfaces/Chat/ConsensusGridView.test.tsx",
    // Phase 247
    "crates/vox-orchestrator-mcp/src/agy_worktree.rs",
    "crates/vox-orchestrator/src/agentos/checkpoint_engine.rs",
    "crates/vox-orchestrator/src/agentos/muse_pool.rs",
    "crates/vox-orchestrator/src/agentos/pr_reactors.rs",
    // Phase 248
    "crates/vox-orchestrator/src/context_handoff.rs",
    "crates/vox-orchestrator-mcp/src/mcp_tool_access.rs",
    // Phase 249
    "crates/vox-gui/ui/src/lib/chat/chunkedThreadLoader.ts",
    "crates/vox-gui/ui/src/lib/chat/deferredHighlight.ts",
    "crates/vox-gui/ui/src/lib/chat/branchTree.ts",
    "crates/vox-gui/ui/src/components/surfaces/Chat/BranchComparisonModal.tsx",
    "crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx",
    "crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx",
    "crates/vox-gui/ui/src/lib/chat/__tests__/branchAndLoader.test.ts",
    // Phase 250
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs",
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs",
    "crates/vox-search/src/fuzzy_trigram.rs",
    "crates/vox-gui/ui/src/lib/chat/promptHistory.ts",
    "crates/vox-gui/ui/src/lib/chat/__tests__/promptHistory.test.ts",
    // Phase 251
    "crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs",
    "crates/vox-orchestrator/src/external_importer.rs",
    "crates/vox-gui/ui/src/lib/connection/routeRacer.ts",
    "crates/vox-gui/ui/src/lib/connection/__tests__/routeRacer.test.ts",
    // Phase 252
    "crates/vox-code-audit/src/detectors/unreasoned_suppression.rs",
    "crates/vox-code-audit/src/detectors/test_clock_and_skip.rs",
    "crates/vox-code-audit/src/detectors/mcp_access_and_error_safety.rs",
    "crates/vox-code-audit/src/detectors/ui_perf_hazards.rs",
    "crates/vox-orchestrator-mcp/tests/transfer_budget.rs",
    // Phases 255–258 (Wave 3 Live Wiring)
    "crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs",
    "crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts",
    "crates/vox-gui/ui/src/components/surfaces/Chat/SessionInteropModal.tsx",
    "crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscriptWave3.test.tsx",
    "crates/vox-gui/ui/src/components/surfaces/Chat/SessionInteropAndCheckpoints.test.tsx",
    // Phases 283–288 (Wave 5 Deep Review & End-to-End Parity Closure)
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs",
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs",
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/harness_issue_judge.rs",
    "crates/vox-gui/src/commands/chat.rs",
    "crates/vox-gui/src/commands/chat_turn.rs",
    "crates/vox-gui/ui/src/lib/chat/streamWatchdog.ts",
    "crates/vox-gui/ui/src/lib/chat/__tests__/streamWatchdog.test.ts",
    "crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx",
    "crates/vox-gui/ui/src/components/ui/Glass.tsx",
    "crates/vox-gui/ui/src/components/ui/Button.tsx",
    "crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx",
    "crates/vox-gui/ui/src/components/dashboard/DashboardGrid.tsx",
];

#[test]
fn wave2_and_wave3_modules_pass_all_four_t3_toestub_detectors() {
    let root = workspace_root();
    let d_supp = UnreasonedSuppressionDetector::new();
    let d_skip = TestClockAndSkipDetector::new();
    let d_mcp = McpAccessAndErrorSafetyDetector::new();
    let d_ui = UiPerfHazardsDetector::new();

    let mut all_findings = Vec::new();
    for rel in TARGET_FILES {
        let abs = root.join(rel);
        let content = std::fs::read_to_string(&abs)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", abs.display(), e));
        let sf = SourceFile::new(PathBuf::from(rel), content);
        for det in [&d_supp as &dyn DetectionRule, &d_skip, &d_mcp, &d_ui] {
            if det.languages().contains(&sf.language) {
                all_findings.extend(det.detect(&sf, None));
            }
        }
    }

    assert!(
        all_findings.is_empty(),
        "Expected 0 T3 TOESTUB violations across {} target files, found {}:\n{:#?}",
        TARGET_FILES.len(),
        all_findings.len(),
        all_findings
    );
}
