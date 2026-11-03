# Model Routing: The Chat Lane Never Picks a Flagship on Cost-Leaning Modes — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task docs/superpowers/plans/2026-09-28-model-routing-chat-lane.md <N>`), per
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, reviews the diff and commits.

**Goal:** Chat turns (the path the GUI shows) get the same guarantee task dispatch got in
[`2026-09-28-model-routing-latest-and-honest.md`](2026-09-28-model-routing-latest-and-honest.md): when the intent
leans on cost or speed rather than quality, a flagship (Elite-tier) model is not picked while another candidate fits.

**Architecture:** Chat resolves models through `resolve_mcp_chat_model_sync_inner` (`vox-orchestrator-mcp`), which calls
`models::select::decide` → `select_inner` → `select_via_scorer`. The recency filter (superseded versions) already
reaches this path through `best_for_internal`. The tier guarantee is added where the axes are known:
`select_via_scorer` runs a first pass that excludes `ModelTier::Elite` when `intent.axes.intelligence < 50` (the same
threshold `select_inner` uses to honour premium aliases), then an unrestricted fallback pass.

**Tech Stack:** Rust (`vox-orchestrator`, `vox-orchestrator-mcp`).

**Prerequisite:** Tasks 1–5 of the model-routing plan are committed (tiers exist on registered specs).

## Global Constraints

- Same as the model-routing plan: no new crate/edge/dependency, no versioned cloud model id literals in non-test code, test-first with RED recorded before implementation, `rustfmt --edition 2024` per changed file, foreground `timeout`-prefixed commands, existing tests are not edited (a broken existing test is a STOP), the agent never stages or commits.
- Axes semantics: `COST_FIRST` (70/15/15), `BALANCED` (33/33/34) and `FAST` (15/70/15) have intelligence < 50 and get the exclusion; `QUALITY_FIRST` (15/15/70) does not.
- `explain_selection`, `best_free_for*`, `cheapest*`, the Thompson fallback and GUI `suggest_model_for_task` are out of scope (see Deferred).

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `crates/vox-orchestrator/src/models/select.rs` | modify | `select_via_scorer` two-pass Elite exclusion |
| `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs` | modify (tests only) | chat-lane seam test through `resolve_mcp_chat_model_sync_inner` |

---

### Task 1: `select_via_scorer` excludes Elite on cost/speed-leaning axes; chat-lane seam test

**Files:**
- Modify: `crates/vox-orchestrator/src/models/select.rs` (`select_via_scorer`; new tests in its test module)
- Test: `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs` (existing `mod tests`)

**Interfaces:**
- Consumes: `ModelCapabilities::tier`, `ModelTier::Elite` (existing); `SelectionAxes` (existing).
- Produces: `select_via_scorer` behaviour only; no new public API.

- [ ] **Step 1: Write the failing select tests.** Append to `select.rs`'s test module (it already imports `ModelRegistry`, `SelectionAxes`, `SelectionIntent`, `TaskCategory`, `select`, `file_serial`; add `use` lines only if the compiler asks):

