---
phase: "2"
slug: "library-index-and-kb-tool-surface"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-03"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: 02-RESEARCH.md "## Validation Architecture", mapped onto plans 02-01 … 02-09 (revision 1: chunk identity shared with Phase 1 `vox_search::kb_ids`; Phase 1 probe/eval reused, not duplicated).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` / `#[tokio::test]` under cargo-nextest 0.9.143 (profiles `default` / `ci` / `ci-gate`) |
| **Config file** | `.config/nextest.toml` |
| **Quick run command** | `cargo nextest run -p <crate touched> -E 'test(/kb_|library|chunk_|corpus::|lexical::/)'` (add `--features tantivy-lexical` for vox-search) |
| **Full suite command** | `cargo nextest run -p vox-db -p vox-search -p vox-orchestrator-mcp -p vox-doc-pipeline -p vox-cli` + `vox ci operations-verify` + `cargo run -p vox-cli -- ci command-compliance` + `vox ci crate-edges` + `cargo hakari generate --diff` + `cargo run -p vox-cli -- kb eval --gate` |
| **Estimated runtime** | ~60–120 s per crate quick run (mcp crate compile dominates); full suite ~8–15 min locally |

Local verification is per crate only (AGENTS: hosted CI is the gate; never workspace-wide clippy/tests locally). Cargo runs through the build broker as plain `cargo`. `vox-doc-pipeline --lint-only --paths` takes comma-separated paths relative to `docs/src/`.

**Cross-phase prerequisites (fail closed):** Phase 1 01-01 (`kb_ids`, `kb_eval`), 01-03 (`crates/vox-db/tests/turso_fts_probe.rs` + spike section), 01-05/01-07 (baseline + gate), 01-06 (`commands/kb.rs`, `kb.eval` row), 01-10/01-11 (golden set + epoch-1 baseline). Plans carry `cross_phase_requires` frontmatter and `<precondition>`s that halt when an artifact is missing.

---

## Sampling Rate

