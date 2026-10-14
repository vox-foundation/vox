# Phase 3: Provenance, Trust Tiers and Leak Closure - Pattern Map

**Mapped:** 2026-10-06 (against HEAD `3fc399283`, after Phase 2 plans 02-01..02-12)
**Files analyzed:** 62 new/modified source and contract files across the 13 plans (03-01..03-13)
**Analogs found:** 54 exact or role-match / 62 (8 have no analog: see "No Analog Found")
**Verification method:** every path and symbol in RESEARCH.md (written 2026-10-03/04) was re-checked with `git grep` / direct reads against the current tree. All analog paths below are git-tracked source under `crates/`, `contracts/`, `docs/` (none are gitignored mirrors).

> Ordering note: 03-01..03-13 PLAN.md files already exist. This map is organized by file role and cites the plan that touches each file.

---

## 0. RESEARCH.md drift after Phase 2 (read this first)

### 0.1 Substantive changes the plans must absorb

| # | RESEARCH.md says | Current tree | Impact |
|---|------------------|--------------|--------|
| D1 | Phase 2 is "expected" to take baseline 95; Phase 3 takes 96 | CONFIRMED. `BASELINE_VERSION = 95` at `crates/vox-db/src/schema/manifest.rs:36`; `repository_baseline_integer: 95` in `contracts/db/baseline-version-policy.yaml`. Phase 3 = **96**. Phase 2 digest was `0xaaa2601b...a9130` | 03-03 bumps 95 to 96, 03-12 re-digests without bumping |
| D2 | "Phase 2 trust tier is an enum / `search_documents` writer gets a `tier` param" (A2) | Phase 2 stored tier as a **validated wire string**, not the enum: `TRUST_TIER_WIRE: [&str;4]` + `validate_trust_tier_wire()` at `crates/vox-db/src/store/ops_memory/library.rs:14-25` (comment: "Phase 3 swaps the type"). `LibraryDocUpsert.trust_tier: &'a str` (:37), `KbIndexState.trust_tier: String` (:98), `IndexDeps.trust_tier` validated at `crates/vox-search/src/library/mod.rs:68`, hard-coded `trust_tier: "trusted"` at `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:142`. **No per-row trust column exists** (02-01 decision `schema:approve-as-proposed`) | 03-05 must replace the string with `ContextTrustTier` in `LibraryDocUpsert`, `IndexDeps`, `KbIndexState`; keep `TRUST_TIER_WIRE` / `validate_trust_tier_wire` only as the parse path (re-exported at `crates/vox-db/src/lib.rs:289-291`, `store/mod.rs:93`, `ops_memory/mod.rs:13-14`) |
| D3 | A `tier` param on `upsert_search_document` covers every `search_documents` writer | **Not sufficient.** Library rows are written by a *separate* raw `INSERT INTO search_documents ...` inside `apply_library_document` (`library.rs:141-160`, corpus `'library'`, `source_uri = "library:{rel_path}"`), not through `upsert_search_document` (`ops_memory/search.rs:41-85`) | 03-03/03-05 must add the `trust_tier` column to that INSERT as well. The 03-05 invariant test (`only tiered writers insert into search_documents`) must whitelist exactly two statements: `search.rs` upsert and `library.rs` apply |
| D4 | VoxKB DDL at `schema/domains/knowledge.rs:78-116` | Now **`knowledge.rs:109-147`** (Phase 2 inserted `kb_index_state` at 78-97 and widened `search_documents` at 46-58, `search_document_chunks` at 63-76). Indexes `idx_kb_entries_*` / `idx_kb_routing_rules_kb_id` are at 143-147 | 03-12 moves lines 109-147 (tables AND the 5 indexes) into `domains/quarantine.rs` (`pub const SCHEMA_QUARANTINE` at :38, gated by the `quarantine` feature in `manifest::baseline_sql`) |
| D5 | Column-ALTER pattern is `ensure_latency_columns` (`open.rs:220-252`) | Best analog is now Phase 2's **`ensure_kb_library_columns`** at `crates/vox-db/src/store/open.rs:231-271` (PRAGMA-guarded const table of `(table, column, decl)`), called inside the version gate at `open.rs:205-208`, with post-ALTER index creation at `open.rs:209-214`. `ensure_latency_columns` is now at `:279` (runs outside the gate every open) | 03-03 copies `ensure_kb_library_columns` for `search_documents.trust_tier` and puts the backfill UPDATE inside the version gate |
| D6 | Phase 2 migration-test analog is `local_tests.rs:886-942` (hopper) | Better analog is Phase 2's own **`kb_library_migration_from_94`** at `library.rs:935-1000` (hand-builds a v94 DB with raw `turso::Builder::new_local`, opens with `VoxDb::open`, asserts legacy row survives and new writes work). Hopper test still exists at `local_tests.rs:887` | 03-03 copies the v95-DB shape from `kb_library_migration_from_94` |
| D7 | Catalog legacy ops at `contracts/operations/catalog.v1.yaml:26687-26926` | Now **26714-26953** (ids `kb.create` at 26714 ... `kb.clip` at 26934). Phase 2's `kb.search` (26954), `kb.get` (26976), `kb.cite` (26998), `kb.status` (27020), `kb.reindex` (27052) follow immediately. `kb.eval` is separate at 20824 | 03-10 deletes exactly the 12 legacy blocks; must not touch 26954+ |
| D8 | Execution web leg `execution.rs:603-631` | Now **`execution.rs:553-580`** (`web_lines` block; the persist loop is 558-580, `source_uri = format!("web-ingest:{engine_name}:{}", h.path)` at 569, `tokio::spawn` + `let _ = persist_text_document_chunk` at 572-578). `LexicalMemoryFallback` trait still at :21-24; `execute_search_plan` ctx param at :257-262 | 03-04 |
| D9 | Tavily CRAG mirror `bundle.rs:206-221` | **`bundle.rs:203-220`** (`source_uri = format!("tavily:{}", url)` at 210; `persist_text_document_chunk` at 211). Direct `blake3::Hasher` still at `bundle.rs:230` (policy violation, do not copy) | 03-04 |
| D10 | `policy.rs:54-57, 201-203, 407-409, 568-574` | `default_persist_web_hits` **55-57**, field + doc comment **200-201** ("mirrored into `search_documents`"), env default **399-401**, env override **560-565** | 03-04 updates the doc comment at :199 |
| D11 | `retrieval.rs`: `parse_kb_mentions` 234-254, `kb_lines` 359-372, struct field 229-230 | `parse_kb_mentions` **236-254**; `kb_lines` field **230**; KbStore block **358-371**; constructor field **411**; `Trusted` stamp **148** (unchanged); `McpMemoryFallback` **256-270**; ctx construction **315-321** | 03-04 / 03-07 / 03-09 |
| D12 | `message.rs` `kb_lines` 823-833, `@kb` 869-895, writeback 1272-1284 | `kb_lines` consumer **823-833** (unchanged); `@kb` KbStore dump **`kb_mention_lines` 868-892** + push **894-897** (`crate::memory::parse_kb_mentions` alias at 871); writeback **1272-1285**. Note aliases: `crate::kb::signal_chat::...` (= `kb_tools`, `lib.rs:178-179`) and `crate::memory::...` (= `memory_tools`). `message_turn_trace_tests` is a `#[path]` child module declared at **`message.rs:2532`** | 03-07 (all `message.rs` edits, single plan, per Pitfall 7) |
| D13 | `web_gather.rs:350-390`, ingest call at :389 | Loop is **344-385**; `db.ingest_research_document_async(&mut req)` at **:383**; `ExternalResearchPacket` built at 355-374 | 03-06 |
| D14 | `kb_tools/mod.rs` has the legacy modules only | Now also has `pub mod library;` (Phase 2). `kb_tools/library.rs` is **879 lines incl. tests**; its header says "Phase 3 deletes `handlers.rs` / `params.rs`, so nothing here imports from them" | 03-10 deletes handlers/params/signal_*, keeps `library.rs` and `pub use handlers::*` must be removed from `mod.rs` |
| D15 | Dispatch arms 1476-1497; schemas 306-321 | Legacy arms still **1476-1497** and `input_schemas.rs` **306-321**; Phase 2 arms are 1498-1508 and 322-326 (interleaved after the legacy block, so delete by exact arm, not by range-to-end) | 03-10 |
| D16 | "One sanitizer": shim `sanitize_evidence` is the only one | **Three** exist: `crates/vox-research-shim/src/research/orchestrator/helpers.rs:11-55` (`pub(super)`, callers `stages.rs:103-105,239,288`), `vox_config::sanitize_chatml` at `crates/vox-config/src/inference.rs:338` (used by `vox-orchestrator-mcp/src/llm_bridge/infer.rs:497-498`), and a private copy `crates/vox-corpus/src/arca_replay.rs:288` | 03-02 only delegates the shim's. Flag to planner: either leave the other two (documented) or add a follow-up; do not claim "one sanitizer" in docs |
| D17 | Static absence grep over `crates contracts` finds only legacy code | `git grep` also hits: `contracts/kb/eval/corpus.jsonl` (Phase 1 eval corpus mentions `KbStore`, `KbRouter`, `signal_chat`, `signal_research`; git flags it binary), `contracts/capability/capability-registry.yaml` and `contracts/capability/model-manifest.generated.json` (24 hits each), `contracts/reports/gui-surface-coverage.v1.json`, `docs/agents/doc-inventory.json`. The 12 legacy names do NOT appear in `contracts/cli/command-registry.yaml` | 03-10's absence test must exclude `contracts/kb/eval/` and regenerated files must come from `operations-sync` / `gui-surface-coverage --write`, never by hand |
| D18 | Phase 2 `vox_kb_status` is a plain status | `kb_status` (`kb_tools/library.rs:396-440`) has two JSON shapes (indexed / not indexed). `vox_kb_status` is in `NOT_READ_ROLE` (`tests/kb_tool_surface.rs:16`, reason WR-08: returns absolute host paths) | 03-13 adds counts to **both** shapes and keeps it non-read-role; findings counts must not leak snapshot absolute paths |
| D19 | `docs/src/architecture/search-retrieval-ssot-2026.md:80` describes `persist_web_hits` | Still at **line 80** (mirrors into `search_do...`); Phase 2 rewrote the table rows around it (line 95 is `vox_kb_status`). `where-things-live.md` already has Phase 2 rows at **217-219** (Library index, `docs/src/library/`) | 03-13 edits line 80 and appends rows next to 217-219 |
| D20 | `VOX_SEARCH_PERSIST_WEB_HITS_DISABLED` description lives in registry + docs | Also in `contracts/config/env-vars.v1.yaml:4834`, `contracts/secrets/managed-env-names.md:448`, `contracts/secrets/managed-env-names.v1.json:4824`, `contracts/secrets/secret-capabilities.v1.json:4923`; registry entry at `crates/vox-secrets/src/spec/registry/missing.rs:2034` (not 2033) | 03-04 / 03-13: update description text in `missing.rs` then run `vox ci secrets-parity`; env-vars.v1.yaml is a separate hand/gen file to check |
| D21 | Hashing "falls to `vox-orchestrator-mcp`, which has `vox-crypto`" | CONFIRMED (`vox-orchestrator-mcp/Cargo.toml:84,100,95`: `vox-redact`, `vox-crypto`, `vox-bounded-fs`). But **`vox-bounded-fs` is read-only** (`read_utf8_path_capped*`, `normalize_text`); there is no write helper. Atomic snapshot write analog is `vox_db::research_doc_io::atomic_write_secure` (`crates/vox-db/src/research_doc_io.rs:11`) | 03-04 uses `atomic_write_secure` for the snapshot |
| D22 | `vox-search` can reach the tier enum / sanitizer via `vox_db::` | CONFIRMED but only via re-export: `vox-search` has **no** direct dep on `vox-orchestrator-types`, `vox-db-types`, or non-dev `vox-crypto` (`Cargo.toml` deps 20-47; `vox-crypto` at :57 is under `[dev-dependencies]`). `vox-db` depends on `vox-orchestrator-types` (`Cargo.toml:13`) and re-exports `vox_db_types::retrieval` at `lib.rs:186`. `vox-research-shim` has `vox-orchestrator-types` but not `vox-crypto`. `vox-ml-cli` has neither `vox-orchestrator-types` nor `vox-db-types` | 03-01 / 03-02 must add `pub use vox_orchestrator_types::trust::{ContextTrustTier, FindingTier}` and `pub use vox_db_types::kb_text` to `vox-db/src/lib.rs` so search/shim/ml-cli/gui reach them with zero new edges |
| D23 | GUI side doors can call "the mcp capture helper" like other paths | GUI commands hold only `GuiDbPool` -> `pool.handle()` (`Arc<VoxDb>`), **no `ServerState`** (`crates/vox-gui/src/commands/knowledge.rs:80-117, 169-248, 251-298`; `research.rs:228-275`). `vox_orchestrator_mcp::chat_tools::chat::research_turn::{Source, persist_research_findings_to_knowledgebase}` is already called from the GUI (`knowledge.rs:288`) as a plain `&VoxDb` + args function | 03-04's `capture_finding` must take `(&VoxDb, repo_root: &Path, CaptureInput)`, not `&ServerState`; the `ServerState` wrapper is a thin layer on top |
| D24 | MENS KbSignals DB | `run_kb_signals_stage` (`vox-ml-cli/src/commands/mens/pipeline.rs:592-686`, invoked at :377) opens `vox_config::paths::REPO_DB_PATH` = `.vox/db/vox.db` (`vox-config/src/paths.rs:599`), a **different** DB than the user DB (`~/Library/Application Support/vox/vox.db`) where GUI/MCP findings land | 03-08 should note the stage only sees repo-DB findings; do not widen to the user DB in Phase 3 |

