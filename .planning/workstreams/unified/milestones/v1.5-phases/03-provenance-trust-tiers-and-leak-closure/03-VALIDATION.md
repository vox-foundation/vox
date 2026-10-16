---
phase: "3"
slug: "provenance-trust-tiers-and-leak-closure"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-03"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest 0.9.143 (Rust `#[test]`/`#[tokio::test]`), proptest 1.11.0, wiremock, rustdoc `compile_fail` doctests; vitest + Playwright for vox-gui UI |
| **Config file** | `.config/nextest.toml` (PR gate profile `ci-gate`); `crates/vox-gui/ui/package.json` (`test`, `typecheck`, `test:e2e`) |
| **Quick run command** | `cargo nextest run -p <touched crate> <test-name filter>` (each task names its filter prefix) |
| **Full suite command** | `cargo nextest run -p vox-orchestrator-types -p vox-db-types -p vox-db -p vox-search -p vox-research-shim -p vox-orchestrator-mcp -p vox-ml-cli -p vox-orchestrator` + `cargo test -p vox-orchestrator-types --doc` + `cargo test -p vox-db-types --doc` + `cargo test -p vox-db --doc` + `pnpm --dir crates/vox-gui/ui test` + `pnpm --dir crates/vox-gui/ui typecheck` |
| **Estimated runtime** | ~600 seconds for the scoped full suite on a loaded laptop (vox-orchestrator-mcp dominates) |

Local verification is per crate only (AGENTS: hosted CI is the gate; never workspace-wide clippy or tests locally). `cargo` goes through the build-broker shim on PATH. Doc lint takes paths relative to `docs/src/`, comma-separated.

---

## Sampling Rate

