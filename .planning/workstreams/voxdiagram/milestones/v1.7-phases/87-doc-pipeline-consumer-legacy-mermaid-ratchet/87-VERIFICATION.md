---
phase: 87-doc-pipeline-consumer-legacy-mermaid-ratchet
verified: 2026-10-17T21:00:00Z
status: passed
score: 2/3 roadmap success criteria verified (SC3 partial)
covered_files:
  - ".planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-01-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-01-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-02-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-02-SUMMARY.md"
  - ".planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-03-PLAN.md"
  - ".planning/workstreams/voxdiagram/phases/87-doc-pipeline-consumer-legacy-mermaid-ratchet/87-03-SUMMARY.md"
  - "contracts/ci/crate-edges.allow.v1.json"
  - "contracts/documentation/legacy-mermaid-baseline.v1.yaml"
  - "crates/vox-cli/src/commands/ci/mermaid_baseline.rs"
  - "crates/vox-cli/src/commands/ci/pre_push.rs"
  - "crates/vox-doc-pipeline/src/lib.rs"
  - "crates/vox-doc-pipeline/src/pipeline/diagrams.rs"
  - "crates/vox-doc-pipeline/src/pipeline/lint.rs"
  - "crates/vox-doc-pipeline/src/pipeline/mermaid_base_gate.rs"
  - "crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs"
  - "crates/vox-doc-pipeline/src/pipeline/mod.rs"
  - "crates/vox-doc-pipeline/src/pipeline/types.rs"
  - "docs/src/architecture/layers.toml"
  - "crates/vox-doc-pipeline/tests/fence_issues_public.rs"
covered_digest: "v3:sha256:b779c2a71b92e24cb918bc84fca9d7679638b4252d27eb3d62e9333a2199d84e"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "SC3 (amended 2026-10-17): the doc pipeline exposes `fence_issues(md)` for the self-heal bot"
    status: partial
    reason: >
      `fence_issues` is `pub(crate)` inside `mod diagrams;`, and `diagrams` is a private module of
      `pipeline` (`pipeline/mod.rs:9`). No `pub use` re-exports it (grep of all of `crates/`: only
      `diagrams.rs` mentions it). Privacy rules make it nameable only from inside `crate::pipeline`.
      The planned caller, `claims/self_heal_bot.rs` (a sibling module of `pipeline` under
      `crate::claims`), cannot reach it, and neither can any external crate. The path the STATE
      todo prescribes, `vox_doc_pipeline::diagrams::fence_issues(md)`, does not exist at all. The
      deferred wiring will not compile at the next `main` merge without this change. The function
      itself is correct (text in, `Vec<DiagramIssue>` out, exercised by the `diagrams::tests`);
      only the exposure is missing.
    artifacts:
      - path: "crates/vox-doc-pipeline/src/pipeline/diagrams.rs"
        issue: "`pub(crate) fn fence_issues` (line 15)"
      - path: "crates/vox-doc-pipeline/src/pipeline/mod.rs"
        issue: "`mod diagrams;` private (line 9); no `pub use diagrams::fence_issues`"
    missing:
      - "Make `fence_issues` `pub` and re-export it, e.g. `pub use diagrams::fence_issues;` in pipeline/mod.rs (mirrors the existing `pub use facet_backlog::...` lines), or make `diagrams` a `pub mod`"
      - "One test that calls it through the public path (from outside `pipeline`) so the exposure is guarded"
      - "Correct the STATE.md pending todo, which names `vox_doc_pipeline::diagrams::fence_issues`, to whatever path is chosen"
deferred:
  - truth: "Self-heal bot calls `fence_issues(md)` from `validate_bot_patch_scope_and_gates` plus one test; baseline re-seed against main's 40 fences / 25 files"
    addressed_in: "next merge of local `main` (ceb1b5d1a) into milestone/v1.7-voxdiagram"
    evidence: "ROADMAP SC3 amendment and STATE.md Pending Todos (voxdiagram), user decision 2026-10-17. `claims/self_heal_bot.rs` does not exist on this branch."
---

# Phase 87: Doc-Pipeline Consumer & Legacy-Mermaid Ratchet Verification Report

**Phase Goal:** The docs lint validates every `vox-diagram` fence and stops the legacy Mermaid count from ever growing, before any migration begins.
**Verified:** 2026-10-17
**Status:** gaps_found (one small, mechanical gap in SC3; DOCS-01 and DOCS-02 are behaviorally achieved)
**Re-verification:** No, initial verification

## Goal Achievement

### Roadmap Success Criteria

