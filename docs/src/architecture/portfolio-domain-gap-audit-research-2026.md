---
title: "Cross-Portfolio Domain Capability Audit & Skill Engine Unification (2026)"
description: "Empirical capability gap audit across 30+ ~/dev projects (brVideo, ngi.video, gigme, ramada, fableforge, open-melvor, Resonance, cineflow) mapped to Vox language/runtime primitives and the FableForge + Open-Melvor + Village + FFScript skill unification architecture."
category: "Architecture SSOTs"
status: "research"
topic: "code-generation"
domain: "language"
authorship: "llm-generated"
authorship_model: "gemini-3-1-pro"
training_eligible: false
---

# Cross-Portfolio Domain Capability Audit & Skill Engine Unification (2026)

<!-- kb:summary -->
Audits thirty-five repositories across media processing, job intelligence, decision engines, interactive fiction, idle simulation, and real-time audio against the Vox compiler and runtime substrate. Identifies the highest-leverage language, database, and UI primitives that eliminate sixty to seventy percent of application boilerplate, and defines the unified FableForge, Open-Melvor, Village, and FFScript skill engine architecture.
<!-- /kb:summary -->

## 1. Executive Summary & Audit Scope

We audited the active repository portfolio under `~/dev` against `/Users/brbrainerd/dev/vox` (`crates/vox-compiler/src/feature_matrix.rs`, `crates/vox-ast/`, `crates/vox-codegen/`, `crates/vox-db/`, `crates/vox-actor-runtime/`, and `.planning/ROADMAP.md` Waves 0–4 / Phases 01–54):

- **Media, Video & Desktop Orchestration:** `brVideo`, `ngi.video`, `cineflow`, `brFrame`, `brUpVideo`, `brAutosub`, `annotate-subs`, `ytfloat`
- **Marketplaces, Decision Engines & Civic/FinTech:** `gigme`, `ramada`, `greaterFool`, `govSim`, `brFinancier`, `peaceboard`, `containment`
- **Interactive Fiction, Idle RPG & Game Simulation:** `fableforge` (`fableforge.to`), `open-melvor` (`@open-idle/engine`), `Resonance`, `resonance-godot48`, `talehall`, `Cataclysm-BN`
- **AI, ML, Neuroscience & Infra:** `brml`, `neurogecko`, `neuroscience-atlas-mapper`, `storage-tier`, `pg-bot-tools`, `ai-dev-box`

### Core Finding: Platform Harness vs. Application Domain Gap

Vox's current `v1.5` Unified Holistic Stream (Phases 01–54) builds a world-class **AI developer harness, knowledge base, autonomy safety floor, code atlas, terminal session host, and publication studio**. However, **none of Phases 01–54 ship the end-user `.vox` application and domain primitives** (`CC-00` through `CC-24` in [`web-app-archetype-coverage-2026.md`](web-app-archetype-coverage-2026.md), plus polyglot tool/form auto-surfaces, reactive DB subscriptions, presigned blob/media pipelines, evaluation-driven ranking, waiting/cost contracts, and deterministic tick/skill engines) that developers manually re-implement across `brVideo`, `ngi.video`, `gigme`, `ramada`, `fableforge`, and `open-melvor`.

---

## 2. Where 60–70% of Boilerplate Lives Across the Portfolio (Ranked by Leverage)

### 2.1 P1: Single-Declaration Polyglot `@tool` → CLI, JSON Schema, Auto-Form UI, HTTP & MCP Surface

**What portfolio repos do today:**

- **`brVideo` + `ngi.video`:** Every media tool requires maintaining 6 synchronized layers:
  1. Python `TOOL_METADATA` + `create_parser()` (`argparse`) in `brVideo`
  2. AST extraction (`brvideo_registry/cli.py`) into `tools-registry.json`
  3. Custom `argparse` → JSON Schema Draft 2020-12 + `uiSchema` translator (`brvideo/api_server/schema.py`, 381 LoC)
  4. React `@rjsf/core` form + `ui/src/form/argv.ts` (113 LoC) + `presets.ts` (167 LoC)
  5. FastAPI `/api/tools/{name}/run` + MCP server (`brvideo/api_server/mcp_server.py`)
  6. Convex `toolsRegistrySync.ts` (7.1 KB) + 5 copy-pasted 70-line action wrappers in `ngi.video/convex/processing.ts` (`startCompression`, `startUpscale`, `startTrim`, `startTranscode`, `startEnhance`) + 220-line per-tool React state machines (`CompressClient.tsx`).
