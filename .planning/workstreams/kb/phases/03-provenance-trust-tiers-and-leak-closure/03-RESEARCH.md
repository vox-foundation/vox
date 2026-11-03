# Phase 3: Provenance, Trust Tiers and Leak Closure — Research

**Researched:** 2026-10-03
**Domain:** Provenance and trust tiering for a git-canonical knowledge base; unreviewed write paths closed; legacy VoxKB retired (Rust: `vox-db`, `vox-search`, `vox-orchestrator-mcp`, `vox-gui`, `vox-ml-cli`)
**Confidence:** HIGH for the current-state map (read from source this session). MEDIUM-HIGH for the target design (it is a recommendation, checked against `layers.toml` and `crate-edges.allow.v1.json`).

<user_constraints>
## User Constraints

No CONTEXT.md exists for this phase. The user chose to plan Phases 1–3 first and hold one discussion for all of them afterwards. The items below are **locked**: the user already decided them in REQUIREMENTS.md, ROADMAP.md, STATE.md and the practices SSOT §5. The planner must honor them. Open items appear under "Open Questions for discussion" at the end of this document.

### Locked Decisions (verbatim from REQUIREMENTS / ROADMAP / STATE / practices §5)
- **TRUST-01:** "Every indexed row and finding has a required trust tier mapped onto `ContextTrustTier`: SystemVerified = curated + mechanical verification evidence; Trusted = curated (merged to git); Advisory = agent-derived; Untrusted = inbox / live web. Writes without a tier fail to compile or are rejected"
- **TRUST-02:** "Web hits are no longer written into the searchable corpus; `persist_web_hits` routes them into the `kb_findings` inbox as Untrusted"
- **TRUST-03:** "Model output never re-enters the KB at elevated trust — the `signal_research` 0.95 confidence floor and the post-turn `accepted=1` chat writeback are removed or routed to the inbox"
- **TRUST-04:** "Each finding stores summary, short quotes, source URL, fetch date and content hash; a full-text snapshot is kept locally under a gitignored path and is never committed"
- **TRUST-05:** "Ingested and injected text passes a sanitizer (Unicode tag block, zero-width/bidi characters, HTML comments, off-allowlist markdown images) backed by property tests and a red-team fixture set"
- **TRUST-06:** "Promotion eligibility fails closed when the trust lookup errors"
- **RET-02:** "All KB retrieval goes through `vox-search` (single retrieval path); the legacy `KbStore` LIKE search, `KbRouter`, signal adapters, `kb_lines` and the 12 legacy `vox_kb_*` CRUD tools are retired, with colliding names replaced rather than aliased"
- **RET-03:** "Legacy `kb_entries`/`knowledge_bases` rows are counted before quarantine; non-empty rows are migrated into the inbox as pending findings"
- **GROW-06:** "Existing side doors (`publish_research_doc`, GUI `ingest_url`/`ingest_text`, scientia/MEMORY sinks) route through the inbox; MENS `KbSignals` reads `kb_findings`"
- Roadmap decision: "GROW-06 (side doors → inbox, MENS `KbSignals` → `kb_findings`) lands in Phase 3 with leak closure, not Phase 9."
- Research decision: "No new crate edges (`vox-search → vox-crypto`, `vox-cli → vox-forge` are user-authorization proposals only)."
- Practices §5: "Captured web findings | DB inbox `kb_findings` with provenance: summary, quotes, URL, date, content hash. Full text is a local gitignored snapshot"
- Practices §5: "Trust | Trust tiers with a ranking penalty, mapped onto the existing `ContextTrustTier` enum (`Untrusted`, `Advisory`, `Trusted`, `SystemVerified`) | One vocabulary. Categorical, not a fake-precise score"
- Practices §5: "Legacy write paths | `persist_web_hits` rerouted into the inbox tier."
- Practices §5: "Legacy VoxKB surface | `KbStore` (`LIKE` search), the signal adapters and the dead Tantivy mirror are retired into `vox-search`-backed retrieval. Colliding `vox_kb_*` tool names are replaced, not aliased"
- Practices §5: "Per-chat scope | `@kb` mentions are kept as a locked v1 control and remapped onto the Library scope" (the remap itself is INJ-09, Phase 7)
- Out of scope (REQUIREMENTS): "Auto-promotion of web content or agent memories into curated docs"; "A single numeric trust score"; "New crate edges `vox-search → vox-crypto`, `vox-cli → vox-forge`".

### Claude's Discretion (no user decision yet; recommendations follow below)
- Where the shared tier enum and the sanitizer live (crate placement without new edges)
- `kb_findings` column set, dedupe key and snapshot path
- How existing user-DB rows (legacy web-mirror rows, `kb_entries`) are tiered or migrated
- How `@kb` behaves between Phase 3 and Phase 7
- What the MENS `KbSignals` stage emits from `kb_findings`
- Plan and wave split

### Deferred Ideas (OUT OF SCOPE for Phase 3)
- Spotlighting delimiters, exfiltration tool gating, `sensitivity` egress check, `kb_context` event, budgeted preamble (INJ-*, Phase 7)
- LLM summarization of captured pages, `vox_kb_propose`/`inbox_list`/`inbox_review`, promotion PR, corroboration (GROW-01..05, Phase 9)
- `authorship`/`training_eligible` facet rules and the `vox-gamify` XP incentive (TAX-*, Phase 4)
- Ranking use of tiers (RET-04, Phase 5); Inbox GUI (Phase 12)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TRUST-01 | Required trust tier on every indexed row and finding; four-way mapping; no-tier writes fail to compile or are rejected | §Pattern 1 (move `ContextTrustTier` to `vox-orchestrator-types`, required enum param, no `Default`); the four-way mapping table; Pitfall 1 (Turso has no CHECK/triggers, so enforcement must be at the Rust type boundary) |
| TRUST-02 | Web hits go to `kb_findings` as Untrusted, never into the corpus | §Sink inventory (rows S1–S3); §Pattern 3 (`WebFindingCapture` trait injected into `SearchRuntimeContext` and `ResearchConfig`) |
| TRUST-03 | Remove the 0.95 floor and the `accepted=1` writeback | §Sink inventory (rows S8–S10); delete `signal_*` and the `message.rs:1272-1284` spawn |
| TRUST-04 | Finding = summary, quotes, URL, fetch date, content hash; gitignored snapshot | §`kb_findings` schema; snapshot at `.vox/cache/kb-findings/<blake3>.txt` (`.vox/cache/` is already gitignored) |
| TRUST-05 | Sanitizer with property tests and a red-team set, at ingest and injection | §Pattern 2 (`SanitizedText` in `vox-db-types`, a superset of the existing `sanitize_evidence`); proptest 1.11.0 is already in the lockfile; fixtures in `contracts/kb/redteam/` |
| TRUST-06 | Promotion eligibility fails closed | §Pattern 5 (a `TrustLookup` tri-state in `trust.rs`; NotApplicable ≠ Unavailable) |
| RET-02 | Single `vox-search` retrieval path; legacy surface retired | §Retirement inventory (every file and contract to touch) |
| RET-03 | Count legacy rows, migrate non-empty rows to pending findings | §Pattern 4 (migration in `vox-orchestrator-mcp`, where `vox-crypto` is available); measured counts: 0/0/0 on both local DBs |
| GROW-06 | Side doors route into the inbox; MENS reads `kb_findings` | §Sink inventory (rows S4–S7, S11–S12); §MENS retarget |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **Test-first:** every new `pub fn` in `crates/*/src/**` needs a `#[test]`/`#[cfg(test)]` block in the same file before the commit (detector `skeleton/untested-pub-api`; the lefthook `tdd-guard` hook blocks the commit). Task briefs start with the failing test.
- **Crypto:** hashing goes through `vox-crypto`. Do not import `blake3`/`sha2`/`sha3` directly outside `crates/vox-crypto/`. (`vox-search/src/ingest.rs:17-19`, `vox-db/src/research.rs:13-15` and `vox-search/src/bundle.rs:232` already break this. Don't copy that pattern.)
- **Dependency discipline:** no new workspace crate edges. `exceptions` entries are USER-AUTHORIZED-ONLY. Never regenerate `crate-edges.allow.v1.json` to admit an edge you added.
- **Secrets:** via `vox_secrets::resolve_secret`. Keep `SecretId::VoxSearchPersistWebHitsDisabled` (do not remove secret IDs casually; `vox ci secret-env-guard` / `secrets-parity` gate them).
- **MCP tools:** edit `contracts/operations/catalog.v1.yaml`, then `vox ci operations-sync --target mcp --write` (and `--target capability`). Never hand-edit `tool-registry.canonical.yaml` or `capability-registry.yaml`. `contracts/mcp/http-read-role-governance.yaml` must match the catalog `http_read_role_eligible` set (hand-maintained, checked).
- **Retired surfaces:** a retirement adds a row to the AGENTS.md "Retired Surfaces" table **and** to `contracts/retirement/retired-surfaces.v1.yaml` (CR-L6 parity gate). Vestigial migration code carries `// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="..." canonical="..."`.
- **Docs:** new or substantially authored `docs/src/**` pages need YAML frontmatter (`title`, `description`, `category`). Research goes to `docs/src/architecture/`, not IDE memory. If a concept is missing from `where-things-live.md`, add the row in the same PR.
- **AI-context exclusion:** edit `.voxignore`, then `vox ci sync-ignore-files`. Do not edit `.cursorignore`/`.aiignore`/`.aiexclude`.
- **Glue code:** VoxScript (`.vox`) only. No new `.sh`/`.py`/`.ps1`.
- **CI locally:** `cargo check -p <crate>` and `cargo nextest run -p <crate>` only. No workspace-wide clippy or tests. `cargo` goes through the build-broker shim on PATH.
- **GUI:** UI changes pass `pnpm --dir crates/vox-gui/ui typecheck` and the tests. Substantially modified surfaces need Playwright steppers with screenshots. In a fresh worktree `vox-gui` won't build without the sidecar and `ui/dist` (`vox run scripts/gui-build.vox`; `pnpm install && pnpm build`).
- **Security guards are mutation-proven:** break the guard, watch the test fail, restore. Check presence before and after, because a concurrent agent or fmt pass can revert the edit.
- **Schema:** bump `BASELINE_VERSION`, update `contracts/db/baseline-version-policy.yaml` (integer and Keccak digest), and add explicit `PRAGMA table_info`-guarded `ALTER TABLE` in `VoxDb::migrate` for existing DBs.
- **Archive:** never read or ingest `docs/src/archive/`.

## Summary

Phase 3 changes the KB's identity model. Today "the KB" is a mutable DB notes store (`knowledge_bases`/`kb_entries`) plus six-plus unreviewed sinks that write web text and model output straight into retrievable corpora. After Phase 3 the model is two-part: a git-canonical index (Phase 2's library rows, `Trusted`) and a single `kb_findings` inbox (`Untrusted`/`Advisory`, `pending`) that nothing auto-injects. The core work is plumbing, not new technology. No new external packages are needed: proptest 1.11.0 is already in `Cargo.lock`, and `vox-crypto`, `vox-redact` and `vox-bounded-fs` already exist.