```rust
    fn tiered_spec(id: &str, tier: crate::models::ModelTier, cost: f64) -> crate::models::ModelSpec {
        crate::models::ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: crate::models::ProviderType::Ollama,
            max_tokens: 200_000,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![
                crate::models::StrengthTag::Codegen,
                crate::models::StrengthTag::Generalist,
            ],
            capabilities: crate::models::ModelCapabilities { tier, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: crate::models::spec::PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    /// The Elite model is the CHEAPEST here, so without the exclusion it wins under cost-leaning axes.
    fn flagship_registry() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        r.register(tiered_spec("acme/flagship-9", crate::models::ModelTier::Elite, 0.0005));
        r.register(tiered_spec("acme/workhorse-9", crate::models::ModelTier::Pro, 0.02));
        r
    }

    #[test]
    #[file_serial]
    fn scorer_excludes_a_flagship_on_cost_first_axes() {
        let intent = SelectionIntent {
            axes: SelectionAxes::COST_FIRST,
            complexity: 10,
            ..SelectionIntent::for_task(TaskCategory::CodeGen)
        };
        let out = select(&intent, &flagship_registry()).expect("a model exists");
        assert_eq!(out.model_id, "acme/workhorse-9");
    }

    #[test]
    #[file_serial]
    fn scorer_excludes_a_flagship_on_balanced_axes() {
        let intent = SelectionIntent {
            axes: SelectionAxes::BALANCED,
            complexity: 10,
            ..SelectionIntent::for_task(TaskCategory::CodeGen)
        };
        let out = select(&intent, &flagship_registry()).expect("a model exists");
        assert_eq!(out.model_id, "acme/workhorse-9");
    }

    #[test]
    #[file_serial]
    fn scorer_allows_a_flagship_on_quality_first_axes() {
        let intent = SelectionIntent {
            axes: SelectionAxes::QUALITY_FIRST,
            complexity: 10,
            ..SelectionIntent::for_task(TaskCategory::CodeGen)
        };
        let out = select(&intent, &flagship_registry()).expect("a model exists");
        assert_eq!(out.model_id, "acme/flagship-9", "quality-first applies no tier exclusion");
    }

    #[test]
    #[file_serial]
    fn scorer_falls_back_to_a_flagship_when_it_is_the_only_candidate() {
        let mut r = ModelRegistry::default();
        r.register(tiered_spec("acme/flagship-9", crate::models::ModelTier::Elite, 0.0005));
        let intent = SelectionIntent {
            axes: SelectionAxes::COST_FIRST,
            complexity: 10,
            ..SelectionIntent::for_task(TaskCategory::CodeGen)
        };
        let out = select(&intent, &r).expect("must not return None when the only candidate is Elite");
        assert_eq!(out.model_id, "acme/flagship-9");
    }
```

If `SelectionIntent::for_task` is not the constructor name used near line 1211, or `complexity` is not a field of `SelectionIntent`, adapt only the constructor expression in these four tests (`rg -n "pub struct SelectionIntent" -A25 crates/vox-orchestrator/src/models/select.rs`) and say so.

- [ ] **Step 2: Write the failing chat-lane seam test.** Append to `resolve.rs`'s `mod tests` (it already has the hermetic-registry pattern in `resolve_with_rationale_populates_reason_on_decide_branch`; mirror it):

```rust
    /// Chat is what the GUI shows: on the default (cost-leaning) axes it must not resolve to a flagship
    /// while a cheaper-tier model is registered, even when the flagship is the cheapest candidate.
    #[test]
    fn chat_lane_does_not_pick_a_flagship_on_default_axes() {
        use vox_orchestrator::models::spec::PricingSource;
        use vox_orchestrator::models::{ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType};

        let cfg = vox_orchestrator::OrchestratorConfig::for_testing();
        let groups = vox_orchestrator::AffinityGroupRegistry::new(vec![]);
        let orch = vox_orchestrator::Orchestrator::with_groups(cfg, groups);

        let mk = |id: &str, tier: ModelTier, cost: f64| ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::PopuliMesh,
            max_tokens: 200_000,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![vox_orchestrator::models::StrengthTag::Generalist],
            capabilities: ModelCapabilities { tier, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::UserConfig,
            supported_parameters: vec![],
        };
        {
            let handle = orch.models_handle();
            let mut registry: std::sync::RwLockWriteGuard<'_, ModelRegistry> =
                vox_orchestrator::sync_lock::rw_write(&*handle);
            registry.register(mk("chat-lane-flagship", ModelTier::Elite, 0.0005));
            registry.register(mk("chat-lane-workhorse", ModelTier::Pro, 0.02));
        }
        let res = McpChatModelResolution {
            allow_cheapest_fallback: false,
            task_category: TaskCategory::Research,
            complexity: 10,
            ..Default::default()
        };
        let mut rationale = None;
        let (model, _free) = resolve_mcp_chat_model_sync_inner(&orch, "explain this design", None, res, &mut rationale)
            .expect("a model resolves");
        assert_eq!(model.id, "chat-lane-workhorse", "picked {}", model.id);
    }
```

