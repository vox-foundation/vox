---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 13
subsystem: mcp
tags: [ret-02, ret-03, trust-01, kb-status, retirement-registry, ssot-docs, changelog, d-02, d-05]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "legacy_kb_counts / kb_findings_counts (03-03), legacy migration (03-08), tier-aware retrieval (03-09), retired tools (03-10), GUI side doors (03-11), quarantined DDL (03-12)"
provides:
  - "vox_kb_status gains legacy_voxkb {knowledge_bases, kb_entries, kb_routing_rules} and inbox {total, pending} in both the not-indexed and indexed shapes; every Phase 2 key unchanged; still off the HTTP read role"
  - "vox kb status prints 'legacy VoxKB' and 'inbox: N pending of M' lines (the interim way to see GUI-filed findings until the Phase 12 Inbox GUI)"
  - "voxkb-kbstore-surface row in retired-surfaces.v1.yaml and retired-symbols.v1.yaml, plus the AGENTS.md Retired Surfaces row"
  - "search-retrieval-ssot section 'Trust tiers and the findings inbox'; where-things-live rows for six Phase 3 concepts; CHANGELOG Unreleased entries for the whole phase"
affects: [phase-04, phase-07, phase-11-doc-02, phase-12-inbox-gui]
requirements-completed: [RET-02, RET-03, TRUST-01]
tech-stack:
  added: []
  patterns: ["additive status payload pinned by a contract test that lists every prior key", "counts-only objects added to a path-bearing tool that stays off the HTTP read role", "documentation-only retirement row backed by a symbol pattern with identifier boundaries"]
key-files:
  created:
    - crates/vox-orchestrator-mcp/tests/kb_status_inbox.rs
  modified:
    - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
    - crates/vox-cli/src/commands/kb.rs
    - contracts/retirement/retired-surfaces.v1.yaml
    - contracts/documentation/retired-symbols.v1.yaml
    - AGENTS.md
    - CHANGELOG.md
    - docs/src/architecture/search-retrieval-ssot-2026.md
    - docs/src/architecture/where-things-live.md
key-decisions:
  - "Count-read failures return err_json with the existing REM_KB_REINDEX constant, never a silent zero (T-03-38)"
  - "The integration test attaches the database by assigning state.db directly, not through with_db_initialized, so the attach-time legacy migration task cannot race the counts"
  - "The AGENTS.md first cell equals agents_md_row exactly; the explanatory text sits in the replacement column"
duration: ~3 h wall (dominated by rustc stalls and rebuilds, see Issues)
completed: 2026-10-08
status: complete
commits: 2
plan_head_before: 3561d797925e6c790c82e98bccbcf55a1bba32a8
actuals:
  tokens: 8800
  tasks: 2
  commits: 2
---

# Phase 3 Plan 13: Status counts, retirement registries, SSOT docs and CHANGELOG Summary

Operators now see the legacy VoxKB row counts and the pending/total inbox counts through `vox_kb_status` and `vox kb status`, the VoxKB retirement is recorded in every registry the repo checks, and the trust model, the findings inbox, the sanitizer boundaries and the phase's breaking changes are documented where contributors and MCP host owners look.

## Commits

- `4a09b6e68` feat(03-13): legacy VoxKB and inbox counts in vox_kb_status and vox kb status (Task 1, tracer)
- `3ec392ef5` docs(03-13): record the VoxKB retirement, trust tiers and the findings inbox (Task 2)

## What was built

- **Task 1 (tracer).** `kb_status` reads `legacy_kb_counts()` and `kb_findings_counts()` once through a private `status_counts` helper and adds the two objects to both success shapes. The objects hold numbers only, so the payload stays safe to keep off the HTTP read role (`NOT_READ_ROLE` still lists `vox_kb_status`). `render_kb_status` appends a `legacy VoxKB:` line and an `inbox: N pending of M` line when the keys are present, after the not-indexed sentence and at the end of the indexed report, and renders byte-for-byte as before when they are absent (older daemon).
- **Task 2.** One registry row `voxkb-kbstore-surface` (documentation-only, backed by a symbol of the same id) names the store, the router and the 12 CRUD tools. The symbol patterns are identifier-bounded (`\bvox_kb_(?:create|list|...|clip)\b`, `\b(?:KbStore|KbRouter)\b`), so Phase 2's `vox_kb_search|get|cite|status|reindex` never match. The SSOT section covers the four-way tier mapping, the D-06 taint rule, the stored-tier source, the injection rules (preamble drops `Untrusted`, explicit tools label, envelope tier is the minimum), the capture chokepoint and snapshot location, the legacy migration and quarantine, DPO-only MENS signals, and the sanitizer with its two out-of-scope prompt helpers. The `VOX_SEARCH_PERSIST_WEB_HITS_DISABLED` bullet now carries the D-04 meaning. The CHANGELOG lists the tools and types retired, the inbox routing, the tier-aware preamble, the GUI side-door change, inert `@kb` mentions, DPO-only MENS signals, the unrecallable SFT export, baseline 96, and the separate-sanitizer note (D16).

## TDD evidence