- **`ramada`:** Adding or tuning a single scoring factor requires editing 6 places across `@ramada/schemas` (Zod), default config, DB provenance, GUI control + Advanced panel, CLI path, and MCP tool.
- **`ytfloat`:** Demonstrates the gold standard via `AppController.buildSpec()`: a single command specification table drives AppKit menus, Carbon global hotkeys, JS bridge payloads, and README tables.

**Vox Improvement (`@tool` / `@form` Polyglot Surface):**

- A single `.vox` declaration with parameter facets (bounds, step, unit, file-picker kind, mutually exclusive groups, preset profile, cost tier) compiles simultaneously to:
  1. Typed CLI subcommand + flags + Nushell/shell completions
  2. Axum/Tauri RPC endpoint + OpenAPI 3.1 + JSON Schema Draft 2020-12
  3. MCP tool definition (`vox-mcp`)
  4. Generated React `<ToolForm tool={...} />` with preset persistence, live CLI argv preview, and streaming execution state.

### 2.2 P2: Real-Time Streaming (`Stream[T]`, `Channel[Send, Recv]`), Reactive DB Subscriptions & Managed Subprocess Runs

**What portfolio repos do today:**

- **`brVideo`:** `brvideo/api_server/runs.py` (290 LoC) and `ui/src/run/useRun.ts` (199 LoC) hand-roll POSIX process groups (`start_new_session=True`), custom `\r` vs `\n` progress-bar stream splitting (so `tqdm`/FFmpeg carriage returns overwrite in-place instead of flooding logs), a 5,000-event ring buffer, SSE `/api/runs/{id}/events` with `Last-Event-ID` replay, gap detection, and two-phase `SIGTERM -> 5s -> SIGKILL` cancellation.
- **`ngi.video`, `fableforge`, `talehall`, `cineflow`, `neurogecko`:** Rely on Convex reactive `useQuery` subscriptions where database mutations automatically push delta updates to subscribed React clients.

**Vox Gap & Improvement:**

- `crates/vox-compiler/src/feature_matrix.rs` lacks `Channel[Send, Recv]` (WebSocket, `CC-00`) and `Stream[T]` (SSE endpoint return type, `CC-02`).
- `vox-actor-runtime` has internal `subscription.rs` pub/sub (`A13-04`), but `.vox` user code cannot yet write `db.Table.subscribe(...)` to push invalidation/delta frames over WebSocket/SSE into generated React `useQuery` hooks.
- `std.process` should expose a first-class managed `ProcessRun` primitive with `\r`/`\n` progress frame parsing, bounded ring buffer replay, and two-phase graceful tree termination.

### 2.3 P3: Evaluation-Driven Ranking, Explainability Provenance, Effect-Enforced Spend Guards & `<WaitingNotice>` UI Contracts

**What portfolio repos do today:**

- **`gigme`:**
  - **Multi-stage ranking with provenance & shadow evaluation:** `observation provenance` → `eligibility` → `ranker_v2` vs `ranker_v1` shadow comparison → `exposure attribution ledger` (`src/ranking/`, `src/evaluation/`), gated in CI by deterministic snapshot comparisons (`gigme eval ordering`, `scripts/helios/ordering-gate.sh`).
  - **Read- vs. Spend-command safety guards:** Read-only paths cannot trigger paid scrapes or prod mutations unless `GIGME_ALLOW_PROD_WRITES=1` and explicit `--refresh` flags are present.
  - **Strict Waiting Notice UI contract:** Every empty, pending, rate-limited, or degraded state in `gigme` MUST use `src/waiting/notice.ts` / `<WaitingNotice>` (enforced by `tests/guardrails/waitingNotices.test.ts`), stating: (1) why data is unavailable, (2) a computed `resumeAt` timestamp or explicit unblock condition, and (3) a link to the cost-confirmed action surface.
- **`ramada`:**
  - Strict separation of **fixed literature-backed domain formulas** (`packages/engine/test/golden.json`) from **user configuration weights/priors**, property-based monotonicity checks (`fast-check`), neutral read-time fallback when a factor is absent (`ramada-absent-factor-coverage`), and DB role separation (`facts_live` read-only vs `ramada_owner` ledger writer).
