---
phase: 87-doc-pipeline-consumer-legacy-mermaid-ratchet
plan: 03
subsystem: doc-pipeline
tags: [rust, ci, pre-push, ratchet, mermaid, base-ref-gate]
status: complete

requires:
  - phase: 87-02
    provides: mermaid_gate::{MERMAID_BASELINE_REL, Baseline, parse}, seeded legacy-mermaid baseline
provides:
  - "mermaid_baseline_gate_failures: pure base-vs-head rule (per-path head count may not exceed base count; deleted baseline fails; unparsable side fails)"
  - "vox-cli commands/ci/mermaid_baseline.rs run_mermaid_baseline_gate: reuses file_at + resolve_base (fail closed)"
  - "Fast pre-push step 'vox-doc-pipeline legacy-Mermaid baseline (shrink-only vs base)' (hosted-CI linux job)"
  - "Docs rows: local-ci-pre-push.md fast tier, where-things-live.md ratchet row"
affects: [88]

requirements-completed: [DOCS-01, DOCS-02]

actuals:
  tokens: 22000
  tasks: 2
  commits: 3
plan_head_before: d95c0613757b7423f6ead73324a0e31f5dd190b5
plan_head_after: 38852a4952eda807df6df284102fb175a683cf1b

tech-stack:
  added: []
  patterns:
    - "Pure text-vs-text gate in vox-doc-pipeline + thin vox-cli git-gathering step, mirroring the facet backlog gate"

key-files:
  created:
    - crates/vox-cli/src/commands/ci/mermaid_baseline.rs
  modified:
    - crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs
    - crates/vox-doc-pipeline/src/pipeline/mod.rs
    - crates/vox-cli/src/commands/ci/mod.rs
    - crates/vox-cli/src/commands/ci/pre_push.rs
    - crates/vox-cli/src/commands/ci/operations_catalog_exposure.rs
    - docs/src/contributors/local-ci-pre-push.md
    - docs/src/architecture/where-things-live.md

key-decisions:
  - "No new crate edge or dependency; the phase total stays at the single 87-01 exception (7 0 numstat)"
  - "Pre-existing capability-index staleness (no crate.vox-diagram row) left to hosted-CI autoregen, recorded in deferred-items.md"

coverage:
  - id: D1
    description: "A PR that raises its own baseline (and adds the matching fence) fails the base-ref gate; new file fails; shrinking passes; deleted baseline fails; unresolvable base fails closed"
    requirement: DOCS-01
    verification:
      - kind: unit
        ref: "vox-cli commands::ci::mermaid_baseline::tests::gate_fails_on_upward_edit_against_the_base_ref"
        status: pass
  - id: D2
    description: "Pure rule: pre-seed/bootstrap/identical/lowered/removed pass; 1 -> 2 and new file give exactly one message; deleted and invalid base/head give one message"
    requirement: DOCS-01
    verification:
      - kind: unit
        ref: "pipeline::mermaid_gate::tests::gate_text_cases"
        status: pass
  - id: D3
    description: "The step runs in the fast pre-push tier and prints 'legacy-Mermaid baseline gate: OK' on the bootstrap path"
    requirement: DOCS-01
    verification:
      - kind: other
        ref: "target/debug/vox ci pre-push (fast): 'legacy-Mermaid baseline gate: OK', scoped doc lint and doctest-md pass, 'profile fast - all checks passed'"
        status: pass
  - id: D4
    description: "vox-diagram fences are doc-lint validated and never doctest-compiled (87-01); closing gates green"
    requirement: DOCS-02
    verification:
      - kind: other
        ref: "nextest -p vox-doc-pipeline 93/93; crate-edges, crate-api-sync, affected-crates --check, contracts-index, vox-arch-check, hakari diff all exit 0"
        status: pass
---

# Phase 87 Plan 03: Base-Ref Gate and Phase Closing Summary

