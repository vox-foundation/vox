---
phase: 83-core-crate-parser-ir-diagnostics
verified: 2026-10-16T00:00:00Z
status: passed
score: 4/5 must-haves verified
covered_files:
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-01-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-01-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-02-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-02-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-03-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-03-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-04-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-04-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-05-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-05-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-06-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-06-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-07-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/83-core-crate-parser-ir-diagnostics/83-07-SUMMARY.md"
  - "crates/vox-diagram/Cargo.toml"
  - "crates/vox-diagram/src/check.rs"
  - "crates/vox-diagram/src/codes.rs"
  - "crates/vox-diagram/src/diag.rs"
  - "crates/vox-diagram/src/fixes.rs"
  - "crates/vox-diagram/src/fixgen.rs"
  - "crates/vox-diagram/src/ir.rs"
  - "crates/vox-diagram/src/lexer.rs"
  - "crates/vox-diagram/src/parser/mod.rs"
  - "crates/vox-diagram/src/print.rs"
  - "crates/vox-diagram/src/resolve.rs"
  - "docs/src/architecture/layers.toml"
covered_digest: "v3:sha256:a4fabb49f17cd948dca3fba1de629dc060377a4d82af12271b58ccfe10cfd32b"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Push the branch / open the draft PR and read the hosted CI run (`linux`, `tests`, `ssot-drift`, `vox ci crate-edges`, `vox-arch-check`, `crate-api-sync`, hakari)"
    expected: "All gates green with no new consumer edge on `vox-diagram`"
    why_human: "Hosted CI has not run (nothing pushed). The installed `vox` binary is stale against HEAD, so `vox ci crate-edges` / `crate-api-sync` / `ssot-drift` were not re-run by this verifier; only plan 83-07's local runs (recorded in its SUMMARY) and this verifier's hakari + arch-check re-runs back SC5."
---

# Phase 83: Core Crate - Parser, IR & Diagnostics Verification Report

**Phase Goal:** Authors and agents can write a `graph` diagram in the VoxDiagram DSL and get every mistake reported at once, with machine-applicable fixes and a canonical form, from a registered and governance-clean L2 `vox-diagram` crate.
**Verified:** 2026-10-16
**Status:** human_needed (all code-level truths verified; hosted-CI half of SC5 cannot be claimed)
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Every DGM-01 construct parses to a typed IR with serde output identical across runs | VERIFIED | `tests/graph_constructs.rs`: `all_constructs_ir` (insta snapshot, max_depth 3), `canonical_example_ir`, `ir_json_is_stable` (4 serialisations byte-equal), `crlf_input_yields_same_ir`; `determinism_guard_no_hash_collections` forbids hash collections in `src/`. All pass in my run. |
| 2 | One `check` returns every error with stable `diagram/*` code, severity, exact span; `fixes[]` of `{span, replacement}` and applying them to the seeded broken set yields a passing diagram | VERIFIED | `diagnostics_all_errors_six_independent`, `every_broken_fixture_emits_exactly_its_expected_codes` (20 broken fixtures), `codes_registry_covers_emitted_codes`, `codes_golden_is_subset` (29 codes); `fixes_repair_seeded_set` (10 repairable fixtures, each invalid, `repair(src,5)` valid and stable, snapshots); `fixes_only_on_errors`. |
| 3 | `fmt` canonical and idempotent; `parse(fmt(x)) == parse(x)` property-tested | VERIFIED | `fmt_props.rs` proptests `p2_fmt_is_idempotent`, `p3_fmt_preserves_ast_and_ir`, `p1`/`p5`; `fmt_golden.rs`; all pass. |
| 4 | Oversized, over-nested, container-endpoint inputs rejected with span-precise errors; adversarial inputs terminate, no panic | VERIFIED | `limits_adversarial.rs`: caps accept N / reject N+1 with line asserted (nodes 502, edges 1004, depth line 6, label, source cap on char boundary), `container_endpoint_is_rejected` (span is the endpoint ref span, `resolve.rs:314`), 10 s-bounded adversarial suite incl. 4000 nested headers, quote/bracket soup, Mermaid tokens, `hostile_props`. Minor: container-endpoint test asserts code and count but not the exact span (see Gaps note). |
| 5 | `vox-diagram` registered at L2 (single crate), `layers.toml` + `where-things-live.md` rows, hakari entry, crate docs; hosted CI gates pass with no new consumer edge | ? UNCERTAIN (WARNING) | Local evidence: `layers.toml:188` `layer = 2` (orphan_exempt, Phase 87 removes); `where-things-live.md` lines 113 and 234; `docs/src/api/vox-diagram.generated.md` + `index.generated.md` row; `Cargo.lock` +16 lines for `vox-diagram` only; `workspace-hack` dep present; `cargo hakari generate --diff` exit 0 (my run); `cargo run -p vox-arch-check` exit 0 (my run); crate-edges baseline and fan-in snapshot unchanged vs merge-base; no `Cargo.toml` outside `crates/vox-diagram` depends on `vox-diagram`. NOT verified here: `vox ci crate-edges`, `crate-api-sync`, `ssot-drift` (stale local `vox` binary) and every hosted-CI job. |

