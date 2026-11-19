---
phase: 2
phase_name: "wire-up-reclassify-dormant-crates"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 6
  patterns: 1
  surprises: 2
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 2 Learnings: wire-up-reclassify-dormant-crates

## Decisions

### Guard lives in ToolReceiptLedger::issue_intent so every issuing path (is...
Guard lives in ToolReceiptLedger::issue_intent so every issuing path (issue, issue_tool_receipt) is covered, not just the facade.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-01-SUMMARY.md

---

### Exactly one crate-edge exception (vox-orchestrator -> vox-mcp-registry, ...
Exactly one crate-edge exception (vox-orchestrator -> vox-mcp-registry, 2026-09-25, user chat approval); edges array, other exceptions, fan-in snapshot untouched.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-01-SUMMARY.md

---

### crate-graph.v1.json regenerated in the same commit; crate-build-map.v1.j...
crate-graph.v1.json regenerated in the same commit; crate-build-map.v1.json deliberately not regenerated (generator cannot run here without unrelated churn; already 60 rows out of parity on HEAD).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-01-SUMMARY.md

---

### Evidence-only for SC#1 / SC#2 clause 2 / SC#3 vox-search
Evidence-only for SC#1 / SC#2 clause 2 / SC#3 vox-search: no source edits (D-01).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-02-SUMMARY.md

---

### No new SMOKE/REJECT exec-policy payloads (D-04)
No new SMOKE/REJECT exec-policy payloads (D-04): see below.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 02-02-SUMMARY.md

## Lessons

### [Process] Commit made by the parent session
**[Process] Commit made by the parent session.** The protected-branch pre-commit assertion (HEAD on `main`, no `git.allow_default_branch_commits`) halted the executor. The parent session then committed `dc0aa6cb9` on `main` per the user's instruction. The executor had staged the 8 paths and passed the Step 6(g) mechanical staged-set check beforehand. The commit message and hashes are as planned.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### [Rule 3 - Blocking, environmental] Corrupt working-tree Cargo.lock
**[Rule 3 - Blocking, environmental] Corrupt working-tree `Cargo.lock`.** After the commit, another session left stray and duplicated lines in the `workspace-hack` section, so every cargo call failed with a TOML parse error at line 21164. The coordinator and user repaired it (HEAD's version restored, other sessions' entries re-derived by cargo). The mutation proof and clippy ran only after the repair. The executor did not touch the lockfile. This does not affect the committed `Cargo.lock`, which is 1 line added, 0 deleted.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### [Tooling] cmp replaced by diff
**[Tooling] `cmp` replaced by `diff`.** `cmp` is blocked by the lean-ctx shell allowlist, so verify block 6's fan-in comparison used `diff` on the same two `.lines` files. The result was identical, `diff=0`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### [Tooling] Log colors and lean-ctx roots
**[Tooling] Log colors and lean-ctx roots.** Cargo logs contain ANSI color codes. I stripped them with `sed` before the greps that need exact strings. The Task 2 cargo runs used `LEAN_CTX_EXTRA_ROOTS=.../target` to get past a lean-ctx "path escapes project root" error caused by the shell hook rooting at another project. This did not affect results.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 02-01-SUMMARY.md

---

### No blocking gaps
No blocking gaps. Two warnings on depth of wiring; both are explicitly scoped out by the ROADMAP Phase 2 scope note and deferred to Phase 5 (TRUST-01), so they do not fail the phase.

**Context:** Verified in phase acceptance checks.
**Source:** 02-VERIFICATION.md

---

### W1 (SC#1 caveat, confirmed)
**W1 (SC#1 caveat, confirmed):** `risk::classify` is called by the gate but its result is advisory only. `run_check_rust_fallback` never reads `ast.risk`; the accept/reject decision comes from `grammar_policy.evaluate(&ast)` (which `classify` itself derives Blocked from) and the separate URL check. Elevated/Safe tiers are logged (`log_exec_risk` in vox-container) or discarded. The criterion "the gate calls the classifier" is literally met; "the classifier's verdict influences the gate" is not, and the phase does not claim it. Also stale comments still reference the non-existent `vox_exec_grammar` crate (`check_terminal.rs:415`); noted in the 02-02 SUMMARY as a deferred follow-up.

**Context:** Verified in phase acceptance checks.
**Source:** 02-VERIFICATION.md

## Patterns

### registry-backed allow-list guard as first statement of the issuing path
registry-backed allow-list guard as first statement of the issuing path

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 02-01-SUMMARY.md

## Surprises

### Estimate variance on 02-01
Estimate variance on 02-01: planned 78000 tokens vs 3000 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 02-01-PLAN.md, 02-01-SUMMARY.md

---

### [Rule 3 - Blocking, environmental] Corrupt working-tree Cargo.lock
**[Rule 3 - Blocking, environmental] Corrupt working-tree `Cargo.lock`.** After the commit, another session left stray and duplicated lines in the `workspace-hack` section, so every cargo call failed with a TOML parse error at line 21164. The coordinator and user repaired it (HEAD's version restored, other sessions' entries re-derived by cargo). The mutation proof and clippy ran only after the repair. The executor did not touch the lockfile. This does not affect the committed `Cargo.lock`, which is 1 line added, 0 deleted.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 02-01-SUMMARY.md