**The legacy-Mermaid baseline is now shrink-only across PRs: the fast pre-push tier (and so hosted CI's `linux` job) compares the head baseline with the base ref and fails any raised count, added file or deleted baseline, even when the PR edited its own baseline upward; bootstrap (absent at base) passes and every git read failure fails closed.**

## Accomplishments

- Task 1 (tracer, `06259739b`): `mermaid_baseline_gate_failures` in `mermaid_gate.rs` plus `pipeline::{MERMAID_BASELINE_REL, mermaid_baseline_gate_failures}` re-export; `commands/ci/mermaid_baseline.rs` (`run_mermaid_baseline_gate`, reusing `file_at` and `resolve_base`, no copy); `OwnedStep` + `step_mermaid_baseline_gate` in `pre_push.rs` (+9/-0 vs `2f0bd1d73`, limit 12); `mod mermaid_baseline;` (+1).
- Task 2 (`38852a495`): fast-tier clause in `local-ci-pre-push.md`, ratchet row in `where-things-live.md`.
- Non-blank sizes: `mermaid_baseline.rs` 119, `mermaid_gate.rs` 381 (< 400).

## TDD record

- RED (pure): with a stub returning `Vec::new()`, `pipeline::mermaid_gate::tests::gate_text_cases` FAILED (11 of 12 passed).
- RED (vox-cli): same stub, `gate_fails_on_upward_edit_against_the_base_ref` FAILED at `mermaid_baseline.rs:106` (`unwrap_err()` on an `Ok(())`: the upward edit was admitted).
- GREEN: 12/12 `pipeline::mermaid_gate::`; 4/4 vox-cli (`mermaid_baseline` + `facet_backlog`).

## Mutation proof (T-87-06)

`(n > was)` (grep 1) mutated to `(n > was + 1)` (grep of mutated form 1). Failing: `pipeline::mermaid_gate::tests::gate_text_cases` and `commands::ci::mermaid_baseline::tests::gate_fails_on_upward_edit_against_the_base_ref`. Restored from a scratchpad copy (`diff -q` identical, `was + 1` grep 0); 12/12 and 1/1 green again.

## Closing gates (all from the repo root, binary rebuilt at the current commit, freshness not skipped)

| Gate | Result |
|---|---|
| `cargo fmt -p vox-doc-pipeline --check`; `rustfmt --edition 2024 --check` on the two vox-cli files | exit 0 |
| `cargo clippy -p vox-doc-pipeline --all-targets -- -D warnings` | fails only on the 27 pre-existing `claims/` findings (out of scope); none in `mermaid_gate.rs` / `mod.rs` |
| `cargo nextest run -p vox-doc-pipeline` | 93/93 passed |
| `vox ci crate-edges` | OK (701 edges; one pre-existing stale-baseline warning for `vox-gui -> vox-db-types`) |
| `vox ci crate-api-sync` / `affected-crates --check` / `contracts-index` | OK / OK / OK (198 entries) |
| `cargo run -p vox-arch-check` | exit 0 (stale-perf WARNs only) |
| `cargo hakari generate --diff` | exit 0, empty |
| scoped doc lint of both edited docs | no hard errors |
| `git diff --numstat 2f0bd1d73 -- contracts/ci/crate-edges.allow.v1.json` | `7 0` (the single 87-01 exception); `fan-in-snapshot.v1.json` and `crates/vox-diagram` unchanged |
| `vox ci pre-push` (fast), as committed | FAILS at `ssot-drift`: capability-index stale (see Deviations) |
| `vox ci pre-push` (fast) with the index regenerated uncommitted, then reverted | passes: `facet backlog gate: OK`, `legacy-Mermaid baseline gate: OK`, scoped lint (8 files) and `doctest-md --strict` OK, `profile fast - all checks passed in 21732ms` |

## Deviations from Plan

**1. [Rule 3 - Blocking] vox-cli lib tests did not compile.** `OperationRow` initializer in the `operations_catalog_exposure.rs` test helper lacked `owner_crate`/`domain` (main already has them). Added `owner_crate: None, domain: None`. Commit `17f84c579`.

**2. [Out of scope, deferred] `ssot-drift` capability-index stale.** `contracts/capabilities/capability-index.v1.json` lacks a `crate.vox-diagram` row; it has never been in any commit, and `contracts/capabilities/` and its generator are identical between `2f0bd1d73` and HEAD, so it predates 87-03. `vox ci capability-index --write` produces exactly that row (plus `capabilityIndex.generated.ts`). I used it only to run the rest of the fast tier, then reverted both files; hosted CI's `ssot-autoregen` owns the real regeneration. Logged in `deferred-items.md`.

## Deferred Issues

See `deferred-items.md` (87-01 `claims/` clippy; 87-03 capability-index and vox-cli test-initializer notes).

## Known Stubs

None.

## Threat Flags

None. T-87-06 (tampering) mitigated by the base-ref compare and mutation proof; T-87-07 (fail-open) mitigated by `resolve_base` and `file_at`, tested with `refs/heads/definitely-missing`; T-87-SC: no dependency or edge added.

## Requirements

DOCS-01 ("Doc lint rejects any new mermaid fence ... via a shrink-only ratchet seeded at the fence count, landed before migration"): lint pass and seed (87-02) plus base-ref compare (this plan) verified. DOCS-02 verified (87-01, already ticked). ROADMAP SC3's self-heal-bot clause remains held for the user decision.

## Self-Check: PASSED

- `mermaid_baseline.rs`, this SUMMARY exist; commits `17f84c579`, `06259739b`, `38852a495` are ancestors of HEAD; `git rev-list --count d95c06137..38852a495` = 3 (matches `commits: 3`).