The repo map turned up **more sinks than the milestone research listed**. Deep research's `web_gather.rs:389` calls `ingest_research_document_async`, which writes full web page bodies into `knowledge_nodes`, `snippets` **and** `search_documents`, with `source_uri` = the raw URL (no `web-ingest:` prefix). The Tavily CRAG path in `bundle.rs:212-219` also mirrors hits. `save_research_doc` is a second, caller-less Tauri command that writes into `docs/src/architecture/`. Deep research also persists the **LLM synthesis** as a `research_synthesis` knowledge node, which the KG leg later injects. The bundle envelope is stamped `ContextTrustTier::Trusted` regardless of content (`retrieval.rs:148`). The real user DB on this machine already holds 40 unreviewed web rows in `search_documents` (26 raw `https:` + 14 `web-ingest:`) and 52 web/research `knowledge_nodes`. It holds **zero** legacy VoxKB rows (both DBs: 0/0/0, schema 94).

The decisive constraint is in the code: Turso/libSQL **rejects `CREATE TRIGGER` and SQL `CHECK`** (`scientia.rs:351-354`). "Rejected" therefore cannot be a DB constraint, and TRUST-01 must be met at the Rust type boundary: a required enum parameter with no `Default`, plus a `compile_fail` doctest. Three placement facts let everything ship with **zero new crate edges**:
- `vox-db` already depends on the L0 crate `vox-orchestrator-types`, so `ContextTrustTier` moves there.
- `vox-db-types` (L0) is reachable from `vox-search` and `vox-research-shim` via `vox_db::` re-exports, so the sanitizer and the `SanitizedText` newtype go there.
- Hashing, snapshots and redaction run in `vox-orchestrator-mcp`, which already has `vox-crypto`, `vox-redact` and `vox-bounded-fs`. `vox-search` and `vox-research-shim` reach it through an injected `WebFindingCapture` trait. That mirrors the existing `LexicalMemoryFallback` injection pattern.

**Primary recommendation:** Land the work in five plans (03-01 foundations: tier enum, sanitizer, trust verdict → 03-02 schema 96 and typed writers → 03-03 sink reroute and injection-boundary sanitization → 03-04 legacy retirement, migration and MENS → 03-05 docs and mutation-proof verification). Every security guard needs a paired positive and negative test, mutation-proven by hand.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Trust tier enum (`ContextTrustTier`) | L0 `vox-orchestrator-types` | re-exported by `vox-orchestrator`, `vox-db` | `vox-db` already depends on it (`socrates_telemetry.rs:17`). One enum, no edge |
| Sanitizer + `SanitizedText` + `ContentHash` newtypes | L0 `vox-db-types` | used by `vox-search`, `vox-research-shim`, `vox-orchestrator-mcp` via `vox_db::` | Pure functions over strings. `vox-db` re-exports `vox_db_types` (`vox-db/src/lib.rs:186`). Lets `vox-db` writers *require* sanitized input by type |
| `kb_findings` table + typed CRUD; `trust_tier` column on `search_documents` | L3 `vox-db` (DB / storage) | — | Schema SSOT; `migrate()` ALTER pattern |
| Capture pipeline (sanitize → redact → hash → snapshot → insert) | L3 `vox-orchestrator-mcp` (`kb_tools/findings.rs`) | — | Only L3 crate holding `vox-crypto` + `vox-redact` + `vox-bounded-fs` + `vox-db` + `vox-search` edges |
| Web-hit capture hook | L3 `vox-search` (trait `WebFindingCapture`) | impl in `vox-orchestrator-mcp` | `vox-search` can't hash (no `vox-crypto` edge). Inject the sink; if none is injected, persist nothing (fail-closed) |
| Deep-research gather capture | L3 `vox-research-shim` (`ResearchConfig.finding_capture`) | impl in mcp | Same trait. Replaces `ingest_research_document_async` in `web_gather.rs` |
| Promotion eligibility verdict (fail-closed) | L3 `vox-search::trust` | consumed in Phase 9 | Owner of `TrustScorer` |
| Legacy VoxKB count + migration | L3 `vox-orchestrator-mcp` | reader fns in `vox-db` | Needs `vox-crypto` for `content_hash`. `vox-db` has no crypto edge |
| GUI side doors (`ingest_url`/`ingest_text`/`publish_research_doc`/`save_research_session_to_kb`) | L5 `vox-gui` (thin Tauri cmds) | call mcp capture helper | `vox-gui → vox-orchestrator-mcp` edge exists |
| MENS `KbSignals` | L3 binary `vox-ml-cli` | reads `vox-db` | Already reads `vox-db` directly (`pipeline.rs:609`) |
| Injection-boundary sanitization | L3 `vox-orchestrator-mcp` (`run_retrieval_bundle`, `sources_from_hits`, Phase 2 `vox_kb_*` outputs) | — | The current chokepoints before `context_parts` |

## Standard Stack

### Core (all already in the workspace; no new dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vox-crypto::{secure_hash, hex_encode}` | workspace 0.6.0 | BLAKE3 content hash for findings | Repo crypto SSOT. `secure_hash` is BLAKE3 [VERIFIED: crates/vox-crypto/src/facades.rs:10-15] |
| `vox-redact::redact_owned` | workspace | Strip secrets/PII from finding summary/quotes | Already used by mcp `agent_loop.rs:1016` and `operation_capture.rs:44` [VERIFIED: grep + crates/vox-redact/src fn list] |
| `proptest` | 1.11.0 (workspace dep `version = "1"`) | Sanitizer property tests | Already used by `vox-orchestrator` and `vox-compiler` dev-deps [VERIFIED: Cargo.toml:397, Cargo.lock:10760-10761] |
| `wiremock` | workspace | Web/LLM stubs in mcp/search tests | Existing chat-turn harness (`message_turn_trace_tests.rs:17-18`) and dispatcher tests (`web_dispatcher.rs:1423`) [VERIFIED: codebase] |
| `turso` | workspace (0.6.x per research) | DB | Existing |

### Do NOT use
- `blake3`/`sha2`/`sha3` crates directly (crypto policy)
- `vox_crypto::hash_fast_hex` for the provenance hash: it is XXH3-128, a non-cryptographic cache hash [VERIFIED: facades.rs:29-43]. It is fine for IDs, wrong for tamper-evident provenance.
- `regex` in `vox-db-types`: not a dependency there (`serde`, `serde_json`, `chrono` only) [VERIFIED: crates/vox-db-types/Cargo.toml]. Write a hand-rolled char/state scanner; the sanitizer is small.
- `trybuild` in the PR gate: trybuild binaries are excluded from the `ci-gate` profile (nightly only) [VERIFIED: .config/nextest.toml:37-49]. Use `compile_fail` doctests (nightly `cargo test --workspace --doc`) [VERIFIED: nightly.yml:1578].

**Installation:** none. Add `proptest = { workspace = true }` under `[dev-dependencies]` of `vox-db-types`. (A dev-dependency on an external crate is not a workspace crate edge.)

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| proptest | crates.io | already locked 1.11.0 in `Cargo.lock` | — | github.com/proptest-rs/proptest | not re-run (no new install) | Approved: existing workspace dep, only newly referenced as a dev-dep |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
This phase installs no new external packages.

## Current-State Sink Inventory (every path that puts non-curated text where retrieval can see it)

This is the core of the phase. Each row was read from source this session.

