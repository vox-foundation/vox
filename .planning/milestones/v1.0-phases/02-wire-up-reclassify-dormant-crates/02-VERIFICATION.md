---
phase: 02-wire-up-reclassify-dormant-crates
verified: 2026-09-26T04:30:00Z
status: passed
score: 3/3 must-haves verified
covered_files:
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/02-wire-up-reclassify-dormant-crates/02-01-PLAN.md"
  - ".planning/phases/02-wire-up-reclassify-dormant-crates/02-01-SUMMARY.md"
  - ".planning/phases/02-wire-up-reclassify-dormant-crates/02-02-PLAN.md"
  - ".planning/phases/02-wire-up-reclassify-dormant-crates/02-02-SUMMARY.md"
  - "contracts/ci/crate-edges.allow.v1.json"
  - "contracts/ci/crate-graph.v1.json"
  - "crates/vox-orchestrator/Cargo.toml"
  - "crates/vox-orchestrator/src/orchestrator/safety.rs"
  - "crates/vox-orchestrator/src/tool_receipt.rs"
  - "crates/vox-orchestrator/tests/tool_receipt_registry_gate.rs"
  - "docs/src/architecture/crate-classification-2026-05-08.md"
covered_digest: "v1:sha256:60af765b504f5b4f52def1dad3e880e2eb340b6ec06197b614fb8241291b2478"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 2: Wire Up / Reclassify Dormant Crates Verification Report

**Phase Goal:** Functionally-complete crates that were never adopted are active in their intended call path, and crate classification matches actual usage.
**Verified:** 2026-09-26
**Status:** passed (with 2 warnings, see Gaps Summary)
**Re-verification:** No, initial verification

Verification was done against committed objects (`git show <sha>:<path>`) for dc0aa6cb9, f4adfae89 and HEAD; test and gate commands ran on the working tree, which was confirmed byte-identical to dc0aa6cb9 for `tool_receipt.rs` and the new test file.

## Goal Achievement

### Observable Truths

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|--------------------|--------|----------|
| 1 | Exec-policy gate calls the exec-grammar risk classifier (real symbol `vox_container::exec_grammar::risk::classify`) | VERIFIED (literal); advisory-only, see W1 | `crates/vox-cli/src/commands/runtime/shell/check_terminal.rs` `run_check_rust_fallback` calls `risk::classify(&mut ast, &grammar_policy)` at HEAD (line ~438), and `parse_pipeline`/`evaluate` follow. Also called from `vox-container` `log_exec_risk`. `cargo run -q -p vox-cli -- ci exec-policy-contract` exit 0 (`rust fallback OK (6 hardcoded + 10 disk)`). No `crates/vox-exec-grammar` exists. |
| 2a | vox-orchestrator depends on vox-mcp-registry and validates tool names via `TOOL_REGISTRY` | VERIFIED (literal); no live caller, see W2 | dc0aa6cb9: `crates/vox-orchestrator/Cargo.toml` adds `vox-mcp-registry.workspace = true` under `[dependencies]`. `tool_receipt.rs` imports `vox_mcp_registry::TOOL_REGISTRY`; first statement of `issue_intent` is `if !TOOL_REGISTRY.iter().any(\|e\| e.name == tool_name) { return Err(ToolReceiptError::UnknownTool{..}) }`, before any UUID, hash or ledger insert. Exact-equality match, returns `Err` (fail closed). |
| 2b | vox-mcp-meta no longer exists in the workspace | VERIFIED | `ls crates` has no `vox-mcp-meta` or `vox-exec-grammar`. `git ls-tree -r HEAD` matches `vox-mcp-meta` only in `contracts/reports/scaling-audit/by-crate/vox-mcp-meta.md` (a historical report, not a crate). No `Cargo.toml` mentions it. |
| 3 | Crate catalog lists vox-search and vox-doc-inventory as CORE | VERIFIED | `docs/src/architecture/crate-classification-2026-05-08.md` at HEAD: `\| vox-search \| CORE \| many \|` (line 78, unchanged); `\| vox-doc-inventory \| CORE \| 1 (vox-cli) \|` (line 86, changed by f4adfae89 from DEAD). The false "no consumers" row in the zero-consumer table was removed. `crates/vox-cli/Cargo.toml:204` does depend on `vox-doc-inventory` unconditionally (claim confirmed). Diff is 1 insertion, 2 deletions, frontmatter and tombstone banner untouched. |

**Score:** 3/3 truths verified, 0 behavior-unverified.

### Independent confirmations requested

