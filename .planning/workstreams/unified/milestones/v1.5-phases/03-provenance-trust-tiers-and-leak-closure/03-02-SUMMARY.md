---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 02
subsystem: trust
tags: [sanitizer, prompt-injection, unicode-smuggling, proptest, red-team, canary]
requires: []
provides:
  - "vox_db_types::kb_text::{sanitize, SanitizedText, ContentHash, FindingOrigin, FindingStatus, IMAGE_HOST_ALLOWLIST}"
  - "vox_db::kb_text re-export (no new crate edge)"
  - "contracts/kb/redteam/cases.v1.jsonl (13 ASCII-escaped red-team cases)"
  - "REDTEAM_DIR + redteam_roots_failure in the Phase 1 canary gate"
affects: [03-03, 03-04, 03-09, 03-13]
tech-stack:
  added: []
  patterns: ["hand-rolled bounded-fixpoint text scanner (no regex)", "private-field smart-constructor newtype with compile_fail doctest", "independent proptest oracle (ranges restated outside the code under test)"]
key-files:
  created:
    - crates/vox-db-types/src/kb_text.rs
    - contracts/kb/redteam/cases.v1.jsonl
    - crates/vox-db-types/proptest-regressions/kb_text.txt
  modified:
    - crates/vox-db-types/src/lib.rs
    - crates/vox-db-types/Cargo.toml
    - Cargo.lock
    - crates/vox-db/src/lib.rs
    - crates/vox-research-shim/src/research/orchestrator/helpers.rs
    - crates/vox-cli/src/commands/kb/gate.rs
    - crates/vox-cli/src/commands/kb/gate/tests.rs
    - .voxignore
    - .cursorignore
    - .aiignore
    - .aiexclude
    - .gitignore
key-decisions:
  - "Unterminated HTML comment strips to end of input; unterminated `![alt](target` is judged on its leading destination token (a later `)` could complete it) and neutralized if not allowed"
  - "Reference and shortcut images (`![a][r]`, `![a]`) are always neutralized: the target lives in a definition elsewhere and cannot be judged, so fail closed"
  - "A destination is relative only if it has no scheme, is not protocol-relative, and contains none of backslash, ampersand, quote, angle or backtick (markdown and HTML decode backslash escapes and entities, so `https\\://x` and `https&colon;//x` are absolute)"
  - "If the 8-pass fixpoint has not converged the input is adversarially nested and the result is the empty string (fail closed, still idempotent)"
  - "HTML img attributes are tokenized with quote awareness so a `src=` inside another attribute value cannot masquerade as the real src; any srcset forces neutralization"
requirements-completed: [TRUST-05]
status: complete
commits: 6
plan_head_before: f9166a17b2ee500b2b16c2b2b9c5b2eb04ebad49
actuals:
  tokens: 10300
  tasks: 2
  commits: 6
duration: 95 min
completed: 2026-10-06
coverage:
  - deliverable: "sanitize() strips tag block, zero-width, bidi, variation selectors, HTML comments, off-allowlist images and neutralizes role tokens"
    human_judgment: false
    verification:
      - {kind: test, ref: "crates/vox-db-types/src/kb_text.rs#tests (16 unit tests)", status: pass}
  - deliverable: "Idempotence and output invariants under property testing"
    human_judgment: false
    verification:
      - {kind: test, ref: "crates/vox-db-types/src/kb_text.rs#tests::props (3 proptests, 100000 cases once)", status: pass}
  - deliverable: "SanitizedText only constructible through the sanitizer"
    human_judgment: false
    verification:
      - {kind: test, ref: "doctest kb_text::SanitizedText compile_fail + passing doctest", status: pass}
  - deliverable: "Red-team set neutralized and ASCII-only"
    human_judgment: false
    verification:
      - {kind: test, ref: "kb_text::tests::redteam_cases_file_is_ascii, redteam_cases_are_neutralized", status: pass}
  - deliverable: "Research shim delegates to the single sanitizer"
    human_judgment: false
    verification:
      - {kind: test, ref: "vox-research-shim helpers tests (6, 5 pre-existing unchanged + 1 new)", status: pass}
  - deliverable: "Red-team dir registered with the canary roots check"
    human_judgment: false
    verification:
      - {kind: test, ref: "vox-cli kb::gate canary_redteam_dir_is_outside_index_roots + canary_failures_do_not_report_redteam_roots_for_default_roots (16 gate tests pass)", status: pass}
---

