# Phase 2: Wire Up & Reclassify Dormant Crates - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-23
**Phase:** 2-Wire Up & Reclassify Dormant Crates
**Areas discussed:** exec-policy gate host crate, TOOL_REGISTRY validation behavior, reclassification target file, test coverage depth

---

## Exec-grammar wiring (originally framed as "which host crate")

Verification during discussion found `crates/vox-exec-grammar` does not exist as a standalone crate — the REQUIREMENTS.md framing ("add dependency from vox-container or vox-cli-core") assumes a crate that isn't there. Exec-policy enforcement already lives in `vox-container-types::exec_grammar::policy`, consumed by `vox-cli/src/commands/runtime/shell/check_terminal.rs`. The original two-option question ("which host crate") was replaced with a question about how research should handle this discovery.

| Option | Description | Selected |
|--------|-------------|----------|
| Research verifies fully | Let research nail down whether check_terminal.rs's existing implementation actually satisfies ADR-026, or whether something is still genuinely missing | ✓ |
| Assume already done, verify only | Treat like Phase 1's crates — skip straight to a verification task | |

**User's choice:** Research verifies fully
**Notes:** Explicitly declined to presuppose the Phase-1 pattern repeats; research must establish ground truth independently.

---

## TOOL_REGISTRY validation behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Fail closed | Reject calls naming a tool not in TOOL_REGISTRY — matches security-conscious patterns elsewhere (capability gating, exec-policy) | ✓ |
| Log/enumerate only | Just surface known vs. unknown tools, no rejection | |

**User's choice:** Fail closed
**Notes:** vox-orchestrator confirmed to have zero references to vox-mcp-registry today (genuinely dormant).

---

## Reclassification target file

Only one genuine path existed here (not a real choice), so it was stated rather than asked: `docs/src/architecture/classification-ssot-2026.md` was checked directly and does not mention `vox-search` or `vox-doc-inventory` at all. Research must locate the actual classification source before any edit is planned.

---

## Test coverage depth

| Option | Description | Selected |
|--------|-------------|----------|
| Bare minimum | One test per new pub fn per AGENTS.md's Test-First Policy gate | |
| Broader coverage | Also add integration-style tests for the full wired path (exec-policy end-to-end, TOOL_REGISTRY fail-closed behavior) | ✓ |

**User's choice:** Broader coverage
**Notes:** This phase adds real wiring code (unlike Phases 1 and 4's pure doc/config edits), so the minimum gate alone was judged insufficient.

---

## Claude's Discretion

- Exact reclassification file/mechanism once research identifies it.
- Exact shape of any new exec-policy wiring code, if research finds genuine gaps.

## Deferred Ideas

None — discussion stayed within Phase 2's boundary.