- **Task 1 RED.** With the three integration tests written and the implementation reverted, `cargo test -p vox-orchestrator-mcp --test kb_status_inbox` ran 4 tests: 1 passed (hermetic-home guard), 3 FAILED, each on the behavior assertion `legacy_voxkb` (`left: Null`) after the Phase 2 assertions held (the payloads shown in the failure output carried every Phase 2 key and no added object). GREEN after restoring the implementation: 4/4.
- **Renderer tests** (`kb_status_renders_legacy_and_inbox_lines_when_not_indexed`, `..._when_indexed`, `kb_status_without_phase3_keys_renders_as_before`) were written together with the renderer; no separate RED run was made because a `vox-cli` test build costs 9 to 10 minutes on this host. They pass, and so do the three pre-existing `render_kb_status_*` tests unchanged.
- **Task 2 mutation proof (T-03-35).** Presence counts with `grep -c`: `vox_kb_query` in `where-things-live.md` was 0, then 1 after appending an unlabelled sentence naming it with a shell append (original saved aside first; the doc is not code, so no Edit-tool constraint applied), then 0 after copying the saved original back; `git diff --stat` afterwards shows only the six intended rows. The retired-symbol check run with the new rows returned `Found 1 retired symbol violations` naming `where-things-live.md:452` and the row `voxkb-kbstore-surface` in the mutated state, and `OK` before and after.

## Evidence (verification run locally, all with `CARGO_INCREMENTAL=0`)

- `cargo test -p vox-orchestrator-mcp --test kb_status_inbox`: 4/4 green.
- `cargo test -p vox-cli --lib -- kb_status`: 6/6 green (3 new, 3 pre-existing).
- `cargo test -p vox-code-audit retirement_parity`: 10/10 green, including `workspace_retirement_parity_is_clean`.
- `vox ci retired-symbol-check` and `vox ci retirement-audit`: both OK (28 surfaces declared). The full `vox` binary does not build on this host (rustc stalls in `vox_cli` codegen), so they ran through a temporary, uncommitted example in `vox-cli-ci` that calls `vox_cli_ci::retired_symbol_check::run` and `vox_cli_ci::retirement_audit::run` on the repo root, which is exactly what `run_body.rs` dispatches to. The example was deleted afterwards (`git status` clean for `vox-cli-ci`).
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/search-retrieval-ssot-2026.md,architecture/where-things-live.md`: "no hard errors".
- Acceptance greps all satisfied: `voxkb-kbstore-surface` in both contract files (and in `retired-surfaces.v1.yaml` as the contract symbol id); the AGENTS.md row text once; `vox_kb_query` and `SystemVerified` present; `kb_findings` in both docs; the old "mirrors SearXNG/DDG/Tavily-style web hits into" sentence gone; `legacy_kb_counts|kb_findings_counts` and `legacy_voxkb` present in the two code files; `vox_kb_status` still in `NOT_READ_ROLE`.

## Verify lines deferred to CI

Not run locally (lean-verification policy plus the build host): `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_status_|kb_tool_surface/)'` as written (the `kb_status_*` tests were run through `cargo test --test`; the `kb_tool_surface` binary and the unit test `kb_status_before_and_after_index` in `library.rs` did not compile on this host within the time box, because rustc stalled; the unit test's Phase 2 assertions are repeated in the new integration test, and `NOT_READ_ROLE` was confirmed by grep); `vox ci ssot-drift` (including the `docs/agents/doc-inventory.json` line counts, already stale at HEAD and regenerated by the `ssot-autoregen` job, never by hand); workspace clippy; the Playwright and `typecheck` UI gates (no UI file changed); the full `vox-cli` and `vox-orchestrator-mcp` suites.

## Deviations from Plan

**1. [Rule 3 - Blocking] The AGENTS.md first cell is plain text, not backticked.** The plan's acceptance grep (`KbStore, KbRouter, legacy vox_kb_\* CRUD tools`) and the registry's `agents_md_row` need the identical plain string, so the row uses it verbatim as the first cell and puts "(retired KB Phase 3, D-02; no aliases)" in the replacement column. **Commit:** 3ec392ef5.

**2. [Rule 2 - Missing verification] A stricter integration test than the plan sketched.** Besides the two shapes, the test asserts that every value in `legacy_voxkb` and `inbox` is a number and that no string anywhere inside them holds a path separator (T-03-46), and seeds the legacy entries as whitespace-only so the attach-time migration can never move rows into the inbox and skew the counts. **Commit:** 4a09b6e68.

**Total deviations:** 2. **Impact:** none on scope.

## Issues Encountered

Same host problem as 03-12: rustc intermittently stalled at near-zero CPU for 15 to 25 minutes on freshly started compile units (`vox_db`, `vox_cli`, two integration-test targets, a lib-test `vox_corpus`). `CARGO_INCREMENTAL=0` plus `LEAN_CTX_DISABLED=1` helped, and a small retry wrapper that kills only this worktree's own rustc processes after four minutes below twenty CPU-seconds made the remaining builds complete. No cargo or rustc process of mine is left running. The wrapper lives in the session scratchpad, not in the repo.

## Known open items (carried, not fixed here)

- Tavily `web_lines` appended in `crates/vox-search/src/bundle.rs` bypass the tier filter. No consumer reads them today; documented in the SSOT's injection rules for Phase 7.
- `legacy_export_covers_all_baseline_tables` (feature `legacy-import`) still fails on the five names recorded in `deferred-items.md`; the `sql-surface-guard` entry there was updated as FIXED in `604a35abd`.
- `vox_config::sanitize_chatml` and the `vox-corpus` arca replay copy remain separate prompt sanitizers (D16); the docs claim only one KB-text sanitizer.

## Known Stubs

None.

## Threat Flags

None. T-03-35 is mitigated and mutation-proven for the retired-symbol rows; T-03-38 (count errors surface) and T-03-46 (counts only, off the HTTP read role) are asserted by the integration test; T-03-18 is accepted and answered by the CHANGELOG entry naming the removed tools and their replacements.

## Self-Check: PASSED

- `kb_status_inbox.rs` exists; commits `4a09b6e68` and `3ec392ef5` found in `git log`; `git rev-list --count 3561d7979..HEAD` = 2 at the time of writing (code and docs commits only).