| # | Success criterion | Status | Evidence |
|---|-------------------|--------|----------|
| SC1 | A new `mermaid` fence under `docs/src` (archive excluded) fails lint with a clear message; baseline seeded from the committed tree (33/21), shrink-only, base-ref compare fails if it grows; prose mentions and 4-backtick examples not counted | VERIFIED | See below |
| SC2 | A malformed `vox-diagram` fence fails lint with `diagram/*` and file:line; a valid one passes; the `vox` doctest runner never compiles it (exact info-string matching) | VERIFIED | See below |
| SC3 | `vox-doc-pipeline` re-exports `vox_diagram`; the doc pipeline exposes `fence_issues(md)`; `crate-edges` passes with exactly one new edge, excepted and citing the 2026-10-16 authorization; `orphan_exempt` removed | PARTIAL (FAILED on `fence_issues` exposure) | Re-export, edge and `orphan_exempt` verified; `fence_issues` is not reachable (see Gaps) |

**Score:** 2/3 success criteria fully verified.

### SC1 evidence (DOCS-01)

- Baseline `contracts/documentation/legacy-mermaid-baseline.v1.yaml` has 21 files / 33 fences. An independent `git ls-files docs/src | grep -v archive | grep -c mermaid-openers` count gives the same 21 / 33, so the seed matches the committed tree.
- Behavioral probe with the built binary (`target/debug/vox-doc-pipeline --lint-only --mermaid-only`) on a scratch tree: the real tree exits 0 ("no hard errors"). A scratch doc with one real fence, one prose mention, and one fence nested in a 4-backtick example produced exactly one `legacy-mermaid-growth` error: `new.md:3 — 1 mermaid fence(s), the legacy-Mermaid baseline allows 0; author new diagrams as vox-diagram`, exit 1. Prose and the nested example were not counted. (The other 21 errors in that probe are stale-entry errors because the scratch tree lacks the baselined docs; expected.)
- Base-ref compare: `mermaid_baseline_gate_failures` (pure rule, `mermaid_base_gate.rs`) and `run_mermaid_baseline_gate` (`vox-cli/.../mermaid_baseline.rs`, reuses `file_at` and `resolve_base`) are wired into the fast pre-push tier as `step_mermaid_baseline_gate` (`pre_push.rs:536`). Tests pass: `gate_fails_on_upward_edit_against_the_base_ref`, `gate_admits_a_git_detected_move_but_not_a_laundered_one`, `gate_text_cases` (bootstrap, deletion, unresolvable base fail closed, shrink passes).
- Review follow-up: `87-REVIEW-FIX.md` closes CR-01 (a whole-tree `--mermaid-only` step, `step_mermaid_lint` at `pre_push.rs:542`, runs regardless of changed extensions; test `mermaid_lint_step_ignores_which_extensions_changed` passes), WR-01..WR-04. Corresponding regression tests all pass in the nextest run.

### SC2 evidence (DOCS-02)

- `lint_one_file` calls `check_diagram_fences` (`lint.rs:189`), which runs `vox_diagram::fence::extract(md, "vox-diagram")` then `vox_diagram::check` and maps spans to file line:col (container prefix compensated). `tracer_broken_fence_fails_lint_with_file_line_col` passes (exit 1, `docs/src/diag.md:18:10: diagram/undeclared-node`).
- `doctest_runner_never_compiles_diagram_fences` passes, with the control `vox` fence being rejected.
- `near_miss_info_strings_are_hard_errors` passes (`vox-diagram,foo`, `vox-diagram foo` are hard `diagram-fence-info` errors, so no fence escapes both lint and renderer). `warnings_print_but_do_not_block` and `committed_docs_diagram_fences_are_valid` pass.

### SC3 evidence

| Item | Status | Evidence |
|------|--------|----------|
| `pub use vox_diagram as diagram` | VERIFIED | `crates/vox-doc-pipeline/src/lib.rs:7`; only `vox-doc-pipeline` and `vox-diagram` Cargo.tomls name `vox-diagram` (vox-cli has no new edge) |
| `crate-edges` passes with exactly one new edge | VERIFIED | `VOX_SKIP_FRESHNESS_CHECK=1 target/debug/vox ci crate-edges` prints "OK (701 live in-tree edges within baseline)". `git diff 2f0bd1d73 HEAD` on `crate-edges.allow.v1.json` is a single added `exceptions` entry `vox-doc-pipeline -> vox-diagram`, `authorized_by: user (explicit chat approval, 2026-10-16)`; `fan-in-snapshot.v1.json` unchanged. The stale-binary freshness error was due only to the docs-only HEAD commit. |
| `orphan_exempt` removed | VERIFIED | `layers.toml:188` vox-diagram row has no `orphan_exempt` |
| `fence_issues(md)` exposed | FAILED | See Gaps |