| # | Sink | Writes to | Text | Callers / status | Phase 3 disposition |
|---|------|-----------|------|------------------|---------------------|
| S1 | `execution.rs:603-631` web leg, `policy.persist_web_hits` | `search_documents` `web-ingest:{engine}:{url}` via `persist_text_document_chunk` | web snippet, fire-and-forget `let _ =` | every `execute_search_plan` caller (MCP search, chat preamble, GUI in-process `vox_search_query`) | Replace with `ctx.web_capture` → `kb_findings` (Untrusted). No sink injected → persist nothing |
| S2 | `bundle.rs:206-221` Tavily CRAG | `search_documents` `tavily:{url}` | Tavily content | `run_search_with_verification` CRAG branch | Same as S1 |
| S3 | `web_gather.rs:350-390` deep-research gather → `VoxDb::ingest_research_document_async` (`research.rs:45-177`) | `knowledge_nodes` (`external_research`), `snippets` (`research_chunk`), **`search_documents` with `source_uri` = raw URL**, `embeddings` | full page `raw_content` | `run_deep` (chat), `vox_research_start`, GUI research | Replace the call with `config.finding_capture` (full raw text → snapshot). **Not in ARCHITECTURE's six-sink list** |
| S4 | `research_turn.rs:507-577` `persist_research_findings_to_knowledgebase` | `knowledge_nodes` `research_query:*` (content = **LLM synthesis** for deep), `research_src:*` | snippets + deep answer | `run_quick:486`, `run_deep:647`, GUI `save_research_session_to_kb` (`knowledge.rs:288`) | Rewrite as `capture_research_findings` → `kb_findings` |
| S5 | GUI `ingest_text` (`knowledge.rs:80-117`) | `knowledge_nodes` `doc:*` | user-pasted | MemoryView Ingest tab | → capture helper (origin `gui_ingest_text`, Untrusted) |
| S6 | GUI `ingest_url` (`knowledge.rs:169-227`) | `knowledge_nodes` `web:*` (full scraped markdown) | scraped page | MemoryView Ingest tab | → capture helper (origin `gui_ingest_url`, Untrusted, snapshot = markdown) |
| S7 | GUI `publish_research_doc` (`research.rs:228-275`) | **`docs/src/architecture/<slug>-2026.md` atomic write** + `store_research_artifact` | LLM report | DocPublishModal | → capture finding (origin `publish_research_doc`, snapshot = draft, `provenance.proposed_path`). Keep `store_research_artifact` (episodic log) |
| S7b | GUI `save_research_doc` (`research.rs:102-153`) | **writes `docs/src/architecture/*-research-2026.md` with `status: "current"`** | caller text | **zero UI callers** (only in generated `gui-surface-coverage.v1.json`) | **Delete** the command and its `main.rs:333` registration |
| S8 | `signal_chat::ingest_chat_turn`, spawned after every turn (`message.rs:1272-1284`) | `kb_entries`, `accepted=1` (`store.rs:146`; DDL default `accepted INTEGER NOT NULL DEFAULT 1`, `knowledge.rs:99`) | assistant output | live (no-op only when no KB exists, `signal_chat.rs:78-81`) | **Delete** (module + spawn) |
| S9 | `signal_research::ingest_research_result` | `kb_entries`, `confidence.max(0.95)` (`signal_research.rs:55`) | LLM synthesis | **zero callers** | **Delete** |
| S10 | `signal_scientia::ingest_scientia_finding` | `kb_entries`, `0.98` (`signal_scientia.rs:52`) | finding text | **zero callers** | **Delete** |
| S11 | `MemoryManager::sync_verified_research_findings` (`manager.rs:650-706`) | `MEMORY.md` + `knowledge_nodes` `research_finding` | research summary | **zero non-test callers** (only `tests/memory_research_sync_test.rs`) | Delete with its test, or reroute. Recommend **delete** (YAGNI; the MEMORY sink is dead) |
| S12 | `vox_kb_clip` / `vox_kb_add_entry` MCP tools | `kb_entries` (clip uses `1.0`, `handlers.rs:257`) | agent/user | tool catalog | **Delete** (RET-02) |
| S13 | `questioning_tools.rs:88` → `persist_questioning_research_artifact_dual_write` (`questioning_telemetry.rs:84-96`) | `search_documents` `vox://publication/{id}` | scientia publication artifact | live MCP tool | Keep in corpus, tier **Advisory** (agent-derived) |
| S14 | `pipeline_cache.rs:46-47` research cache | `knowledge_nodes` `research_cache` (serialized research result JSON) | LLM output | live | Leave the writer (it is a cache). Map `research_cache` → Untrusted at read (KG leg) |
| S15 | `vox db research ingest-url/-file` CLI (`db_research/ingest.rs:70,144`) | same as S3 | operator-supplied external research | explicit operator action | Keep the packet store (`knowledge_nodes`/`snippets`, read by `list_map.rs:15`), but **drop the `search_documents` mirror** from `ingest_research_document_async` (or pass `Untrusted`) |
| — | `daemon_extra.rs:95` `research.persist_claims`, `scientia_research_fts` | scientia claims/FTS | episodic research log | `/research search` only | **Keep separate** (episodic log, not KB retrieval). ARCHITECTURE agrees |

**Measured on this machine (read-only `sqlite3 -readonly`, 2026-10-03):**
- `~/Library/Application Support/vox/vox.db` (schema 94): `knowledge_bases`=0, `kb_entries`=0, `kb_routing_rules`=0. `search_documents` by prefix: `https:`=26, `web-ingest:`=14. `knowledge_nodes`: `research_source`=26, `external_research`=26, `research_cache`=1.
- `/Users/brbrainerd/dev/vox/.vox/store.db` (schema 94): all three VoxKB tables 0. `search_documents` and `knowledge_nodes` empty.
- This resolves the STATE.md todo "Count legacy `kb_entries`/`knowledge_bases` rows on real user DBs" **for this machine**. Other users' DBs still need the in-code count + migrate.

## Architecture Patterns

### System Architecture Diagram (after Phase 3)

```
 web providers ─┐                      GUI (L5)                        chat turn
 (SearXNG/DDG/  │            ingest_url / ingest_text /             (message.rs)
  Tavily)       │            publish_research_doc /                     │
                ▼            save_research_session_to_kb                │
   vox-search execute_search_plan ──┐        │                          │
   bundle.rs CRAG ──────────────────┤        │            run_quick / run_deep
   research-shim web_gather ────────┤        │                 │
                                    ▼        ▼                 ▼
                     WebFindingCapture (trait; None ⇒ persist nothing)
                                    │
                                    ▼
            vox-orchestrator-mcp kb_tools/findings.rs::capture()
              sanitize (vox-db-types) → redact (vox-redact) →
              blake3 hash (vox-crypto) → snapshot .vox/cache/kb-findings/<hash>.txt
                                    │
                                    ▼
            vox-db kb_findings  (status=pending, tier ∈ {Untrusted, Advisory})
                    │                                    │
                    │ never auto-injected                ▼
                    ▼                           vox-ml-cli KbSignals (reviewed rows only)
         (Phase 9 review/promote → PR → git)

 git docs (docs/src/**, library/) ─ Phase 2 indexer ─► search_documents(trust_tier=trusted)
                                                            │
 run_retrieval_bundle (vox-search only) ◄───────────────────┘
   hits carry tier; preamble drops Untrusted; every line sanitized ─► context_parts
 (KbStore / kb_lines / @kb KbStore dump / signal_* : deleted)
```

### Recommended Project Structure (delta)
```
crates/vox-orchestrator-types/src/trust.rs        # NEW: ContextTrustTier (moved), as_db_str/parse, min()
crates/vox-db-types/src/kb_text.rs                # NEW: sanitize(), SanitizedText, ContentHash, FindingTier, FindingOrigin
crates/vox-db/src/store/ops_memory/kb_findings.rs # NEW: insert_pending / list / unqueued_training / mark_queued / legacy readers
crates/vox-db/src/store/ops_memory/kb.rs          # DELETE except legacy readers (move into kb_findings.rs, deprecation-marked)
crates/vox-search/src/capture.rs                  # NEW: WebFindingCapture trait + CapturedWebHit
crates/vox-search/src/trust.rs                    # MODIFIED: TrustLookup + promotion_eligibility()
crates/vox-orchestrator-mcp/src/kb_tools/findings.rs   # NEW: capture(), McpFindingCapture impl, migrate_legacy_kb_entries()
crates/vox-orchestrator-mcp/src/kb_tools/{handlers,params,signal_*}.rs  # DELETE
crates/vox-orchestrator/src/knowledge_base/       # DELETE (KbStore, KbRouter, types)
contracts/kb/redteam/cases.v1.jsonl               # NEW: escaped-ASCII payloads + expected neutralization
```