- **After every task commit:** the task's `<automated>` commands (crate-scoped nextest with the task's name prefix)
- **After every plan wave:** the full suite command above, plus `vox ci crate-edges`, `vox ci operations-sync --target all` (check mode, from wave 5) and `cargo check -p vox-gui` when the GUI prerequisites exist
- **Before `/gsd-verify-work`:** full suite green; every mutation proof in the per-plan SUMMARYs logged red-then-green; `vox ci retired-symbol-check`, `vox ci retirement-audit`, `vox ci check-codex-ssot`, `vox ci sync-ignore-files --verify`, `vox ci gui-surface-coverage` green
- **Max feedback latency:** 120 seconds per task-level command

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | TRUST-01 | T-03-05 | One tier enum, no Default, unknown strings and node types map to Untrusted | unit + compile_fail | `cargo nextest run -p vox-orchestrator-types trust` ; `cargo test -p vox-orchestrator-types --doc` | ❌ W0 (`trust.rs` new) | ⬜ pending |
| 03-01-02 | 01 | 1 | TRUST-06 | T-03-04 | Unavailable/Retracted → Ineligible; NotApplicable/Clean → Eligible | unit (table + wiremock) | `cargo nextest run -p vox-search trust::` | ✅ (`trust.rs` extended) | ⬜ pending |
| 03-02-01 | 02 | 1 | TRUST-05 | T-03-01 | One sanitizer reachable as `vox_db::kb_text`; shim delegates | unit | `cargo nextest run -p vox-db-types kb_text` ; `cargo nextest run -p vox-research-shim helpers` | ❌ W0 (`kb_text.rs` new) | ⬜ pending |
| 03-02-02 | 02 | 1 | TRUST-05 | T-03-01/02/03/19 | Comments, images, fixpoint, newtypes; idempotent; red-team ASCII-only | property + fixture + compile_fail | `cargo nextest run -p vox-db-types kb_text` ; `cargo test -p vox-db-types --doc` ; `vox ci sync-ignore-files --verify` | ❌ W0 (`contracts/kb/redteam/cases.v1.jsonl`, proptest dev-dep) | ⬜ pending |
| 03-03-01 | 03 | 2 | TRUST-01, TRUST-04 | — | Governance sign-off recorded before DDL changes | checkpoint:decision | n/a (human decision) | n/a | ⬜ pending |
| 03-03-02 | 03 | 2 | TRUST-04, RET-03, TRUST-01 | T-03-05, T-03-06 | Findings round-trip with provenance and dedupe; FindingTier cannot be Trusted; legacy counts zero when absent; upgrade creates the table | integration + compile_fail | `cargo nextest run -p vox-db kb_findings` ; `cargo nextest run -p vox-db baseline_policy_matches_compiled_schema` ; `cargo nextest run -p vox-db --features legacy-import legacy_export_covers_all_baseline_tables` | ❌ W0 (`tests/kb_findings_store.rs`) | ⬜ pending |
| 03-03-03 | 03 | 2 | TRUST-01 | T-03-20 | Raw SQL reads Untrusted; Library rows backfilled from kb_index_state (single source); publication Advisory | unit (upgrade) | `cargo nextest run -p vox-db trust_tier` | ✅ (`local_tests.rs` extended) | ⬜ pending |
| 03-04-01 | 04 | 3 | TRUST-02, TRUST-04 | T-03-07 | Web hit → one Untrusted pending finding + snapshot; zero corpus rows; no sink → no write | integration (wiremock SearXNG) | `cargo nextest run -p vox-search capture_` ; `cargo nextest run -p vox-orchestrator-mcp capture_` | ❌ W0 (`web_capture_test.rs`, `kb_findings_capture.rs`) | ⬜ pending |
| 03-04-02 | 04 | 3 | TRUST-05, TRUST-04, TRUST-02 | T-03-08/09/10/23 | Red-team neutralized at ingest; secrets redacted; caps; dedupe; hash-only snapshot name on a gitignored path; CRAG rerouted | integration | `cargo nextest run -p vox-orchestrator-mcp capture_` ; `cargo nextest run -p vox-search -E 'test(/capture_\|crag\|ingest/)'` | ❌ W0 | ⬜ pending |
| 03-05-01 | 05 | 4 | TRUST-01 | T-03-37 | Library tier typed at one construction site; rows and kb_index_state always agree (incl. skipped rows) | integration (agreement) | `cargo nextest run -p vox-search kb_library_` ; `cargo nextest run -p vox-db library` | ✅ (Phase 2 `kb_library_index.rs` extended) | ⬜ pending |
| 03-05-02 | 05 | 4 | TRUST-01 | T-03-05, T-03-21 | Tier-less writer call does not compile; only tiered writers insert | compile_fail + static invariant | `cargo nextest run -p vox-db -E 'test(/tiered_writer\|search_document\|questioning\|research/)'` ; `cargo test -p vox-db --doc upsert_search_document` | ❌ W0 (`tests/tiered_writer_invariant.rs`) | ⬜ pending |
| 03-06-01 | 06 | 4 | TRUST-02, TRUST-04, GROW-06 | T-03-07 | Quick research turn adds zero corpus rows, files Untrusted pending findings with snapshots | integration (hermetic state + wiremock) | `cargo nextest run -p vox-orchestrator-mcp inbox_` ; `cargo check -p vox-gui` | ❌ W0 | ⬜ pending |
| 03-06-02 | 06 | 4 | TRUST-03, TRUST-02 | T-03-11, T-03-12 | Gather hits and deep synthesis go only to the inbox; docs/src research writer deleted | integration | `cargo nextest run -p vox-research-shim -E 'test(/finding_capture\|slug_from_query\|helpers/)'` ; `cargo nextest run -p vox-orchestrator-mcp inbox_` | ✅ (shim tests extended) | ⬜ pending |
| 03-07-01 | 07 | 4 | TRUST-03, RET-02 | T-03-11 | Seeded legacy KB + chat turn → no kb_entries row, no finding (red before deletion) | integration (wiremock OpenRouter) | `cargo nextest run -p vox-orchestrator-mcp kb_leak_` | ❌ W0 (`message_kb_leak_tests.rs`) | ⬜ pending |
| 03-07-02 | 07 | 4 | RET-02, GROW-06 | T-03-26, T-03-27 | Legacy KB text never reaches the prompt; @ parser kept; MEMORY sink deleted | integration | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_leak_\|parse_kb_mentions\|retrieval_bundle/)'` ; `cargo nextest run -p vox-orchestrator memory` | ❌ W0 | ⬜ pending |
| 03-08-01 | 08 | 4 | RET-03 | T-03-14, T-03-29 | Legacy rows counted first; non-empty → pending Untrusted; empty skipped; idempotent; zeros when absent; runs at DB attach | integration (seeded DB) | `cargo nextest run -p vox-orchestrator-mcp legacy_kb_` | ❌ W0 (`kb_legacy_migration.rs`) | ⬜ pending |
| 03-08-02 | 08 | 4 | GROW-06 | T-03-13 | KbSignals reads reviewed findings only; DPO only; no SFT | unit | `cargo nextest run -p vox-ml-cli kb_signals_` | ✅ (tests module extended) | ⬜ pending |
| 03-09-01 | 09 | 5 | TRUST-01, RET-02 | T-03-15 | web-ingest row never Trusted, absent from preamble, labelled in explicit queries; envelope tier = min; research_cache never a hit | integration | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_tiers_\|retrieval_bundle\|context_envelope/)'` | ❌ W0 (`kb_retrieval_tiers.rs`) | ⬜ pending |
| 03-09-02 | 09 | 5 | TRUST-05 | T-03-16, T-03-33 | Red-team neutralized at bundle, sources and KB-tool boundaries | fixture-driven integration | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_injection_\|sources\|context_block\|kb_tool/)'` | ❌ W0 | ⬜ pending |
| 03-10-01 | 10 | 5 | RET-02 | T-03-18 | One-way tool retirement confirmed by a human | checkpoint:decision | n/a (human decision) | n/a | ⬜ pending |
| 03-10-02 | 10 | 5 | RET-02 | T-03-17, T-03-35 | Legacy tools absent from catalog, registries, coverage report, dispatch and schemas; Phase 2 tools intact | static absence + dispatch parity + CLI checks | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/legacy_voxkb_\|every_registry_tool_has_static_dispatch\|kb_tool_surface/)'` ; `vox ci operations-sync --target all` ; `vox ci gui-surface-coverage` | ❌ W0 (`legacy_voxkb_retired.rs`) | ⬜ pending |
| 03-11-01 | 11 | 6 | GROW-06, TRUST-02 | T-03-31 | Ingest URL → pending finding, no knowledge node; toast says pending review | unit + vitest | `cargo nextest run -p vox-gui gui_inbox_` ; `pnpm --dir crates/vox-gui/ui test -- MemoryView` | ✅ (vitest exists, updated) | ⬜ pending |
| 03-11-02 | 11 | 6 | GROW-06 | T-03-12, T-03-31 | Ingest Text and Publish file findings; publish writes no docs/src file; caller-less docs writer deleted | unit + vitest + typecheck | `cargo nextest run -p vox-gui gui_inbox_` ; `pnpm --dir crates/vox-gui/ui test -- MemoryView DocPublishModal ResearchView` ; `pnpm --dir crates/vox-gui/ui typecheck` ; `vox ci gui-surface-coverage` | ✅ | ⬜ pending |
| 03-11-03 | 11 | 6 | GROW-06 | T-03-32 | Visual evidence of the new copy; "Published" absent | Playwright | `pnpm --dir crates/vox-gui/ui test:e2e -- kb-inbox-side-doors` | ❌ W0 (`e2e/kb-inbox-side-doors.spec.ts`) | ⬜ pending |
| 03-12-01 | 12 | 6 | RET-02 | T-03-35 | Store, router, types and CRUD absent; migration still works | static absence | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/legacy_voxkb_\|legacy_kb_/)'` | ✅ (extends 03-10 test) | ⬜ pending |
| 03-12-02 | 12 | 6 | RET-03 | T-03-36 | DDL quarantined (not dropped); digest re-updated, integer unchanged | unit + CLI check | `cargo nextest run -p vox-db -E 'test(/quarantine\|baseline_policy_matches_compiled_schema\|kb_findings_/)'` ; `cargo nextest run -p vox-db --features quarantine quarantined_table_present_when_feature_on` ; `vox ci check-codex-ssot` | ✅ | ⬜ pending |
| 03-13-01 | 13 | 6 | RET-03 | T-03-38 | Status keeps Phase 2 fields, adds legacy + inbox counts; CLI renders them | integration + unit | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_status_\|kb_tool_surface/)'` ; `cargo nextest run -p vox-cli kb_status` | ❌ W0 (`kb_status_inbox.rs`) | ⬜ pending |
| 03-13-02 | 13 | 6 | RET-02, TRUST-01 | T-03-35 | Retirement registered; trust model documented; CHANGELOG lists breaking changes | CLI checks + doc lint | `cargo nextest run -p vox-code-audit retirement_parity` ; `vox ci retired-symbol-check` ; `vox ci retirement-audit` ; `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/search-retrieval-ssot-2026.md,architecture/where-things-live.md` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-db-types/Cargo.toml`: `proptest = { workspace = true }` under `[dev-dependencies]` (03-02-02)
- [ ] `contracts/kb/redteam/cases.v1.jsonl`: ASCII-escaped payloads + canary, `.voxignore` entry, canary registered with the Phase 1 leak check (03-02-02)
- [ ] `vox_db::LEGACY_VOXKB_DDL` test-support constant so legacy-seeding tests survive the 03-12 quarantine (03-03-02)
- [ ] New test files, each created test-first inside its task: `crates/vox-db/tests/kb_findings_store.rs`, `crates/vox-db/tests/tiered_writer_invariant.rs`, `crates/vox-search/tests/web_capture_test.rs`, `crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs`, `crates/vox-orchestrator-mcp/src/chat_tools/chat/message_kb_leak_tests.rs`, `crates/vox-orchestrator-mcp/tests/kb_legacy_migration.rs`, `crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs`, `crates/vox-orchestrator-mcp/tests/legacy_voxkb_retired.rs`, `crates/vox-orchestrator-mcp/tests/kb_status_inbox.rs`, `crates/vox-gui/ui/e2e/kb-inbox-side-doors.spec.ts`
- [ ] wiremock SearXNG/OpenRouter fixtures reuse `vox-search/tests/web_leg_toggle_test.rs` and `message_turn_trace_tests.rs` helpers (no new harness)
- [ ] vox-gui prerequisites in the executing worktree (sidecar via `vox run scripts/gui-build.vox`, `pnpm --dir crates/vox-gui/ui install && pnpm --dir crates/vox-gui/ui build`, Playwright chromium) before 03-06-01 and 03-11

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Frozen-core schema sign-off | TRUST-01, TRUST-04 | Governance decision, not a code property | 03-03 Task 1: record option and signer in 03-03-SUMMARY.md and the PR body |
| One-way legacy tool retirement | RET-02 | Breaks a published MCP contract outside git | 03-10 Task 1: record option in 03-10-SUMMARY.md (03-12 and 03-13 read it) |
| Mutation proofs for every security guard | TRUST-01..06, RET-02, RET-03, GROW-06 | AGENTS requires breaking each guard by hand (cargo-mutants not installed) | Per task: `grep -c` presence before, apply the named mutation, run the task command and see red, restore, `grep -c` after, run green; log all in the plan SUMMARY |
| `compile_fail` doctests in the PR gate | TRUST-01, TRUST-05 | Doctests run in nightly CI, not the PR gate (`nightly.yml:1578`) | Run `cargo test -p vox-orchestrator-types --doc`, `cargo test -p vox-db-types --doc` and `cargo test -p vox-db --doc` locally before marking 03-01, 03-02, 03-03 and 03-05 done |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags (vitest runs as `vitest run`)
- [ ] Feedback latency < 120s per task command
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