### Must-have truths from PLANs (spot summary)

| Plan | Truths | Status |
|------|--------|--------|
| 87-01 (5) | lint prints file:line:col; doctest runner proof; near-miss hard error; one excepted edge; re-export + orphan_exempt gone | All VERIFIED (tests pass; crate-edges OK). `vox-arch-check` was not re-run in this pass; SUMMARY claims it, the layers.toml evidence above is direct. |
| 87-02 (5) | whole-tree fail on growth/unlisted; fence-aware counting; fail-closed baseline; seed/tighten tools; real baseline 33/21 | All VERIFIED (tests `tree_tests::*`, `counts_are_fence_aware`, `seed_then_tighten_then_refuse_growth`; independent 33/21 count) |
| 87-03 (4) | fast-tier base-ref compare; bootstrap/delete/unresolvable semantics; shrink passes; phase gates | VERIFIED for the first three. Phase gates: doc-pipeline suite 98/98 pass, vox-cli gate tests 3/3 pass, crate-edges OK. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| doc-pipeline suite | `cargo nextest run -p vox-doc-pipeline` | 98 run, 98 passed | PASS |
| vox-cli gate tests | `cargo nextest run -p vox-cli --lib mermaid` | 3 run, 3 passed | PASS |
| crate edges | `vox ci crate-edges` (freshness override) | OK, 701 edges | PASS |
| real-tree ratchet | `vox-doc-pipeline --lint-only --mermaid-only` | exit 0 | PASS |
| new fence rejected | same, scratch tree with one new fence | exit 1, growth error | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| DOCS-01 | 87-02, 87-03 | New mermaid fences rejected via shrink-only ratchet, seeded 33/21 | SATISFIED | SC1 evidence |
| DOCS-02 | 87-01 | `vox-diagram` fences validated; `vox` doctest never compiles them | SATISFIED | SC2 evidence |

No orphaned requirements: DOCS-03 and DOCS-04 are mapped to Phase 89 in REQUIREMENTS.md.

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `crates/vox-doc-pipeline/src/claims/*` | 26 clippy findings (`cargo clippy -p vox-doc-pipeline --all-targets -D warnings`) | Info (pre-existing, classified honestly) | Every finding is under `claims/` (resolver, prune, mod, ledger, suppressions, gate), none in files this phase edited. Hosted CI clippy on affected crates will flag them; not a phase gap. Recorded in `deferred-items.md`. |
| `contracts/capabilities/capability-index.v1.json` | ssot-drift: missing `crate.vox-diagram` row | Info (pre-existing, generated) | Predates the phase (never in the committed index); `ssot-autoregen` regenerates on PR. Not an SC breach. |
| `crates/vox-doc-pipeline/src/pipeline/mod.rs` | 812 lines, over the 500-line guideline | Info | Pre-existing size; the phase added small edits and moved rule logic out into `mermaid_base_gate.rs`. |
| Debt markers (TBD/FIXME/XXX) in phase-authored files | none found | n/a | |

### Human Verification Required

None for this phase. Hosted CI (deferred to the milestone PR per user) is the remaining unrun gate; it is not a verification item.

### Gaps Summary

One gap, mechanical and small. The amended SC3 says the doc pipeline must expose `fence_issues(md)` so the self-heal bot can call it at the next `main` merge. The function exists and is correct, but it is `pub(crate)` in a private module, so no path outside `crate::pipeline` can name it. `claims::self_heal_bot` (same crate, different module) could not call it, and the STATE todo's path `vox_doc_pipeline::diagrams::fence_issues` does not exist. Fix: make it `pub`, add `pub use diagrams::fence_issues;` in `pipeline/mod.rs`, add one test via the public path, and correct the STATE todo wording. The self-heal-bot wiring and baseline re-seed themselves are legitimately deferred and are not counted as gaps.

---

_Verified: 2026-10-17_
_Verifier: Claude (gsd-verifier)_

## Gap Closure (2026-10-17)

- `fence_issues` is now `pub` and re-exported as `vox_doc_pipeline::pipeline::fence_issues` (commit above). New test `crates/vox-doc-pipeline/tests/fence_issues_public.rs` failed first with E0432, now passes; `cargo nextest run -p vox-doc-pipeline` 99/99. STATE todo path corrected. SC3 verified.