### 0.2 Paths/symbols confirmed unchanged (no action)

`ContextTrustTier` at `crates/vox-orchestrator/src/context_envelope.rs:87-95` (still no `Copy/Hash/Ord`); `trust.rs` functions (`check_retraction` 47-73, `score_hit_trust` 190-198, `is_plausibly_academic` 216, `score_hit_trust_for_url` 231-236); `ingest.rs` (`blake3_hex` 17-19, `persist_text_document_chunk` 22-36, `ingest_markdown_tree` 41+); `research.rs` in `vox-db` (`ingest_research_document_async` 45-177, `upsert_search_document` call 123-130); `questioning_telemetry.rs:84-94`; `research_turn.rs:507-577` (callers 486, 647) and `run_deep` `ResearchConfig` at 593-603; GUI `ingest_text` 80-117, `ingest_url` 169-248, `save_research_session_to_kb` 251-298, `save_research_doc` 102-153, `publish_research_doc` 228-275, `main.rs:285-287,333,336`; `MemoryManager::sync_verified_research_findings` `manager.rs:650`; `scientia.rs:353` "libSQL rejects `CREATE TRIGGER`"; `.gitignore:109,111` (`.vox/cache/`); `REPO_CACHE_DIR` `vox-config/src/paths.rs:524`; `.voxignore` has no `.vox/` entry (only `contracts/kb/eval/corpus.jsonl` at :137 for contracts/kb); `vox-db-types` deps are `serde, serde_json, chrono` only (no `regex`, no proptest yet).

---

## File Classification