**Score:** 4/5 truths verified, 1 uncertain (hosted CI), 0 behavior-unverified.

### Behavior spot-checks (run by this verifier)

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Full crate suite | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` | 184 run, 184 passed | PASS |
| Lints | `cargo clippy -p vox-diagram --all-targets -- -D warnings` | exit 0 | PASS |
| Hakari | `cargo hakari generate --diff` | exit 0 | PASS |
| Arch check | `cargo run -p vox-arch-check` | exit 0 (only pre-existing stale-evidence warnings, none for vox-diagram) | PASS |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-diagram/{Cargo.toml, src/**}` (4,869 lines, all modules under 400 lines) | VERIFIED | Substantive and wired: `check.rs` -> lexer -> parser -> resolve -> `CheckReport`; public API in `lib.rs` (`check`, `parse`, `fmt`, `apply_fixes`, `repair`, `GRAMMAR_CARD`, `fence`, `sequence_ir`). |
| `src/fence.rs`, `src/sequence_ir.rs` | VERIFIED | Fence scanner tests in `fence_scan.rs`; sequence IR types with serde round trip in `sequence_ir.rs` (no parser, as planned). |
| `docs/src/architecture/layers.toml`, `where-things-live.md`, generated API page | VERIFIED | Present as above. |
| `contracts/ci/crate-graph.v1.json` | VERIFIED | Contains `vox-diagram`. |

### Key Links

| From | To | Status |
|------|----|--------|
| `check()` | lexer, parser, `resolve::resolve`, `MAX_SOURCE_BYTES` gate | WIRED (exercised by tracer and limit tests) |
| `repair`/`apply_fixes` | `fixgen` / diagnostics `fixes[]` | WIRED (seeded set, mutate-and-repair, hostile props) |
| `GRAMMAR_CARD` | `fmt`/`check` | WIRED (`grammar_card.rs` dogfoods; 1,747 bytes) |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DGM-01 | 83-01, 02, 03, 05, 07 | SATISFIED | SC1 evidence |
| DGM-03 | 83-04, 83-07 | SATISFIED | SC2 evidence; all-errors, no cascades |
| DGM-04 | 83-06, 83-07 | SATISFIED | SC2 fixes evidence; `repair_props`, `fix_safety` |
| DGM-05 | 83-05 | SATISFIED | SC3 evidence |
| DGM-06 | 83-01, 02, 04 | SATISFIED | SC4 evidence |

All five phase IDs appear in plan frontmatter and in REQUIREMENTS.md (marked Complete, Phase 83). No orphaned Phase-83 requirements. DGM-02 is mapped to Phase 86, correctly out of scope.

### Review state

`83-REVIEW.md` iteration 2 found 1 Critical (EOF-closer ordering for Mermaid-header documents); `83-REVIEW-FIX.md` iteration 2 records it fixed (commit `6e862a0ec`) with a regression test and mutation proof. No iteration-3 re-review exists, so the final fix is guarded by tests only, not independently re-reviewed. The iteration-1 fix report flags several fixes "requires human verification" (ordering, scope placement, lone-CR policy, empty-label fix, Mermaid header); each has a named regression test that passes in my run.

### Anti-Patterns

No unreferenced `TBD`/`FIXME`/`XXX` markers found as blockers in the phase's source; `unwrap_used`/`expect_used`/`panic` are denied outside tests by `lib.rs`; `#![forbid(unsafe_code)]`. No stubs: all public functions are backed by tests.

### Human Verification Required

1. **Hosted CI run**
   **Test:** Push the branch via draft PR; read `linux`, `tests`, `ssot-drift`, `vox ci crate-edges`, `vox-arch-check`, `crate-api-sync`, hakari results.
   **Expected:** Green, with no new consumer edge to `vox-diagram`.
   **Why human:** Nothing has been pushed, and the local `vox` binary is stale so those `vox ci` gates were not re-run by this verifier.

### Gaps Summary

No code-level gaps. Notes (non-blocking):
- `container_endpoint_is_rejected` does not assert the exact span (the code reports the endpoint ref span, so SC4 holds, but a span regression would not be caught by that test).
- The final review fix (`6e862a0ec`) has had no third review pass.
- Hosted-CI items are unverified, not failed.

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_

## Human Validation

2026-10-16: user accepted local evidence for SC5 (hosted CI gates); hosted CI will run on the milestone PR.
