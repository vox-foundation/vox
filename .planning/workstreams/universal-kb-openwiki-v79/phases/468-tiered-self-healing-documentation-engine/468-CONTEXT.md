# Phase 468 Context: Tiered Self-Updating & Self-Fixing Documentation Engine

**Workstream:** `universal-kb-openwiki-v79`
**Phase:** `468`
**Requirements:** `WIKI-03`, `WIKI-04`
**Depends on:** Phases 466 (`GRAPH-01..04`) and 467 (`WIKI-01..02`)

## Architectural Decisions (Locked)
1. **Two-Tier Self-Healing Closed Loop (`WIKI-03`, `WIKI-04`):**
   - **Tier 1 (`WIKI-03` — Zero-LLM Deterministic Blast-Radius Auto-Heal):** Given modified source files/symbols (or `git diff`), resolve reverse `documented_in` / `cites_symbol` / `governed_by_adr` impacts and `#Lstart-Lend` references across `docs/wiki/` + `docs/`, regenerate `<!-- openwiki:auto:start id="..." -->` fences in place, and shift broken `#Lstart-Lend` anchors when the referenced symbol still exists in its file — with `0` LLM calls and `0` modifications to human prose outside fences.
   - **Tier 2 (`WIKI-04` — Four-Gate Verified Prose Heal Queue):** Detect semantic prose/summary drift via `verify_doc_mechanically` (`vox-search::kb_staleness`) and claim auditing, synthesize grounded minimal patches via `vox_actor_runtime::llm::llm_chat` (with offline deterministic grounded fallback when no provider endpoint is configured), and gate every candidate through all 4 deterministic checks before queuing in `VoxDb::kb_heal_proposals`:
     1. `kb_rewrite_guard` (or fence-safe / retired-symbol / negation / numeric invariant check) + `[path:Lstart-Lend]` citation check
     2. `vox-doc-pipeline` frontmatter + facet lint (`0` hard errors)
     3. `vox-diagram` structural validation (`0` errors, `0` legacy Mermaid)
     4. `.vox` doctest compilation check (`0` errors)
2. **Schema Digest Preservation (`crates/vox-db`):**
   - `kb_heal_proposals` is provisioned idempotently via `schema_extensions::apply_kb_heal_proposals_table` (and table ensure helper in `ops_memory::kb_heal_proposals`) so `SCHEMA_FRAGMENTS` / `baseline_sql()` and `contracts/db/baseline-version-policy.yaml` remain bit-identical (`BASELINE_VERSION = 99`).
3. **Human & Operator Controls:**
   - Operators inspect, apply, `--auto-apply`, demote-to-legacy, or reject staged proposals via `vox ci wiki-sync`, `vox repo --wiki {sync,heal,heal-list,heal-apply:<id>,heal-reject:<id>,heal-demote:<id>}`, `vox_repo_catalog_refresh`, or optional draft PR specs via `BotMetricsLedger::plan_or_update_draft_pr`.