- **`greaterFool`:**
  - Dual-trace Fuzzy-Trace Theory (FTT) extraction (verbatim quantitative trace vs. categorical/ordinal gist trace) evaluated against cognitive decision benchmarks (Asian Disease, Allais Paradox).

**Vox Gap & Improvement:**

- **Compile-time Effect Enforcement for Spend & Prod Writes:** Extend Vox's `uses { ... }` algebraic effect system (`crates/vox-compiler/`) with `uses { Spend, Write, ExternalApi }` so read-only queries and dry-run CLI commands are **statically guaranteed** never to incur API/scrape spend or mutate production ledgers.
- **First-Class Waiting/Degraded State Contract in `.vox` Views:** Upgrade component `while loading` / `on error` (`A1-04` in `web-app-archetype-coverage-2026.md`) with a typed `when waiting(reason, resume_at, action_href)` clause that compiles to a guardrail-verified `<WaitingNotice>` contract.
- **Declarative `@ranker` / `@eval_shadow` & Monotone Property Goldens:** Support factor provenance tuples `(raw, normalized, weight, contribution, confidence, absent_neutral)` and built-in golden ordering gates (`vox test --ordering-gate`).

### 2.4 P4: Hybrid Multimodal Search (`@embed`, `@searchable`, FTS5 + HNSW + Phonetic) & Enforced Multi-Tenancy (`@table(scoped_by: ...)`)

**What portfolio repos do today:**

- **`brVideo` & `annotate-subs`:** 1,049 LoC of hand-written SQLite FTS5 + trigram virtual tables + sync triggers (`brvideo/registry_db.py`), USearch HNSW ANN vector indexes (`srt_search/ann_index.py`), Double Metaphone phonetic matching (`phonetic_index.py`), and CLIP/SigLIP visual frame sharding (`visual_shards.py`).
- **`gigme` & `ramada`:** PostGIS spatial queries + pgvector + tsvector in Postgres, plus painful PGlite (test) vs. Postgres (prod) parity workarounds (`gigme-pglite-race-tests.md`).
- **`fableforge`:** OpenAI `text-embedding-3-small` 512-dim embeddings + k-NN adjacency cache in Convex (`convex/rpg/nearness/`).

**Vox Gap & Improvement:**

- In `crates/vox-compiler/src/feature_matrix.rs:568`, `@embed` is still `EMBED_UNIMPLEMENTED`, and `.vox` user code lacks `db.search(...)` (`CC-14`/`CC-16`), partial `db.Table.patch(id, { ... })` updates (`HirDbTableOp::Update` currently overwrites all columns), `JOIN` / `db.bucket` aggregations (`CC-11`), and `@table(scoped_by: tenant)` compile-time row scoping (`CC-07`).
- Bridge `vox-search` (Tantivy BM25 + HNSW vector + RRF hybrid fusion) and `vox-db` directly into `.vox` `@table` declarations via `@searchable(fts: [...], vector: ..., phonetic: [...])`.

### 2.5 P5: Presigned Cloud Blob Storage (`Upload[T]`, `BlobStore`), OPFS Browser Caching & Webhook/Metered Billing (`@webhook`, `@metered`)

**What portfolio repos do today:**

- **`ngi.video`, `cineflow`, `brVideo`, `talehall`:** Repeated Cloudflare R2 / S3 presigned `PUT`/`GET` signing (`convex/uploads.ts`), multipart/chunked upload progress (`brvideo/cloud_storage/`), and browser OPFS 4 MB chunked media caching (`cineflow/lib/storage/opfs-media-store.ts`).
- **`ngi.video`, `cineflow`, `fableforge`, `talehall`:** Hand-rolled credit reservation, debit, refund-on-failure, tier gating (`convex/credits.ts`, `convex/lib/credits.ts`), sliding-window rate limiters (`rateLimiting.ts`), and HMAC webhook verification (`@webhook` is currently `DECORATOR_WEBHOOK_RUNTIME_UNIMPLEMENTED` on `RustAxum`/`RustTauri` in `feature_matrix.rs:589`).

**Vox Gap & Improvement:**