| New/Modified File | Plan | Role | Data Flow | Closest Analog | Match |
|-------------------|------|------|-----------|----------------|-------|
| `crates/vox-orchestrator-types/src/trust.rs` (NEW) | 03-01 | model (L0 enum) | transform | `crates/vox-orchestrator/src/context_envelope.rs:87-95` (the enum being moved) + `crates/vox-orchestrator-types/src/vcs_capability.rs` (L0 newtype/enum module layout) | exact (move) |
| `crates/vox-orchestrator-types/src/lib.rs` | 03-01 | config (mod list) | n/a | itself, `lib.rs:8-21` (`pub mod` + `pub use` list) | exact |
| `crates/vox-orchestrator/src/context_envelope.rs` | 03-01 | model | transform | itself (replace enum with `pub use`) | exact |
| `crates/vox-search/src/trust.rs` (add `TrustLookup`, `PromotionEligibility`) | 03-01 | service | request-response | itself: `check_retraction` 47-73, `score_hit_trust_for_url` 231-236 | exact |
| `crates/vox-db-types/src/kb_text.rs` (NEW) | 03-02 | utility (sanitizer + newtypes) | transform | `helpers.rs:11-55` `sanitize_evidence` (logic to superset) + `crates/vox-db-types/src/ids.rs` (newtype layout) | role-match |
| `crates/vox-db-types/src/lib.rs`, `Cargo.toml` | 03-02 | config | n/a | `lib.rs:11-60` (`pub mod` + `pub use` per module); proptest dev-dep line from `crates/vox-orchestrator/Cargo.toml:151` | exact |
| `crates/vox-db/src/lib.rs` | 03-02/03-03/03-12 | config (re-exports) | n/a | itself `lib.rs:186` (`pub use vox_db_types::retrieval`), `:289-291` (Phase 2 re-export block) | exact |
| `crates/vox-research-shim/.../helpers.rs` | 03-02 | utility | transform | itself (delegate body to `vox_db::kb_text::sanitize`) | exact |
| `contracts/kb/redteam/cases.v1.jsonl` (NEW) | 03-02 | config (fixture) | file-I/O | `contracts/kb/eval/corpus.jsonl` (+ `.voxignore:137`) | role-match |
| `.voxignore` (+ derived `.cursorignore/.aiignore/.aiexclude`) | 03-02 | config | n/a | `.voxignore:137` entry; derive via `vox ci sync-ignore-files` | exact |
| `crates/vox-db/src/schema/domains/knowledge.rs` (kb_findings DDL, search_documents.trust_tier) | 03-03 | model (DDL) | CRUD | itself `knowledge.rs:46-58` + `kb_index_state` `:81-97` | exact |
| `crates/vox-db/src/schema/manifest.rs`, `contracts/db/baseline-version-policy.yaml` | 03-03/03-12 | config | n/a | Phase 2 bump (`manifest.rs:35-36` comment line `// 95:`; policy yaml `repository_baseline_*`) | exact |
| `crates/vox-db/src/store/open.rs` | 03-03 | migration | batch | `ensure_kb_library_columns` `open.rs:231-271` + gate `:205-214` | exact |
| `crates/vox-db/src/store/ops_memory/kb_findings.rs` (NEW) | 03-03 | model (typed CRUD) | CRUD | `crates/vox-db/src/store/ops_memory/library.rs` (types 27-115, `impl VoxDb` 134+, tests 575+) | exact |
| `crates/vox-db/src/store/ops_memory/mod.rs`, `store/mod.rs` | 03-03/03-12 | config | n/a | `ops_memory/mod.rs:1-15` (`mod library;` + `pub use library::{...}`) | exact |
| `crates/vox-db/src/codex_legacy.rs` (legacy counts reader) | 03-03 | utility | CRUD (read) | `codex_legacy.rs:31-37` `table_exists` | exact |
| `crates/vox-db/src/local_tests.rs`, `tests/kb_findings_store.rs` | 03-03 | test | CRUD | `local_tests.rs:887`, `tests/kb_library_store.rs:1-40`, `library.rs:935-1000` | exact |
| `crates/vox-search/src/capture.rs` (NEW) | 03-04 | service (trait) | event-driven (fire-and-forget) | `execution.rs:21-24` `LexicalMemoryFallback` | exact |
| `crates/vox-search/src/context.rs` | 03-04 | model | n/a | `with_trace_id` builder `context.rs:50-56` | exact |
| `crates/vox-search/src/execution.rs`, `bundle.rs`, `ingest.rs`, `policy.rs` | 03-04/03-05/03-09 | service | request-response | themselves (see D8-D10) | exact |
| `crates/vox-search/tests/web_capture_test.rs` (NEW) | 03-04 | test | request-response | `tests/web_leg_toggle_test.rs:1-70` | exact |
| `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs` (NEW) | 03-04/03-08 | service | CRUD + file-I/O | `kb_tools/library.rs` (hasher 119-121, `require_db`/`err_json` 41-47, reindex 127-147, tests 470+) | role-match |
| `crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs` (NEW), `kb_legacy_migration.rs`, `kb_retrieval_tiers.rs`, `kb_status_inbox.rs` | 03-04/08/09/13 | test | CRUD | `kb_tools/library.rs` `indexed_state()` 470-486; `tests/kb_tool_surface.rs` | role-match |
| `crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs` | 03-04/07/09 | service | request-response | itself (D11) | exact |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` | 03-06/09 | service | request-response | itself `:507-577` | exact |
| `crates/vox-research-shim/.../{config,web_gather,pipeline}.rs`, `research/{persistence,types}.rs` | 03-06 | service | event-driven | `web_gather.rs:344-385`; `ResearchConfig` | exact |
| `crates/vox-gui/src/commands/{knowledge,research}.rs`, `main.rs` | 03-06/03-11 | controller (Tauri cmd) | request-response | themselves (D23) | exact |
| GUI UI: `MemoryView.tsx`, `DocPublishModal.tsx`, `researchActions.ts`, `ResearchView.tsx` + tests | 03-11 | component | request-response | existing `*.test.tsx` beside each | exact |
| `crates/vox-gui/ui/e2e/kb-inbox-side-doors.spec.ts` (NEW) | 03-11 | test | request-response | `e2e/chat-turn-trace.spec.ts`, `e2e/research-trace-panel.spec.ts` | role-match |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` + `message_kb_leak_tests.rs` (NEW) | 03-07 | controller + test | request-response | `message_turn_trace_tests.rs` (child module via `#[path]`, `message.rs:2532`) | exact |
| `crates/vox-orchestrator/src/memory/manager.rs`, `tests/memory_research_sync_test.rs` | 03-07 | service/test | batch | delete only | n/a |
| `crates/vox-orchestrator-mcp/src/server_state.rs` | 03-08 | provider | event-driven | `with_db_initialized` `server_state.rs:646-668` (spawn pattern at 652-657) | exact |
| `crates/vox-ml-cli/src/commands/mens/pipeline.rs` | 03-08 | service | batch | itself `:592-686` | exact |
| `crates/vox-db/src/store/ops_memory/search.rs`, `research.rs`, `questioning_telemetry.rs`, `vox-cli/.../db_research/retrieval.rs` | 03-05 | model/service | CRUD | themselves | exact |
| `crates/vox-db/tests/tiered_writer_invariant.rs` (NEW) | 03-05 | test (static) | file-I/O | `crates/vox-search/tests/tantivy_mirror_removed.rs` | exact |
| `crates/vox-orchestrator-mcp/tests/legacy_voxkb_retired.rs` (NEW) | 03-10/12 | test (static) | file-I/O | `crates/vox-search/tests/tantivy_mirror_removed.rs` | exact |
| contracts: `catalog.v1.yaml`, `tool-registry.canonical.yaml`, `capability-registry.yaml`, `http-read-role-governance.yaml`, `gui-surface-coverage.v1.json` | 03-10/11 | config | batch | Phase 2 02-06 / 02-11 flow (catalog row -> `operations-sync --target all --write`) | exact |
| `contracts/retirement/retired-surfaces.v1.yaml`, `contracts/documentation/retired-symbols.v1.yaml`, `AGENTS.md`, `CHANGELOG.md` | 03-13 | config/docs | n/a | `vox-ars-crate` row pair (retired-surfaces.yaml `documentation-only` + `backed_by_contract`) | exact |
| `docs/src/architecture/search-retrieval-ssot-2026.md`, `where-things-live.md` | 03-13 | docs | n/a | Phase 2 02-08 edits (`where-things-live.md:217-219`) | exact |

---

## Pattern Assignments

### A. Trust tier enum (03-01): `crates/vox-orchestrator-types/src/trust.rs`

