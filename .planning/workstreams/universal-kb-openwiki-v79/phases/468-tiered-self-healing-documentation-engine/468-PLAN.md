# Phase 468 Plan: Tiered Self-Updating & Self-Fixing Documentation Engine

**Phase:** `468`
**Requirements:** `WIKI-03`, `WIKI-04`

## Execution Sequence

1. **`VoxDb` `kb_heal_proposals` Persistence (`WIKI-04`):**
   - Add `apply_kb_heal_proposals_table` in `crates/vox-db/src/schema_extensions.rs`.
   - Create `crates/vox-db/src/store/ops_memory/kb_heal_proposals.rs` (`NewKbHealProposal`, `KbHealProposalRow`, `kb_heal_proposal_upsert`, `kb_heal_proposals_list`, `kb_heal_proposal_get`, `kb_heal_proposal_update_status` + unit tests).
   - Re-export in `ops_memory/mod.rs`, `store/mod.rs`, and `crates/vox-db/src/lib.rs`.

2. **Tier-1 Blast-Radius Auto-Heal & Tier-2 4-Gate Engine (`vox-doc-pipeline`):**
   - Create `crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs` + `openwiki_heal_tests.rs`, exported via `pipeline/mod.rs` and `pipeline/openwiki.rs`:
     - `resolve_blast_radius(repo_root, changed_paths_or_symbols)`
     - `repair_shifted_line_anchors_in_markdown(repo_root, doc_rel_path, markdown)`
     - `heal_tier1_deterministic(repo_root, changed_paths_or_symbols, write, output_override)`
     - `render_unified_diff(original_md, patched_md, doc_path)`
     - `verify_tier2_candidate_four_gates(repo_root, doc_rel_path, original_md, patched_md, evidence_citations)`
     - `synthesize_grounded_prose_patch(repo_root, doc_rel_path, symbol_name, new_fact, evidence_citation, draft_subsystem)`
     - `apply_heal_patch_to_file(repo_root, doc_rel_path, patched_md)` & `demote_doc_to_legacy(repo_root, doc_rel_path)`

3. **Verified Prose Heal Coordinator (`vox-search`) & CLI / MCP Wiring:**
   - Add `stage_verified_prose_heal_proposal` and `apply_staged_heal_proposal` in `crates/vox-search/src/promotion.rs` routing LLM generation strictly through `vox_actor_runtime::llm::llm_chat` with deterministic offline fallback and `kb_staleness` + `kb_rewrite_guard` verification.
   - Extend `CiCmd::WikiSync` (`crates/vox-cli-ci/src/cmd_enums.rs`, `crates/vox-cli/src/commands/ci/run_body.rs`) and `vox repo --wiki` (`crates/vox-cli-research/src/repo.rs`).
   - Extend `vox_repo_catalog_refresh` (`wiki_sync`, `wiki_heal_tier1`, `wiki_heal_list`, `wiki_heal_propose`, `wiki_heal_apply`, `wiki_heal_reject`, `wiki_heal_demote`) in `crates/vox-orchestrator-mcp/src/{input_schemas/vcs_and_workspace.rs,repo_catalog_tools.rs}`.

4. **Format, Test & Artifacts:**
   - `rustfmt --edition 2024` on all modified/created Rust files.
   - Verify `vox-db`, `vox-doc-pipeline`, `vox-search`, `vox-orchestrator-mcp`, `vox-cli` baseline, zero home-path literals, write `468-{VERIFICATION,REVIEW,SUMMARY}.md`, and update `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`, `state.json` to Phase 469.