- Ship `Upload[T]` + `BlobStore` (`CC-01`: local dev store + S3/R2 presigned URL flow + HTTP `Range` streaming + optional OPFS client cache helper).
- Ship `@webhook(provider: Stripe | GitHub | Clerk | HmacSha256)` (`CC-04`), `@rate_limit` (`CC-13`), and `@metered(credits: ...)` transactional credit reserve/capture/refund wrappers (`CC-10`).

### 2.6 P6: Compiler Decorator Parity & External React Component Interop (`import react`)

- **Decorator Parity (`crates/vox-compiler/src/feature_matrix.rs`):** Close the remaining `Support::Unverified` and `Support::Unsupported` entries across `Interpreter`, `RustAxum`, `RustTauri`, and `TypeScript` so `.vox` decorators behave identically across script and compiled modes.
- **External React Interop (`docs/src/architecture/external-frontend-interop-plan-2026.md` Phase 5):** Complete `import react ...` component interop so `.vox` views can seamlessly host complex ecosystem islands (`@xyflow/react` node graphs in `cineflow`/`fableforge`, `@niivue/niivue` WebGL brain atlases in `neurogecko`, and custom canvas/audio viewports in `Resonance`).

---

## 3. FableForge (`fableforge.to`) ↔ `open-melvor` ↔ `The Village` ↔ `FFScript` (`RPG.SkillSystem`) Unification Architecture

### 3.1 Complementary Strengths of Each System Today

| Dimension | `fableforge` (`RPG.SkillSystem` + `FFScript` + `The Village`) | `open-melvor` (`@open-idle/engine`) | Unified "Best of All Worlds" Target |
| :--- | :--- | :--- | :--- |
| **Psychometric Spine** | **3-Layer Spine:** Innate **Aptitudes** → **16 Faculties** across **Mind** (`reasoning`, `knowledge`, `perception`, `memory`, `focus`), **Body** (`strength`, `endurance`, `agility`, `dexterity`, `reflexes`), **Heart** (`boldness`, `discipline`, `curiosity`, `empathy`, `composure`, `integrity`) → **Living Skill Constellation** | Flat 28 Melvor skills; no underlying character faculties or narrative attributes | **16 Faculties + Aptitudes** govern learning rate (`xpPercent`), initial action interval floor, and margin-band resolution across all idle & narrative skills |
| **Skill Graph & Transfer** | **Nearness Engine (`convex/rpg/nearness/`):** 4-signal hybrid similarity (`e_sem` embeddings + `e_apt` faculty cosine + `e_beh` `skill2vec` co-use + `e_tag`) → k-NN adjacency → **Transfer Splash (`transfer.ts`)** & **Emergent Discovery (`discover_skill`)** | Isolated skills (except Summoning synergies & Agility/Astrology global modifiers) | Practicing an idle or narrative action awards **direct Skill/Action XP** AND splashes **Nearness Transfer XP** to adjacent skills in the constellation |
| **Competence vs. Rust vs. Action Mastery** | `mastery` (permanent competence `L^1.5`) × `sharpness` (0..1 elapsed-time rust by `decayClass: cognitive \| physical` in `decay.ts`) = `effectiveSkill` | **Dual-Track Mastery (`mastery.ts`):** Per-Action/Recipe Mastery (1–99/120) + Per-Skill **Mastery Pool** (25%/50% spillover, Mastery Tokens, **10%/25%/50%/95% Pool Checkpoints**) | **Clean 3-Axis Model (Zero Collision):** 1. `skillCompetence` (level + faculty ceiling) × `sharpness` (rust) = `effectiveSkill`; 2. `actionMastery` (per-recipe/node 1–99); 3. `masteryPool` (per-skill 10/25/50/95% checkpoints) |
| **Modifier Pipeline** | Basic equipment/condition flat bonuses (`equipment.ts`) | **`ModifierPipeline` (`modifier-pipeline.ts`):** Composable `intervalPercent` (200ms floor), `preservationChance` (80% cap), `doublingChance` (100% cap), `masteryXpPercent`, `skillXpPercent`, `flatIntervalMs` | Unified `ModifierPipeline` fed by **16 Faculties**, **Equipment**, **Village Buildings/Blessings**, **Astrology/Summoning Passives**, **Sharpness**, and **Mastery Pool Checkpoints** |
| **Execution Loop & Offline Catch-Up** | Narrative turn-based ops (`exercise_skill`, `skill_check_op`); `village/offlineSimulation.ts` only emits cosmetic hourly NPC flavor text (**zero real offline progression**) | **Deterministic 50ms `ActionTimer` (`tick-loop.ts`)** + **`simulateOfflineProgress` (`offline.ts`)** fast-forwarding up to 24h (`1,728,000` ticks in 1,000-tick batches) with exact `OfflineProgressReport` deltas | **Real Deterministic Offline & Active Tick Loop** in both Village & Story modes: produces unified **"While You Were Away" Chronicle + Skill/Mastery/Item Recap** |
| **Domain Loops** | Narrative d20 Margin-Band checks (`resolve.ts`), Power Economy (`Reach => Use-Cost`), basic 1-shot recipe crafting (`recipes.ts`) | **4 Complete Idle Loops:** Gathering (multi-tree, rock HP/respawn, fishing junk/special, farming stages), Artisan (preservation + doubling), Passive (Astrology, Summoning, Agility, Township), Combat | Village locations (`forest`, `mine`, `forge`, `apothecary`, `tavern`, `shrine`, `training_grounds`) & FFScript scenes host full Gathering, Artisan, Passive, and Combat loops |

