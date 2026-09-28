# Chat Surface Trace and Latest-Model Routing — Implementation Plan

> **Execution method (binding).** Claude Code drives Gemini Flash through the `agy` CLI, one `<task>` per headless run,
> and verifies, reviews and commits each result itself. Driver rules, guard hook and the `/drive-task` skill live in
> the local `.agents/` kit described in
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> Tasks marked **`owner: claude`** are not driven: they need web data, judgment across generated files, or a human.
>
> **Design source:** [`chat-surface-design-critique-2026-09-28.md`](../../src/architecture/chat-surface-design-critique-2026-09-28.md)
> (the one-home-per-fact rule, the 40-row matrix, the anti-spam policy and the canonical vocabulary).
>
> **Sequencing:** start after Phase 5 (`.planning/phases/05-*`) lands, because both touch `agent_loop.rs`,
> `ChatTurnEventRow.tsx` and `ChatExecutionRail.tsx`. Tasks run strictly in the order of the table below (shared
> working tree and git index; a test-first task leaves its crate red on purpose).

## Why this plan exists (evidence, 2026-09-28)

1. **The default Efficient lane can pick Opus.** The live dispatch path (`crates/vox-orchestrator/src/runtime.rs`
   `AiTaskProcessor::process`, `resolve_task_cost_policy` at `:66-94`) reads only `cost_preference` and
   `force_free_pool` from the clutch profile. `mode::effective_axes()` (`mode.rs:305-311`) is called only by its own
   tests. `scoring.rs::quality_score` (`:153-162`) is binary on `is_free` plus a log of context length, so Haiku,
   Sonnet and Opus get the same "quality"; `CostPreference::Economy` only adds a soft bonus. On high-complexity tasks
   the largest-context paid model wins.
2. **Stale pins.** `premium_alias` in `contracts/orchestration/model-routing.v1.yaml:85-92` and
   `model-pins.v1.yaml` route codegen/debugging/security/review to `anthropic/claude-3-7-sonnet`, research to
   `google/gemini-2.0-flash`, planning to `openai/o3-mini`; `visual-review.config.v1.json` escalates to
   `anthropic/claude-opus-4.8`; `vox-config` carries more literal defaults (`bootstrap_inference.rs:32-55`,
   `config/vox_config.rs:51`, `operator_registry.rs` / `config_registry.rs` env defaults, `routing_policy.rs:154,158`).
3. **No recency signal.** `catalog.rs` parses OpenRouter `/api/v1/models` but never deserializes `created`, and sets
   `canonical_slug` to `id` (`:206`). OpenRouter documents no `-latest` alias; `openrouter/auto` ranks by spend share,
   not recency. "Latest in family" must be computed from `created`.
4. **Measurement barely counts, and the bandit is not wired.** Live scoreboard data (success, quality, p50, cost per
   success from `model_scoreboard`) enters scoring capped at 0.15; the Thompson bandit that the rail's "exploit" label
   describes (`routing/engine.rs`) is not called by the canonical `best_for_internal`; `pareto.rs` (ADR-046) is
   reporting-only. The key-availability gate is applied only in callers' predicates, so direct `best_for*` calls can
   choose a model whose provider has no key.
5. **Two selection engines.** `select.rs` (premium-alias, intent axes) serves only the research pipeline
   (`research_dispatch.rs:828`); tasks and chat use `registry.rs::best_for_internal` + `scoring.rs`.
6. **The GUI never showed a real routing decision.** Screenshots showed `opus-4-8 · exploit / Alt: sonnet-4-6 /
   haiku-4-5` from `e2e/lib/tauriMock.ts`, and nobody could tell it was fake. Two emitted turn-event kinds
   (`delegation_spawned`, `research_milestone`) and ~75 engine events never reach the chat.

## Rules every task follows (bug prevention, learned from Phase 5 runs)

- **Test first, with an adversarial case.** Every task names at least one: an absent optional field, two sources that
  collide, one failing item among passing ones, or a failure halfway through. A plain round-trip is not enough.
- **Seams are tested with the real producer.** Where a value crosses tasks (Rust event → TS render, catalog →
  resolver → scorer → DTO → chip), the consumer's test uses the producer's real output or the shared contract file,
  never a hand-written guess of it.
- **Guards and filters are mutation-proven.** Break the guard, confirm the named test fails, restore, and confirm with
  a byte-identical check. A test that passes against both versions is deleted and rewritten.
- **Generated files are never hand-edited.** A task that changes a source contract lists every generated output in
  `<files>` and regenerates by running the generator; Claude runs the ssot-drift loop afterwards.
