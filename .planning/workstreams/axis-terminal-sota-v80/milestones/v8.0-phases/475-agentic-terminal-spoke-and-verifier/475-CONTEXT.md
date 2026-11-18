# Phase 475 Context: `OrganicSpokeKind::AgenticTerminal` (Spoke 9) & Tier-3B Stateful PTY Trajectory Round-Trip Verifier

**Workstream:** `axis-terminal-sota-v80`
**Phase:** 475
**Requirements:** `TERM-SPOKE-01`, `TERM-SPOKE-02`, `TERM-SPOKE-03`, `TERM-SPOKE-04`
**SSOT:** [`terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)

## Decisions
1. Preserve existing `SpokeKind::ALL` (`5`) and `OrganicSpokeKind::ALL` (`8`) array count invariants across `vox-orchestrator` and `vox-corpus` test suites instead of changing core array lengths.
2. Expose Spoke 9 (`"agentic_terminal"`) via `OrganicSpokeKind::AGENTIC_TERMINAL_TASK_FAMILY`, paired with `ShellSpokeTier::{StatelessPipeline, AgenticTerminal}` mapping cleanly to `SpokeKind::StructuredShell`.
3. Add Tier-3B multi-turn verification (`verify_terminal_session_trajectory` over `TerminalTrajectoryStep`) enforcing safety tiers (`ExecPolicyV1`), interactive `CREDENTIAL_PROMPT_USER_ONLY` yielding, and zero credential/secret leakage.