### Pattern 1: Trust tier as a required, default-less enum (TRUST-01)
**What:** Move `ContextTrustTier` from `vox-orchestrator/src/context_envelope.rs:87-95` to `vox-orchestrator-types` and `pub use` it from the old path. Verbatim today:
```rust
// crates/vox-orchestrator/src/context_envelope.rs:87-95 [VERIFIED]
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
Add `Copy, Hash, PartialOrd, Ord`. Declaration order already gives ascending trust, so `min()` is the taint operator. Do **not** add `Default`. The serde wire strings (`untrusted`, `advisory`, `trusted`, `system_verified`) are unchanged, which keeps `contracts/communication/context-envelope.schema.json` valid. Every `search_documents` writer gains a required `tier: ContextTrustTier` parameter. `kb_findings` writes take a narrower `FindingTier { Untrusted, Advisory }` (findings can never be `Trusted`, by construction), with `From<FindingTier> for ContextTrustTier`.

**Why the type boundary and not SQL:** [VERIFIED: crates/vox-db/src/schema/domains/scientia.rs:351-354] "DB-level triggers are NOT used because Turso/libSQL rejects `CREATE TRIGGER` ("experimental feature; enable with --experimental-triggers"), the same constraint that rules out SQL CHECK here." Existing DBs also need `ALTER TABLE … ADD COLUMN`, and SQLite requires a DEFAULT for `NOT NULL` there. Use `DEFAULT 'untrusted'`, which makes any raw-SQL insert fail-*closed* (lowest tier), not "rejected".

**The four-way mapping (document it in `search-retrieval-ssot-2026.md` and encode it in one `fn`):**

| Tier | Meaning | Phase 3 rows |
|------|---------|--------------|
| SystemVerified | curated + mechanical verification evidence | none yet (Phase 11 sets the evidence). Define `tier_for_curated(has_mechanical_evidence: bool)` now |
| Trusted | curated: merged-to-git docs indexed from `docs/src/**` (excl. `archive/`) + `docs/src/library/` | Phase 2 library indexer rows |
| Advisory | agent-derived from non-Untrusted inputs | `vox://publication/*` (S13); KG `fact`/`concept` memory nodes |
| Untrusted | inbox, live web, or anything derived from Untrusted input (taint) | all `kb_findings` from web/GUI/research/legacy; legacy `web-ingest:`/`tavily:`/raw-`http(s)` rows; KG `external_research`, `web_scrape`, `web_research_source`, `research_synthesis`, `research_finding`, `research_cache`, `document`; **unknown → Untrusted** |

**Taint rule (recommend locking):** a finding derived by an LLM from Untrusted sources is Untrusted, not Advisory. So the deep-research synthesis is Untrusted. This reading resolves an apparent conflict in TRUST-01's wording ("Advisory = agent-derived; Untrusted = inbox").

### Pattern 2: Sanitizer as a smart constructor (TRUST-05)
**What:** `vox_db_types::kb_text::SanitizedText(String)` has a private field and only one constructor, `SanitizedText::new(raw: &str)`, which runs `sanitize`. `vox-db`'s `NewKbFinding { summary: SanitizedText, quotes: Vec<SanitizedText>, … }` makes unsanitized ingest a compile error.

**Reuse:** `vox-research-shim/src/research/orchestrator/helpers.rs:11-55` `sanitize_evidence` already strips some ranges and neutralizes ChatML/Llama/Claude/Gemma/DeepSeek role tokens. Verbatim ranges [VERIFIED: helpers.rs:17-25]: `'\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'`. It is `pub(super)` and lacks the tag block, U+2060–2064, variation selectors, HTML comments and images. Make the new function a **superset** (keep the token neutralization) and turn `sanitize_evidence` into a one-line delegate. That leaves one sanitizer.

**Order matters for idempotence:** (1) strip invisible codepoints → (2) strip HTML comments (an unterminated `<!--` strips to end-of-input, fail-closed) → (3) neutralize images (inline `![a](u)`, reference `![a][r]`, HTML `<img …>`) unless the target is a relative path or on an allowlist (default allowlist empty) → (4) neutralize role tokens. Removing a comment can *create* an image: `!<!-- x -->[a](https://evil)` → `![a](https://evil)`. So either loop to a fixpoint (bounded, e.g. ≤4 passes) or prove `sanitize(sanitize(s)) == sanitize(s)` with proptest. Do both.

**Ranges to strip:**
- U+E0000–E007F (tag block) [CITED: blogs.cisco.com unicode-tag-prompt-injection]
- U+200B–U+200F, U+2060–U+2064, U+FEFF (zero-width / invisible)
- U+202A–U+202E, U+2066–U+2069 (bidi)
- **U+FE00–FE0F and U+E0100–E01EF variation selectors**: a documented 2025 byte-smuggling channel [CITED: paulbutler.org/2025/smuggling-arbitrary-data-through-an-emoji/]. Not in REQUIREMENTS but the same class; recommend including.

Stripping the tag block breaks subdivision-flag emoji (e.g. England). That is acceptable for KB text.

### Pattern 3: Inject the capture sink; absence means no write (TRUST-02)
Follow the existing `LexicalMemoryFallback` trait-object injection (`execution.rs:21-24`, implemented by `McpMemoryFallback` in `retrieval.rs:256-270`):
```rust
// PROPOSED (vox-search/src/capture.rs) — names are recommendations, not existing API [ASSUMED]
pub trait WebFindingCapture: Send + Sync {
    /// Fire-and-forget; impl owns spawning, hashing, snapshotting, inserting.
    fn capture(&self, hits: Vec<CapturedWebHit>);
}
pub struct CapturedWebHit {
    pub url: String, pub title: String, pub snippet: String,
    pub full_text: Option<String>, pub engine: String, pub query: String,
    pub origin: vox_db::kb_text::FindingOrigin,
}
// SearchRuntimeContext gains: pub web_capture: Option<Arc<dyn WebFindingCapture>>
// ResearchConfig (vox-research-shim) gains: pub finding_capture: Option<Arc<dyn WebFindingCapture>>
```
`execution.rs:607` becomes `if policy.persist_web_hits && let Some(cap) = ctx.web_capture.as_ref()`. Keep the policy flag and `SecretId::VoxSearchPersistWebHitsDisabled` [VERIFIED: policy.rs:54-57 default `true`; 407-409; 568-574]. The flag's meaning changes from "mirror into corpus" to "capture into inbox". Update its doc comment (`policy.rs:201`) and `search-retrieval-ssot-2026.md:80`.

**Dedupe:** `dedupe_key = hex(blake3(origin || url || content_hash))` with `UNIQUE` and `ON CONFLICT DO NOTHING`. SQLite treats NULLs in a composite `UNIQUE` as distinct, so a nullable `source_url` must not be in the key directly. The preamble's web leg re-searches on every turn, so without dedupe the inbox floods.

**Capture once per deep turn:** `web_gather` captures sources with `full_text` (the raw page becomes the snapshot). `run_deep` captures only the synthesis finding (origin `research_deep`, sources listed in `provenance_json`). `run_quick` captures its `trace.sources` (snippet = snapshot).

### Pattern 4: Legacy migration runs where crypto is available (RET-03)
`vox-db` has no `vox-crypto` edge, and `VoxDb::migrate` has no injection point, so the row copy can't run inside `migrate()` without adding another direct-`blake3` violation. Recommended:
1. 03-02 moves the VoxKB DDL from `knowledge.rs:78-116` into `domains/quarantine.rs`, so fresh DBs no longer create it. Existing DBs keep their tables. **Do not** add them to `QUARANTINE_DROP_TABLES` (`migration.rs:139-188`) in this phase: `precheck_quarantine_tables_empty` would abort forever on any DB with rows.
2. `vox-db` keeps two deprecation-marked readers: `legacy_kb_counts()` (returns 0s when `sqlite_master` lacks the table, the same check as `codex_legacy.rs:31-37`) and `legacy_kb_entries_page()`.
3. `vox_orchestrator_mcp::kb_tools::findings::migrate_legacy_kb_entries(&db) -> LegacyKbReport { knowledge_bases, kb_entries, kb_routing_rules, migrated, skipped_empty }` counts first. It copies each non-empty `content` row as a pending Untrusted finding (origin `legacy_kb_entry`). `provenance_json` holds `kb_name`, `source_signal`, `source_ref`, `routing_confidence`, `accepted`, `mens_queued`, `legacy_id`. The dedupe key is derived from `legacy_id`, so the copy is idempotent. Call it once at `ServerState` init (every chat/MCP surface constructs one), and print the report in `vox kb status` (Phase 2 tool).

### Pattern 5: Fail-closed promotion verdict (TRUST-06)
`trust.rs` collapses errors to neutral (the module doc at `trust.rs:1-4` says "Fail-open: any network or parse error yields a neutral trust score"). `check_retraction` returns `None` on failure (`:68-73`), and `score_hit_trust` returns `1.0` on any failure (`:190-198`). Keep that for ranking. Add a separate tri-state:
```rust
// PROPOSED [ASSUMED names]
pub enum TrustLookup { Clean, Retracted, NotApplicable, Unavailable { reason: String } }
pub enum PromotionEligibility { Eligible, Ineligible { reason: String } }
pub fn promotion_eligibility(l: &TrustLookup) -> PromotionEligibility // Unavailable|Retracted ⇒ Ineligible
```
**Trap:** `score_hit_trust_for_url` skips the lookup for non-academic URLs (`trust.rs:231-236`). That is `NotApplicable`, **not** `Unavailable`. If the two are conflated, every non-academic web finding becomes unpromotable, which silently disables Phase 9. Phase 3 ships the pure function and its test; Phase 9 calls it.

### Anti-Patterns to Avoid
- **Tier as `Option<ContextTrustTier>` or with `#[serde(default)]`.** Either reintroduces the default the requirement forbids.
- **Hashing inside `vox-search`/`vox-db` with `blake3`.** It extends an existing policy violation. Inject the work instead.
- **Stamping the whole retrieval envelope `Trusted`** (`retrieval.rs:147-148` today). Derive it as the `min` of contributing hit tiers.
- **"Approved" meaning injectable.** In the new identity model `status=approved` means "approved for drafting a PR". Only merged git is curated.
- **Testing the chat writeback removal on a fresh DB alone.** Without `kb_entries`, a resurrected writer fails silently (`let _ =`) and the test still passes. Seed the legacy table (see Validation).

## kb_findings schema (recommended; baseline 96)

```sql
-- PROPOSED [ASSUMED column names]; place in schema/domains/knowledge.rs
CREATE TABLE IF NOT EXISTS kb_findings (
    id               TEXT PRIMARY KEY,               -- uuid v4
    dedupe_key       TEXT NOT NULL UNIQUE,           -- blake3(origin|url|content_hash)
    status           TEXT NOT NULL DEFAULT 'pending',-- pending|approved|rejected|promoted|stale (validated in Rust)
    trust_tier       TEXT NOT NULL DEFAULT 'untrusted', -- only untrusted|advisory (FindingTier)
    origin           TEXT NOT NULL,                  -- FindingOrigin snake_case
    title            TEXT NOT NULL DEFAULT '',
    summary          TEXT NOT NULL,                  -- sanitized + redacted
    quotes_json      TEXT NOT NULL DEFAULT '[]',     -- ≤ N short sanitized excerpts
    source_url       TEXT,
    fetched_at_ms    INTEGER NOT NULL,
    content_hash     TEXT NOT NULL,                  -- 'blake3:' || 64 hex of raw text
    snapshot_relpath TEXT,                           -- .vox/cache/kb-findings/<hex>.txt
    provenance_json  TEXT NOT NULL DEFAULT '{}',     -- query, engine, session/turn, model_id, legacy fields
    review_note      TEXT, reviewed_by TEXT, reviewed_at_ms INTEGER,
    mens_queued      INTEGER NOT NULL DEFAULT 0,
    created_at_ms    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_kb_findings_status ON kb_findings(status, created_at_ms);
-- plus: ALTER TABLE search_documents ADD COLUMN trust_tier TEXT NOT NULL DEFAULT 'untrusted' (PRAGMA-guarded)
-- plus backfill: UPDATE … SET trust_tier='advisory' WHERE source_uri LIKE 'vox://publication/%'; library rows rewritten trusted by reindex
```
- **Snapshot:** `.vox/cache/kb-findings/<hex>.txt`. `.vox/cache` is the existing const [VERIFIED: crates/vox-config/src/paths.rs:524 `pub const REPO_CACHE_DIR: &str = ".vox/cache";`] and is gitignored [VERIFIED: .gitignore:109 `.vox/cache/`, :111 `**/.vox/cache/`]. **No `.gitignore` edit is needed.** Add `.vox/cache/kb-findings/` and `contracts/kb/redteam/` to `.voxignore` (it has no `.vox/` entry today [VERIFIED: .voxignore grep]), then run `vox ci sync-ignore-files`. The filename comes from the hash only, so there is no URL-derived path and no traversal. Write atomically and size-cap the file.
- **Quote caps:** `MAX_QUOTES = 3`, `MAX_QUOTE_CHARS = 300` [ASSUMED values; copyright posture is a Phase 9 legal question]. In Phase 3 the summary is the sanitized snippet. LLM summarization is GROW-02 (Phase 9).
- **Version sequencing:** today `pub const BASELINE_VERSION: i64 = 94;` [VERIFIED: crates/vox-db/src/schema/manifest.rs:35]. Phase 2 is expected to take 95 [ASSUMED: Phase 2 not yet planned], so Phase 3 takes 96. **Columns must ship with a version bump.** `ensure_latency_columns` (`open.rs:220-252`) exists precisely because columns added without a bump never reached DBs already at the baseline. Update `contracts/db/baseline-version-policy.yaml` `repository_baseline_integer` and `repository_baseline_digest_hex`, and add a manifest.rs comment line. If another branch takes 96 first, take the next free integer at merge time.

## Retirement Inventory (RET-02) — every touchpoint

| Surface | Location | Action |
|---------|----------|--------|
| `KbStore`, `KbRouter`, `KnowledgeBase`/`KbEntry`/`KbEntrySource`/routing types | `crates/vox-orchestrator/src/knowledge_base/{mod,store,router,types}.rs`; re-export `vox-orchestrator/src/lib.rs:147-148` | delete |
| 12 handlers + params | `kb_tools/handlers.rs` (`kb_create`…`kb_clip`, lines 22-269), `kb_tools/params.rs` | delete (keep the Phase 2 `kb_tools/` files) |
| signal adapters | `kb_tools/signal_{chat,research,scientia}.rs`; `kb_tools/mod.rs:4-6` | delete |
| dispatch arms | `dispatch.rs:1476-1497` | delete |
| input schemas | `input_schemas.rs:306-321` | delete |
| catalog ops | `contracts/operations/catalog.v1.yaml:26687-26926`: ids `kb.create`, `kb.list`, `kb.delete`, `kb.add_entry`, `kb.delete_entry`, `kb.list_entries`, `kb.review_entry`, `kb.get_feed`, `kb.add_rule`, `kb.list_rules`, `kb.query`, `kb.clip` → MCP names `vox_kb_create` … `vox_kb_clip` | delete, then `vox ci operations-sync --target mcp --write` and `--target capability --write` (regenerates `tool-registry.canonical.yaml`, `capability-registry.yaml`, `model-manifest.generated.json`) |
| read-role governance | `contracts/mcp/http-read-role-governance.yaml:16-20` (`vox_kb_get_feed`, `vox_kb_list`, `vox_kb_list_entries`, `vox_kb_list_rules`, `vox_kb_query`) | delete the 5 lines (hand-maintained; the check compares against the catalog) |
| `kb_lines` | `retrieval.rs:229-230` field, `:359-372` KbStore query, `:412`; `message.rs:823-833` | delete |
| `@kb` KbStore dump | `message.rs:869-895` | delete the consumer. **Keep** `parse_kb_mentions` (`retrieval.rs:234-254`, tested at `:446-468`) for the Phase 7 remap |
| post-turn writeback | `message.rs:1272-1284` | delete |
| VoxDb VoxKB ops | `vox-db/src/store/ops_memory/kb.rs` (all `kb_*` fns), row types `KbRow`/`KbEntryRow`/`KbRuleRow` in `vox-db/src/lib.rs` | delete, except the deprecation-marked legacy readers |
| VoxKB DDL | `schema/domains/knowledge.rs:78-116` | move to `domains/quarantine.rs` (feature-gated off) |
| MENS stage | `vox-ml-cli/src/commands/mens/pipeline.rs:592-688` reads `kb_unqueued_training_entries` | retarget to `kb_findings` (see below) |
| comment | `scope_guard.rs:13` mentions `vox_kb_*` writes | update |
| retired-surface registry | AGENTS.md "Retired Surfaces" table + `contracts/retirement/retired-surfaces.v1.yaml` (+ optionally `contracts/documentation/retired-symbols.v1.yaml`) | add rows: `KbStore`/`KbRouter`/legacy `vox_kb_*` CRUD → `vox_kb_search`/`kb_findings`. `enforcement.kind: documentation-only`, or `cli-check` if an absence check is added |
| CHANGELOG | `## [Unreleased]` | note: `vox_kb_query`/`vox_kb_clip` removed, not aliased; external MCP callers break |