- **After every task commit:** the quick command for the crate(s) the task touched
- **After every plan wave:** full suite command above (the `kb eval --gate` step from wave 5 on)
- **Before `/gsd-verify-work`:** full suite green + `vox kb eval --gate` (02-09) + Claude Code UAT human-check
- **Max feedback latency:** 120 seconds (quick run)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | IDX-01, IDX-07 | T-02-01 | Frozen-core DDL and feature default land only after human sign-off | checkpoint:decision | n/a (blocking-human, two answers) | n/a | ⬜ pending |
| 02-01-02 | 01 | 1 | IDX-01 | T-02-01 | v94 DB migrates additively; rowids stable across diffs | integration | `cargo nextest run -p vox-db -E 'test(/kb_library_migration_from_94\|library_store_\|trust_tier_wire\|baseline_policy/)'` + `cargo nextest run -p vox-db --test turso_fts_probe` | ❌ W0 (tests in library.rs / vox-db tests); probe ✅ from Phase 1 | ⬜ pending |
| 02-01-03 | 01 | 1 | IDX-01, IDX-02, IDX-07 | T-02-02, T-02-03, T-02-04, T-02-05 | Lenient query parse; hash only via injected vox-crypto; identity only from kb_ids | tracer integration | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/kb_tracer\|chunk_h1_not_duplicated\|kb_sections_project_heading_paths\|kb_ids\|kb_eval/)'` | ❌ W0 `crates/vox-search/tests/kb_library_index.rs` | ⬜ pending |
| 02-02-01 | 02 | 1 | IDX-06 | T-02-07 | N/A | lint integration | `cargo run -p vox-doc-pipeline -- --lint-only --paths library/index.md` | ❌ W0 `docs/src/library/index.md` | ⬜ pending |
| 02-02-02 | 02 | 1 | IDX-06 | T-02-08 | Category drift caught (mutation-checked) | unit | `cargo nextest run -p vox-doc-pipeline -E 'test(valid_categories_match_sidebar_sections)'` | ❌ W0 (test added in lint.rs) | ⬜ pending |
| 02-03-01 | 03 | 2 | IDX-02, IDX-03 | T-02-31 | Chunk heading_paths == kb_ids::doc_heading_paths on every real doc; frontmatter never in bodies | integration + unit | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/chunk_\|kb_chunking_end_to_end/)'` | ❌ W0 `crates/vox-search/tests/kb_library_chunking.rs` | ⬜ pending |
| 02-03-02 | 03 | 2 | IDX-02 | T-02-SC | Supply chain: audited crate, hakari clean | unit | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/chunk_oversize\|chunk_small_section\|chunk_split_is_deterministic\|chunk_paths_match_kb_ids/)'` + `cargo hakari generate --diff` | ❌ W0 | ⬜ pending |
| 02-03-03 | 03 | 2 | IDX-04 | T-02-03 | Archive (any depth) / symlink never indexed via the shared kb_ids filter (mutation-proven) | integration | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/kb_archive_excluded\|walker_matches_kb_filter/)'` | ❌ W0 | ⬜ pending |
| 02-04-01 | 04 | 2 | IDX-01 | — | N/A | tracer integration | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/kb_reindex_unchanged\|kb_reindex_one_section\|kb_reindex_insert_middle_section\|kb_kept_hash_keeps_vector/)'` | ❌ W0 (extends kb_library_index.rs) | ⬜ pending |
| 02-04-02 | 04 | 2 | IDX-01, IDX-03, IDX-05, IDX-07 | T-02-05, T-02-11 | Divergence rebuild; errored files never pruned | integration | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/kb_reindex_prune\|kb_no_orphan_embeddings\|kb_manifest_and_cards\|kb_status_reports_errors\|kb_lexical_backend\|kb_lexical_divergence_rebuilds\|resolve_index_dir/)'` | ❌ W0 | ⬜ pending |
| 02-04-03 | 04 | 2 | IDX-01, IDX-07 | T-02-10 | Readers return library-corpus rows only; engine probe still green | integration | `cargo nextest run -p vox-db -E 'test(/kb_store_\|library_document_by_path\|library_doc_chunks\|library_chunk_by_hash/)'` + `cargo nextest run -p vox-db --test turso_fts_probe` | ❌ W0 `crates/vox-db/tests/kb_library_store.rs`; probe ✅ from Phase 1 | ⬜ pending |
| 02-05-01 | 05 | 2 | IDX-07 | T-02-13 | N/A | tracer (smoke) | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/execute_search_plan\|a2a\|rrf\|semcov/)'` | ✅ (existing smoke) + ❌ W0 `crates/vox-search/tests/tantivy_mirror_removed.rs` | ⬜ pending |
| 02-05-02 | 05 | 2 | IDX-07 | T-02-14 | N/A | static guard | `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(tantivy_mirror_removed)'` | ❌ W0 | ⬜ pending |
| 02-06-01 | 06 | 3 | RET-01 | T-02-18 | Query cap 1 KiB, limit clamp | tracer integration | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_search\|kb_tool_surface\|every_registry_tool_has_static_dispatch/)'` + `vox ci operations-verify` | ❌ W0 `crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs` | ⬜ pending |
| 02-06-02 | 06 | 3 | RET-01, IDX-05 | T-02-15, T-02-20 | Path guard = kb_ids::is_kb_corpus_path + hardening, before lookup; no FS reads (mutation-proven) | unit + integration | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/validate_doc_path\|validate_chunk_hash\|kb_get\|kb_cite\|kb_status/)'` | ❌ W0 | ⬜ pending |
| 02-06-03 | 06 | 3 | RET-01, IDX-05 | T-02-16, T-02-17, T-02-19, T-02-21 | No root args; single-flight (mutation-proven); not read-role eligible | integration | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_reindex\|kb_tool_surface/)'` | ❌ W0 | ⬜ pending |
| 02-07-01 | 07 | 4 | RET-01 | T-02-22 | Resource reads go through daemon vox_kb_get only | tracer (real in-process daemon) | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_resources_and_tools_via_daemon\|kb_resources::/)'` | ✅ `stdio_daemon_routing_tests.rs` (extended) | ⬜ pending |
| 02-07-02 | 07 | 4 | RET-01 | T-02-22, T-02-23 | kb:// URI guard (mutation-proven) | unit | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/parse_kb_uri\|kb_resource_templates\|list_kb_doc_resources/)'` | ❌ W0 (in kb_resources.rs) | ⬜ pending |
| 02-08-01 | 08 | 4 | IDX-05 | T-02-25 | CLI never opens a second writer | unit + clap | `cargo nextest run -p vox-cli -E 'test(/unwrap_tool_envelope\|render_kb_status\|kb_cli/)'` + `vox ci operations-verify` + `cargo run -p vox-cli -- ci command-compliance` | ❌ W0 (kb.rs ✅ from Phase 1) | ⬜ pending |
| 02-08-02 | 08 | 4 | IDX-05 | T-02-25 | N/A | unit + clap | `cargo nextest run -p vox-cli -E 'test(/render_index_report\|kb_cli/)'` + `cargo run -p vox-cli -- ci command-compliance` | ❌ W0 | ⬜ pending |
| 02-08-03 | 08 | 4 | IDX-07 | T-02-27 | Restart note documented | doc lint | `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md,architecture/search-retrieval-ssot-2026.md` | ✅ | ⬜ pending |
| 02-09-01 | 09 | 5 | IDX-07, RET-01 | T-02-28, T-02-29 | Doc-level baseline never loosened; canary assert before the Library index build | tracer (eval) | `cargo nextest run -p vox-search --features tantivy-lexical --test kb_eval_library` + `cargo run -p vox-cli -- kb eval --gate` | ❌ W0 `crates/vox-search/tests/kb_eval_library.rs` | ⬜ pending |
| 02-09-02 | 09 | 5 | IDX-07 | T-02-28, T-02-32 | Qrel changes only via a new epoch + note; held-out metrics never consulted | eval + integration | `cargo nextest run -p vox-search --features tantivy-lexical --test kb_eval_library -E 'test(chunk_qrels_resolve_exactly_to_library_chunks)'` + `cargo run -p vox-cli -- kb eval --gate` | ❌ W0 | ⬜ pending |
| 02-09-03 | 09 | 5 | RET-01, IDX-05, IDX-06 | T-02-27, T-02-30 | Fresh daemon; temp page never committed | CLI + human-check | `cargo run -p vox-cli -- kb status --json` + Claude Code human-check | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Each listed test file is created by the first task that needs it (test-first, AGENTS Test-First Policy); there is no separate Wave 0 plan.