**Analog:** the enum being moved, `crates/vox-orchestrator/src/context_envelope.rs:87-95` (verbatim, no `Copy/Hash/Ord`, serde snake_case):
```rust
/// Trust tier used by context conflict and injection policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTrustTier {
    Untrusted,
    Advisory,
    Trusted,
    SystemVerified,
}
```
**Copy:** declaration order is the ranking (ascending trust), so add `Copy, Hash, PartialOrd, Ord` and use `std::cmp::min` as the taint operator. Do NOT add `Default` and do NOT put `#[serde(default)]` anywhere.

**Layout analog:** `crates/vox-orchestrator-types/src/lib.rs:8-21` (`pub mod x;` then `pub use x::{...};`). That crate deps are only `serde` + `thiserror` (`Cargo.toml`), so `trust.rs` may not pull in anything else; use `serde::{Serialize, Deserialize}` with `#[derive]` (the crate lists `serde` with `features = ["derive"]`).

**Back-compat re-export** (replace the enum in `context_envelope.rs`): `pub use vox_orchestrator_types::trust::ContextTrustTier;`. Existing users: `context_envelope.rs:192,483,595` and `vox-orchestrator-mcp/src/memory_tools/retrieval.rs:148` (`vox_orchestrator::ContextTrustTier::Trusted`) keep resolving. Schema `contracts/communication/context-envelope.schema.json` is unaffected (wire strings unchanged).

**Wire parse analog (Phase 2 string tier):** `ops_memory/library.rs:14-25`:
```rust
pub const TRUST_TIER_WIRE: [&str; 4] = ["untrusted", "advisory", "trusted", "system_verified"];
pub fn validate_trust_tier_wire(tier: &str) -> Result<(), StoreError> { /* case-sensitive */ }
```
`ContextTrustTier::as_db_str()` / `from_db_str()` must produce exactly these four strings; **unknown string -> `Untrusted`** (fail-closed) for reads, while writers keep rejecting unknown strings. Keep a test asserting `TRUST_TIER_WIRE` equals the enum's strings (Phase 2 test `trust_tier_wire_accepts_the_four_tiers_only` at `library.rs:620-627` is the shape).

**Test shape:** inline `#[cfg(test)] mod tests` in the same file (test-first policy; `skeleton/untested-pub-api`).

### B. Fail-closed promotion verdict (03-01): `crates/vox-search/src/trust.rs`

**Analog:** the same file. Fail-open ranking path stays untouched; add a sibling tri-state.

Current fail-open shape to leave alone (`trust.rs:47-73`, `:190-198`, `:231-236`):
```rust
pub async fn check_retraction(&self, doi: &str) -> Option<bool> { ... .send().await.ok()?; ... None on failure }
pub async fn score_hit_trust_for_url(title: &str, doi: Option<&str>, url: &str) -> f64 {
    if !is_plausibly_academic(url) { return 1.0; }   // <- this branch is NotApplicable, NOT Unavailable
    score_hit_trust(title, doi).await
}
```
**New pattern:** `TrustScorer::lookup_for_promotion(url, doi) -> TrustLookup` returning `NotApplicable` for the `!is_plausibly_academic` branch, `Unavailable{reason}` where `check_retraction` returns `None`, `Retracted`/`Clean` for `Some(true)/Some(false)`. `promotion_eligibility` is a pure `match` (Unavailable|Retracted -> Ineligible).
**Test analog:** the existing wiremock cases in this file (`check_retraction_returns_none_on_http_error` at :278 uses `MockServer` + `with_base_urls`, `trust.rs:32`) for the `Unavailable` path; plus a 4-row table test. File module doc (`:1-4`) says "Fail-open"; add a sentence that promotion is the one fail-closed consumer.

### C. Sanitizer + newtypes (03-02): `crates/vox-db-types/src/kb_text.rs`

**Analog for logic:** `crates/vox-research-shim/src/research/orchestrator/helpers.rs:11-55` (superset it, then make it delegate). The existing char-retain and token-neutralization order to preserve:
```rust
out.retain(|c| !matches!(c,
    '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'));
out = out.replace("<|im_start|>", "[im_start]").replace("<|im_end|>", "[im_end]");
// ... [INST]/<<SYS>>, <antThinking>, <start_of_turn>, DeepSeek <｜User｜>
```
Add (per RESEARCH Pattern 2, order matters): U+E0000-E007F, U+2060-2064, U+FE00-FE0F, U+E0100-E01EF -> HTML comments (unterminated strips to EOF) -> images (`![a](u)`, `![a][r]`, `<img>`) -> role tokens, looped to a bounded fixpoint (<=4 passes). No `regex` (not a dep of `vox-db-types`): hand-rolled scanner.