# Phase 3 Plan 02: Single KB-text sanitizer, newtypes and red-team set Summary

One idempotent hand-rolled sanitizer in L0 `vox-db-types` (tag block, zero-width, bidi, variation selectors, HTML comments, off-allowlist images, role tokens, bounded fixpoint) reachable as `vox_db::kb_text`, with the research shim's `sanitize_evidence` reduced to a one-line delegate, `SanitizedText`/`ContentHash`/`FindingOrigin`/`FindingStatus` for 03-03, and a 13-case ASCII-escaped red-team fixture registered with the Phase 1 canary gate and excluded from AI context.

## Tasks

| Task | Commits | Result |
|------|---------|--------|
| 1 (tracer) tag-block payload stripped by one sanitizer reached from the shim via vox-db | RED `8a3e5edcd`, GREEN `6816dbfa8` | 4 kb_text tests + shim test; 5 pre-existing helpers tests unchanged and green |
| 2a full sanitizer, newtypes, proptests, fixtures | RED `17dd7f62e`, GREEN `3a816e4c1` | 19 kb_text tests (incl. 3 proptests) + 2 doctests |
| 2b canary registration, `.voxignore` | RED `ce66f31d6`, GREEN `b4d2615bb` | 2 new gate tests, 16/16 `kb::gate` tests green |

Tracer feedback gate: auto/`end-of-phase` mode, automated-only `<verify>`; the three tracer verify commands (kb_text nextest, shim helpers nextest, `cargo check -p vox-search -p vox-research-shim -p vox-orchestrator-mcp`) were re-run end to end and passed before expanding.

## TDD

- RED 1: 4 kb_text tests failed on behavior assertions against a stub that returned the input unchanged.
- RED 2: 9 of 19 kb_text tests failed against stub `ContentHash`/enums and the Task 1 sanitizer (comments, images, newtypes, redteam).
- RED 3: `canary_redteam_dir_is_outside_index_roots` failed at the `.expect("a covering root must fail")` against a stub returning `None`.
- GREEN for all three; no REFACTOR commit.

## Mutation proof (T-03-01, T-03-03)

Edits were made on the working tree with a scratchpad copy restored after each run; `grep -c` presence counts bracket each edit.

1. Tag-block arm (T-03-01). Marker `'\u{E0000}'..='\u{E007F}'` in `is_banned`: 1 -> 0 after mutation. `cargo nextest run -p vox-db-types kb_text`: 14 passed, 5 failed (`props::output_holds_no_banned_codepoint_or_comment_or_absolute_image`, `redteam_cases_are_neutralized`, `strips_tag_block_zero_width_invisible_bidi_and_variation_selectors`, `sanitized_text_runs_the_sanitizer`, `empty_and_all_banned_inputs`). Restored: marker 1; 19/19 green.
   - First attempt was a finding, not a pass: only 4 tests failed and the proptest stayed green because its oracle called the mutated `is_banned`. The oracle now restates the ranges independently (`independent_banned`), after which the proptest fails under this mutation as required.
2. Fixpoint loop (T-03-03). Marker `for _ in 0..MAX_PASSES` 1 and `return one_pass(raw)` 0 before; 1 and 1 after (single pass inserted). 16 passed, 3 failed: `fixpoint_neutralizes_constructions_that_touch_after_removal`, `props::sanitize_is_idempotent` (minimal input `"<!<|im_start|>"`), `redteam_cases_are_neutralized` (comment-constructed-image case). Restored: loop 1, mutated 0; 19/19 green.

## Property test findings

Raising `PROPTEST_CASES` to 100000 surfaced real bypasses and oracle bugs that were fixed in the sanitizer or the oracle (seeds kept in `proptest-regressions/kb_text.txt`):

- `![](https://evil.example/x.png` (unterminated destination) passed through; fixed by judging the leading token (see key decisions).
- `<img src="<img src="https://evil...">` was kept because the mis-tokenized src (`<img src=`) looked relative; fixed by rejecting quote, angle and backtick characters in a "relative" target.
- Two oracle over-matches (alt text containing `src="https`, a relative path containing `https://` mid-string) were tightened; neither was a sanitizer bug.

## Acceptance criteria