No GUI caller uses any legacy `vox_kb_*` tool [VERIFIED: grep over `crates/vox-gui/ui/src` and `e2e`, zero hits].

**"Colliding names replaced, not aliased":** Phase 2's new tools (`vox_kb_search/get/cite/status/reindex`) don't share literal names with the 12. The collision is the shared `vox_kb_*` namespace and overlapping semantics (`query`≈`search`, `clip`≈Phase 9 `propose`). The rule means no alias map and no shim entry. **Cross-phase coordination:** Phase 2's catalog ids must not reuse `kb.query`/`kb.clip`.

## MENS `KbSignals` retarget (GROW-06)
Today the stage turns **every** unqueued `kb_entries` row with `accepted=1` into an SFT record (`pipeline.rs:633-651`), which means auto-accepted assistant output trains MENS. **Recommendation:** read `kb_findings WHERE mens_queued = 0 AND reviewed_by IS NOT NULL AND status IN ('approved','rejected')`. Emit **DPO pairs only** (approved vs rejected sharing a `provenance.query`) and **no SFT** from findings, because finding summaries are web or LLM text (Pitfall 3; TAX-06 forbids training on web-promoted text without attestation). Phase 3 itself produces no reviewed rows, so the stage emits nothing until Phase 9. That is the safe default. Keep the stage name `kb_signals` (`action_prelude.rs:167`). [Flagged under Open Questions.]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Content hash | ad-hoc FNV/xxh3 (`daemon_extra.rs:112-116` does FNV inline) | `vox_crypto::secure_hash` + `hex_encode` | Policy and tamper-evidence |
| Secret scrubbing in findings | new regexes | `vox_redact::redact_owned` | Already the mcp standard |
| Quarantine bookkeeping | a new drop or table mechanism | `domains/quarantine.rs` + `quarantine` feature; `precheck_quarantine_tables_empty` | Existing, tested (`migration.rs:190-294`) |
| Column add on existing DBs | blind `ALTER` | the `PRAGMA table_info` guard in `migrate()` (`open.rs:137-203`) | Fresh DBs error with "duplicate column" otherwise |
| Second sanitizer | a parallel function | extend into a `vox-db-types` superset; shim delegates | One SSOT |
| Web/LLM test doubles | custom fakes | `wiremock` + `ServerState::new_test()` + the `message_turn_trace_tests.rs` harness | Existing |
| Compile-fail evidence | trybuild in the PR gate | a `compile_fail` doctest (precedent: `vox-scientia/src/review/mod.rs:79-81` `ApprovalToken`) | trybuild is nightly-only here |

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | User DBs: legacy `kb_entries`/`knowledge_bases`/`kb_routing_rules` (0 rows on both local DBs; unknown on other machines). `search_documents` legacy web rows: 14 `web-ingest:` + 26 raw `https:` on the user DB. `knowledge_nodes` web/LLM nodes: 26 `external_research`, 1 `research_cache`, plus research/GUI node types elsewhere. `embeddings` rows written for S3 chunks (model `"research_ingest"`) | **Data migration:** (a) legacy VoxKB rows → pending findings (Pattern 4); (b) ALTER-backfill `search_documents.trust_tier` (default `untrusted`; `vox://publication/%` → `advisory`). **Code edit:** preamble drops Untrusted chunk/KG hits; KG tier derived from `node_type`. Do not delete rows (an optional purge stays an open question) |
| Live service config | MCP hosts (Claude Code, Cursor) may have the 12 legacy tool names in allowlists, permission rules or prompts. These live outside git | CHANGELOG + release note. Nothing to migrate in-repo |
| OS-registered state | None: verified by grep; no launchd/systemd/task registration references KB names | none |
| Secrets/env vars | `VOX_SEARCH_PERSIST_WEB_HITS_DISABLED` (`SecretId::VoxSearchPersistWebHitsDisabled`, `vox-secrets/src/spec/ids.rs:394`, registry `missing.rs:2033`, `contracts/secrets/*.json`) | Keep the name; update the description text in the registry entry and the SSOT doc (meaning changes to "inbox capture"). Run `vox ci secrets-parity` |
| Build artifacts / generated contracts | `tool-registry.canonical.yaml`, `capability-registry.yaml`, `model-manifest.generated.json`, `gui-surface-coverage.v1.json` (lists `save_research_doc`) | Regenerate via `operations-sync`/`capability-sync`; the `ssot-autoregen` bot covers PR drift. Fix the inputs, not the outputs |
| Already-trained MENS data | `kb_entries` rows with `mens_queued=1` were already exported as SFT | Not reversible. Record in CHANGELOG/docs (0 rows here) |

