---
phase: 02-wire-up-reclassify-dormant-crates
plan: 02
subsystem: docs
tags: [exec-policy, crate-classification, verification]
requires: ["02-01"]
provides:
  - "Command-output evidence that ROADMAP SC#1 (exec-policy risk classification), SC#2's vox-mcp-meta clause and SC#3's vox-search half were already satisfied"
  - "vox-doc-inventory reclassified DEAD -> CORE in crate-classification-2026-05-08.md"
affects: []
key-files:
  modified:
    - docs/src/architecture/crate-classification-2026-05-08.md
key-decisions:
  - "Evidence-only for SC#1 / SC#2 clause 2 / SC#3 vox-search: no source edits (D-01)."
  - "No new SMOKE/REJECT exec-policy payloads (D-04): see below."
metrics:
  completed: 2026-09-25
status: complete
commits: 1
---

# Phase 2 Plan 02: Verification and vox-doc-inventory reclassification Summary

SC#1, SC#2's vox-mcp-meta clause and SC#3's vox-search half were verified as already true; vox-doc-inventory's false DEAD label in the classification doc is corrected to CORE (commit `f4adfae89`).

## Task 1: evidence (no edits)

- **SC#1.** `cargo run -q -p vox-cli -- ci exec-policy-contract` exited 0: `exec-policy-contract: rust fallback OK (6 hardcoded + 10 disk)`; `exec-policy-contract OK (schema + rust fallback; pwsh not on PATH — pwsh path skipped)`. Log: `target/phase02-02-exec-policy.txt`.
  - Call site 1: `risk::classify(&mut ast, &grammar_policy);` in `crates/vox-cli/src/commands/runtime/shell/check_terminal.rs` (`run_check_rust_fallback`), reached by the CI gate through `run_check_for_ci` (referenced twice in `exec_policy_contract.rs`).
  - Call site 2: `exec_grammar::risk::classify(&mut ast, &policy);` in `crates/vox-container/src/lib.rs` (`log_exec_risk`), invoked via `crate::log_exec_risk(&opts.image);` in both `docker.rs` and `podman.rs`.
  - Naming correction: ROADMAP SC#1 and REQUIREMENTS.md name a `vox_exec_grammar` crate. The real symbol is `vox_container::exec_grammar::risk::classify`, re-exported from `vox-container-types`. No standalone crate ever existed (`crates/vox-exec-grammar` absent).
- **SC#2, second clause.** `crates/vox-mcp-meta` does not exist. No tracked `Cargo.toml` or `Cargo.lock` mentions `vox-mcp-meta` or `vox-exec-grammar`. A positive control (`vox-mcp-registry` is found by the same pathspecs) shows the absence check is not vacuous. Deletion commit `72bde3718`.
- **SC#3, vox-search half.** The classification doc has exactly one `| vox-search | CORE | many |` row, and `vox-search = { workspace = true }` is under `[dependencies]` in both `crates/vox-cli/Cargo.toml` and `crates/vox-orchestrator/Cargo.toml`.

## D-04 (exec-policy payload determination)

No SMOKE/REJECT payload was added. D-04's exec-policy clause applies "if new wiring is needed per D-01's outcome", and none was needed. A new payload would also not be additive for `risk::classify`: it derives Blocked from the same `ExecPolicy::evaluate` call the gate runs next, and its Safe/Elevated outputs are advisory. `run_check_rust_fallback` never reads `ast.risk`, and `contracts/terminal/exec-policy.v1.yaml` has no risk-tier vocabulary, so no payload can be rejected because of `classify`. Any new payload would exercise `evaluate`, which the two REJECT payloads plus the 10-entry disk corpus already cover.

## Finding for the user

The literal SC#1 is met, but the computed risk tier is observability-only today: it is logged by `log_exec_risk` and discarded by `run_check_rust_fallback`. Making Elevated mean "requires confirmation" would be a new feature with a contract change, outside REQ-dead-crate-wire-up.

## Task 2: reclassification

`docs/src/architecture/crate-classification-2026-05-08.md`: the summary row is now `| vox-doc-inventory | CORE | 1 (vox-cli) | ... (crates/vox-cli/Cargo.toml:204); reclassified from DEAD 2026-09-25 |` and the false deletion-candidate row is removed. The doc is 203 lines, lines 1-16 (frontmatter, H1, tombstone banner) hash unchanged, the vox-search and both vox-mcp-registry rows are untouched, and `vox-doc-pipeline --lint-only` reports `no hard errors`. The commit touches only this file (numstat 1 added, 2 deleted).

## Not done (deliberate)

- Stale `vox-exec-grammar` crate-name mentions in comments (`check_terminal.rs:415`, `crates/vox-container/src/lib.rs:28`). Editing files under `crates/vox-cli/src/commands/` triggers the `command-sync` pre-commit generator, which rewrites `contracts/cli/command-registry.yaml` and the command-catalog baseline, both held dirty by another session. Follow-up once that work lands.
- The classification doc's two `vox-mcp-registry` rows (summary table and deletion candidates) are now stale too: the crate has consumers in vox-cli, vox-corpus, vox-orchestrator-mcp, vox-skill-discovery, and vox-orchestrator after 02-01. Outside this requirement; optional follow-up.

## Deviations

- The commit trailer is `Claude Sonnet 5`, per the session's attribution instruction, not the `Opus 5.5` text in the plan.
- The plan was executed inline by the orchestrating session, not by a subagent, on `main` per the user's instruction.