- **Big contract files get one targeted edit**, never a re-serialize (`git diff --numstat` must match the task's size).
- **"Commit X before Y" is a split run.** Part A does only the pinning step; Claude commits; part B does the rest.
- **Foreground only, `timeout`-prefixed**, `cargo` scoped with `-p`, `rustfmt` per file, never `cargo fmt`.
- **Model ids never appear as literals** in routing, config defaults, GUI code or mocks. Tests assert *shape* (family,
  recency, source), not a version string. The only allowed literals are local MENS revisions (AGENTS.md exception)
  and the dated seed/bootstrap contract files.
- **Every chat-visible field is server-derived** (registry names, ledger ids, counts, catalog-resolved ids); raw model
  output never becomes chrome.

## Order of work

| # | Task | Track | Owner |
|---|---|---|---|
| 1 | Parse `created` and `canonical_slug` from OpenRouter | B routing | agy |
| 2 | `latest_in_family` resolver | B | agy |
| 3 | Seed-score contract (warp.dev defaults + public leaderboard snapshot) | B | **claude** |
| 4 | Seed loader and an honest `quality_score` | B | agy |
| 5 | Mode objectives enforced on the real dispatch path (Efficient never flagship) | B | agy |
| 6 | Provider-key availability filter with reasons | B | agy |
| 7 | Premium aliases become family selectors (contracts + loader) | B | agy |
| 8 | Remove literal model defaults from `vox-config` | B | agy |
| 9 | Rewrite id-pinned tests and the routing drift gate to assert shape | B | agy |
| 10 | Research engine uses the same family resolution | B | agy |
| 11 | Scoreboard rolls up by family and learns from measured calls | B | agy |
| 12 | Offline last-known-good catalog refresh script | B | **claude** |
| 13 | Turn-event contract and both-sides seam tests | A trace | agy |
| 14 | `routing_decision` turn event and routing DTO | A | agy |
| 15 | `turnTrace.ts` pure builder | A | agy |
| 16 | Turn-correlated engine events into chat, coalesced | A | agy |
| 17 | `TurnTrace` component replaces loose chips and the ModelBadge | A | agy |
| 18 | Verbosity control | A | agy |
| 19 | Status bar cards: Engine, Spend, Routing, Needs you | C surfaces | agy |
| 20 | Rail: Routing section, drop global blocks, live context meter | C | agy |
| 21 | Composer: remove duplicates, full mode names, Risk and Check replies | C | agy |
| 22 | Honest research popover | C | agy |
| 23 | Mocks from contracts, and a no-stale-model-id guard | C | agy |
| 24 | One label source, no Latin in English mode | D visual | agy |
| 25 | Status colour tokens and a guard | D | agy |
| 26 | Type scale, contrast, glows, heading order (axe zero serious) | D | agy |
| 27 | Fix stale review states; delete orphans | D | agy |
| 28 | Golden seam: real agent turn → events → trace render | E integration | agy |
| 29 | Review capture and live look | E | **claude + human** |

---

<tasks>

<task type="tracer">
  <name>Task 1: Parse `created` and `canonical_slug` from OpenRouter /models</name>
  <files>crates/vox-orchestrator/src/catalog.rs, crates/vox-orchestrator/src/models/spec.rs, crates/vox-orchestrator/tests/fixtures/openrouter-models.sample.json (new), crates/vox-orchestrator/tests/openrouter_catalog_parse.rs (new)</files>
  <read_first>
    - catalog.rs `OpenRouterModelData` (~:36-55) and `refresh()` (~:116-221), especially `canonical_slug: m.id.clone()` (~:206).
    - models/spec.rs `ModelSpec` fields.
  </read_first>
  <behavior>
    - Fixture: 6 real-shaped entries (copy the shape of a live `/api/v1/models` item: `id`, `canonical_slug`, `created`, `pricing`, `context_length`), including one `:free` variant, one entry with no `created`, and two entries in one family with different `created`.
    - `parse_models_response(json) -> Vec<ModelSpec>`: `spec.created_at == Some(unix)` when present, `None` when absent (adversarial: absent must not become 0 or now). `spec.canonical_slug` comes from the API field and falls back to `id` only when the field is missing.
    - Existing parse of pricing / context_length unchanged (assert one known value).
  </behavior>
  <action>
1. RED: add the fixture and the integration test; run `timeout 1200s cargo test -p vox-orchestrator --test openrouter_catalog_parse` (expected compile failure or assertion failure).
2. Add `created: Option<u64>` and `canonical_slug: Option<String>` to `OpenRouterModelData` (serde default), `created_at: Option<u64>` to `ModelSpec` (default None everywhere else it is constructed — `rg -n "ModelSpec \{" crates/` and add `created_at: None` or `..Default::default()` at each site the compiler names). Extract the JSON → specs mapping into `pub(crate) fn parse_models_response` if it is inline in `refresh()`, without changing behaviour.
3. GREEN; `rustfmt --edition 2024` each changed .rs file.
  </action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --test openrouter_catalog_parse --lib catalog 2>&1 | tail -30</automated></verify>
  <done>Specs carry the real `created` and `canonical_slug`; absence stays `None`.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: `latest_in_family` resolver</name>
  <files>crates/vox-orchestrator/src/models/family.rs (new), crates/vox-orchestrator/src/models/mod.rs</files>
  <read_first>
    - Task 1's `ModelSpec.created_at` / `canonical_slug`.
    - catalog.rs family inference (~:102, :256-260) — reuse its org/family parsing if it fits; do not duplicate.
  </read_first>
  <behavior>
    - `family_key(slug) -> String`: `anthropic/claude-opus-5.5` → `anthropic/claude-opus`; `anthropic/claude-sonnet-4.6` → `anthropic/claude-sonnet`; **`anthropic/claude-3-7-sonnet` → `anthropic/claude-sonnet`** (old naming shape, adversarial); `deepseek/deepseek-v4-flash` → `deepseek/deepseek-flash`; `qwen/qwen3-coder:free` → `qwen/qwen-coder` with the `:free` suffix reported separately; `openai/gpt-5-mini` → `openai/gpt-mini`.
    - `latest_in_family(specs, family, opts) -> Option<&ModelSpec>`: the max `created_at`; ties broken by the higher parsed version; specs with `created_at == None` lose to any dated spec; `opts.exclude_preview` drops `-preview`/`-beta`/`-exp`; `opts.free_only` keeps only `:free` / zero-priced; an unknown family → `None` (adversarial).
    - `families(specs) -> BTreeMap<String, Vec<&ModelSpec>>`.
  </behavior>
  <action>
Test-first in `family.rs` `#[cfg(test)] mod tests` with the cases above, RED, then implement. Pure functions, no I/O. Add `pub mod family;` to `models/mod.rs`. Doc comment on `family_key` states the rule: strip version tokens (digits, dots, dash-joined digit runs) and qualifier suffixes (`:free`, `-preview`, `-beta`, `-exp`), keep org and name words in order. `ponytail:` comment naming the ceiling (heuristic parse; if providers adopt a family field upstream, prefer it).
  </action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::family 2>&1 | tail -25</automated></verify>
  <done>Any family resolves to its newest dated member, with old and new naming shapes in one family.</done>
</task>

<task type="checkpoint:owner" gate="blocking">
  <name>Task 3: Seed-score contract (owner: claude)</name>
  <files>contracts/orchestration/model-seed-scores.v1.json (new), contracts/orchestration/model-seed-scores.schema.json (new)</files>
  <action>
Claude authors this from the model-ranking research (warp.dev published defaults per mode, plus one machine-readable public leaderboard snapshot whose terms allow automated use). Keyed by **family** (Task 2's `family_key`), never by version: `{ "family": "deepseek/deepseek-flash", "intelligence": 0-100, "responsiveness": 0-100, "tier": "flagship|strong|efficient|free", "sources": [{"name","url","as_of"}] }`, top-level `as_of` date. `tier: flagship` is what the Efficient lane excludes (Task 5). Families with no data are listed with `"intelligence": null` so the gap is explicit. Commit with a body citing the sources.
  </action>
  <done>A dated, sourced, family-keyed prior that Task 4 can load.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 4: Seed loader and an honest `quality_score`</name>
  <files>crates/vox-orchestrator/src/models/seed.rs (new), crates/vox-orchestrator/src/models/mod.rs, crates/vox-orchestrator/src/models/scoring.rs</files>
  <read_first>
    - scoring.rs `quality_score` (~:153-162), `efficiency_score` (~:165-175), `auto_score_model` (~:321-410).
    - contracts/orchestration/model-seed-scores.v1.json (Task 3).
  </read_first>
  <behavior>
    - `seed_for(family) -> Option<SeedScore>` from the contract embedded with `include_str!` (path relative to the crate, as `models/spec.rs:315` does for the bootstrap catalog).
    - `quality_score(m)` uses the family seed's `intelligence` when present; the old paid/free + context heuristic is the fallback only when the seed is absent or null.
    - Adversarial: a free model with a high seed outranks a paid model with a low seed on quality; Opus-family and Haiku-family specs no longer score equal; a spec whose family is missing from the contract still scores (fallback) and does not panic.
    - A test loads every family present in `model-catalog.bootstrap.v1.json` and asserts each has a contract row (null allowed) — the seam between the two contracts.
  </behavior>
  <action>Test-first, then implement. No change to `efficiency_score` or weights in this task.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::seed models::scoring 2>&1 | tail -30</automated></verify>
  <done>Quality reflects the model, not "is it paid".</done>
</task>

<task type="tracer">
  <name>Task 5: Mode objectives enforced on the real dispatch path</name>
  <files>crates/vox-orchestrator/src/runtime.rs, crates/vox-orchestrator/src/mode.rs, crates/vox-orchestrator/src/models/registry.rs, crates/vox-orchestrator/tests/mode_objectives.rs (new)</files>
  <read_first>
    - runtime.rs `resolve_task_cost_policy` (~:66-94) and the `best_for_task_with_filter` call (~:735).
    - mode.rs `ClutchProfile::resolve` (~:133-169), `effective_axes` (~:305-311, test-only today).
    - registry.rs `best_for_internal` (~:871-960).
  </read_first>
  <behavior>
    Integration test through `best_for_task_with_filter` with the predicate built by the real `resolve_task_cost_policy`, over a fixture catalog of latest-per-family specs: opus (flagship), sonnet (strong), deepseek-flash (efficient), gemini-flash (efficient), a qwen `:free` model.
    - Efficient on a HIGH-complexity task never returns a `tier: flagship` family (adversarial: the exact case that picks Opus today).
    - Efficient when the only available candidate is flagship → returns it with a reason containing `only candidate` (adversarial: must not return None).
    - Genius → the highest-intelligence latest family.
    - Free → only zero-priced / `:free`.
    - Balanced → neither the cheapest nor the flagship when both extremes exist (weighted).
    - Responsive (only if `ClutchProfile` defines it; otherwise assert it does not exist and skip) → the lowest seed/measured latency among the quality floor.
    - Fallback stays in lane: when the chosen Efficient candidate errors, the retry picks the next candidate that satisfies the same Efficient constraints, never a flagship (adversarial; the fallback graph in `model-routing.v1.yaml:24-27` has no cost ceiling today).
  </behavior>
  <action>
1. RED.
2. Carry the resolved profile (quality level + axes) through `resolve_task_cost_policy` instead of discarding it; build the candidate predicate from it: Efficient excludes flagship-tier families unless no other candidate passes the other filters. Replace the soft Economy bonus with the objective "maximize quality per dollar subject to a quality floor" (floor = the median seed intelligence of available candidates; `ponytail:` comment naming it as tunable).
3. `effective_axes` is now called from the real path (delete it if the profile carries the same data; do not leave it test-only).
4. GREEN. Mutation proof: delete the flagship exclusion, run the Efficient high-complexity test into `target/models-mutant-flagship.txt`, it must FAIL; restore; `git diff` shows only the intended change.
  </action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --test mode_objectives 2>&1 | tail -30 && grep -q "FAILED" target/models-mutant-flagship.txt</automated></verify>
  <done>Each mode's promise holds on the path tasks actually take, and Efficient cannot silently pick a flagship.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 6: Provider-key availability filter with reasons</name>
  <files>crates/vox-orchestrator/src/models/key_guard.rs, crates/vox-orchestrator/src/models/registry.rs, crates/vox-orchestrator/tests/mode_objectives.rs</files>
  <read_first>
    - key_guard.rs `provider_secret_is_available` and the ProviderType→SecretId map (~:24-53).
    - How `best_for_internal` applies key availability today (it may not); `rg -n "provider_secret_is_available" crates/vox-orchestrator/src`.
  </read_first>
  <behavior>
    Extend `mode_objectives.rs` with an injectable availability set (a test double, not env vars):
    - No OpenRouter key but a DeepSeek key → a `ProviderType::DeepSeek` candidate is chosen directly; the reason names `deepseek key`.
    - No cloud keys → local MENS candidate; reason `no cloud keys`.
    - A key present for a provider whose models are all filtered out by the mode → falls to the next provider, never errors (adversarial).
  </behavior>
  <action>Test-first. Selection must never pick a candidate whose provider has no resolvable secret (Clavis, `vox_secrets::resolve_secret` — never read env directly). Today the key gate lives only in callers' `pred` closures (`select.rs` ~:126, :823, :854), so a direct `registry.best_for()` / `best_for_task()` call bypasses it: move the gate into `best_for_internal`'s own filter chain (registry.rs ~:882-940) and add a test that calls `best_for_task` with no predicate and still never returns a keyless provider (adversarial; mutation-prove by removing the gate). Return the reason alongside the choice (reuse the existing selection-reason type if one exists; `rg -n "reason" crates/vox-orchestrator/src/models/registry.rs`).</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --test mode_objectives 2>&1 | tail -30</automated></verify>
  <done>Routing branches on the keys that actually exist and says why.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 7: Premium aliases become family selectors</name>
  <files>contracts/orchestration/model-routing.v1.yaml, contracts/orchestration/model-pins.v1.yaml, contracts/orchestration/visual-review.config.v1.json, crates/vox-config/src/model_routing.rs, crates/vox-orchestrator/src/models/select.rs, crates/vox-orchestrator/src/models/registry.rs</files>
  <read_first>
    - model-routing.v1.yaml `premium_alias` (~:85-92); model-pins.v1.yaml (~:35-66, :90); visual-review.config.v1.json (:3-4).
    - vox-config model_routing.rs (~:103, :186) — how the YAML is embedded and typed.
    - select.rs `select_via_premium_alias` (~:802-836); registry.rs `premium_alias_for` (~:1276).
  </read_first>
  <behavior>
    - The contract values are family keys (`anthropic/claude-sonnet`), not versions. The Rust type is renamed from an id to a `FamilySelector`, so a stray version string fails to deserialize (adversarial: a `claude-3-7-sonnet` value in the YAML is a load error with a message naming the field).
    - `premium_alias_for(category)` resolves through Task 2's `latest_in_family` against the live registry; with an empty catalog it returns `None` and selection falls through to the scorer (adversarial).
  </behavior>
  <action>
Edit each contract value in place (one targeted edit per key; `git diff --numstat -- contracts/` stays in tens of lines). Update the loader types and both callers. If `vox ci ssot-drift` or `model_routing_check` names a generated output, list it in the commit message; Claude runs the drift loop and Task 9 rewrites the check itself.
  </action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-config model_routing 2>&1 | tail -15 && timeout 1500s cargo test -p vox-orchestrator --lib models::select models::registry 2>&1 | tail -20</automated></verify>
  <done>No contract routes to a version; every alias resolves to the newest member of its family.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 8: Remove literal model defaults from vox-config</name>
  <files>crates/vox-config/src/bootstrap_inference.rs, crates/vox-config/src/config/vox_config.rs, crates/vox-config/src/operator_registry.rs, crates/vox-config/src/config_registry.rs, crates/vox-config/src/routing_policy.rs</files>
  <read_first>
    - bootstrap_inference.rs (~:32-55), vox_config.rs (:51), operator_registry.rs (~:320-334, :646), config_registry.rs (~:1321-1349, :1601), routing_policy.rs (~:154-158, :290-299).
  </read_first>
  <behavior>
    - A unit test in `vox-config` scans these five files' non-test source for versioned model ids (regex covering `claude-(opus|sonnet|haiku)-\d`, `gpt-\d`, `gemini-\d`, `o\d-mini`, `llama\d`, `deepseek-v\d`) and fails on any match outside `#[cfg(test)]` (the guard; mutation-prove it by re-adding one literal).
    - Defaults become family selectors (or `None` = "let the router choose"); an explicit user/env override still wins (adversarial: env override set → exact value honoured).
  </behavior>
  <action>Test-first. Where a default must name something (e.g. an env var's fallback), use a family key and resolve it at the orchestrator. Do not touch local MENS revision pins.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-config 2>&1 | tail -20</automated></verify>
  <done>Config carries no stale versions and cannot regain them silently.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 9: Id-pinned tests and the routing drift gate assert shape</name>
  <files>crates/vox-cli-ci/src/model_routing_check.rs, crates/vox-audit/src/panel.rs, crates/vox-actor-runtime/src/llm/cascade.rs, crates/vox-cli/src/commands/chat.rs, crates/vox-cli/src/commands/audit_route.rs</files>
  <read_first>
    - model_routing_check.rs (drift check of premium_alias vs model-pins); panel.rs (~:39-51, :694-752); cascade.rs (~:407-421); chat.rs (~:119, :155-213; keep the local qwen dispatch); audit_route.rs (:320-376).
  </read_first>
  <behavior>
    - The drift gate checks that every alias is a family key present in the bootstrap catalog, and fails on a versioned value (adversarial fixture).
    - Rewritten tests assert family / provider / recency-source, not exact version strings, and still fail when routing picks the wrong family (keep one negative case per test).
  </behavior>
  <action>Rewrite assertions, not behaviour. Local MENS / Ollama literal handling in chat.rs stays.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-cli-ci model_routing 2>&1 | tail -10 && timeout 1500s cargo test -p vox-audit panel 2>&1 | tail -10 && timeout 1500s cargo test -p vox-actor-runtime --lib llm::cascade 2>&1 | tail -10</automated></verify>
  <done>Tests protect routing semantics without freezing model versions.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 10: Research engine uses the same family resolution</name>
  <files>crates/vox-orchestrator/src/models/select.rs, crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs</files>
  <read_first>select.rs `select` (~:671-836), `select_via_scorer` (~:838-929); research_dispatch.rs (~:828).</read_first>
  <behavior>
    - `select_with_default_registry(&SelectionIntent::research())` over a fixture catalog returns the latest member of the chosen family and applies Task 5's objective for its mode (adversarial: a catalog containing both `gemini-2.0-flash` and a newer flash → the newer one).
  </behavior>
  <action>Test-first. Prefer delegating to the shared objective/resolver over duplicating it; if the two engines can be one, say so in the commit body and leave the merge to a follow-up.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::select 2>&1 | tail -20</automated></verify>
  <done>No selection path can return a stale family member.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 11: Scoreboard rolls up by family and learns from measured calls</name>
  <files>crates/vox-orchestrator/src/models/registry.rs, crates/vox-orchestrator/src/models/scoring.rs, crates/vox-orchestrator/src/models/scoreboard.rs (new if registry.rs is near its line budget)</files>
  <read_first>registry.rs `ModelScore` and where it is updated; `rg -n "LlmCallCompleted|llm_call_completed" crates/vox-orchestrator/src` for the measurement event.</read_first>
  <behavior>
    - Measured latency (p50), success rate and $/1k tokens update a per-**family** score; a new version of a family inherits the family history (adversarial: the resolved id changes between two calls → one family entry, not two).
    - Responsiveness in scoring uses measured p50 when ≥ N samples (N named constant), else the seed.
    - A failure streak lowers a family's score and recovers after successes (bounded).
    - Measured data outweighs the seed once confident: today `scoreboard_feedback_boost` (scoring.rs ~:58-108) is capped at 0.15 of the score; above `MIN_CALLS_FOR_CONFIDENT_RANK` samples the measured terms carry at least half the quality and latency weight (adversarial: a family with a high seed but a 20% measured success rate loses to a lower-seeded family at 95%).
    - The Thompson bandit (`routing/bandit.rs`, `routing/engine.rs::pick_with_auto_score_thompson`) that the GUI's "explore/exploit" label describes is not on the canonical path (`best_for_internal` does a plain `max_by`). Wire its draw into `best_for_internal` as a bounded exploration term (off for Free, small for Efficient) — or, if that needs a design call, STOP and say so; do not leave the GUI describing an engine that does not run.
  </behavior>
  <action>Test-first. Check the non-blank line count of registry.rs first; if the change would push it past the 500-line governance limit, put the new code in `scoreboard.rs`.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models:: 2>&1 | tail -25</automated></verify>
  <done>Rankings move with evidence, keyed by family so upgrades keep their history.</done>
</task>

<task type="checkpoint:owner" gate="blocking">
  <name>Task 12: Offline last-known-good catalog refresh (owner: claude)</name>
  <files>scripts/refresh-model-catalog.vox (new), contracts/orchestration/model-catalog.bootstrap.v1.json (generated)</files>
  <action>
Claude writes a `.vox` script (AGENTS.md VoxScript-first) that fetches `/api/v1/models`, keeps the latest member per family plus pricing/context/created, and rewrites the bootstrap catalog with an `as_of`. The resolver labels any pick sourced from the bootstrap file `resolved_from: "bootstrap"`; the GUI (Task 19) never presents such a pick as current. Run it, review the diff, commit.
  </action>
  <done>Offline and CI runs use a dated fallback, not a silent pin.</done>
</task>

<task type="tracer">
  <name>Task 13: Turn-event contract and both-sides seam tests</name>
  <files>contracts/gui/turn-event-kinds.v1.json (new), crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs, crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.ts (new)</files>
  <read_first>agent_loop.rs `turn_event_for_result` and `receipt_turn_event`; ChatTurnEventRow.tsx.</read_first>
  <behavior>
    - The contract lists every turn-event `kind` with its required fields and one example object each: `skill_activated`, `delegation_spawned`, `research_milestone`, `tool_receipt`, `receipt_claims`, and `routing_decision` (added by Task 14; list it now with `"status": "planned"`).
    - Rust test: every kind `turn_event_for_result` / `receipt_turn_event` can emit is in the contract, and each example deserializes into what Rust would emit for it (adversarial: add a kind in Rust without the contract → test fails).
    - Vitest: every non-planned contract kind has a render branch that returns non-null for its example and null when a required field is missing (adversarial).
  </behavior>
  <action>Test-first. Today `delegation_spawned` and `research_milestone` have no render branch, so the vitest side is RED by design; Task 17 turns it green. Mark those two `it.todo` only if the driver note says so; otherwise leave RED and STOP after writing the contract and the Rust side.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib agent_loop 2>&1 | tail -15</automated></verify>
  <done>Rust and the GUI agree on turn events through one file, checked from both sides.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 14: `routing_decision` turn event and routing DTO</name>
  <files>crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs, crates/vox-orchestrator/src/models/registry.rs, crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs, crates/vox-gui/ui/src/types/tauri.ts, contracts/gui/turn-event-kinds.v1.json</files>
  <read_first>`RoutingSummaryDto` in dashboard_api.rs (~:374) and types/tauri.ts (~:99-110); Task 5/6 selection reason.</read_first>
  <behavior>
    - Each assistant turn emits one `routing_decision`: `{family, resolved_id, resolved_from: "catalog"|"bootstrap"|"local", mode, objective, reason, ranking: [{family, score}] (top 3)}`. Every field is server-derived.
    - `RoutingSummaryDto` gains the same `family`, `resolved_from` and `reason`.
    - Adversarial: `resolved_from: "bootstrap"` is emitted when the catalog is empty; the contract example for `routing_decision` loses `"status": "planned"`.
  </behavior>
  <action>Test-first; keep `turn_event_for_result`'s rule that nothing comes from model output.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib agent_loop 2>&1 | tail -15 && timeout 1500s cargo test -p vox-orchestrator-mcp --lib dashboard_api 2>&1 | tail -10 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>Every turn says which model answered, where that id came from, and why.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 15: `turnTrace.ts` pure builder</name>
  <files>crates/vox-gui/ui/src/lib/turnTrace.ts (new), crates/vox-gui/ui/src/lib/turnTrace.test.ts (new)</files>
  <behavior>
    `buildTurnTrace(events, verbosity) -> { summary, steps, interrupts, defaultExpanded }`:
    - summary counts: tools, receipts verified/unverified, delegations, research waves, cost, duration, resolved model (only when `resolved_from === 'catalog'`; otherwise the family plus `(offline)`).
    - interrupts = fabricated/unverified claims, injection_detected, budget exceeded, scope_violation, approval required.
    - consecutive identical events coalesce (`3× tool_timed_out`); unknown kinds are dropped (adversarial); a step missing required fields is dropped, not rendered as blank.
    - defaultExpanded: quiet → false; normal → true only if a step failed; verbose → true.
    - Adversarial: 100 timeouts → one step with count 100; a `tool_receipt` with `verified:false` becomes an interrupt only when a `receipt_claims` flagged it, otherwise a failed step.
  </behavior>
  <action>Pure TS, no React. Test-first.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/turnTrace.test.ts</automated></verify>
  <done>One tested function decides what a turn's trace says and when it opens.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 16: Turn-correlated engine events into chat, coalesced</name>
  <files>crates/vox-gui/ui/src/lib/chatCorrelation.ts, crates/vox-gui/ui/src/lib/chatCorrelation.test.ts</files>
  <read_first>chatCorrelation.ts agentEvent cases (~:259-430); `AgentEventKind` in crates/vox-orchestrator/src/events.rs (~:116) for exact wire names.</read_first>
  <behavior>
    - Attach to the active turn's `events[]`: tool_timed_out, compaction_triggered, context_truncated, injection_detected, budget_alert, scope_violation, replan_triggered, semantic_drift_detected, doubt_reported, pav_phase_changed, llm_call_completed. Wire names are copied from events.rs, not guessed.
    - Events for another session or no active turn are ignored (adversarial); hopper/mesh/workflow families are never attached.
  </behavior>
  <action>Test-first with one fixture per kind built from the Rust serde shape (read the enum's `#[serde]` attributes).</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/chatCorrelation.test.ts</automated></verify>
  <done>The engine's turn-relevant events reach the turn they belong to.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 17: `TurnTrace` component replaces loose chips and the ModelBadge</name>
  <files>crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx (new), crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx (new), crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx, crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx, crates/vox-gui/ui/e2e/chat-turn-trace.spec.ts (new)</files>
  <behavior>
    - Collapsed: one row under the assistant message, e.g. `claude-sonnet-4.6 · 3 tools · 3 receipts ✓ · $0.004 · 4.1s`, with a disclosure button (`aria-expanded`). Expanded: ordered steps; each step reuses `PhaseChip` / `ChatAgentEventRow` visuals where they fit.
    - Interrupt chips stay inline above the collapsed row. `skill_activated` keeps its "not this one" action.
    - `delegation_spawned` and `research_milestone` render as steps (turns Task 13's vitest green).
    - The ModelBadge is removed; its information is in the summary.
    - Playwright: one turn collapsed, same turn expanded, one with a fabricated-claims interrupt; screenshots `chat-trace-collapsed.png`, `chat-trace-expanded.png`, `chat-trace-interrupt.png`. Uses the contract examples as mock payloads (import the JSON), not hand-written events.
  </behavior>
  <action>Test-first (vitest), then Playwright. Foreground, `timeout 300s`; the dev server on :1420 is reused.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat src/lib && timeout 300s pnpm --dir crates/vox-gui/ui typecheck && timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/chat-turn-trace.spec.ts e2e/chat-trust-chips.spec.ts --project=chromium --reporter=line</automated></verify>
  <done>Every turn has a quiet, expandable account of what happened.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 18: Verbosity control</name>
  <files>crates/vox-gui/ui/src/hooks/useChatVerbosity.ts, crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx, crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx, crates/vox-gui/ui/src/lib/chatTranscriptTimeline.ts</files>
  <behavior>A Quiet / Normal / Verbose segmented control in the chat header sets the trace's default expansion (Task 15 rules); `verbose` is no longer identical to `normal` (adversarial: assert it differs); the choice persists in the existing localStorage key.</behavior>
  <action>Test-first.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/hooks src/components/surfaces/Chat && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>The user chooses how much trace they see.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 19: Status bar cards — Engine, Spend, Routing, Needs you</name>
  <files>crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx, crates/vox-gui/ui/src/hooks/useHudTiles.ts, crates/vox-gui/ui/src/components/layout/BottomStatusBar.test.tsx, crates/vox-gui/ui/e2e/status-bar-surfaces.spec.ts</files>
  <read_first>BottomStatusBar.tsx (~:132-198); useHudTiles.ts (~:18-26); the critique's matrix rows 1-10.</read_first>
  <behavior>
    - Engine `9 agents · 44 queued`, Spend `$12.34 / $50` (popover: OpenRouter vs local vs this session), Mesh (one source: `useMeshNodes`), Routing (`Auto → <family>`, the resolved id only when `resolved_from === 'catalog'`), Needs you (approvals + questions), freshness pill.
    - Adversarial: `resolved_from: 'bootstrap'` renders the family and `(offline)`, never a version; zero cap renders `$12.34` without `/ $0`.
    - Configure menu labels equal the card names.
  </behavior>
  <action>Test-first; update the Playwright spec's expectations and capture `status-bar-cards.png`.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/layout && timeout 300s pnpm --dir crates/vox-gui/ui typecheck && timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/status-bar-surfaces.spec.ts --project=chromium --reporter=line</automated></verify>
  <done>Global engine facts live in one place, compactly, with detail on demand.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 20: Rail — Routing section, no global blocks, live context meter</name>
  <files>crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx, crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.test.tsx, crates/vox-gui/ui/src/hooks/useChatExecutionData.ts, crates/vox-gui/ui/src/hooks/useChatExecutionData.test.ts</files>
  <behavior>
    - "Intents" becomes "Routing": `Routed to <resolved or family> — <reason>`; alternatives and the bandit state are behind a disclosure, with a tooltip for explore/exploit.
    - The Agents roster and the Resources block are removed; "Session spend" stays as the one session-scoped number.
    - The context meter refreshes when a turn completes (adversarial: two turns → two fetches).
    - Lock chips from Phase 5 still render (regression assertion).
  </behavior>
  <action>Test-first; update fixtures to contract-shaped routing data (no version literals).</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat src/hooks && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>The rail is about this session, and says why the model was chosen.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 21: Composer — no duplicates, full mode names, Risk and Check replies</name>
  <files>crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx, crates/vox-gui/ui/src/components/surfaces/Loquela/DriveConsole.tsx, crates/vox-gui/ui/src/lib/driveConsole.ts, crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.tsx, crates/vox-gui/ui/src/lib/slashRouter.ts</files>
  <behavior>
    - Remove the "session $x / $y" text (a global figure mislabelled as session).
    - Mode labels `Free · Efficient · Balanced · Genius` (plus Responsive only if the backend defines it) with a visible one-line hint on hover/focus; `ClutchId` wire values unchanged (adversarial: the payload still sends `efficiency`).
    - Risk button reads `Risk: Moderate`; the grounding toggle moves into the Risk popover as `Check replies`.
    - Run button aria text matches the visible shortcut hint; the unreachable `plan` send mode is either added to the menu or removed (pick removal unless a test references it).
  </behavior>
  <action>Test-first; update Loquela tests that assert the old labels.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela src/lib && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>The composer says each thing once, in plain words.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 22: Honest research popover</name>
  <files>crates/vox-gui/ui/src/components/common/StatusBarCluster.tsx, crates/vox-gui/ui/src/components/common/StatusBarCluster.test.tsx</files>
  <behavior>Every health line comes from `getResearchEngineStatus`; hardcoded "Online", "✓ Wikipedia (Live)" and "0 Plaintext Keys" are removed; missing data renders "unknown" (adversarial: status fetch fails → no green checks).</behavior>
  <action>Test-first.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/layout && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>The popover never claims health it did not measure.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 23: Mocks from contracts, and a no-stale-model-id guard</name>
  <files>crates/vox-gui/ui/e2e/lib/tauriMock.ts, crates/vox-gui/ui/e2e/chat-trust-chips.spec.ts, crates/vox-gui/ui/src/__tests__/noVersionedModelIds.test.ts (new)</files>
  <behavior>
    - Mock model lists and routing summaries use family keys plus `resolved_from`, derived from `contracts/orchestration/model-seed-scores.v1.json` families (import the JSON), never versions.
    - Guard test: no file under `src/` or `e2e/` (excluding the guard itself) contains a versioned cloud model id (same regex family as Task 8); mutation-prove by adding `opus-4-8` to a fixture.
  </behavior>
  <action>Test-first; fix the other test files the guard names (`ChatModelPicker.test.tsx`, `Loquela.test.tsx`, `useChatExecutionData.test.ts`, `ChatExecutionRail.test.tsx`, `sessionChatStore.test.ts`, `chatCorrelation.test.ts`, `modelPicker.test.ts`, `axisDrive.test.ts`) by switching to family keys. If the guard names more than those files, STOP with the list.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>Screenshots can never again show a model version nobody resolved.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 24: One label source, no Latin in English mode</name>
  <files>crates/vox-gui/ui/src/lib/lexicon.ts, crates/vox-gui/ui/src/lib/navigation.ts, crates/vox-gui/ui/src/lib/lexicon.test.ts</files>
  <behavior>`NAV_LABELS` is derived from `LEXICON` (one source); English mode shows Market, Review, Search Index (one name each); a test asserts no English label equals its Latin one or contains a code name (Loquela, Oratio, Mercatus, Scientia, Axis Inspector, Secretary) (adversarial list).</behavior>
  <action>Test-first.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>Each concept has one English name everywhere.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 25: Status colour tokens and a guard</name>
  <files>crates/vox-gui/ui/src/components/surfaces/Chat/*.tsx, crates/vox-gui/ui/src/components/surfaces/Loquela/*.tsx, crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx, crates/vox-gui/ui/src/__tests__/statusColorTokens.test.ts (new)</files>
  <behavior>Status meaning (pass / fail / warn / info) uses `--color-status-*` tokens; the guard fails on `text-(amber|rose|emerald|cyan)-\d{3}` or raw `#0b0b0e` in those directories (adversarial: mutation-prove with one reintroduced class). Brass stays for brand accents, not warnings.</behavior>
  <action>Test-first. If the directory glob in `<files>` exceeds 12 files with changes, STOP and list them so Claude can split the task.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run && timeout 300s pnpm --dir crates/vox-gui/ui typecheck</automated></verify>
  <done>Amber means "look here" again.</done>
</task>

<task type="auto">
  <name>Task 26: Type scale, contrast, glows, heading order</name>
  <files>crates/vox-gui/ui/src/index.css, crates/vox-gui/ui/src/components/surfaces/Chat/*.tsx, crates/vox-gui/ui/src/components/surfaces/Loquela/*.tsx</files>
  <behavior>Data text ≥ 11 px; two tracked-caps styles (display 0.13em, micro 0.08em); Loquela and `index.css:125` glows removed (Limes conventions); chat empty/error heading order fixed. The review capture's axe results for `chat--*--wide` show zero `serious` or `critical` violations.</behavior>
  <action>Claude runs the capture (`VOX_REVIEW_CAPTURE=1 ... capture.spec.ts --grep " -- wide$"`) before and after and compares `entries-*.jsonl`.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui typecheck && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat src/components/surfaces/Loquela</automated></verify>
  <done>The chat surface is readable and passes contrast.</done>
</task>

<task type="auto">
  <name>Task 27: Fix stale review states; delete orphans</name>
  <files>crates/vox-gui/ui/e2e/review/states.ts, crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx, crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.test.tsx, crates/vox-gui/ui/src/components/chat/ChatMessage.tsx, crates/vox-gui/ui/src/components/chat/ResearchSummaryCard.tsx, crates/vox-gui/ui/src/components/layout/AttentionStrip.tsx, crates/vox-gui/ui/src/App.tsx</files>
  <behavior>The `model-picker-open`, `session-menu-open` and `rails-overlay-open` chat states target elements that exist (or are removed); orphans with no importers are deleted (`rg -n "<Name>" crates/vox-gui/ui/src` must show only the file itself before deleting — adversarial: STOP if any importer exists); the dead `chatDock` block in App.tsx is removed.</behavior>
  <action>Verify each orphan has zero importers before deleting.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui typecheck && timeout 600s pnpm --dir crates/vox-gui/ui exec vitest run</automated></verify>
  <done>The review bundle captures every chat state, and dead UI code is gone.</done>
</task>

<task type="tracer">
  <name>Task 28: Golden seam — real agent turn to trace render</name>
  <files>crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs, contracts/gui/turn-event-kinds.v1.json, crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx</files>
  <behavior>
    - Rust: a wiremock-backed `run_agent_turn` over a fixture catalog (latest-per-family) produces `routing_decision` + `tool_receipt` events; the test asserts they equal the contract's `golden_turn` example after normalizing ids/timestamps (adversarial: change the fixture catalog's newest member → the golden no longer matches and the test says which field).
    - Vitest: `TurnTrace` renders the same `golden_turn` and shows the resolved model, tool count and verified receipt.
  </behavior>
  <action>Add `golden_turn` to the contract; both tests read it.</action>
  <verify><automated>cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator-mcp --lib agent_loop 2>&1 | tail -15 && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/TurnTrace.test.tsx</automated></verify>
  <done>What the engine emits is exactly what the chat shows, proven by one shared example.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 29: Review capture and live look (owner: claude + human)</name>
  <action>
Claude runs the review capture at all three viewports, compares axe results against the 2026-09-28 baseline in the critique, starts the `vox-gui-limes` dev server, drives one real chat turn against a keyed backend on the Efficient mode, and screenshots: the trace collapsed and expanded, the status bar cards, and the rail's Routing section showing a catalog-resolved, non-flagship model. The human confirms.
  </action>
  <done>The routing fix and the trace are visible in the real app, not only in mocks.</done>
</task>

</tasks>
