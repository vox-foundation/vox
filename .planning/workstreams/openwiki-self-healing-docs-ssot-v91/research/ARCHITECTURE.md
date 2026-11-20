# Architecture: Self-Healing OpenWiki & Multi-Repo KB Loop (`v9.1`)

**Workstream:** `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/`  
**Researched:** 2026-11-19

## 1. Crate Layer Boundaries (`docs/src/architecture/layers.toml`)

- **L1 (`vox-graph-reader`):** Pure filesystem + `tree-sitter` + `syn` + `ignore` + `petgraph` + `leiden-rs` + `blake3`. Never depends on `vox-db`, `vox-search`, or `vox-actor-runtime`.
- **L2 (`vox-diagram`):** Pure diagram DSL lexer/parser/checker/renderer. Consumed by `vox-doc-pipeline`.
- **L3 (`vox-doc-pipeline`):** Depends on `vox-graph-reader` (new L3 -> L1 allowed edge), `vox-diagram` (L2), `vox-kb-facets` (L0), `vox-bounded-fs` (L1), `vox-compiler` (L3). Houses deterministic claim extraction, ground-truth resolution, drift gates, 5-layer polyglot OpenWiki generation, Tier-1 anchor repair, and 4-gate Tier-2 validation. Accepts an injected verifier/synthesizer callback or trait for LLM calls so `vox-doc-pipeline` stays lightweight and testable offline.
- **L3 (`vox-search`):** Owns `library/*` (DB + Tantivy `(repository_id, corpus_tier)` sharding), `kb_staleness`, `kb_rewrite_guard`, and `promotion` (which calls `vox_actor_runtime::llm` + `vox_doc_pipeline` 4-gate checks and persists `kb_heal_proposals` into `VoxDb`).
- **L3 (`vox-orchestrator-mcp`):** Thin MCP surface delegating `wiki_*` actions to `vox-doc-pipeline` (`openwiki_polyglot` / `openwiki_heal`) and `wiki_heal_*` / `vox_kb_*` to `vox-search` + `state.db`. No inline stub Markdown templates.
- **L5 (`vox-orchestrator-d`, `vox-cli`, `vox-cli-research`, `vox-gui`):**
  - `vox-orchestrator-d` runs a background non-blocking KB sync manager (on startup, repo switch, doc write, and periodic/HEAD change check).
  - `vox-cli` (`commands/ci/run_body.rs`) connects via daemon IPC DB proxy / canonical `VoxDb` (`vox_db::DbConfig::resolve_canonical` or daemon RPC) — never writing proposals to `.vox/store.db`.
  - `vox-gui` communicates exclusively via `voxTransport` -> MCP / daemon RPCs (`0` direct `VoxDb` handles).

## 2. Partitioned Multi-Repo KB & Tantivy Topology

To support Vox (`docs/src/` Astro database for `voxlang.org` + root guides) and arbitrary external polyglot repos (`docs/`, `adr/`, `README.md`, `VOX.md`, `AGENTS.md`, `Vox.toml`/`.vox/kb.yaml` overrides) simultaneously with zero cross-repo leakage and no separate `docs/wiki/` folder on Vox:

1. **Corpus Key & Tantivy Directory Partitioning:**
   - Replace single global `lexical::default_index_dir()` with `lexical::resolve_scoped_index_dir(...)` keyed by `(repository_id, corpus_tier)` for external repos while keeping Vox's `KB_LIBRARY_CORPUS` / `contracts/kb/eval/` BEIR compatibility intact.
   - Default Vox BEIR golden set (`contracts/kb/eval/`) continues targeting `(repository_id = "vox", corpus_tier = DocsSrc)` cleanly.
   - Read-only external repos (`--read-only`) store Tantivy + graph caches under `~/.vox/repos/<repository_id>/cache/{kb,graphify,openwiki}/` and never write inside the target checkout.
