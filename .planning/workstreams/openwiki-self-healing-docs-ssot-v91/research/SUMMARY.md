# Project Research Summary

**Workstream:** `openwiki-self-healing-docs-ssot-v91` (Milestone `v9.1`)  
**Researched:** 2026-11-19  
**Research SSOT:** [`docs/src/architecture/self-healing-openwiki-and-docs-reality-findings-2026.md`](../../../../docs/src/architecture/self-healing-openwiki-and-docs-reality-findings-2026.md)

## Executive Summary

Vox already ships the foundational crates for a universal, self-healing documentation engine (`vox-graph-reader` with `tree-sitter` + `syn` + `leiden-rs`, `vox-diagram`, `vox-doc-pipeline`, `vox-search`, `vox-db`, and `vox-actor-runtime::llm`), yet the live documentation database is unindexed (`0` rows in `kb_index_state`), OpenWiki generation targets a nonexistent parallel `docs/wiki/` silo rather than Vox's canonical `docs/src/` (`docs-astro` → `voxlang.org`) Astro corpus, and active docs carry `1,547` detected deficits and stale narratives (`expl-architecture.md`, `mens-training.md`, `cli.md`, `tut-knowledgebase-and-openwiki.mdx`, and `110+` docs citing deleted crates). Benchmark patterns from Swimm (AST-coupled anchors), CodeWiki/DeepWiki (hierarchical module synthesis), and 2025–2026 NLI claim-verification research align directly with Vox's existing stack—requiring **zero new third-party dependencies**—once eight broken internal seams are closed.

## Key Findings

- **Stack (`STACK.md`):** Zero new `Cargo.lock` or `package.json` packages are needed. `vox-graph-reader` (`L1`) already holds `tree-sitter` (`rust`, `typescript`, `python`), `syn`, `ignore`, `petgraph`, `leiden-rs`, and `blake3`; adding `vox-graph-reader` as an `L3 -> L1` dependency of `vox-doc-pipeline` complies with `layers.toml`, while `vox_actor_runtime::llm` handles model-agnostic NLI verification and prose synthesis in `vox-search` / `vox-cli-research`.
- **False-Green Verification & Suppression Cheats (`FEATURES.md`, `PITFALLS.md`):** `self_heal_bot.rs` mocks LLM verification without calling an LLM or reading source files (self-citing the doc itself), `llm_verifier.rs` fakes 100% golden precision via string comparison, `resolver.rs` whitelists 58 deleted crates and passes unverified `crates/`/`docs/` paths and `vox` subcommands, and `gate.rs` skips `architecture/`, `adr/`, `contributors/`, `ci/`, and root guides.
- **Split-Brain Runtime & Truncated Living Docs (`ARCHITECTURE.md`):** `vox ci wiki-sync` opens `.vox/store.db` instead of daemon `vox.db` and skips polyglot sync whenever `layers.toml` exists; MCP `repo_catalog_tools.rs` writes 1-node placeholder stubs in `docs/wiki/` instead of invoking `vox-doc-pipeline` over `docs/src/` (`voxlang.org`) or `<canonical_docs_root>/`; `openwiki_polyglot.rs` silently truncates at `60` entries / `40` crates; `vox_kb_reindex` fails on external repos lacking `docs/src/`; and `.github/workflows/docs-drift-refresh.yml` runs a nonexistent `vox audit docs-reality` command.

## Implications for Roadmap

1. **Truth Before Automation (`Phase 534`):** Remove resolver cheats and expand `evaluate_drift_gate` behind a shrink-only baseline ratchet first so every subsequent improvement is measured against real codebase truth.
2. **Unified Persistence & Auto-Indexing (`Phase 535`):** Partition KB/Tantivy by `(repository_id, corpus_tier)`, fix daemon auto-indexing on startup/`HEAD`/doc-edits, and collapse `.vox/store.db` split-brain onto canonical `VoxDb`/daemon IPC.
3. **In-Place Living Sync Before LLM Healing (`Phases 536–537`):** Eliminate MCP stubs and 60-entry caps using `vox-graph-reader` AST/Leiden synthesis and Tier-1 `#Lstart-Lend` repair directly over `docs/src/` (`voxlang.org`) / `<docs_root>/` (`Phase 536`), then wire AST-span-grounded `vox_actor_runtime::llm` NLI verification + 4-gate section-scoped prose diffs (`Phase 537`).
4. **Autonomous Loops, Corpus Cleanup & Cross-Repo Proof (`Phases 538–540`):** Fix CI refresh/read-only verify workflows and staleness demotion (`Phase 538`), sweep Vox's own untruths, enrich living AST-coupled sections directly in `docs/src/`, and archive/demote superseded ADRs (`Phase 539`), and prove zero-touch polyglot multi-repo healing + Axis Studio + Starlight gates (`Phase 540`).

## Sources

- [Self-Healing OpenWiki & Documentation Reality Audit Findings (2026)](../../../../docs/src/architecture/self-healing-openwiki-and-docs-reality-findings-2026.md)
- [How Vox documentation works (`docs-system-ssot.md`)](../../../../docs/src/architecture/docs-system-ssot.md)
- [Universal Multi-Repo KB & Self-Healing OpenWiki Research (2026)](../../../../docs/src/architecture/universal-kb-graphify-openwiki-multirepo-research-2026.md)
- `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/research/{STACK,FEATURES,ARCHITECTURE,PITFALLS}.md`