### 3.2 Concrete Unification Blueprint (`fableforge` + `open-melvor`)

1. **Extract/Link Pure Deterministic Core (`packages/skill-engine` in `fableforge` synced with `@open-idle/engine`):**
   - Keep `open-melvor`'s zero-dependency TypeScript modules (`mastery.ts`, `modifier-pipeline.ts`, `timer.ts`, `tick-loop.ts`, `offline.ts`, `skills/gathering.ts`, `skills/production.ts`, `skills/passive-progression.ts`) pure and deterministic so they run identically inside:
     - **Browser Web Worker / UI client** (smooth 50ms progress bars and instant tick feedback),
     - **Convex Mutations/Actions (`fableforge/convex/rpg/`)** (authoritative server verification & offline catch-up commit upon session resume), and
     - **`open-melvor` standalone desktop/web app**.
2. **Harmonize Terminology & Data Schema (`convex/rpg/skills/types.ts` + `characterSkills`):**
   - Preserve `characterSkills.mastery` (or alias in UI as **Competence**) and `characterSkills.sharpness` (`0.0 .. 1.0` from `decay.ts`).
   - Add `skillMasteryPool: { poolXp: number, maxPoolXp: number, claimedCheckpoints: number[] }` on `characterSkills` (or `villageCharacterState`).
   - Add `characterActionMastery` table (indexed by `by_character_skill_action`: `characterId`, `skillKey`, `actionId`, `xp`, `level`) for per-recipe / per-gathering-node mastery (`1..99`).
   - **How Rust (`sharpness`), Faculties, and Melvor Mastery Interact:**
     - **Faculty Lean → Interval & XP Scaling:** Each skill's `facultyWeights` (e.g., `smithing`: `strength 0.35, dexterity 0.35, focus 0.30`) feeds a natural baseline bonus into `ModifierPipeline` (`skillXpPercent`, `intervalPercent`).
     - **Sharpness (Rust) Knock-Off:** When a skill is rusty (`sharpness < 1.0`), active idle ticks or narrative `exercise_skill` ops restore `sharpness` rapidly over the first few actions while scaling `effectiveSkill`.
     - **Nearness Splash (`transfer.ts`) on Idle Batches:** When an idle session or `simulateOfflineProgress` finishes $N$ actions on `skillKey`, the total base XP earned is passed once through `computeSplashAwards(totalBaseXp, neighbors)` (`convex/rpg/skills/transfer.ts`)—giving O(k) batch splash XP to semantically/behaviorally related skills without per-tick DB overhead!