- [ ] Resolve the 02-01 Task 1 checkpoint (tantivy-lexical placement) first, so vox-search KB tests actually run in PR CI (research Pitfall 2; guard test `tantivy_lexical_enabled_by_default`)
- [ ] Phase 1 artifacts present (fail-closed preconditions): `crates/vox-search/src/kb_ids.rs`, `crates/vox-db/tests/turso_fts_probe.rs`, `crates/vox-cli/src/commands/kb.rs`, `contracts/kb/eval/*`, `contracts/kb/retrieval-baseline.v1.json`
- [ ] `crates/vox-search/tests/kb_library_index.rs` — tracer + IDX-01 tests (vox-crypto dev-dep test hasher) — 02-01, 02-04
- [ ] `crates/vox-search/tests/kb_library_chunking.rs` — IDX-02/03/04 fixtures in tempdirs + `chunk_paths_match_kb_ids` over the real tree — 02-03
- [ ] `crates/vox-db/tests/kb_library_store.rs` — store renumber + readers — 02-04 (v94 migration test lands in 02-01; engine probe stays in Phase 1's turso_fts_probe.rs)
- [ ] `crates/vox-search/tests/tantivy_mirror_removed.rs` — static guard — 02-05
- [ ] `crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs` — catalog/registry/dispatch/schema/governance parity — 02-06
- [ ] `crates/vox-search/tests/kb_eval_library.rs` — Library retriever determinism + exact qrel resolution — 02-09
- [ ] sidebar/VALID_CATEGORIES parity test in `crates/vox-doc-pipeline/src/pipeline/lint.rs` — 02-02

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Frozen-core schema 95 governance sign-off + tantivy-lexical default placement | IDX-01, IDX-07 (enabling) | Policy requires a named human approver; one-way feature-default door; user asked to decide open questions | 02-01 Task 1 checkpoint (blocking-human; one schema: and one feature: answer) |
| External MCP host (Claude Code) can call the five tools and read kb://doc + kb://chunk | RET-01 | Needs a real external host session | 02-09 Task 3 `<human-check>`: `claude mcp add vox-kb-uat -- <worktree>/target/debug/vox mcp`, call each tool, read one doc + one chunk, after restarting the daemon on fresh binaries |
| Deepened chunk qrels + new epoch note (only if any qrel changed) | IDX-07 (eval) | Golden-set edits are human-reviewed (CODEOWNERS /contracts/) | 02-09 Task 3 human-check item 8 |
| Library sidebar section renders | IDX-06 | docs-astro node_modules are not installed in this worktree | Hosted CI docs build (or `pnpm --dir docs-astro install && pnpm --dir docs-astro build` locally) |
| github-slugger anchors match live site heading ids | IDX-02 (citations) | Depends on the Astro build's slugger (research A6) | Open a `vox_kb_cite` link from the UAT on the built docs site |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
</content>
</invoke>