## Common Pitfalls

### Pitfall 1: A DB constraint is assumed to reject tier-less writes
**What goes wrong:** the plan says "CHECK(trust_tier IN …)" or adds a trigger, and it fails or does nothing on Turso.
**Why:** Turso rejects `CREATE TRIGGER` and CHECK (`scientia.rs:351-354`).
**How to avoid:** type-level enum param; `DEFAULT 'untrusted'` (fail-closed) for the ALTER on existing DBs; a `compile_fail` doctest plus a unit test that a tier-less raw-SQL row reads back as `Untrusted`.

### Pitfall 2: The writeback-removal test passes vacuously
**What goes wrong:** with `kb_entries` quarantined, a resurrected writer errors into `let _ =`, and "no rows" looks green.
**How to avoid:** seed the legacy tables (KB + matching keyword rule) in the test DB, run a plain chat turn through the wiremock harness, and assert the `kb_entries` count is unchanged **and** `kb_findings` has no `chat_turn` origin. Mutation: re-insert a raw `INSERT INTO kb_entries` at the old spawn site and watch the test go red.

### Pitfall 3: Sanitizer non-idempotence through construction
**What goes wrong:** removing one construct creates another (`!<!-- -->[a](u)` becomes an image; `<\u{200B}!--` becomes a comment).
**How to avoid:** use the fixed pass order plus a fixpoint loop, and proptest `sanitize(sanitize(s)) == sanitize(s)` and `no banned codepoint ∈ sanitize(s)` over arbitrary strings, including generators biased toward `<!--`, `![`, `](`, `-->` and banned codepoints.

### Pitfall 4: NotApplicable conflated with Unavailable (TRUST-06)
**What goes wrong:** fail-closed on "no lookup performed" disables all promotion of non-academic sources.
**How to avoid:** the explicit `TrustLookup::NotApplicable` variant and a test table covering all four variants.

### Pitfall 5: Double capture / inbox flood
**What goes wrong:** the preamble web leg plus `run_quick` plus `web_gather` capture the same URL repeatedly.
**How to avoid:** a `dedupe_key` UNIQUE with ON CONFLICT DO NOTHING; one capture point per path (Pattern 3).

### Pitfall 6: Retrieval still surfaces legacy untrusted rows as context
**What goes wrong:** new rows stop, but 40 existing web rows keep flowing into the preamble through the DocumentChunks and KG legs.
**How to avoid:** backfill `untrusted`; the `AutoChatPreamble` trigger drops Untrusted hits (explicit tool queries may return them, labelled); test "a `web-ingest:` row is never returned with tier Trusted".

### Pitfall 7: Parallel-plan merge damage in `message.rs` (2533 lines)
**What goes wrong:** two plans edit adjacent hunks (lines 823-895 and 1272-1284); the move-plus-edit merge class described in AGENTS applies.
**How to avoid:** put **all** `message.rs` edits in one plan (03-04), and run `grep -c` for duplicated fns after merges.

### Pitfall 8: GUI build prerequisites in a fresh worktree
**What goes wrong:** `cargo test -p vox-gui` fails in `tauri-build` (missing sidecar and `ui/dist`).
**How to avoid:** keep the GUI commands thin and put the logic and tests in `vox-orchestrator-mcp::kb_tools::findings`. Use vitest for the UI copy change. Run `vox run scripts/gui-build.vox` only if a `vox-gui` Rust test is required.

### Pitfall 9: The red-team fixtures themselves inject agents
**What goes wrong:** IDE agents or hooks read raw payload files. This session's PostToolUse injection scan fired on `helpers.rs` test strings.
**How to avoid:** store payloads as **ASCII-escaped JSON** (`󠁁` surrogate pairs for tag chars) in `contracts/kb/redteam/cases.v1.jsonl`, so no raw invisible codepoints sit in git. Add the directory to `.voxignore`. Keep it outside the `docs/src` indexer roots, and add its canary to Phase 1's canary check so it never reaches the index or the MENS corpus. (`vox-corpus` reads specific `contracts/` files only [VERIFIED: grep]. It does not read a `contracts/kb/` path.)

## Code Examples

### Seeded legacy-DB migration test shape (follows the existing pattern)
```rust
// Pattern source: crates/vox-db/src/local_tests.rs:886-942 (migrate_adds_resource_id_to_a_pre_existing_hopper_inbox)
// PROPOSED test in vox-orchestrator-mcp (needs vox-crypto for hashes) [ASSUMED names]
#[tokio::test]
async fn legacy_kb_entries_are_counted_then_copied_as_pending_findings() {
    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory).await.unwrap();
    db.connection().execute_batch(LEGACY_VOXKB_DDL /* copied verbatim from quarantine.rs */).await.unwrap();
    // seed: 1 KB, 2 entries (one empty content), 1 rule
    let report = migrate_legacy_kb_entries(&db).await.unwrap();
    assert_eq!((report.knowledge_bases, report.kb_entries, report.kb_routing_rules), (1, 2, 1));
    assert_eq!((report.migrated, report.skipped_empty), (1, 1));
    let rows = db.kb_findings_list(Some("pending"), 10).await.unwrap();
    assert!(rows.iter().all(|r| r.trust_tier == ContextTrustTier::Untrusted));
    // idempotent
    assert_eq!(migrate_legacy_kb_entries(&db).await.unwrap().migrated, 0);
}
```

### compile_fail evidence for TRUST-01 (precedent: vox-scientia ApprovalToken)
```rust
/// ```compile_fail
/// // A search_documents write without a tier does not compile.
/// # async fn f(db: &vox_db::VoxDb) { db.upsert_search_document("u", "t", "text/markdown", "h").await; }
/// ```
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Strip zero-width + bidi only | Also the tag block and variation selectors (U+FE00–FE0F, U+E0100–E01EF) | Emoji/variation-selector smuggling, Feb 2025 | Add both ranges |
| Trust as a numeric confidence (0.95/0.98 floors) | Categorical tiers + provenance; taint = min of inputs | Practices SSOT 2026-10-03 | Findings carry no confidence column |
| Mutable DB "knowledge bases" | Git-canonical index + reviewed inbox | This milestone | `approved` ≠ curated |

## Assumption-Delta: the KB identity model changes

- **Before:** the KB is a set of user-named DB buckets (`knowledge_bases`). Entries become "accepted" (`accepted=1` by default) and are immediately retrievable (`kb_lines`), @-mentionable by bucket name, and SFT-exported.
- **After:** the only curated KB is merged git (Trusted/SystemVerified). Everything else is a pending, Untrusted/Advisory **finding** that is never auto-injected. A finding becomes knowledge only via a reviewed PR (Phase 9).
- **Promote-vs-add-alongside implication:** findings never "upgrade in place". There is no DB transition from finding to curated. Promotion should **add alongside**: a new `docs/src/library/` page, or an appended, sourced section, rather than rewriting human text. That matches "accumulate, don't replace" (practices principle 4). Phase 3 must therefore not give `kb_findings` any status that retrieval treats as curated, and must keep `status`, `trust_tier` and "injectable" as separate concepts.
- **User-visible regressions to communicate:** (1) user-named KBs and `@kb-name` bucket mentions stop working until Phase 7 remaps `@kb` onto Library scope. (2) GUI "Ingest URL/Text" and "Publish research doc" no longer make content retrievable or write `docs/src`; they file pending findings that no GUI shows until Phase 12. Only toasts and `vox kb status` counts are visible in between. (3) Legacy `accepted=1` entries are downgraded to pending.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Phase 2 takes schema 95; Phase 3 takes 96 | kb_findings schema | Version collision; resolve at merge time |
| A2 | Phase 2 provides `vox kb status`, the rewritten `kb_tools/` and the library writer that Phase 3 adds `tier` to | multiple | Phase 3 must create stubs, or Phase 2 should take the `tier` param from the start (recommended to the Phase 2 planner) |
| A3 | Proposed type and function names (`WebFindingCapture`, `SanitizedText`, `FindingTier`, `TrustLookup`, `migrate_legacy_kb_entries`, column names) | Patterns | None functional; naming only |
| A4 | Quote caps 3 × 300 chars | schema | Copyright posture (Phase 9 legal decision) |
| A5 | Taint rule: an LLM synthesis of web input is Untrusted, not Advisory | Pattern 1 | If the user wants Advisory, two finding origins change tier |
| A6 | Dropping Untrusted hits from the auto preamble now (before Phase 7) is acceptable | Pitfall 6 | Slightly less autonomous web context in chat until Phase 7 spotlighting |
| A7 | `ServerState` init is a safe place for the one-shot legacy migration | Pattern 4 | Startup latency is negligible at 0 rows. A failure must log, not abort startup |
| A8 | Variation-selector stripping is in scope although REQUIREMENTS doesn't list it | Pattern 2 | Low; same attack class |
| A9 | Chat GUI renders no remote images today (grep found no `<img`/markdown-image rendering in `surfaces/Chat`) | Security | If wrong, a live exfil channel exists before Phase 6/8 |

## Open Questions for discussion (batch with the Phase 1–2 discussion)

