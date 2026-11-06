---
phase: 288-code-audit-parity-and-workspace-remediation
plan: 01
subsystem: audit
tags: [vox-code-audit, oxlint-plugin-t3code, toestub, t3-wave5]
requires:
  - phase: 287-unified-message-id-and-live-gui-actions
    provides: [gui-pipeline]
provides:
  - T3 detector IDs in `ALL_KNOWN_IDS` with `diagnostic_id: Some(...)` (`T3-AUDIT-PARITY-01`)
  - Full `oxlint-plugin-t3code` parity for suppression / flaky-test detectors (`T3-AUDIT-PARITY-02`)
  - MCP access/error + UI layout/perf detectors + `GitExec` in `t3_wave3_tools` (`T3-AUDIT-PARITY-03`)
  - Cluster-6 43-site remediation verified clean (`T3-AUDIT-PARITY-04`)
affects: [vox-code-audit, vox-gui, vox-cli, vox-orchestrator-mcp]
tech-stack:
  added: []
  patterns: [diagnostic-id-catalog, preceding-line-toestub-ignore, raf-throttled-resize-observer]
key-files:
  created:
    - crates/vox-code-audit/src/detectors/unreasoned_suppression.rs
    - crates/vox-code-audit/src/detectors/test_clock_and_skip.rs
    - crates/vox-code-audit/src/detectors/mcp_access_and_error_safety.rs
    - crates/vox-code-audit/src/detectors/ui_perf_hazards.rs
  modified:
    - crates/vox-code-audit/src/diagnostics/catalog.rs
    - crates/vox-code-audit/src/detectors/mod.rs
    - crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/harness_issue_judge.rs
    - crates/vox-orchestrator-mcp/src/browser_tools.rs
    - crates/vox-cli/src/commands/run.rs
    - crates/vox-cli/src/commands/extras/ludus/battle.rs
    - crates/vox-cli/src/commands/diagnostics/stub_check/fix_pipeline.rs
    - crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx
    - crates/vox-gui/ui/src/components/gamify/store.ts
    - crates/vox-gui/ui/src/components/surfaces/Approvals/ApprovalsView.tsx
    - crates/vox-gui/ui/src/lib/chatCorrelation.ts
requirements-completed: [T3-AUDIT-PARITY-01, T3-AUDIT-PARITY-02, T3-AUDIT-PARITY-03, T3-AUDIT-PARITY-04]
completed: 2026-10-25
---

# Phase 288 Plan 01 Summary: T3 Detector Catalog, oxlint Parity & 43-Site Remediation

**Status:** Complete
**Date:** 2026-10-25
**Requirements:** `T3-AUDIT-PARITY-01`, `T3-AUDIT-PARITY-02`, `T3-AUDIT-PARITY-03`, `T3-AUDIT-PARITY-04`

## Changes Delivered

1. **Catalog registration (`T3-AUDIT-PARITY-01`):**
   - Added `LINT_UNREASONED_SUPPRESSION`, `TEST_FLAKY_WAIT_AND_UNREASONED_SKIP`, `MCP_ACCESS_AND_ERROR_SAFETY`, `UI_LAYOUT_AND_PERF_HAZARDS` to `diagnostics/catalog.rs` and `ALL_KNOWN_IDS`.
   - All four detectors emit `diagnostic_id: Some(...)` and are registered in `detectors/mod.rs`.

2. **Suppression / flaky-test parity (`T3-AUDIT-PARITY-02`):**
   - `unreasoned_suppression.rs`: Rust `#[allow]`/`#[expect]`, `.vox` + `toestub-ignore(...) -- <reason>`, preceding-line exemption, inline `@ts-expect-error <explanation>`.
   - `test_clock_and_skip.rs`: inline `#[cfg(test)]`, `.skip`/`.todo`/`xit`, sleep / `waitForTimeout` / rAF-promise waits.

3. **MCP + UI detectors & GitExec (`T3-AUDIT-PARITY-03`):**
   - `mcp_access_and_error_safety.rs`: `ToolResult::err`/`CallToolResult::error` with `:?`, `error = ?`, thick-handler `Command::new`, allowlisted exec modules (`git_exec`, `external_mcp`, `lifecycle`, `agy_doctor`).
   - `ui_perf_hazards.rs`: `transition-all`, raw `ResizeObserver`, `delete obj[k]`, CSS `:has(`, `scrollbar-gutter: stable` without `both-edges`; 8-line rAF look-ahead; preceding-line `toestub-ignore` exemption.
   - `t3_wave3_tools.rs` Git subprocesses routed through `GitExec` (no raw `std::process::Command::new`).

4. **43-site Cluster 6 remediation (`T3-AUDIT-PARITY-04`):**
   - 15 GUI TS suppressions reasoned; 4 CLI bare `toestub-ignore` reasoned; `harness_issue_judge` `error = %e`; 21 `transition-all` removed (fixture retained with ignore); ResizeObservers rAF-throttled / ignored; intentional `delete` sites annotated.
   - Focused toestub scan of the four T3 rules on `vox-gui/ui/src`, remediations targets: **UI = 0**, **MCP = 0**, **gui unreasoned = 0** for the Cluster 6 inventory.
   - Residual: pre-existing Rust `#[allow]`/`#[expect]` without `reason =` in `vox-cli` / `vox-orchestrator-mcp` (~82 warnings) — **outside** the Wave 5 Cluster 6 43-site list; follow-on debt, not a Phase 288 gap.

## Verification

```text
cargo test -p vox-code-audit --lib unreasoned_suppression   # 2 passed
cargo test -p vox-code-audit --lib test_clock_and_skip      # 3 passed
cargo test -p vox-code-audit --lib mcp_access_and_error_safety  # 2 passed
cargo test -p vox-code-audit --lib ui_perf_hazards          # 4 passed

target/debug/toestub --mode audit --format json \
  --rules vox/lint/unreasoned-suppression,vox/test/flaky-wait-and-unreasoned-skip,vox/mcp/access-and-error-safety,vox/ui/layout-and-perf-hazards \
  crates/vox-gui/ui/src crates/vox-cli/src crates/vox-orchestrator-mcp/src
# → UI residual 0, MCP residual 0, gui unreasoned 0 (Cluster 6 inventory clean)
```

## Notes

- Working tree remains heavily dirty from parallel milestones; Phase 288 did **not** mass-commit. Bertrand owns git hygiene.
- `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` is still an untracked new file on `main` (earlier wave artifact) — include when committing T3 work.