2. **Incremental Daemon Refresh:**
   - Maintain per-file `blake3` hashes in `search_documents` (`file_hash`) and `kb_index_state` (`indexed_commit`, `chunker_version`).
   - Incremental runs skip unchanged files (`O(changed)` Tantivy delta updates or rebuild only when changed/pruned > 0), keeping daemon boot overhead sub-second when clean.

## 3. End-to-End Self-Healing Data Flow

1. **Trigger:** `git diff` (pre-push / PR / daemon watch / CI nightly) or manual `vox ci wiki-sync --heal` / Axis `Scan & Propose Heals`.
2. **AST & Blast-Radius Analysis:** `vox-graph-reader` incrementally parses changed `.rs`, `.ts/.tsx/.js/.jsx`, `.py`, `.go`, `.vox`, `.sql` files, updates `Code <-> Doc <-> Decision` cross-edges, and maps modified symbols/files to impacted doc paths (plus claims referencing missing/retired items).
3. **Tier-1 Deterministic Auto-Heal (`0` LLM calls):**
   - Regenerates `<!-- vox:auto:start id="..." -->` / `<!-- openwiki:auto:start id="..." -->` fences and `*.generated.md` hubs directly inside `docs/src/` on Vox (`voxlang.org`) and `<canonical_docs_root>/` on external repos.
   - Repairs shifted `#Lstart-Lend` line anchors and renamed symbol signatures in-place across `docs/src/` (and `<canonical_docs_root>/`).
4. **Tier-2 NLI Claim Verification & Section Patching (`vox_actor_runtime::llm`):**
   - Extracts prose claims paired with resolved AST spans / `git diff` snippets / CLI & config contracts.
   - Evaluates entailment (`Verified | Drifted | Unanchored`) with `(claim_hash, symbol_span_hash)` caching and budget caps.
   - For `Drifted` claims, synthesizes a minimal section- or paragraph-scoped unified diff citing verified `[path:Lstart-Lend]` evidence.
   - Validates every diff through all 4 deterministic gates (`verify_tier2_candidate_four_gates` + `kb_rewrite_guard` + NLI re-check).
5. **Staging & Application:**
   - Persists validated diffs into daemon `VoxDb::kb_heal_proposals`.
   - Operators review/apply/edit/demote/reject via Axis `OpenWikiHealStudio`, CLI `vox ci wiki-sync --apply-proposal` / `--auto-apply`, or weekly `docs-drift-refresh.yml` Draft PRs (`<= 3` open branches, auto-pause below `80%` acceptance precision).
   - Applied/edited docs trigger immediate incremental KB re-index.

## 4. Seven-Phase Build Order (`Phases 534–540`)

1. **Phase 534 (`TRUTH-01..04`):** Remove resolver cheats and expand drift gate first so every downstream phase measures real, unmasked truth.
2. **Phase 535 (`KB-AUTO-01..04`):** Fix multi-repo KB indexing, daemon auto-sync, and unified `VoxDb` so the retrieval/proposal substrate is live.
3. **Phase 536 (`WIKI-UNIFY-01..04`):** Unify CLI/MCP/GUI living docs sync directly over `docs/src/` (`voxlang.org`) / `<docs_root>/` without truncation caps and wire Tier-1 AST anchor auto-heal.
4. **Phase 537 (`LLM-HEAL-01..04`):** Replace mock claim verifier + golden eval with real AST-grounded `vox_actor_runtime::llm` NLI verification + 4-gate section-level Tier-2 prose healing.
5. **Phase 538 (`CI-LOOP-01..04`):** Repair CI workflows (`docs-drift-refresh.yml`, `docs-self-heal.yml`), pre-push blast-radius checks, and staleness `review_by` / `legacy` demotion.
6. **Phase 539 (`CORPUS-01..04`):** Sweep Vox's own untruths, enrich living AST-coupled sections in `docs/src/`, triage superseded ADRs/archive candidates, and drive `status: "current"` deficits to `0`.
7. **Phase 540 (`VERIFY-01..04`):** Run polyglot external-repo E2E proofs (read-write & `--read-only`), Axis GUI studio checks, Starlight/Playwright gates, and `/gsd-code-review --fix`.