1. **Findings tier for agent-derived content.** Should all pending findings be Untrusted, with Advisory reserved for agent output grounded only in curated context (recommended, taint rule)? Or is any agent-derived finding Advisory per the literal TRUST-01 text?
2. **Legacy web rows already in `search_documents`/`knowledge_nodes` (40 + 53 on this machine).** Recommended: tier them Untrusted and exclude them from the auto preamble; keep them for explicit queries. Alternatives: purge them (irreversible) or convert them to findings (floods the inbox).
3. **`persist_web_hits` default.** Recommended: keep it on, now meaning "capture into inbox" with dedupe. The alternative is default-off until the Phase 9 review tools exist.
4. **MENS `KbSignals` from findings.** Recommended: DPO only, from human-reviewed rows, no SFT. Alternative: SFT on approved findings (conflicts with Pitfall 3 / TAX-06).
5. **`@kb` between Phase 3 and Phase 7.** Recommended: remove the consumer and keep the parser, so `@name` is temporarily inert. Alternative: route `@name` into a Phase 2 `vox_kb_search` path filter early.
6. **Snapshot location with two DBs** (repo `.vox/store.db` vs user `~/Library/Application Support/vox/vox.db`). Recommended: content-addressed under the repo `.vox/cache/kb-findings/`, with `snapshot_relpath` relative to the repo root.
7. **The `vox-search → vox-crypto` edge proposal (user-authorized).** It would replace the trait-injected hashing and fix the existing direct-`blake3` uses in `ingest.rs`/`bundle.rs`. The recommendation is not to take it in Phase 3; the injection works without it.
8. **`where-things-live.md` rows.** AGENTS requires adding rows in the same PR, while DOC-02 (Phase 11) owns the full KB entry. Recommended: Phase 3 adds minimal rows for `kb_findings`, the sanitizer and the trust tier, and Phase 11 completes them.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo (broker shim) | all Rust | ✓ | 1.98.1 | — |
| cargo-nextest | tests | ✓ | 0.9.143 | `cargo test -p` |
| vox CLI | operations-sync, sync-ignore-files, doc lint | ✓ | 0.6.0+build.6474 | `scripts/vox-dev.sh` |
| node / pnpm | GUI vitest/typecheck | ✓ | v26.10.0 / pnpm present | — |
| sqlite3 | read-only legacy counts | ✓ | /usr/bin/sqlite3 | — |
| cargo-mutants | — | ✗ | — | Manual mutation (edit guard → test red → restore), per AGENTS |
| vox-gui sidecar + `ui/dist` in this worktree | `cargo test -p vox-gui` | ✗ (fresh worktree) | — | Keep logic in mcp; `vox run scripts/gui-build.vox` if needed |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest 0.9.143 (Rust); proptest 1.11.0; wiremock; vitest (GUI) |
| Config file | `.config/nextest.toml` (PR gate profile `ci-gate`) |
| Quick run command | `cargo nextest run -p <crate> <filter>` |
| Full suite command | `cargo nextest run -p vox-orchestrator-types -p vox-db-types -p vox-db -p vox-search -p vox-research-shim -p vox-orchestrator-mcp -p vox-ml-cli` + `pnpm --dir crates/vox-gui/ui test` + `pnpm --dir crates/vox-gui/ui typecheck` |

### Phase Requirements → Test Map
| Req / SC | Behavior | Type | Automated Command | File Exists? |
|----------|----------|------|-------------------|-------------|
| TRUST-01 / SC1 | Tier enum has no `Default`; writers require the tier; `compile_fail` doctest | doctest (nightly) + `cargo check` | `cargo test -p vox-db --doc` | ❌ Wave 0 |
| TRUST-01 / SC1 | Exhaustive mapping fn; db-str round-trip; unknown string → Untrusted | unit | `cargo nextest run -p vox-orchestrator-types trust` | ❌ |
| TRUST-01 | ALTER-added column backfills `untrusted`; publication → `advisory`; migrate idempotent | unit (local_tests pattern) | `cargo nextest run -p vox-db migrate_adds_trust_tier` | ❌ |
| TRUST-01 | A `web-ingest:` row is never returned as Trusted; envelope tier = min of hits | unit | `cargo nextest run -p vox-orchestrator-mcp retrieval` | ❌ |
| TRUST-06 / SC1 | `promotion_eligibility`: Unavailable/Retracted → Ineligible; NotApplicable/Clean → Eligible | unit (table) | `cargo nextest run -p vox-search trust::` | ❌ |
| TRUST-02 / SC2 | Quick + deep research turn (wiremock SearXNG + model): corpus row counts (`search_documents`, `search_document_chunks`, `knowledge_nodes`, `snippets`, `embeddings`) unchanged; `kb_findings` gained Untrusted pending rows with summary/url/fetched_at/content_hash; snapshot file exists | integration | `cargo nextest run -p vox-orchestrator-mcp research_turn_writes_only_findings` | ❌ |
| TRUST-02 | No capture sink injected → zero writes anywhere (fail-closed) | unit | `cargo nextest run -p vox-search capture` | ❌ |
| TRUST-04 / SC2 | Snapshot path is under `.vox/cache/kb-findings/`, which the repo `.gitignore` ignores | unit (path) + one `git check-ignore` assertion in a temp repo, or a string check against `.gitignore` | `cargo nextest run -p vox-orchestrator-mcp snapshot_path_is_gitignored` | ❌ |
| TRUST-03 / SC3 | Seeded legacy KB + plain chat turn → `kb_entries` count unchanged, no `chat_turn` finding | integration | `cargo nextest run -p vox-orchestrator-mcp chat_turn_writes_no_kb_rows` | ❌ |
| TRUST-03 / SC3 | Deep synthesis finding is Untrusted/pending; no confidence column exists | integration | same file | ❌ |
| GROW-06 / SC3 | Capture helper used by GUI paths: `ingest_url`/`ingest_text`/`publish_research_doc`/`save_research_session_to_kb` → pending findings, no `docs/src` write (test the helper with origin variants; assert `publish` writes no file under a temp repo root) | unit | `cargo nextest run -p vox-orchestrator-mcp findings::` | ❌ |
| GROW-06 | GUI toast copy reflects "pending review" | vitest | `pnpm --dir crates/vox-gui/ui test -- MemoryView DocPublishModal` | ✅ (update) |
| GROW-06 / SC3 | MENS `KbSignals` reads `kb_findings`; unreviewed rows emit nothing; reviewed pair → DPO | unit | `cargo nextest run -p vox-ml-cli kb_signals` | ❌ |
| TRUST-05 / SC4 | proptest: no banned codepoint in output; idempotent; output has no `<!--`, no off-allowlist `![`/`<img` | property | `cargo nextest run -p vox-db-types kb_text` | ❌ |
| TRUST-05 / SC4 | Red-team set: every case's expected neutralization holds at ingest (capture helper) **and** at injection (`run_retrieval_bundle` lines, `sources_from_hits`, `vox_kb_get` output) | fixture-driven unit | `cargo nextest run -p vox-orchestrator-mcp redteam` | ❌ |
| RET-02 / SC5 | Catalog/registry free of the 12 names; operations-sync check is clean | CLI check | `vox ci operations-sync --target mcp` (check mode) + `vox ci ssot-drift` | ✅ (existing gate) |
| RET-02 / SC5 | Source absence: no `KbStore`, `KbRouter`, `kb_lines`, `signal_chat`/`signal_research`/`signal_scientia`, legacy `vox_kb_*` names in `crates/` or `contracts/` (excluding retirement registries) | grep check | `rg -n "KbStore|KbRouter|kb_lines|signal_(chat|research|scientia)|vox_kb_(create|list|delete|add_entry|delete_entry|list_entries|review_entry|get_feed|add_rule|list_rules|query|clip)\b" crates contracts -g '!contracts/retirement/**'` → no output | n/a |
| RET-03 / SC5 | Seeded-DB migration: counts reported before copy; non-empty rows → pending; empty skipped; idempotent; absent tables → zeros | integration | `cargo nextest run -p vox-orchestrator-mcp legacy_kb_entries` | ❌ |

### Mutation-proof steps (each security guard; check presence before and after the edit)
| Guard | Mutation | Expected red test |
|-------|----------|-------------------|
| Required tier | add `impl Default for ContextTrustTier`, or make the writer param `Option<_>` | `compile_fail` doctest compiles → doctest fails (nightly). Locally: `cargo test -p vox-db --doc` |
| Fail-closed eligibility | change the `Unavailable` arm to `Eligible` | `trust::promotion_eligibility_fails_closed` |
| NotApplicable ≠ Unavailable | map `NotApplicable` → `Ineligible` | the same table test (non-academic case) |
| Capture-sink absence = no write | re-add `persist_text_document_chunk` in `execution.rs` | `capture::no_sink_no_writes` (row-count assertion) |
| Research turn writes only findings | restore the `upsert_knowledge_node` call in quick research | `research_turn_writes_only_findings` |
| Chat writeback removed | reinsert a raw `INSERT INTO kb_entries` at the old spawn site | `chat_turn_writes_no_kb_rows` (seeded table) |
| Sanitizer tag-block arm | delete the `'\u{E0000}'..='\u{E007F}'` arm | proptest + red-team tag case |
| Sanitizer idempotence | remove the fixpoint loop | proptest idempotence (a seed with `!<!-- -->[`) |
| Preamble drops Untrusted | remove the filter | `web_ingest_row_never_trusted` |
| Snapshot gitignored | change the dir to `docs/src/…` | `snapshot_path_is_gitignored` |

### Sampling Rate
- **Per task commit:** `cargo nextest run -p <touched crate> <filter>`
- **Per wave merge:** the full-suite command above, plus `cargo check -p vox-gui` (if the sidecar exists) and `vox ci operations-sync --target mcp` (check mode)
- **Phase gate:** full suite green; mutation table executed and logged in the SUMMARY; `cargo test -p vox-db --doc`