- `pub use vox_db_types::kb_text` in `crates/vox-db/src/lib.rs:185`: PASS. `kb_text::sanitize` in the shim helper: PASS. `retain` count inside `sanitize_evidence`: 0 PASS. `regex` in vox-db-types Cargo.toml: absent PASS.
- `.voxignore` lines `contracts/kb/redteam/` (140) and `.vox/cache/kb-findings/` (142): PASS. `cases.v1.jsonl`: 13 lines, 0 non-ASCII bytes: PASS. `proptest` under `[dev-dependencies]` (line 14): PASS. `REDTEAM_DIR` const plus use: PASS.
- `cargo test -p vox-db-types --doc`: both SanitizedText doctests pass (compile_fail confirmed failing to compile).
- `vox ci sync-ignore-files --verify`: PASS (run via `cargo run -p vox-cli -- ci sync-ignore-files`, since the installed `vox` is stale).
- `vox ci crate-edges`: PASS ("689 live in-tree edges within baseline"), run via `cargo run -p vox-cli -- ci crate-edges` because the only dependency change is the external dev-dependency `proptest` (no workspace edge). It prints a pre-existing warning `stale baseline edge vox-gui -> vox-db-types`, not from this plan; tightening it is left to a maintainer.
- `cargo clippy -p vox-db-types -p vox-cli -p vox-research-shim -p vox-db --lib --tests -- -D warnings`: clean. `cargo check -p vox-search -p vox-research-shim -p vox-orchestrator-mcp`: clean.

## Deviations from Plan

**1. [Rule 3 - Blocking] `.gitignore` swallowed the red-team fixture**
- **Found during:** Task 2 RED commit (`git add` refused `contracts/kb/redteam/cases.v1.jsonl`; `.gitignore:189` ignores `*.jsonl`).
- **Fix:** added `!contracts/kb/redteam/*.jsonl` next to the existing `!contracts/kb/eval/*.jsonl` exception. `.gitignore` was not in the plan's file list.
- **Commit:** `17dd7f62e`

**2. [Rule 2 - Missing critical] hardening beyond the plan's behavior list**
- Reference-style and shortcut images always neutralized; unterminated image destinations judged on the leading token; `\`, `&`, quote, angle and backtick characters disqualify a "relative" target; HTML img attributes tokenized with quote awareness and `srcset` forces neutralization; fail-closed empty result when the 8-pass loop does not converge. All are covered by tests and found by property testing or direct bypass analysis. Files: `crates/vox-db-types/src/kb_text.rs`.
- Plan said the fixpoint cap is 8 passes (PATTERNS said 4); 8 was used as in the plan task text.

**3. Plan-level note:** the plan's `<verify>` calls `vox ci sync-ignore-files --verify` and `vox ci crate-edges` with the installed `vox`, which is stale; both were run from source instead of skipping them.

## Deferred / unverified

- Full `cargo nextest run -p vox-research-shim` (195 tests): 194 passed, 1 timed out, 4 flaky-then-passed. The slow and timed-out tests are the `scientia_phase_0a_pipeline_smoke` and `provider_retrieval_log_test` integration tests (>120 s each, provider/network bound; the run takes ~540 s). All 6 `helpers` tests, including the 5 pre-existing sanitizer tests, pass. The baseline timing at `f9166a17b` was not measured, so "pre-existing" is an inference (the sanitizer is linear-time and bounded), not a measurement. Logged here rather than fixed: out of scope.

## Known limits (documented, not defects)

- Stripping the Unicode tag block breaks subdivision-flag emoji (accepted, D-06).
- A shortcut image form (`Wow![great]`) in benign prose becomes `Wow(image omitted: great)`; accepted cost of failing closed.
- Only images are neutralized, not plain links. Other fetching embeds (`<svg>`, `<source srcset>`, `<video poster>`, `<iframe>`, CSS `url()`) are not covered here; 03-04/03-09 should keep rendering paths free of raw HTML or add elements here if any render HTML.
- `vox_config::sanitize_chatml` and the `vox-corpus` arca replay copy remain separate prompt helpers (D16); 03-13 documents them. Nothing here claims "the only sanitizer".

## Known Stubs

None.

## Threat Flags

None. T-03-01, T-03-02, T-03-03, T-03-19 and T-03-40 are mitigated as planned; T-03-SC accepted (proptest 1.11.0 already locked, `Cargo.lock` gained one line for the dependency edge).

## Self-Check: PASSED

- Files exist: `crates/vox-db-types/src/kb_text.rs`, `contracts/kb/redteam/cases.v1.jsonl`, `crates/vox-db-types/proptest-regressions/kb_text.txt`.
- Commits exist: `8a3e5edcd`, `6816dbfa8`, `17dd7f62e`, `3a816e4c1`, `ce66f31d6`, `b4d2615bb`.