| Check | Result |
|-------|--------|
| (a) guard checks `e.name == tool_name` against `TOOL_REGISTRY`, fails closed | PASS (see 2a). `ToolReceiptError::UnknownTool` is returned; `safety.rs::issue_tool_receipt` propagates the `Result`. |
| (b) `crate-edges.allow.v1.json` gained exactly one exception | PASS. Diff is a single 7-line insertion at `exceptions[0]`: `vox-orchestrator -> vox-mcp-registry`, `authorized_by: user (explicit chat approval, 2026-09-25)`. `jq` hashes of `.edges` identical before/after; `.exceptions[1:]` after == `.exceptions` before; all other top-level keys identical. Exception count 34 -> 35. |
| (c) `crate-build-map.v1.json` and `fan-in-snapshot.v1.json` not in commit | PASS. `git show dc0aa6cb9 --name-only` matches neither. Commit touches 8 paths, all expected (incl. `crate-graph.v1.json`, +1 line `vox-mcp-registry` in the orchestrator dependency list, and `Cargo.lock` +1 line). |
| (d) `cargo test -p vox-orchestrator --test tool_receipt_registry_gate` | PASS: 3 passed, 0 failed (`issue_tool_receipt_rejects_near_miss_names`, `..._rejects_unknown_tool_fail_closed`, `..._accepts_every_registry_entry`). |
| (e) `cargo run -q -p vox-cli -- ci exec-policy-contract` | PASS: exit 0 (re-run, exit code captured directly). pwsh path skipped (pwsh not on PATH). |
| (f) classification doc rows | PASS: both CORE (see truth 3). |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-orchestrator/src/tool_receipt.rs` | VERIFIED | Substantive guard plus `ToolReceiptError`; re-exported in `lib.rs`. |
| `crates/vox-orchestrator/tests/tool_receipt_registry_gate.rs` | VERIFIED | 3 tests, pass (includes exhaustive accept of every registry entry and near-miss rejection). |
| `crates/vox-orchestrator/Cargo.toml` | VERIFIED | Unconditional dependency. |
| `contracts/ci/crate-edges.allow.v1.json` | VERIFIED | Exactly one authorized exception, rest byte-equal. |
| `contracts/ci/crate-graph.v1.json` | VERIFIED | +1 element, consistent with the new edge. |
| `docs/src/architecture/crate-classification-2026-05-08.md` | VERIFIED | vox-doc-inventory CORE. |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| `Orchestrator::issue_tool_receipt` (`safety.rs:11`) | `ToolReceiptLedger::issue_intent` | direct call, `?`/Result propagation | WIRED |
| `issue_intent` | `vox_mcp_registry::TOOL_REGISTRY` | `.iter().any(\|e\| e.name == tool_name)` | WIRED |
| `run_check_rust_fallback` | `risk::classify` | direct call per pipeline stage | WIRED (result unused, W1) |
| `run_check_for_ci` -> `exec_policy_contract` gate | `run_check_rust_fallback` | 02-02 SUMMARY claim; gate exits 0 with "rust fallback OK" output naming the fallback | WIRED |

### Data-Flow Trace (Level 4)

Not applicable to the guard (pure allow-list check). For SC#1, `ast.risk` set by `classify` is not read downstream (see W1).

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| REQ-dead-crate-wire-up | 02-01, 02-02 | SATISFIED (literal SCs) | Truths 1-3. REQUIREMENTS.md still shows it `Pending` in both the checkbox and traceability table; the orchestrator should flip it on close-out. |

No orphaned requirements mapped to Phase 2.

### Anti-Patterns Found

No TBD/FIXME/XXX debt markers introduced in the phase's implementation files (guard, test, doc). Not re-scanned exhaustively beyond those files.

### Known out-of-scope failures (not counted)

`ci crate-edges` (3 violations: vox-gui->vox-db-types, vox-gui->vox-research-shim, vox-research-shim->vox-compiler), fan-in-budget (10 regressions), crate-build-map-parity (60 drifted rows) are pre-existing on HEAD and were not re-run or attributed to this phase. The phase's own edge is covered by an exception entry, so it should not add a new crate-edges violation.

### Human Verification Required

None.

### Gaps Summary

No blocking gaps. Two warnings on depth of wiring; both are explicitly scoped out by the ROADMAP Phase 2 scope note and deferred to Phase 5 (TRUST-01), so they do not fail the phase.

- **W1 (SC#1 caveat, confirmed):** `risk::classify` is called by the gate but its result is advisory only. `run_check_rust_fallback` never reads `ast.risk`; the accept/reject decision comes from `grammar_policy.evaluate(&ast)` (which `classify` itself derives Blocked from) and the separate URL check. Elevated/Safe tiers are logged (`log_exec_risk` in vox-container) or discarded. The criterion "the gate calls the classifier" is literally met; "the classifier's verdict influences the gate" is not, and the phase does not claim it. Also stale comments still reference the non-existent `vox_exec_grammar` crate (`check_terminal.rs:415`); noted in the 02-02 SUMMARY as a deferred follow-up.
- **W2 (SC#2 depth):** `ToolReceiptLedger::issue_intent` / `Orchestrator::issue_tool_receipt` have no production callers at HEAD (only the new integration test and `safety.rs` unit tests call them; `git grep` across `crates/*/src` finds no other use). The dependency and fail-closed guard are real and tested, but no live MCP dispatch currently passes tool names through it, so today the guard blocks nothing in production. ROADMAP explicitly assigns live dispatch wiring to Phase 5 (TRUST-01). Consider reflecting this in the Phase 5 plan so it is not lost.
- Minor doc drift: the classification doc still labels `vox-mcp-registry` DEAD/"0 consumers" (line 85, and line 139); it is now consumed by vox-cli, vox-corpus and vox-orchestrator. Outside the stated SC#3 (vox-search, vox-doc-inventory only) and the page is a tombstoned snapshot, so not a gap.

---

_Verified: 2026-09-26_
_Verifier: Claude (gsd-verifier)_