3. **Upgrade `The Village` (`fableforge/src/lib/games/village/`) & Crafting (`convex/lib/gameSystems/crafting/recipes.ts`):**
   - Replace cosmetic-only `src/lib/games/village/offlineSimulation.ts` with a hybrid **Chronicle + Deterministic Tick Catch-Up** engine:
     - When a player returns after $\Delta t$ (clamped to `maxOfflineHours = 24`), run `simulateOfflineProgress` for their active Village job/station (`woodcutting`, `mining`, `forging`, `herbalism`, `scribes_study`, `guard_patrol`) alongside NPC schedule chronicle events.
     - Present a unified **"While You Were Away" Modal** showing: (a) Village narrative events, (b) Items gathered/crafted + resources preserved/doubled, (c) Skill Competence & Action Mastery level-ups, (d) Mastery Pool checkpoints unlocked (`10% / 25% / 50% / 95%`), and (e) Nearness Splash XP rippled across the character's skill constellation.
   - Wire `ModifierPipeline` directly into `convex/lib/gameSystems/crafting/recipes.ts`:
     - `calculateCraftingTime` uses `ModifierPipeline.getEffectiveInterval(baseTimeMs, skillId, actionId)`.
     - Ingredient consumption rolls `ModifierPipeline.getPreservationChance(skillId, actionId)` (up to 80% chance to preserve rare reagents).
     - Output generation rolls `ModifierPipeline.getDoublingChance(skillId, actionId)` + `ActionMastery` quality tiers.
4. **Extend `FFScript` (`packages/ffscript/src/schema.ts` & `convex/lib/ffscript/operationRegistry.ts`):**
   - Add 4 declarative operations to the existing `RPG.SkillSystem` family (`exercise_skill`, `skill_check_op`, `discover_skill`):
     1. `start_idle_action`: `{ skillKey, actionId, stationId?, maxTicks? }` — begins a continuous tick-driven gathering/artisan/training action.
     2. `stop_idle_action`: `{ reason? }` — commits accumulated ticks, awards direct + Nearness splash XP, and updates bank/inventory.
     3. `resolve_offline_ticks`: `{ elapsedMs }` — executes deterministic batch catch-up (`simulateOfflineProgress`) + Nearness splash + Sharpness recovery and returns `OfflineProgressReport`.
     4. `grant_mastery_pool_xp`: `{ skillKey, actionId?, poolXp, consumeToken? }` — awards or spends Melvor-style Mastery Pool XP / Mastery Tokens and evaluates `10%/25%/50%/95%` pool checkpoints.
5. **Vox-Native Path:**
   - Express the deterministic `ActionTimer`, `MasteryPool`, `ModifierPipeline`, and `NearnessSplash` contract as a canonical `.vox` reference module / template so any Vox game or simulation app gets 50ms client/worker tick loops + authoritative server batch verification out of the box.

---

## 4. Portfolio-Wide Ponytail Audit (YAGNI, DRY, SSOT, Cull, Integrate)

Applying the `ponytail` / `ponytail-audit` hierarchy (**skip (YAGNI) → reuse codebase → stdlib → native platform → installed dep → one-line → minimum code**) across `/Users/brbrainerd/dev` and the Vox GSD roadmap yields concrete code deletions, SSOT consolidations, and roadmap plan reductions.

### 4.1 Executed & Ranked Portfolio Reductions (`~/dev`)

- **`shrink:` `ngi.video/convex/processing.ts` (-183 LoC applied, -120 LoC next):**
  - Replaced 5 copy-pasted 70-line Convex action starters (`startCompression`, `startUpscale`, `startTrim`, `startTranscode`, `startEnhance`) with a single `dispatchProcessingJob(...)` helper while preserving identical exported Convex action signatures.
  - **Next cull:** Route all client callers through `startTool(toolId, arguments)` backed by `tools-registry.json` and delete the 5 legacy wrapper exports (`-120 LoC`).
- **`ssot:` `fableforge` ↔ `open-melvor` ↔ `The Village` ↔ `FFScript` Unified Skill Engine (Implemented & Tested):**
  - Centralized all Melvor idle/mastery tunables into `fableforge/convex/config/rpg/mechanics.ts` (`MECHANICS.melvor`).
  - Built `fableforge/convex/rpg/skills/melvorEngine.ts` (`MasteryPool`, `MasteryController`, `UnifiedModifierPipeline`, `simulateUnifiedIdleProgress`) and wired it into `src/lib/games/village/offlineSimulation.ts`, `convex/lib/gameSystems/crafting/recipes.ts`, and `convex/lib/ffscript/skillIdleOps.ts` (`convex/games/village/ffscript.ts`), verified by `convex/rpg/skills/__tests__/melvorEngine.test.ts` (`4/4` passing).
  - **Next cull:** Point `open-melvor/packages/engine` directly at the shared pure TypeScript skill-engine package instead of maintaining two separate copies of `MasteryPool` and `ModifierPipeline`.