If `McpChatModelResolution` has no `complexity` field, or its type is not `u8`, read the struct (`rg -n "pub struct McpChatModelResolution" -A30 crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/types.rs`) and adapt only that line. The test must not set `clutch` (the default, unconfigured path is what chat uses).

- [ ] **Step 3: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::select::tests::scorer_ > target/chat-lane-red-a.txt 2>&1; tail -20 target/chat-lane-red-a.txt; timeout 1500s cargo test -p vox-orchestrator-mcp --lib -- model_route_policy::resolve::tests::chat_lane > target/chat-lane-red-b.txt 2>&1; tail -20 target/chat-lane-red-b.txt`
Expected: `scorer_excludes_a_flagship_on_cost_first_axes`, `..._on_balanced_axes` and `chat_lane_does_not_pick_a_flagship_on_default_axes` FAIL (the cheaper flagship wins); `scorer_allows_a_flagship_on_quality_first_axes` and `scorer_falls_back_...` pass. If the two chat-lane and cost-first tests already pass, the fixture is not making the flagship win: STOP and report the scores you can see, do not proceed.

- [ ] **Step 4: Implement** — in `select_via_scorer`, replace the single `registry.best_for_with_filter(...)` call:

```rust
    let model = registry.best_for_with_filter(
        intent.task,
        intent.complexity,
        cost_pref,
        intent.allow_free_in_performance_mode,
        |m| supports_intent_constraints(m, &intent_clone) && ModelRegistry::key_is_present_for(m),
        None,
    )?;
    drop(_axes_guard);
```

with

```rust
    // Cost- or speed-leaning intents (intelligence axis below the premium-alias threshold of 50) must
    // not resolve to a flagship while another candidate fits; fall back only when nothing else does.
    let no_flagship = intent.axes.intelligence < 50;
    let pick = |exclude_elite: bool| {
        registry.best_for_with_filter(
            intent.task,
            intent.complexity,
            cost_pref,
            intent.allow_free_in_performance_mode,
            |m| {
                (!exclude_elite || m.capabilities.tier != crate::models::ModelTier::Elite)
                    && supports_intent_constraints(m, &intent_clone)
                    && ModelRegistry::key_is_present_for(m)
            },
            None,
        )
    };
    let model = if no_flagship { pick(true).or_else(|| pick(false)) } else { pick(false) }?;
    drop(_axes_guard);
```

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::select 2>&1 | tail -20; timeout 1500s cargo test -p vox-orchestrator-mcp --lib -- model_route_policy 2>&1 | tail -20`
Expected: all pass, including every pre-existing `select` and `model_route_policy` test. If a pre-existing test fails, STOP and name it with the assertion.

- [ ] **Step 6: Mutation proof** — change `let no_flagship = intent.axes.intelligence < 50;` to `let no_flagship = false;`, run the Step 3 commands into `target/chat-lane-mutant.txt`, confirm the three tests named in Step 3 FAIL, restore, confirm `git diff` shows only the Step 4 edit and the tests.

- [ ] **Step 7: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/select.rs crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs
git commit -m "fix(models): the chat lane never picks a flagship on cost- or speed-leaning axes"
```

---

## Deferred

- `registry.rs#explain_selection` (its own comment says it must match the real path), `best_free_for*` / `cheapest*`, the Thompson fallback in `registry_model_resolve.rs`, GUI `suggest_model_for_task`: apply the same exclusion and recency filter in a follow-up once this lands.
- Responsive/FAST axes currently also exclude flagships; if a "Responsive" mode is added to the GUI, decide whether it should.

## Execution Order

Drive after Tasks 1–5 of the model-routing plan are committed. `select.rs` is also touched by that plan's Task 6; drive this plan after Task 6 (sequential — shared file).