### Wave 0 Gaps
- [ ] `crates/vox-db-types/Cargo.toml`: `proptest = { workspace = true }` dev-dep
- [ ] `contracts/kb/redteam/cases.v1.jsonl` (+ canary added to Phase 1's canary list; `.voxignore` entry)
- [ ] Shared test helper: seeded legacy VoxKB DDL constant (copied from the quarantine DDL) for the mcp tests
- [ ] wiremock fixtures for SearXNG + model in the mcp research-turn test (reuse the `message_turn_trace_tests.rs` helpers)

### Recommended plan / wave split
| Wave | Plan | Contents | Depends |
|------|------|----------|---------|
| 1 | **03-01 Foundations** | Move `ContextTrustTier` (+derives, db-str, `min`, mapping fn); `vox-db-types::kb_text` (sanitizer, `SanitizedText`, `ContentHash`, `FindingTier`, `FindingOrigin`) + proptest; red-team fixtures; shim `sanitize_evidence` delegates; `trust.rs` `TrustLookup` + `promotion_eligibility` | Phase 2 merged |
| 2 | **03-02 Schema 96 + typed writers** | `kb_findings` DDL + ops; `search_documents.trust_tier` ALTER + backfill; tier param on every `search_documents` writer (`ingest.rs`, `research.rs`, `questioning_telemetry.rs`, Phase 2 library writer); VoxKB DDL → quarantine; legacy readers; baseline policy + digest | 03-01 |
| 3 | **03-03 Capture + reroute (no `message.rs`)** | `WebFindingCapture` trait in `SearchRuntimeContext`/`ResearchConfig`; mcp `findings::capture` (sanitize/redact/hash/snapshot/insert); reroute S1–S7, S15; delete `save_research_doc`; GUI cmds call the helper; UI toast copy + vitest; injection-boundary sanitize in `run_retrieval_bundle`, `sources_from_hits`, `vox_kb_*` outputs; hit tier propagation; preamble drops Untrusted | 03-02 |
| 4 | **03-04 Retirement + migration + MENS** | All `message.rs` edits (`kb_lines`, `@kb` consumer, writeback spawn); delete `KbStore`/`KbRouter`/types, the 12 handlers/params/dispatch/schemas/catalog/governance, signal adapters, the S11 MEMORY sync; `migrate_legacy_kb_entries` at `ServerState` init + `vox kb status` line; MENS `KbSignals` → `kb_findings`; operations-sync regen; AGENTS + `retired-surfaces.v1.yaml` rows; CHANGELOG | 03-02 (and 03-03, to avoid shared-file churn) |
| 5 | **03-05 Docs + verification** | Four-way mapping table and the `persist_web_hits` semantics in `search-retrieval-ssot-2026.md`; `where-things-live.md` rows; execute and log the mutation table; doc-lint the touched pages | 03-03, 03-04 |

03-01's trust-verdict part and its red-team fixtures can run in parallel with the tier/sanitizer work inside Wave 1. 03-03 and 03-04 are serialized because of shared mcp files (`retrieval.rs`, `research_turn.rs`, `message.rs`).

## Security Domain

### Applicable ASVS Categories (L1)
| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V1 Architecture (trust boundaries) | yes | Explicit tiers; the inbox is never auto-injected; one capture chokepoint |
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (light) | Remove model-callable KB write tools (`vox_kb_clip`/`add_entry`); writes only via capture paths or human gestures |
| V5 Validation, Sanitization, Encoding | **yes** | `SanitizedText` smart constructor at ingest; sanitize again at injection; image/HTML-comment neutralization |
| V6 Stored Cryptography | yes | BLAKE3 via `vox_crypto::secure_hash` for content hashes; no direct crypto crates |
| V7 Error Handling & Logging | yes | Replace `let _ =` swallowing on capture with `tracing::warn`; fail-closed on missing sink, unknown tier string, trust-lookup error |
| V8 Data Protection | yes | Snapshots local under a gitignored path, excluded from AI context via `.voxignore`; `vox_redact` on summary/quotes |
| V11 Business Logic | yes | Promotion eligibility tri-state; `approved` ≠ curated; dedupe |
| V12 Files & Resources | yes | Snapshot filename from hash only (no traversal); size cap; atomic write |
| V13 API | yes | Retired tools removed from catalog/registry (no aliases); CHANGELOG |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation (this phase) |
|---------|--------|----------------------------------|
| Stored indirect prompt injection via web captures (R1) | Tampering / Elevation | Inbox-only (Untrusted, pending, not injected); sanitize at ingest and injection; preamble drops Untrusted |
| Poisoning loop: model output re-ingested at high trust (R2, S8–S10, S4 synthesis) | Tampering | Delete writers; synthesis becomes an Untrusted finding; no confidence field |
| Hidden-instruction smuggling (tag block, variation selectors, zero-width, bidi) | Tampering | Codepoint strip + proptest + red-team cases |
| Exfiltration via markdown/HTML images | Information disclosure | Neutralize off-allowlist images in sanitized text. The render-side guard (no remote image auto-load) is Phase 6/8 (A9) |
| Provenance loss (no URL, date or hash; whole-bundle `Trusted` stamp) | Repudiation | `kb_findings` provenance columns + snapshot hash; per-hit tier; envelope = min tier |
| Unreviewed writes into curated docs (`publish_research_doc`, `save_research_doc`) | Tampering | Reroute or delete; only Phase 9 PRs touch git |
| Secrets in captured text | Information disclosure | `vox_redact::redact_owned` on stored summary/quotes; snapshot stays local (Phase 7 owns egress) |

## Sources

### Primary (HIGH: read from source this session)
- `crates/vox-orchestrator/src/context_envelope.rs:87-95, 311-324` (ContextTrustTier, ContextBudget); `crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs` (whole file: Trusted stamp at :148, kb_lines :359-372, parse_kb_mentions :234-254)
- `crates/vox-search/src/{policy.rs:54-57,201-203,407-409,568-574; execution.rs:21-24,421-502,603-631; bundle.rs:206-221; ingest.rs:17-36; trust.rs:1-4,44-80,130-236; context.rs:10-21}`
- `crates/vox-orchestrator-mcp/src/kb_tools/{mod,signal_chat,signal_research,signal_scientia}.rs`, `handlers.rs:206-269`; `chat_tools/chat/message.rs:740-895,1272-1284`; `chat_tools/chat/research_turn.rs:41-47,173-185,387-399,446-577,581-679`; `daemon_extra.rs:95-154`; `dispatch.rs:1476-1497`; `input_schemas.rs:306-321`; `lib.rs:159-160,178`
- `crates/vox-orchestrator/src/knowledge_base/{types,store,mod}.rs`; `memory/manager.rs:649-706`
- `crates/vox-db/src/{schema/domains/knowledge.rs:1-140; schema/domains/quarantine.rs:1-80; schema/domains/scientia.rs:345-356; schema/manifest.rs:11-35; store/open.rs:89-252; migration.rs:100-308; research.rs:1-185; questioning_telemetry.rs:60-97; store/ops_memory/kb.rs:348-430; codex_legacy.rs:20-60,176-194; local_tests.rs:886-942; schema_extensions.rs:1-60}`
- `crates/vox-research-shim/src/research/orchestrator/{web_gather.rs:196-209,340-400; helpers.rs:1-157; pipeline_cache.rs:32-50}`
- `crates/vox-gui/src/commands/{knowledge.rs:60-297; research.rs:90-275}`; `crates/vox-gui/src/main.rs:285-287,333-336`; `crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx:239-271`
- `crates/vox-ml-cli/src/commands/mens/pipeline.rs:592-688`
- `crates/vox-crypto/src/facades.rs:1-105`; `crates/vox-config/src/paths.rs:9-599` (consts); `.gitignore:108-133`; `.voxignore`
- `contracts/ci/crate-edges.allow.v1.json` (edge set); `docs/src/architecture/layers.toml:1-60,123-271`; `contracts/operations/catalog.v1.yaml:26687-26926`; `contracts/mcp/http-read-role-governance.yaml:1-20`; `contracts/db/baseline-version-policy.yaml`; `contracts/retirement/retired-surfaces.v1.yaml`; `contracts/documentation/retired-symbols.v1.yaml`; `.config/nextest.toml:30-52`; `.github/workflows/nightly.yml:1578`
- Read-only `sqlite3` counts on `~/Library/Application Support/vox/vox.db` and `/Users/brbrainerd/dev/vox/.vox/store.db`
- `.planning/workstreams/kb/{ROADMAP,REQUIREMENTS,STATE}.md`; `research/{SUMMARY,ARCHITECTURE,PITFALLS}.md`; `docs/src/architecture/knowledge-base-practices-research-2026.md`

### Secondary (MEDIUM)
- [Cisco: Understanding and mitigating Unicode tag prompt injection](https://blogs.cisco.com/ai/understanding-and-mitigating-unicode-tag-prompt-injection): tag block U+E0000–E007F; strip before the model
- [Paul Butler: Smuggling arbitrary data through an emoji (2025)](https://paulbutler.org/2025/smuggling-arbitrary-data-through-an-emoji/): variation selectors U+FE00–FE0F / U+E0100–E01EF as a byte channel

### Tertiary (LOW)
- [SecureLayer7: Unicode tag smuggling](https://securelayer7.net/learn/ai-security/unicode-tag-smuggling); [Repello: emoji prompt injection](https://repello.ai/blog/prompt-injection-using-emojis): corroborating

## Metadata

**Confidence breakdown:**
- Current-state map / sinks / retirement inventory: HIGH (read from source, line-cited)
- Placement without new edges: HIGH (edge set and layers read)
- Target design (names, schema columns, migration home): MEDIUM (recommendations, flagged A1–A9)
- Sanitizer ranges: MEDIUM-HIGH (requirements + primary attack write-ups)

**Research date:** 2026-10-03
**Valid until:** 2026-11-02, or until Phase 2 is planned, whichever comes first (re-check A1/A2 then)
