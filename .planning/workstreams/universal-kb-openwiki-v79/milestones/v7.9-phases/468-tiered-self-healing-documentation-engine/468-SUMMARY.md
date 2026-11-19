# Phase 468 Summary — Tiered Self-Updating & Self-Fixing Documentation Engine

**Milestone:** `v7.9` (`universal-kb-openwiki-v79`)
**Phase:** `468` (`WIKI-03`, `WIKI-04`)
**Status:** Complete

## Delivered Changes
- **`crates/vox-db/src/{schema_extensions.rs,lib.rs,store/mod.rs,store/ops_memory/{mod.rs,kb_heal_proposals.rs}}`:**
  - Added idempotent `kb_heal_proposals` table (`apply_kb_heal_proposals_table`) plus `NewKbHealProposal`, `KbHealProposalRow`, `kb_heal_proposal_upsert`, `kb_heal_proposals_list`, `kb_heal_proposal_get`, and `kb_heal_proposal_update_status`.
- **`crates/vox-doc-pipeline/src/pipeline/{openwiki_heal.rs,openwiki_heal_tests.rs,openwiki.rs,mod.rs}`:**
  - Implemented `resolve_blast_radius`, `repair_shifted_line_anchors_in_markdown`, `heal_tier1_deterministic` (Tier-1 zero-LLM sync + anchor shift repair), `render_unified_diff`, `verify_tier2_candidate_four_gates` (RewriteGuard + DocLint + VoxDiagram + Doctest), `synthesize_grounded_prose_patch`, `apply_heal_patch_to_file`, and `demote_doc_to_legacy`.
- **`crates/vox-search/src/promotion.rs`:**
  - Implemented `stage_verified_prose_heal_proposal` and `apply_staged_heal_proposal` connecting `verify_doc_mechanically`, `verify_page_one_rewrite`, `vox_actor_runtime::llm::llm_chat` (with grounded deterministic fallback), and `kb_heal_proposals`.
- **`crates/vox-cli-ci/src/cmd_enums.rs`, `crates/vox-cli/src/{cli_dispatch/mod.rs,commands/ci/run_body.rs}`, `crates/vox-cli-research/src/repo.rs`, `crates/vox-orchestrator-mcp/src/{input_schemas/vcs_and_workspace.rs,repo_catalog_tools.rs}`:**
  - Exposed `--heal`, `--auto-apply`, `--apply-proposal`, `--reject-proposal` on `vox ci wiki-sync`; `sync`, `heal`, `heal-list`, `heal-apply:<id>`, `heal-reject:<id>`, `heal-demote:<id>` on `vox repo --wiki`; and `wiki_sync`, `wiki_heal_tier1`, `wiki_heal_list`, `wiki_heal_propose`, `wiki_heal_apply`, `wiki_heal_reject`, `wiki_heal_demote` on `vox_repo_catalog_refresh`.