- **`delete:` `brVideo` root script sprawl (-4,500+ LoC):**
  - Delete 38+ one-off root scripts in `/Users/brbrainerd/dev/brVideo/` (`fix_config_imports_properly.py`, `fix_all_config_calls.py`, `ultimate_test_runner.py`, `comprehensive_ci_local_runner.py`, `comprehensive_local_ci_runner.py`, `final_slow_test_validator.py`, `final_cleanup_slow_tests.py`, and ad-hoc root `test_*.py` files) that duplicate `pytest` and git history.
- **`ssot:` Video satellite repos (`brFrame`, `brUpVideo`, `brAutosub`, `annotate-subs`) → `brVideo` Registry SSOT:**
  - `brFrame` (frame extraction), `brUpVideo` (upscaling), and `brAutosub` (Whisper subtitles) duplicate FFmpeg/GPU/CLI scaffolding already inside `brVideo/packages/brvideo-registry`. Archive standalone satellite wrappers and treat `brVideo` tool registry as the single backend SSOT consumed by `ngi.video` and `cineflow`.
- **`ssot:` `Resonance` vs. `resonance-godot48` & `neurogecko` vs. `neuroscience-atlas-mapper`:**
  - Freeze legacy `Resonance` in favor of `resonance-godot48` (active Godot 4.8 target); extract one shared `@niivue/niivue` atlas viewer module between `neurogecko` and `neuroscience-atlas-mapper`.
- **`shrink:` `gigme` & `ramada` schema/contract duplication:**
  - Replace parallel Drizzle + Zod + hand-written TypeScript interface files with `drizzle-zod` (`createSelectSchema` / `createInsertSchema`) in existing TS repos, and `@table` / `@tool` / `@ranker` SSOT declarations in `.vox`.

### 4.2 Ponytail Streamlining of Vox GSD Wave 5 (`Phases 64–69`: 24 Plans → 12 Plans)

Instead of building parallel subsystems or 24 standalone ceremony plans, Wave 5 (`Phases 64–69`) is streamlined to **12 focused plans (2 plans per phase)** that directly reuse existing Waves 1–3 crates:

| Phase | Capability | Reused Vox SSOT Crate / Primitive (Zero Reinvention) | Plan Count |
| :--- | :--- | :--- | :--- |
| **Phase 64** (`POLYTOOL-01..04`) | `@tool` & `@form` 1-to-6 surface emission | Reuses `vox-scientia-jsonschema-codegen`, `vox-mcp-registry`, and `Phase 51` `CommandCatalog` / Nushell completion emitter | **2 plans** (`64-01..02`, `-2 plans`) |
| **Phase 65** (`STREAM-01..04`) | `Stream[T]`, `Channel[Send,Recv]`, `db.Table.subscribe`, `ProcessRun` | Reuses `crates/vox-actor-runtime/src/subscription.rs` and `Phase 46` `vox-terminal-core` process-group lifecycle | **2 plans** (`65-01..02`, `-2 plans`) |
| **Phase 66** (`RANK-01..04`) | `uses { Spend, Write }` guards, `when waiting` `<WaitingNotice>`, `@ranker` gates | Extends existing `crates/vox-compiler/src/typeck/` `uses` effect checker and `A1-04` view state lowering | **2 plans** (`66-01..02`, `-2 plans`) |
| **Phase 67** (`USERDB-01..04`) | `@embed`, `@searchable`, `db.Table.patch`, `@table(scoped_by: tenant)` | Reuses `Phase 37` `vox-search` (Tantivy BM25 + HNSW + RRF) and `vox-db` (`HirDbTableOp`) | **2 plans** (`67-01..02`, `-2 plans`) |
| **Phase 68** (`CLOUD-01..04`) | `Upload[T]`/`BlobStore`, `@webhook` HMAC, `@rate_limit`, `@metered` | Reuses `vox-crypto` (BLAKE3/HMAC-SHA256) and `vox-secrets` (`resolve_secret`) | **2 plans** (`68-01..02`, `-2 plans`) |
| **Phase 69** (`SIM-01..04`) | Decorator parity sweep, `import react`, `.vox` Deterministic Tick/Skill Engine | Reuses `crates/vox-compiler/src/feature_matrix.rs` gate and ports `fableforge/convex/rpg/skills/melvorEngine.ts` 1:1 to `.vox` | **2 plans** (`69-01..02`, `-2 plans`) |