**Existing test cases to keep passing** (move them into the shim's delegate tests; they must still pass unchanged): `helpers.rs:88-156` (`test_sanitize_evidence_*`, `test_sanitize_chatml_delegates_to_sanitize_evidence`). `sanitize_evidence` is `pub(super)` and `sanitize_chatml` wraps it (`helpers.rs:5-7`); keep both names, body becomes `vox_db::kb_text::sanitize(text)` (shim already depends on `vox-db`).

**Newtype layout analog:** `crates/vox-db-types/src/ids.rs` (string newtypes; module declared and re-exported in `lib.rs` as `pub mod ids; pub use ids::{...}`). Smart-constructor pattern for `SanitizedText`: private field, single `new(&str)`; a `compile_fail` doctest proves the field is private. Precedent for `compile_fail` doctests: `crates/vox-scientia/src/review/mod.rs:79-81` (`ApprovalToken`, cited in RESEARCH; trybuild is nightly-only in this repo, `.config/nextest.toml:37-49`).

**Property tests:** add `proptest = { workspace = true }` under `[dev-dependencies]` of `crates/vox-db-types/Cargo.toml` (that file currently has no dev-deps section; copy the line from `crates/vox-orchestrator/Cargo.toml:151`). `Cargo.lock` is in 03-02's file list; the dev-dep does not change the crate-edge set.

**Red-team fixture analog:** `contracts/kb/eval/corpus.jsonl` (Phase 1 JSONL under `contracts/kb/`, excluded from AI context by `.voxignore:137`). New `contracts/kb/redteam/cases.v1.jsonl` stores payloads as **ASCII-escaped JSON** (`\ud83d...` surrogate pairs for tag chars) so git holds no raw invisible codepoints, and gets its own `.voxignore` line, then `vox ci sync-ignore-files` for the three derived ignore files (never hand-edit them). Loader for the fixture: tests in `vox-db-types` and, for ingest/injection levels, mcp tests, read it with `include_str!` the way `tests/kb_tool_surface.rs:19-25` does for contract files.

**Note (injection scan):** reading `helpers.rs` tests trips PostToolUse injection scanners (documented RESEARCH Pitfall 9). They are inert test strings.

### D. Schema 96 + `kb_findings` store (03-03)

**DDL analog:** `crates/vox-db/src/schema/domains/knowledge.rs:81-97` (`kb_index_state` is the freshest Phase 2 table: `CREATE TABLE IF NOT EXISTS`, plain `TEXT NOT NULL`, no CHECK/trigger). Turso rejects `CREATE TRIGGER` and CHECK (`scientia.rs:351-354`), so tier validity is enforced in Rust (`FindingTier`), and the ALTER default is the fail-closed `'untrusted'`.

**Migration analog (copy exactly):** `crates/vox-db/src/store/open.rs:231-271`
```rust
async fn ensure_kb_library_columns(conn: &turso::Connection) -> Result<(), StoreError> {
    const COLUMNS: [(&str, &str, &str); 10] = [
        ("search_documents", "corpus", "TEXT NOT NULL DEFAULT ''"),
        ...
    ];
    for (table, column, decl) in COLUMNS {
        let mut rows = conn.query(&format!("PRAGMA table_info({table})"), ()).await?;
        let mut has_column = false;
        while let Some(row) = rows.next().await? {
            if row.get::<String>(1)? == column { has_column = true; }
        }
        drop(rows);
        if !has_column {
            conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};")).await?;
        }
    }
    Ok(())
}
```
Add `("search_documents", "trust_tier", "TEXT NOT NULL DEFAULT 'untrusted'")` (make it `;11`), call from the version gate right after `ensure_kb_library_columns(conn)` (`open.rs:208`), put the one-time backfill `UPDATE` inside the gate (it must read `kb_index_state` and only retag `corpus='library'` rows from `kb_index_state.trust_tier`; `vox://publication/%` -> `advisory`; everything else stays `untrusted`). Index creation after ALTERs: `open.rs:209-214` (`CREATE INDEX IF NOT EXISTS ...` goes there, not in the baseline DDL, because v95 DBs lack the column when `baseline_sql()` runs).

**Version bump checklist (Phase 2's 02-01 flow):** `manifest.rs:36` `= 96`, add comment line `// 96: feat(kb): kb_findings inbox + search_documents.trust_tier (KB Phase 3)` after the `// 95:` line (`manifest.rs:35`); update `contracts/db/baseline-version-policy.yaml` (`repository_baseline_integer`, digest, and add `# re-updated for 96:` comment line after the `# re-updated for 95:` line). Guard test: `baseline_policy_matches_compiled_schema` (`manifest.rs:242-265`).

**Typed CRUD analog:** `crates/vox-db/src/store/ops_memory/library.rs`
- Types first (27-115): `#[derive(Debug, Clone)]`, borrowed `&'a str` fields, doc comment "No `Default` on purpose: every writer must choose a trust tier" (line 27). `NewKbFinding` copies this style with `SanitizedText` / `ContentHash` / `FindingOrigin` / `FindingTier` fields and no `Default`.
- `impl crate::VoxDb { pub async fn ... }` with `params![]` from `turso::params` (line 5-6) and `self.transaction(async move { ... })` for multi-statement writes (`library.rs:156`). For single statements use `breaker.call(|| async move { conn.execute(..).await?; Ok::<_, StoreError>(()) })` as in `search.rs:54-84`.
- Dedupe: `INSERT ... ON CONFLICT(dedupe_key) DO NOTHING` (mirrors the `ON CONFLICT(source_uri) DO UPDATE` in `search.rs:57-63`).
- Re-export chain (3 places, as Phase 2 did): `ops_memory/mod.rs:1-15` (`mod kb_findings; pub use kb_findings::{...};`), `store/mod.rs:93`, `vox-db/src/lib.rs:289-291`.

**Legacy readers (03-03):** `legacy_kb_counts()` returns zeros if the table is absent: copy `table_exists` from `crates/vox-db/src/codex_legacy.rs:31-37` (`SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1`). Readers carry `// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="kb-phase-3-retirement" canonical="kb_findings"`. Legacy row types currently at `vox-db/src/lib.rs:319-347+` (`KbRow`, `KbEntryRow`, `KbRuleRow`), legacy ops `ops_memory/kb.rs` (fns: `kb_create` 5, `kb_list` 31, `kb_delete` 60, `kb_add_entry` 78, `kb_list_entries` 131, `kb_review_entry` 175, `kb_delete_entry` 199, `kb_get_feed` 238, `kb_add_rule` 278, `kb_list_rules` 315, `kb_search_entries` 350, `kb_unqueued_training_entries` 395, `kb_mark_mens_queued` 437, `kb_find_duplicate` 461, `kb_get_entry` 488). 03-03 adds the readers; 03-12 deletes the rest of `kb.rs`.

**Test shapes:**
- Store: `crates/vox-db/tests/kb_library_store.rs:1-40` (integration test using `vox_db::VoxDb::open_memory()` + helper builders `chunk()/doc()`); in-file `#[cfg(test)]` tests as at `library.rs:575-` for rejects-bad-input (`library_store_rejects_bad_trust_tier_and_duplicate_hashes`, :921).
- Upgrade: copy `kb_library_migration_from_94` (`library.rs:935-1000`) but start at a hand-built **v95** DB (a `search_documents` without `trust_tier`), insert one `library:` doc + one `web-ingest:` doc + one `vox://publication/x` doc, open, then assert tiers `trusted`/`untrusted`/`advisory` and that a raw INSERT without tier reads back `untrusted`.
- Doctest: `compile_fail` as in RESEARCH Code Examples (fn signature must match the real `upsert_search_document` once 03-05 adds `tier`).

### E. Web-hit capture seam (03-04)

**Trait + injection analog:** `crates/vox-search/src/execution.rs:21-24`:
```rust
/// Substring scan fallback for markdown memory when BM25 returns empty.
pub trait LexicalMemoryFallback: Send + Sync {
    fn substring_search_lines(&self, query: &str) -> Result<Vec<String>, String>;
}
```
impl lives in the consumer (`McpMemoryFallback`, `retrieval.rs:256-270`) and is passed as `Option<&dyn LexicalMemoryFallback>` (`execution.rs:262`). For capture the sink is carried **inside `SearchRuntimeContext`** (it is already passed to `execute_search_plan` at `execution.rs:257` and to `run_search_with_verification`, so no signature churn). Copy the builder style of `context.rs:50-56`:
```rust
#[must_use]
pub fn with_trace_id(mut self, trace_id: Option<String>) -> Self { self.trace_id = trace_id; self }
```
-> `with_web_capture(mut self, c: Option<Arc<dyn WebFindingCapture>>) -> Self`; `SearchRuntimeContext::new` (4 args; called from tests, `retrieval.rs:315`, `research_turn.rs:615`, `tests/web_leg_toggle_test.rs:23`) must keep its signature and default the new field to `None`. Also update the `Debug` impl (`context.rs:25-35`; add `has_web_capture`).

**Replace the web-leg write** (`execution.rs:558-580`, current shape):
```rust
if policy.persist_web_hits && let Some(db) = db_opt.as_ref() {
    for h in &hits { ... let source_uri = format!("web-ingest:{engine_name}:{}", h.path);
        tokio::spawn(async move { let _ = crate::ingest::persist_text_document_chunk(...).await; }); }
}
```
-> `if policy.persist_web_hits && let Some(cap) = ctx.web_capture.as_ref() { cap.capture(hits.iter().map(CapturedWebHit::from).collect()); }`. **No sink = no write anywhere** (fail-closed). Same replacement at `bundle.rs:203-220` (Tavily). `persist_text_document_chunk` (`ingest.rs:22-36`) loses its two web callers; it keeps `ingest_markdown_tree` (`:41`) and gets a `tier` param in 03-05 (mirror-search-corpus callers -> `Untrusted`).
**Do not copy:** direct `blake3` in `ingest.rs:17-19`, `bundle.rs:230` (crypto policy; the hash is done in mcp via `vox_crypto`).

**Test analog for no-sink/zero-writes:** `crates/vox-search/tests/web_leg_toggle_test.rs:1-70` (isolated wiremock SearXNG, `SearchPolicy { searxng_url: Some(..), enable_wikipedia: false, ... ..Default::default() }`, `SearchRuntimeContext::new(PathBuf::from("."), None, ...)`). For a DB-backed variant use `Arc::new(VoxDb::open_memory().await.unwrap())` as `kb_tools/library.rs` tests do (`indexed_state`, line ~480) and assert `search_documents`/`search_document_chunks` row counts are unchanged.

### F. Capture helper + finding model (03-04): `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs`

**Closest analog:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs` (same crate, same lane, Phase 2 style).

**Imports/helpers to copy** (`library.rs:8-24, 41-51, 119-121`):
```rust
use std::sync::Arc;
use vox_db::{VoxDb /* ... */};
use crate::params::ToolResult;
use crate::server_state::ServerState;

fn require_db(state: &ServerState) -> Result<Arc<VoxDb>, String> {
    state.db.clone().ok_or_else(|| REM_KB_DB.to_string())
}
fn err_json(msg: impl Into<String>, remediation: &str) -> String {
    ToolResult::<()>::err_with_remediation(msg, remediation).to_json()
}
fn hasher(bytes: &[u8]) -> String { vox_crypto::hex_encode(&vox_crypto::secure_hash(bytes)) }
```
The content hash is `'blake3:' + hasher(raw)` (RESEARCH schema). **Never** use `vox_crypto::hash_fast_hex` for provenance (XXH3-128; note `research_turn.rs:519,542` and GUI `ingest_text:93` do use it for node ids, which are being deleted).

**Redaction:** `vox_redact::redact_owned(&text)` (`vox-redact/src/lib.rs:67`; existing callers `chat_tools/chat/agent_loop.rs:1016`, `operation_capture.rs:45`).

**Atomic snapshot write:** `vox_db::research_doc_io::atomic_write_secure(&dest, bytes)` (`crates/vox-db/src/research_doc_io.rs:11`; callers `crates/vox-cli-research/src/lib.rs:485`, GUI `publish_research_doc` via `spawn_blocking`, `research.rs:252-257`). Wrap in `tokio::task::spawn_blocking` as the GUI does. Path = `repo_root.join(vox_config::paths::REPO_CACHE_DIR).join("kb-findings").join(format!("{hex}.txt"))` (`REPO_CACHE_DIR = ".vox/cache"`, `vox-config/src/paths.rs:524`; `vox-orchestrator-mcp` has `vox-config.workspace = true` at `Cargo.toml:87`, so the const is reachable). Filename from hash only (no URL-derived component). Size-cap before writing.

**Snapshot-is-gitignored test (RESEARCH `snapshot_path_is_gitignored`):** assert `include_str!("../../../../.gitignore")` (path relative to the test file) contains `.vox/cache/`. Mutation: change dir to `docs/src/...` and watch it fail. `.gitignore:109,111` is the evidence.

**Signature (flag D23):** `pub async fn capture_finding(db: &VoxDb, repo_root: &Path, input: CaptureInput) -> Result<CaptureOutcome, String>`; `McpFindingCapture { db: Arc<VoxDb>, repo_root: PathBuf }` implements `WebFindingCapture` by `tokio::spawn`-ing `capture_finding` and logging failures with `tracing::warn!` (RESEARCH V7: replace `let _ =`). `web_capture_for_state(&ServerState) -> Option<Arc<dyn WebFindingCapture>>` returns `None` when `state.db` is `None`.

**`ServerState`-based tests:** copy `indexed_state()` (`library.rs:~470-486`): `ServerState::new_test().await.with_db_initialized(db).await`, `state.repository.root = temp repo`, `#[serial]` where env is touched. For web stubs use wiremock like `message_turn_trace_tests.rs:17-18`.

**Wiring in retrieval:** `run_retrieval_bundle` (`retrieval.rs:315-321`) builds the ctx; chain `.with_web_capture(web_capture_for_state(state))` after `.with_trace_id(trace)`. `run_deep` also builds a ctx (`research_turn.rs:615-620`): same.

### G. Research-turn reroute (03-06)

**Analog:** `research_turn.rs:507-577` `persist_research_findings_to_knowledgebase(db: Option<&VoxDb>, query, sources: &[Source], summary: Option<&str>) -> usize` (returns count; fire-and-forget callers at 486 and 647; also called from GUI `knowledge.rs:288`). Keep the `usize` return contract for the GUI (`save_research_session_to_kb` returns `Result<usize, String>`). New fns `capture_research_sources` (one finding per source, snippet is snapshot, origin quick/session) and `capture_research_synthesis` (one `research_deep` finding, `FindingTier::Untrusted` per taint rule, source URLs in `provenance_json`, no confidence field). The `Source` struct is already pub and used by the GUI (`knowledge.rs:269-281`). The `trace.push(StageRecord::new("knowledge_base", ...))` blocks at 493-501 and 654-664 should be kept but retitled ("filed N pending findings").
**`ResearchConfig` extension:** `run_deep` builds it with `..ResearchConfig::default()` (`research_turn.rs:593-603`), so a new `finding_capture: Option<Arc<dyn WebFindingCapture>>` field must default to `None` (derive/impl `Default`) and must not break the many other `ResearchConfig { .. }` constructors; grep before adding a required field.
**Deep gather:** replace `web_gather.rs:344-385` loop's `ExternalResearchPacket`/`ingest_research_document_async` block with `config.finding_capture` (full `raw_content` -> snapshot); keep `db.create_research_source` at :345. Test file to extend: `crates/vox-research-shim/tests/scientia_phase_0a_persistence.rs` (in 03-06's list).
**Remove:** `persist_to_docs` branch and its writer (`research/persistence.rs`, `types.rs` field; `research_turn.rs:608` sets `persist_to_docs: false`).

### H. GUI side doors (03-06 Rust, 03-11 UI)

**Analog:** the commands themselves (`knowledge.rs:80-117`, `169-248`, `251-298`; `research.rs:228-275`). Each becomes: validate input (keep existing empty/URL/robots checks, keep `check_robots_allowed` and the `vox_search::scraper::fetch_and_extract` call in `ingest_url`), then call `capture_finding(db.as_ref(), repo_root, CaptureInput { origin: FindingOrigin::GuiIngestUrl, ... })` and return the finding id (string) instead of a node id. `publish_research_doc` stops calling `atomic_write_secure` into `docs/src/architecture`; keep `store_research_artifact(session_id, "{}", &content)` (episodic log), keep the slug traversal check (`research.rs:238-240`), put the proposed path in `provenance_json`. **Delete** `save_research_doc` (`research.rs:102-153`) and its registration `main.rs:333`; the new `PublishDocResult` shape must still satisfy `researchActions.ts` / `DocPublishModal.tsx` until 03-11 updates them; coordinate the Rust struct and the TS type in the same wave (03-06 then 03-11 is already the order).
**UI tests:** update the existing `MemoryView.test.tsx`, `DocPublishModal.test.tsx`, `ResearchView.test.tsx`, `researchActions.test.ts` (vitest; command `pnpm --dir crates/vox-gui/ui test`); new Playwright spec modeled on `e2e/chat-turn-trace.spec.ts` / `research-trace-panel.spec.ts` using `installTauriMock` / `installTauriMockRich` and screenshots into `crates/vox-gui/ui/review-bundle/latest/` (AGENTS.md "GUI Visual Verification Invariant"). `contracts/reports/gui-surface-coverage.v1.json` is regenerated (`vox ci gui-surface-coverage --write`); never hand-edited. In a fresh worktree `vox-gui` Rust tests fail in `tauri-build` until the sidecar and `ui/dist` exist (AGENTS.md Perennial Bug Patterns); keep logic and tests in mcp.

### I. Deleting the chat leaks (03-07): `message.rs`, `retrieval.rs`, signal adapters

**Edit map in `message.rs` (all in one plan, Pitfall 7):**
- `kb_lines` consumer `:823-833` -> delete.
- `@kb` dump `:868-897` (`kb_mention_lines` + `[MENTIONED KNOWLEDGE BASES]` push) -> delete; leave `crate::memory::parse_kb_mentions` and `retrieval.rs:236-254` + its tests `:446-468` intact (Phase 7 remap).
- writeback `:1272-1285` (the `// KB signal adapter: fire-and-forget` block) -> delete.
**`retrieval.rs`:** remove `kb_lines` field (`:229-230`), constructor arg (`:411`), and the `:358-371` block; the `RetrievalBundle` derives `Default` (`:221`) so removing a field is safe.
**Seeded test (Pitfall 2):** new `crates/vox-orchestrator-mcp/src/chat_tools/chat/message_kb_leak_tests.rs`, declared exactly like the existing child module (`message.rs:2532`: `#[path = "message_kb_leak_tests.rs"] mod ...` under `#[cfg(test)]`). Copy the harness from `message_turn_trace_tests.rs`: `test_state()` builds a `hermetic_stub` `ServerState` (lines 28-60), `register_pinned`, `point_openrouter_at` / `restore_openrouter` under `CHAT_MESSAGE_ENV_LOCK`, `plain_body()` wiremock completion, `try_run_agent_turn`. Seed `LEGACY_VOXKB_DDL` (from 03-03; table DDL copied from `knowledge.rs:109-147`) with `db.connection().execute_batch(..)`, then attach with `state.with_db_initialized(db)` and assert `kb_entries` count unchanged and no `chat_turn` finding. Mutation proof: re-insert a raw `INSERT INTO kb_entries` at the old spawn site.
**Delete:** `kb_tools/signal_{chat,research,scientia}.rs` (168 / 73 / 63 lines), `MemoryManager::sync_verified_research_findings` (`manager.rs:650-706`) + `crates/vox-orchestrator/tests/memory_research_sync_test.rs`. Remove `pub mod signal_*;` from `kb_tools/mod.rs`.

### J. Tier propagation + injection-boundary sanitization (03-09)

**Analog for the preamble trigger:** `retrieval.rs:272-278` `map_trigger` (`RetrievalTriggerMode::AutoChatPreamble` -> `BundleTrigger::AutoChatPreamble`). `run_retrieval_bundle` builds `policy` at `:307`; set `policy.exclude_untrusted_hits = true` when `trigger == AutoChatPreamble` (new `SearchPolicy` field, serde default false, like `persist_web_hits` at `policy.rs:199-201`).
**Envelope tier = min of hits** replaces the blanket stamp at `retrieval.rs:148`:
```rust
trust: vox_orchestrator::ContextTrust {
    trust_tier: vox_orchestrator::ContextTrustTier::Trusted,   // <- replace with min over hit tiers
    authority_rank: 70, ...
```
Add a `trust_tier: ContextTrustTier` (or `min_trust_tier`) to `RetrievalEvidenceEnvelope` (struct above `:101`; it is `Serialize`d into `structured_payload` at `:187` and has a contract test `tests/retrieval_evidence_context_envelope_contract.rs`, in 03-09's file list, update both together).
**Chunk hit tiers:** `vox-db` reader `trust_tiers_for_chunk_ids` in `ops_memory/search.rs` (new, in 03-09's list) reads `search_documents.trust_tier` joined through `search_document_chunks.document_id` (join shape: `library.rs:117-119` `CHUNK_SELECT`).
**Sanitize at injection:** map every bundle line through `SanitizedText::new(...)`; also `sources_from_hits` and the numbered research sources (`research_turn.rs:473` `trace.sources = sources_from_hits(&report.hits, 8)`), and every Phase 2 tool output that returns body text (`kb_get`, `kb_cite`, `hit_json` snippet at `kb_tools/library.rs:73-86`, `get_by_path` 284, `get_by_hash` 316). Red-team test: iterate `contracts/kb/redteam/cases.v1.jsonl` through each boundary.
**KG leg tier from node_type:** `fact`/`concept` -> Advisory; everything else (incl. `external_research`, `web_scrape`, `web_research_source`, `research_synthesis`, `research_finding`, `research_cache`, `document`) and unknown -> Untrusted, via `tier_for_knowledge_node_type` (03-01). `research_cache` nodes are never search results (`pipeline_cache.rs`, in shim; reads in `ops_memory/knowledge.rs` in 03-09's list).

### K. Legacy migration + MENS (03-08)

**Migration analog (spawn at attach):** `server_state.rs:646-668` `with_db_initialized`, spawn pattern at 652-657:
```rust
let reg = Arc::clone(&self.model_prompt_registry);
let db_reg = db.clone();
tokio::spawn(async move {
    if let Err(e) = reg.populate_from_db(&db_reg).await {
        tracing::warn!("model_prompt_registry populate_from_db failed: {e}");
    }
});
```
Copy: spawn `migrate_legacy_kb_entries(&db_for_kb, &repo_root)`, `tracing::warn!` on `Err` (never abort startup; RESEARCH A7), `tracing::info!` with the counts before copy ("counted first"). It runs on every `ServerState` DB attach, so idempotence via `dedupe_key` derived from `legacy_id` is mandatory.
**Test:** copy `kb_library_migration_from_94` (D6) + the RESEARCH "Seeded legacy-DB migration test shape"; seed through `LEGACY_VOXKB_DDL` + raw `INSERT`s (1 KB, 2 entries one empty, 1 rule); assert counts-before-copy, `(migrated, skipped_empty)`, idempotence, and zeros when tables are absent (`VoxDb::open_memory()` on schema 96 has no VoxKB tables once 03-12 lands; before 03-12 it does, so build the test to create the DDL explicitly).
**MENS stage analog:** `pipeline.rs:592-686` (`run_kb_signals_stage`): open `REPO_DB_PATH` -> list unqueued -> write `mix_sources/kb_signals.jsonl` -> mark queued. Retarget the two calls to `kb_findings_unqueued_reviewed(10_000)` / `kb_findings_mark_mens_queued`; keep the `serde_json::json!({"type":"dpo","source":"kb",...})` record shape and the early-return-when-empty branches; **delete the `"type":"sft"` block** (decision D-06: DPO-only). Pairing key changes from `(source_signal, source_ref)` to `provenance.query`. Stage name `KbSignals => "kb_signals"` (`populi/action_prelude.rs:167`) stays. Add `#[cfg(test)]` tests in `pipeline.rs` (file has a test module after `is_lock_error`, `:700`).

### L. Tiered-writer invariant + retirement absence tests (03-05, 03-10, 03-12)

**Analog (copy structure verbatim):** `crates/vox-search/tests/tantivy_mirror_removed.rs:1-51`:
```rust
const BANNED: &[&str] = &["TantivyDocsIndex", ...];
const FILES: &[&str] = &["src/lib.rs", ..., "../vox-orchestrator-mcp/src/memory_tools/retrieval.rs"];
fn manifest_path(rel: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join(rel) }
#[test] fn tantivy_mirror_removed() { assert!(!manifest_path("src/x.rs").exists(), ...);
    for rel in FILES { let src = std::fs::read_to_string(manifest_path(rel)).unwrap_or_else(...);
        for banned in BANNED { assert!(!src.contains(banned), "{rel} mentions removed symbol `{banned}`"); } } }
```
`legacy_voxkb_retired.rs` banned list: `KbStore`, `KbRouter`, `kb_lines`, `signal_chat`, `signal_research`, `signal_scientia`, and the 12 legacy tool ids (use the `([^a-z_]|$)` boundary: tool ids like `vox_kb_list` are prefixes of nothing else but `vox_kb_list_entries`/`vox_kb_list_rules`, so match with the quote or a non-ident following char). Scope to `crates/` source files plus the catalog/registry files, and **exclude** `contracts/kb/eval/` (D17), `contracts/retirement/`, `contracts/documentation/retired-symbols.v1.yaml`, `docs/`.
`tiered_writer_invariant.rs`: walk `crates/**/*.rs` (this repo's convention is `std::fs` + manual recursion or `walkdir`, a dep of `vox-search` only; `vox-db` tests can use `std::fs::read_dir`), assert every `INSERT INTO search_documents` / `INSERT INTO kb_findings` literal occurs only in the allowlisted files (`ops_memory/search.rs`, `ops_memory/library.rs`, `ops_memory/kb_findings.rs`). Existing test-only raw inserts at `crates/vox-db/tests/ops_memory_tests.rs:139,189,252` must be allowlisted or rewritten through a tiered fn (they insert `(source_uri, title)` only and rely on the column default `'untrusted'`).
**Sql-surface guard:** `vox ci sql-surface-guard --all` flags raw `query`/`execute` outside `store/ops_*.rs` (Phase 2 hit this: `deferred-items.md`, RESOLVED in 02-07). Any new test in `vox-search`/`vox-orchestrator-mcp` that seeds legacy tables with raw SQL (03-07, 03-08, 03-13) must go through a `VoxDb` method or be allowlisted; seeding via `db.connection().execute_batch(..)` in a unit test inside `kb_tools/library.rs` style is how Phase 2 handled it only after moving SQL into `VoxDb` methods (`library_chunk_positions`, `embedding_count`). Plan for a `#[doc(hidden)]`-style test seeding helper in `vox-db` (e.g. `legacy_voxkb_seed_for_tests`) or an allowlist entry; check `crates/vox-cli/src/commands/ci` sql-surface-guard allowlist.

### M. Contract and registry edits (03-10, 03-11, 03-13)

**Flow (Phase 2 02-06 / 02-11 precedent):** edit `contracts/operations/catalog.v1.yaml` (delete the 12 `kb.*` blocks at 26714-26953), then `vox ci operations-sync --target all --write` (regenerates `tool-registry.canonical.yaml`, `capability-registry.yaml`, `model-manifest.generated.json`, `contracts/reports/operations-catalog-inventory.v1.json`; Phase 2 summaries list exactly these four plus `contracts/cli/command-registry.yaml` for CLI rows), then delete `dispatch.rs:1476-1497` arms and `input_schemas.rs:306-321` arms by hand (`input_schemas.rs:307` `parse_obj(r#"{"type":"object","additionalProperties":false}"#)` is the shared no-arg shape used by `vox_kb_status`/`vox_kb_reindex` at 325-326, keep). `contracts/mcp/http-read-role-governance.yaml:16-20` (5 legacy lines) is hand-maintained; the Phase 2 parity test `tests/kb_tool_surface.rs` (`read_role_tools()` reads it, lines 18-24) must stay green: it asserts exactly `vox_kb_search/get/cite` are read-role.
**Retirement rows (03-13):** copy the `vox-ars-crate` pair. `retired-surfaces.v1.yaml` row (documentation-only, needs `backed_by_contract` + `contract_symbol_id`):
```yaml
  - id: voxkb-kbstore-surface
    retired_pattern: "KbStore / KbRouter / legacy vox_kb_* CRUD tools"
    canonical: "vox_kb_search / vox_kb_get / vox_kb_cite (Library) and kb_findings (inbox)"
    agents_md_row: "KbStore, KbRouter, legacy vox_kb_* CRUD tools"
    enforcement:
      kind: documentation-only
      backed_by_contract: "contracts/documentation/retired-symbols.v1.yaml"
      contract_symbol_id: "voxkb-kbstore-surface"
    rationale: ...
```
plus a matching `symbols:` entry (`id`, `pattern`, `replacement`, `rationale`) in `contracts/documentation/retired-symbols.v1.yaml`, plus a row in the AGENTS.md "Retired Surfaces" table whose first-cell text equals `agents_md_row`. (Alternative `kind: cli-check` pointing at the 03-10 absence test is stronger but needs a wired check; do not claim `cli-check` unless it is registered.) CHANGELOG: `## [Unreleased]` is at `CHANGELOG.md:14`.
**Secrets/env text:** update `crates/vox-secrets/src/spec/registry/missing.rs:2034` description and the contracts listed in D20, then `vox ci secrets-parity` and `vox ci secret-env-guard`. Do not remove the `SecretId`.

---

## Shared Patterns

### S1. Test-first and mutation-proof (AGENTS.md)
Every new `pub fn` in `crates/*/src/**` needs an in-file `#[cfg(test)]` test (detector `skeleton/untested-pub-api`, lefthook `tdd-guard`). Phase 2 precedent for "tests in the same file as the code": `ops_memory/library.rs:575+`, `kb_tools/library.rs:~460+`, `trust.rs:239+`. Mutation table lives in 03-RESEARCH.md "Mutation-proof steps"; record presence-before/after in each SUMMARY.

### S2. Crate-edge discipline (no new edges)
Verified edge facts (D22): types flow `vox-orchestrator-types` / `vox-db-types` -> `vox-db` (re-export) -> `vox-search` / `vox-research-shim` / `vox-gui` / `vox-ml-cli`. Hash/redact/snapshot only in `vox-orchestrator-mcp` (has `vox-crypto`, `vox-redact`, `vox-bounded-fs`, `vox-db`, `vox-search`, `vox-orchestrator-types`). Never edit `contracts/ci/crate-edges.allow.v1.json`; `vox ci crate-edges` must stay clean.

### S3. DB write path (Turso)
`breaker.call(|| async move { conn.execute(sql, params![..]).await?; Ok::<_, StoreError>(..) })` for single statements (`search.rs:54-84`); `self.transaction(async move {..})` for multi-statement (`library.rs:156`). No `CHECK`, no triggers. Reads that must tolerate absent tables: `sqlite_master` probe (`codex_legacy.rs:31-37`).

### S4. Error/response shape for MCP tools
`ToolResult::<()>::err_with_remediation(msg, remediation).to_json()` and `ToolResult::ok(json!({..})).to_json()` (`kb_tools/library.rs:19,41-47,91-96`). Replace `let _ =` swallowing on capture paths with `tracing::warn!`.

### S5. Generated artifacts
Never hand-edit: `tool-registry.canonical.yaml`, `capability-registry.yaml`, `model-manifest.generated.json`, `operations-catalog-inventory.v1.json`, `gui-surface-coverage.v1.json`, `docs/src/reference/cli-command-surface.generated.md`, `.cursorignore/.aiignore/.aiexclude`. Fix inputs; regenerate (`vox ci operations-sync --target all --write`, `vox ci gui-surface-coverage --write`, `vox ci sync-ignore-files`). The `ssot-autoregen` PR job covers drift, but run the generator locally so tests (`kb_tool_surface`) pass.

### S6. Docs frontmatter
Any new `docs/src/**` page needs `title/description/category`. Phase 3 edits existing pages only (`search-retrieval-ssot-2026.md`, `where-things-live.md`); keep their frontmatter intact; lint with `cargo run -p vox-doc-pipeline -- --lint-only --paths <file>`.

### S7. Verification commands (local scope only)
`cargo check -p <crate>` and `cargo nextest run -p <crate> <filter>` per plan; `cargo test -p vox-db --doc` for the `compile_fail` doctest; `vox ci operations-sync --target mcp` (check mode); `vox ci sql-surface-guard --all`; `vox ci crate-edges`; `vox ci secrets-parity`. No workspace-wide clippy/test locally. All `cargo` goes through the broker shim on PATH.

---

## No Analog Found

| File / Concern | Role | Data Flow | Reason |
|----------------|------|-----------|--------|
| HTML-comment + markdown-image neutralizer inside `kb_text::sanitize` | utility | transform | Nothing in the repo strips `<!--` or `![..](..)`; only the shim's zero-width/bidi/role-token logic exists. Use RESEARCH Pattern 2 ordering |
| `proptest` in `vox-db-types` | test | transform | The dev-dep is new to this crate; use `vox-orchestrator`'s existing proptest usage (`Cargo.toml:151`) and `vox-compiler` for strategy shape. Idempotence/no-banned-codepoint generators are new |
| Red-team ASCII-escaped fixture loader | test | file-I/O | No escape-aware JSONL loader exists; `contracts/kb/eval/corpus.jsonl` is the format neighbor (plain text) |
| `FindingTier` (two-variant narrowing of `ContextTrustTier`) with `From` impl | model | transform | No similar narrowing enum; design from RESEARCH Pattern 1 |
| `compile_fail` doctest for "no tier does not compile" | test | n/a | Only analog is `vox-scientia` `ApprovalToken`; verified present at `crates/vox-scientia/src/review/mod.rs:79` (` ```compile_fail `) |
| Content-addressed snapshot dir writer (`.vox/cache/kb-findings/<hash>.txt`) | utility | file-I/O | `atomic_write_secure` exists for single files, but no content-addressed cache writer with size cap exists; `vox-bounded-fs` is read-only. Compose `atomic_write_secure` + a length check |
| `TrustLookup` fail-closed tri-state | service | request-response | Only fail-open lookups exist (`trust.rs`); shape comes from RESEARCH Pattern 5 |
| Playwright spec for inbox side doors | test | request-response | Role-match only (`chat-turn-trace.spec.ts`, `research-trace-panel.spec.ts`); the "pending review" toast surface is new |

---

## Metadata

**Analog search scope:** `crates/vox-db{,-types}`, `crates/vox-orchestrator{,-types,-mcp}`, `crates/vox-search`, `crates/vox-research-shim`, `crates/vox-gui` (Rust + `ui/`), `crates/vox-ml-cli`, `crates/vox-config`, `crates/vox-secrets`, `contracts/{operations,mcp,capability,retirement,documentation,db,kb,config,secrets}`, `docs/src/architecture`, plus Phase 2 summaries 02-01..02-12 and `deferred-items.md`.
**Method note:** `ctx_*` read tools refuse paths outside the active project root in this worktree; native `Read`, `awk` and `git grep` were used instead. `git grep -E` does not support `\b` on this macOS build (first pass returned zero hits); the boundary form `([^a-z_]|$)` was used.
**Files scanned:** ~70 read or grepped
**Pattern extraction date:** 2026-10-06
