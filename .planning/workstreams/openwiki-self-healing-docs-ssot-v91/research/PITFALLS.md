# Domain Pitfalls & Mitigations (`v9.1`)

**Workstream:** `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/`  
**Researched:** 2026-11-19

## 1. Anti-Ouroboros Self-Citation Loop
- **Pitfall:** `self_heal_bot.rs` currently cites the Markdown doc itself (`[{rel}:L1-L{line}]`) as verification evidence. If an LLM sees self-citations or other docs as primary ground truth, stale claims validate themselves forever.
- **Prevention:** Enforce in `enforce_citation_requirement` and `verify_citations_mechanically_for_doc` that NLI claim evidence citations MUST point to non-Markdown ground truth (source files `.rs`, `.ts`, `.tsx`, `.py`, `.go`, `.vox`, `.sql`, contracts `.yaml`/`.toml`/`.json`, or `.github/workflows/*.yml`), rejecting doc-to-same-doc self-citations.

## 2. Split-Brain SQLite Databases (`.vox/store.db` vs. Daemon `vox.db`)
- **Pitfall:** `vox-orchestrator-d` and `vox-orchestrator-mcp` store KB tables and `kb_heal_proposals` in the canonical app-data `vox.db` (`~/Library/Application Support/vox/vox.db` on macOS), while `vox ci wiki-sync` in `run_body.rs` opened `.vox/store.db` directly—meaning CLI `--apply-proposal` and GUI `OpenWikiHealStudio` looked at completely different databases, and direct opens risk `flock` contention.
- **Prevention:** Route all CLI `kb` / `wiki-sync` proposal reads/writes through `daemon_db_proxy` / canonical `VoxDb` resolution matching `vox-orchestrator-d`, with hermetic `VOX_DB_PATH` overrides in tests.

## 3. `kb_rewrite_guard` Rejecting Legitimate Number Updates
- **Pitfall:** `verify_page_one_rewrite` / `verify_tier2_candidate_four_gates` currently rejects any patch where a number in original `kb:summary` disappears (`!orig_nums.is_subset(&new_nums)`). While this prevents silent omission of metrics/timeouts, it would block healing if a doc falsely claimed `32` surfaces when code now has `33`, or `10s` timeout changed to `30s`.
- **Prevention:** Keep the strict subset guard when no code/contract citation justifies the replacement; allow a cited numeric substitution ONLY when the new value appears verbatim in the verified source/contract span cited in `evidence_citations`.

## 4. Removing Resolver Cheats Without Breaking `pre-push` Immediately
- **Pitfall:** Removing the 58 hardcoded aliases and `!target.contains("nonexistent")` bypasses from `resolver.rs` and expanding `gate.rs` to `architecture/` + `adr/` will immediately surface `1,547+` existing deficits. If done without regenerating `baseline.v1.json` or before fixing CI workflows, `vox ci pre-push` and `docs-quality.yml` will block unrelated developer commits.
- **Prevention:** In Phase 534, remove cheats and update `baseline.v1.json` atomically as a **shrink-only ratchet** (capturing existing pre-existing deficits cleanly while blocking any *new* deficit), then burn down and tighten `baseline.v1.json` to `0` across `status: "current"` docs in Phase 539.

## 5. Parity-Test Lock-In on Broken CI Commands
- **Pitfall:** `docs_system_parity.rs` and `ci_workflow_contract.rs` explicitly assert that `docs-drift-refresh.yml` contains `"docs-reality --update-ledger"`. Fixing the workflow without updating the parity probes/tests in the same plan breaks `vox ci ssot-drift` and `cargo test`.
- **Prevention:** Update `docs-drift-refresh.yml`, `docs-system-ssot.md`, `docs_system_parity.rs`, and `ci_workflow_contract.rs` together in Phase 538.

## 6. Cross-Repo Tantivy Index Corruption / Leakage
- **Pitfall:** Sharing one Tantivy directory across multiple `repository_id`s mixes external repo documents into Vox queries or drops documents during single-repo pruning (`docs_pruned`).
- **Prevention:** Key every Tantivy index directory and `kb_index_state` row by `(repository_id, corpus_tier)` and pass explicit `repository_id` filters in `vox_kb_search` / `vox_search_query` tests asserting `0` cross-repo hits in `repo_local` mode.

## 7. Legacy Mermaid Ratchet & `vox-diagram` Regressions
- **Pitfall:** Generating ` ```mermaid ` fences in polyglot OpenWiki pages or Tier-2 heal patches violates `mermaid_gate` and `verify_tier2_candidate_four_gates`.
- **Prevention:** All OpenWiki synthesis must emit ` ```vox-diagram ` blocks checked by `vox_diagram::check`, and Tier-2 gates reject any newly introduced `mermaid` fence.
