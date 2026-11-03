# Model Routing That Keeps Itself Current, and Shows Its Work — Implementation Plan

> **For agentic workers:** Execution is **Claude Code driving Gemini Flash via the `agy` CLI**, one task per headless
> run (`/drive-task docs/superpowers/plans/2026-09-29-model-routing-self-maintaining.md <N>`, where `<N>` is `1a`,
> `1b`, `2` … `9`, `11`), per
> [`docs/src/contributors/antigravity-driven-execution.md`](../../src/contributors/antigravity-driven-execution.md).
> The agent never stages or commits; Claude Code re-runs every check, runs the mutation proofs, reviews the diff and
> commits. Task 10 is re-anchored by Claude before it is driven; Task 12 is Claude's. Steps use checkbox (`- [ ]`)
> syntax. Code blocks are transcribed exactly.

**Goal:** (1) A new model release, a price cut or a benchmark rescale changes routing with **no code edit and no
hand-kept list**; (2) every routing decision can be **explained from the selector's own ranking** (scores, provenance
of every number, and why each other model lost); (3) routing **health is checked on every catalog refresh** and shown
in `vox doctor` and the GUI, so drift is detected rather than rediscovered.

**Architecture.**
- *Self-maintaining.* A new `models::reference::RoutingReference` is derived from the registered models on every
  catalog merge (`ModelRegistry::apply_routing_reference`): the intelligence-index scale (top benchmark), the
  unbenchmarked-proxy scale (fitted to benchmarked models) and the Elite/Pro price bands (paid-price quantiles
  calibrated so today's catalog reproduces today's $20/M and $4/M bands). Each value falls back to the constant it
  replaces when fewer than 20 samples exist. The registry holds the reference (no globals) and stamps each spec
  with a `QualityPrior { value, source }`; `quality_score` reads the stamp. A model with no benchmark inherits the
  newest benchmarked member of its family (direct-provider ids join through their dated `canonical_slug`), and an
  OpenRouter `~vendor/…-latest` alias target is never treated as superseded.
- *Explain by construction.* `auto_score_model` returns the `total` of a new `ScoreParts`, and
  `best_for_with_filter` returns the first entry of a new `Ranking` built by the same filter chain, which records
  an `Exclusion` reason for every model it drops. `best_for_task_in_mode` returns its `Ranking`; `decide()`'s
  `alternatives` become the ranked runners-up (today they are the first five candidates in `HashMap` order) and it
  gains per-candidate scores.
- *Health.* `models::health::check_routing_health` probes the registry for invariants after each refresh; the
  result is logged, persisted as a user preference and read by `vox doctor` (which today reads a preference key
  nothing writes, so its Model Catalog check can never pass).
- *Surface.* Two Tauri commands (`explain_routing`, `get_routing_health`) feed a `RoutingExplainer` panel on the
  Models surface, replacing the jargon "Decision Preview". It explains **task dispatch** (the same
  `best_for_task_under_gate` call `runtime.rs` makes), says so, and says which live gates it cannot see. The
  Routing status card shows routing health (Task 10). Chat's own selector (`decide()`) gets its explanation in a
  follow-up (Deferred). <!-- AMENDED: G8, G9 -->

**Tech Stack:** Rust (`vox-orchestrator`, `vox-research-shim`, `vox-cli`, `vox-gui`), TypeScript/React 19 + Tailwind
4 (`crates/vox-gui/ui`), vitest 3, Playwright 1.62.

**Evidence (measured 2026-09-28/29, OpenRouter `/api/v1/models`, 460 entries, 369 after skips):**
- Live routing after the first model-routing plan is correct (Efficient cx10 → `xiaomi/mimo-v2.6-pro`, Genius cx10
  → `anthropic/claude-opus-5.5`, no Opus-class model in any Efficient/Balanced pick), but rests on eight hand-kept
  pieces: `QUALITY_INDEX_REFERENCE = 60`, the `$20/M` / `$4/M` bands, `UNBENCHMARKED_QUALITY_SCALE = 0.6`,
  slug-parsed families, OpenRouter-only benchmarks, `AnthropicDirectCatalog`'s `id.contains("opus")` tier rule
  (`catalog.rs`), the DeepSeek off-peak bonus keyed on `"deepseek"`/`"r1"` in the id (`scoring.rs`), the
  `latency_score` fallback keyed on `llama-3` / `groq` / `cerebras` in the id (`scoring.rs#latency_score`;
  <!-- AMENDED: A14 -->), and tests that name models.
- Derived on that catalog: top index 57.6; fitted unbenchmarked scale 0.495 (vs 0.6); 343 paid models, $20/M at
  quantile 0.93 and $4/M at 0.70 (so those quantiles reproduce today's bands exactly).
- 7 families have an unbenchmarked newest release with a benchmarked predecessor, e.g.
  `anthropic/claude-sonnet-5.5` (released 2026-09-28, scored as "unknown") vs `claude-sonnet-5` (index 38.2).
- OpenRouter's 18 `~…-latest` aliases are finer than the slug heuristic
  (`~deepseek/deepseek-v4-flash-latest` and `~deepseek/deepseek-flash-latest` are separate lines the heuristic
  merges), so an alias may not *override* `family_key` (that would split families and stop older members being
  superseded); it may only protect its target from being superseded.
- `decide()`'s `alternatives` (`select.rs#decide`) are `candidates.iter().filter(..).take(5)` over
  `registry.list_models()` — HashMap order — and the trace plan (Decision 3) had to omit `ranking` because no
  per-candidate scores exist. The Models view's "Decision Preview" renders these as
  `state=… · intel=0.xx · eff=0.xx · lat=0.xx` and "alternatives: …".
- `models/pareto.rs` is **reporting-only** by ADR-046 ("nothing here feeds model selection"): it is not reused for
  selection here.
- `vox doctor` Model Catalog check reads user preference `"catalog_refresh"`
  (`checks_standard/model_catalog.rs#run`); every writer uses `"model_catalog_last_refresh"`
  (`registry.rs`, `catalog_refresh.rs#persist_catalog_refresh_timestamp`).

**Prerequisites:** `2026-09-28-model-routing-latest-and-honest.md` Tasks 1–6 (committed: `a7004ce11` … `5ed86c47e`)
and `2026-09-28-model-routing-chat-lane.md` Task 1 committed (it edits `select.rs#select_via_scorer`; Task 5 here
edits `select.rs#decide`). Tasks 8–11 additionally need the **whole** `2026-09-28-chat-turn-trace.md` committed
(its Tasks 4, 5 and 6 all edit `lib/turnEvents.ts`) <!-- AMENDED: G12 -->. <!-- AMENDED: R15 — shared files found by
the review: --> Task 7 needs trace Task 3 committed (both edit `vox-gui/src/commands/models.rs` and
`ui/src/types/tauri.ts`); Task 9 needs `2026-09-28-chat-surfaces-consolidation.md` Task 8 committed (both edit
`e2e/lib/tauriMock.ts` at the `get_routing_summary_live` case); Task 10 needs the surfaces plan's Task 2 and
`2026-09-28-chat-visual-language.md` Tasks 1 and 4–6 committed (status bar, `App.tsx`).

## Global Constraints

- No new crate, crate edge or dependency. `vox-cli` and `vox-gui` already depend on `vox-orchestrator`.
- No versioned cloud model id literal in code, tests, mocks or fixtures. Fixtures use the fictional `acme/widget-*`
  and `acme/m-*` ids.
- `contracts/orchestration/model-pins.v1.yaml` (council-ratified) and `model-routing.v1.yaml` are not edited.
- `models/pareto.rs` is not used for selection (ADR-046).
- Test-first for every task: write the tests, run them, and save the failing output to
  `target/routing-sm-t<N>-red.txt` **before** touching implementation code. Every guard gets a mutation proof, run by
  Claude Code after the agent finishes (the agent does not run mutation proofs).
- Commands are foreground and `timeout`-prefixed: cargo `timeout 1500s`, pnpm/vitest/playwright `timeout 300s`.
  Cargo is always scoped with `-p`; one test filter before `--`, several after it. Format each changed `.rs` file
  with `rustfmt --edition 2024 <file>`; never `cargo fmt`.
- Run only the targeted tests plus one full `cargo test -p <crate> --lib` at the end of a Rust task. Do **not** run
  `--tests` integration suites (Claude does).
- tdd-guard scans whole files: every `.rs` file you touch that has a `pub fn` must contain an in-file
  `#[cfg(test)]` test, in a module compiled with default features (confirm with
  `cargo test -p <crate> --lib -- --list`).
- Existing tests are not edited except where a task names the edit ("sanctioned edit"). Any other existing test
  that fails is a STOP: report its name and failing assertion.
- Files over 500 non-blank lines (`registry.rs`, `select.rs`, `catalog.rs`, `catalog_refresh.rs`,
  `vox-gui/src/commands/models.rs`) get only the edits shown; new code goes in new files.
- The agent never runs `git add` / `git commit`.

## GUI design rules (Tasks 7–11)

These are the observability principles the explainer is built to; each is checked by a test.

1. **Explain from the decision, never beside it.** Every number shown comes from `Ranking` / `ScoreParts` / the
   registry. Nothing is recomputed in TypeScript except display formatting.
2. **Progressive disclosure.** One line answers "which model and why" (chosen model, mode sentence); the ranked
   table answers "compared with what"; a collapsed `<details>` answers "why not the others" (exclusions grouped by
   reason, with counts and at most three examples); health is a one-line footer that only draws attention when
   something is wrong.
3. **Provenance on every number.** Quality reads "benchmark 46.3", "from acme/widget-5.0 (38.2)" or "estimate";
   price reads "$0.90/M" or "—" with "price not published". A missing value is "—", never 0.
4. **Plain language, one vocabulary.** Mode names and objectives come from `lib/turnEvents.ts` (the trace plan's
   label owner); exclusion reasons from one map in `lib/routingLabels.ts`. No `intel=`, `eff=`, `lat=` or other
   code names in the UI.
5. **Honest scope.** The panel explains *how task dispatch would choose now* and says so; it names the live gates it
   cannot see (today's exploration budget, provider usage limits) and the age of its telemetry. It does not explain
   chat, which uses a different selector, and never claims to replay a past turn. <!-- AMENDED: G8, G9, G13 -->
6. **Accessible and stable.** A real `<table>` with a caption and `scope="col"` headers; the chosen row is marked in
   text ("Chosen"), not by colour alone; score bars are `aria-hidden` with the number beside them;
   `tabular-nums` for numeric columns; colours only via `var(--color-*)` tokens (`--color-status-warn` for health
   problems); no layout shift when the panel loads (fixed-height skeleton).

## File Structure

| File | Status | Tasks | Responsibility |
|---|---|---|---|
| `crates/vox-orchestrator/src/models/reference.rs` | create | 1a | `RoutingReference`, `ReferenceSource`, `derive`, `quality_prior` |
| `crates/vox-orchestrator/src/models/spec.rs` | modify | 1a, 2 | `QualityPrior`, `QualitySource`; `quality_prior` (1a) and `is_alias_target` (2) fields |
| `crates/vox-orchestrator/src/models/scoring.rs` | modify | 1a, 3 | `proxy_quality`; `quality_score` reads the stamp (1a); `ScoreParts`, `auto_score_parts` (3) |
| `crates/vox-orchestrator/src/models/tiering.rs` | modify | 1a | `derive_tier_with` |
| `crates/vox-orchestrator/src/models/mod.rs` | modify | 1a, 4, 6 | `pub mod reference;` `pub mod ranking;` `pub mod health;` |
| `crates/vox-research-shim/src/selection/virtual_models.rs` | modify | 1a, 2 | the two `ModelCapabilities` literals |
| `crates/vox-orchestrator/src/models/registry.rs` | modify | 1b, 2, 4 | `reference` field, stamping in `register`, `apply_routing_reference` (1b); inheritance (2); `matches_strength` visibility, `best_for_internal` / `best_for_task_with_filter` delegate to the ranking (4) |
| `crates/vox-orchestrator/src/orchestrator/catalog_refresh.rs` | modify | 1b, 6 | `apply_routing_reference` after each merge (1b); health check + persistence (6) |
| `crates/vox-orchestrator/src/models/tests.rs` | modify | 1b, 2 | `routing_reference_tests`, `family_inheritance_tests` |
| `crates/vox-orchestrator/src/models/family.rs` | modify | 2 | `join_key`, `family_benchmarks`; alias targets never superseded |
| `crates/vox-orchestrator/src/catalog.rs` | modify | 2 | parse `alias_target`; flag alias targets |
| `crates/vox-orchestrator/src/models/ranking.rs` | create | 4, 5 | `Exclusion`, `RankedModel`, `Ranking`, `rank_with_filter`, `rank_pass`, `rank_task_with_filter` |
| `crates/vox-orchestrator/src/models/mode_select.rs` | modify | 5 | `ModeSelection.ranking`; exclusion reasons |
| `crates/vox-orchestrator/src/models/select.rs` | modify | 5 | `decide()`: ranked `alternatives`, `ranking` field |
| `crates/vox-orchestrator/src/models/health.rs` | create | 6 | `RoutingHealth`, `Violation`, `check_routing_health`, preference keys |
| `crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/model_catalog.rs` | modify | 6 | correct key; "Model routing" check |
| `crates/vox-gui/src/commands/routing_explain.rs` | create | 7 | DTOs, `explanation_dto`, `explain_routing`, `get_routing_health` |
| `crates/vox-gui/src/commands/mod.rs`, `crates/vox-gui/src/main.rs` | modify | 7 | module + two handler entries |
| `crates/vox-gui/src/commands/models.rs` | modify | 7 | `registry_with_scoreboard` becomes `pub(crate)` (one word) |
| `crates/vox-gui/ui/src/types/tauri.ts` | modify | 7 | TS mirror of the DTOs |
| `crates/vox-gui/ui/src/lib/routingLabels.ts` + `.test.ts` | create | 8 | exclusion labels, quality/price formatting |
| `crates/vox-gui/ui/src/lib/turnEvents.ts` | modify | 8 | `MODE_OBJECTIVES`, `modeObjective` (extends the trace plan's label owner) |
| `crates/vox-gui/ui/src/hooks/useRoutingExplanation.ts` + `.test.ts` | create | 8 | fetch explanation + health |
| `crates/vox-gui/ui/src/components/surfaces/Models/RoutingExplainer.tsx` + `.test.tsx` | create | 8 | the panel |
| `crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx` + `.test.tsx` | modify | 9 | explainer replaces Decision Preview; `$/M` units |
| `crates/vox-gui/ui/e2e/lib/tauriMock.ts` | modify | 9 | default responses for the two commands |
| `crates/vox-gui/ui/e2e/routing-explainer.spec.ts` | create | 11 | Playwright + screenshots |

---

### Task 1a: `RoutingReference` — derived scales with provenance and fallbacks (pure)

**Files:** Create `crates/vox-orchestrator/src/models/reference.rs`. Modify `models/spec.rs` (new types after
`ModelCapabilities`; new field after `intelligence_index`), `models/scoring.rs` (`quality_score`, new
`proxy_quality`), `models/tiering.rs` (new `derive_tier_with`), `models/mod.rs` (`pub mod reference;` in
alphabetical position), `crates/vox-research-shim/src/selection/virtual_models.rs` (both `ModelCapabilities { … }`
literals get `quality_prior: None,`).

**Interfaces — produces:** `spec::QualityPrior { value: f64, source: QualitySource }`,
`spec::QualitySource::{Benchmark { index }, Inherited { index, from }, Estimate}`,
`ModelCapabilities::quality_prior: Option<QualityPrior>`, `reference::{RoutingReference, ReferenceSource,
MIN_DERIVE_SAMPLE, ELITE_PRICE_QUANTILE, PRO_PRICE_QUANTILE, INHERITED_INDEX_DISCOUNT}`,
`RoutingReference::{FALLBACK, derive, quality_prior}`, `scoring::proxy_quality` (`pub(super)`),
`tiering::derive_tier_with`.

- [ ] **Step 1: Write the failing tests** — create `reference.rs` containing only this module (the implementation
  goes above it in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::spec::{ModelCapabilities, PricingSource, QualityPrior, QualitySource};
    use crate::models::StrengthTag;

    fn spec(id: &str, provider_type: ProviderType, out_per_1k: f64, index: Option<f32>) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 128_000,
            cost_per_1k: out_per_1k,
            cost_per_1k_input: out_per_1k / 4.0,
            cost_per_1k_output: out_per_1k,
            is_free: out_per_1k == 0.0,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities { intelligence_index: index, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::OpenRouter,
            supported_parameters: vec![],
        }
    }

    /// 40 paid OpenRouter models: output prices 0.0005 … 0.0395 per 1k, indexes 10 … 49.
    fn catalog(price_scale: f64, index_scale: f32) -> Vec<ModelSpec> {
        (0..40)
            .map(|i| {
                spec(
                    &format!("acme/m-{i}"),
                    ProviderType::OpenRouter,
                    (0.0005 + 0.001 * f64::from(i)) * price_scale,
                    Some((10 + i) as f32 * index_scale),
                )
            })
            .collect()
    }

    #[test]
    fn a_small_catalog_uses_the_fallback_constants() {
        let small: Vec<ModelSpec> = catalog(1.0, 1.0).into_iter().take(MIN_DERIVE_SAMPLE - 1).collect();
        assert_eq!(RoutingReference::derive(&small), RoutingReference::FALLBACK);
        assert_eq!(RoutingReference::derive(std::iter::empty::<&ModelSpec>()), RoutingReference::FALLBACK);
    }

    #[test]
    fn quality_reference_is_the_top_benchmark_and_follows_a_rescale() {
        let r = RoutingReference::derive(&catalog(1.0, 1.0));
        assert_eq!(r.quality_source, ReferenceSource::Derived);
        assert_eq!(r.quality_reference, 49.0);
        assert_eq!(r.benchmarked, 40);
        assert_eq!(RoutingReference::derive(&catalog(1.0, 2.0)).quality_reference, 98.0);
    }

    #[test]
    fn bands_are_the_configured_quantiles_of_paid_prices() {
        // n = 40: Elite at floor(0.93 * 39) = 36, Pro at floor(0.70 * 39) = 27.
        let r = RoutingReference::derive(&catalog(1.0, 1.0));
        assert_eq!(r.bands_source, ReferenceSource::Derived);
        assert!((r.elite_min_out - 0.0365).abs() < 1e-12, "{}", r.elite_min_out);
        assert!((r.pro_min_out - 0.0275).abs() < 1e-12, "{}", r.pro_min_out);
        assert_eq!(r.priced, 40);
    }

    #[test]
    fn tiers_survive_a_market_wide_price_move() {
        let base = RoutingReference::derive(&catalog(1.0, 1.0));
        let halved = RoutingReference::derive(&catalog(0.5, 1.0));
        assert!((halved.elite_min_out - base.elite_min_out * 0.5).abs() < 1e-12);
        assert!((halved.pro_min_out - base.pro_min_out * 0.5).abs() < 1e-12);
        for (a, b) in catalog(1.0, 1.0).iter().zip(catalog(0.5, 1.0).iter()) {
            assert_eq!(
                crate::models::tiering::derive_tier_with(false, a.cost_per_1k_output, base.elite_min_out, base.pro_min_out),
                crate::models::tiering::derive_tier_with(false, b.cost_per_1k_output, halved.elite_min_out, halved.pro_min_out),
                "{}",
                a.id
            );
        }
    }

    #[test]
    fn free_and_local_models_do_not_shape_the_reference() {
        let mut specs = catalog(1.0, 1.0);
        let base = RoutingReference::derive(&specs);
        for i in 0..50 {
            specs.push(spec(&format!("acme/free-{i}"), ProviderType::OpenRouter, 0.0, None));
            specs.push(spec(&format!("local/big-{i}"), ProviderType::Ollama, 1.0, Some(99.0)));
        }
        assert_eq!(RoutingReference::derive(&specs), base);
    }

    #[test]
    fn unbenchmarked_scale_is_fitted_to_benchmarked_models() {
        let specs = catalog(1.0, 1.0);
        let r = RoutingReference::derive(&specs);
        // Median of index/top over 10..=49 is 29.5/49; every proxy is equal (same context, all paid).
        let proxy = crate::models::scoring::proxy_quality(&specs[0]);
        let expected = ((29.5 / 49.0) / proxy).clamp(0.3, 1.0);
        assert!((r.unbenchmarked_scale - expected).abs() < 1e-12, "{} vs {expected}", r.unbenchmarked_scale);
    }

    #[test]
    fn quality_prior_says_where_it_came_from() {
        let r = RoutingReference::derive(&catalog(1.0, 1.0));
        let benched = r.quality_prior(&spec("acme/b", ProviderType::OpenRouter, 0.01, Some(24.5)), None);
        assert!((benched.value - 0.5).abs() < 1e-12);
        assert_eq!(benched.source, QualitySource::Benchmark { index: 24.5 });
        let unbenched = spec("acme/u", ProviderType::OpenRouter, 0.01, None);
        let inherited = r.quality_prior(&unbenched, Some((24.5, "acme/b")));
        assert!((inherited.value - f64::from(24.5_f32 * INHERITED_INDEX_DISCOUNT) / 49.0).abs() < 1e-6); // <!-- AMENDED: R1 — f32 constant, f64 value -->
        assert_eq!(inherited.source, QualitySource::Inherited { index: 24.5, from: "acme/b".into() });
        let estimate_or = r.quality_prior(&unbenched, None);
        let estimate_direct = r.quality_prior(&spec("acme/d", ProviderType::Anthropic, 0.01, None), None);
        assert_eq!(estimate_or.source, QualitySource::Estimate);
        assert!(estimate_or.value < estimate_direct.value, "only OpenRouter's long tail is scaled down");
    }

    #[test]
    fn the_fallback_prior_equals_the_unstamped_formula() {
        for m in [
            spec("acme/b", ProviderType::OpenRouter, 0.01, Some(47.5)),
            spec("acme/u", ProviderType::OpenRouter, 0.01, None),
            spec("acme/d", ProviderType::Anthropic, 0.01, None),
            spec("local/q", ProviderType::Ollama, 0.0, None),
        ] {
            assert_eq!(
                RoutingReference::FALLBACK.quality_prior(&m, None).value,
                crate::models::scoring::quality_score(&m),
                "{}",
                m.id
            );
        }
    }

    #[test]
    fn quality_score_reads_a_stamped_prior() {
        let mut m = spec("acme/s", ProviderType::OpenRouter, 0.01, Some(47.5));
        m.capabilities.quality_prior = Some(QualityPrior { value: 0.123, source: QualitySource::Estimate });
        assert_eq!(crate::models::scoring::quality_score(&m), 0.123);
    }
}
```

Add `pub mod reference;` to `models/mod.rs` now so the module compiles.

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::reference > target/routing-sm-t1a-red.txt 2>&1; tail -20 target/routing-sm-t1a-red.txt`
Expected: FAIL to compile — `cannot find type RoutingReference`, `QualityPrior`, `proxy_quality`, `derive_tier_with`.

- [ ] **Step 3: Types in `spec.rs`.** Directly after the closing `}` of `impl ModelCapabilities { … }` add:

```rust
/// A model's quality prior (0–1) and where it came from, so every surface can say why a model is
/// rated as it is. Stamped by the registry from its `models::reference::RoutingReference`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityPrior {
    pub value: f64,
    pub source: QualitySource,
}

/// Provenance of a [`QualityPrior`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QualitySource {
    /// The model's own Artificial Analysis intelligence index.
    Benchmark { index: f32 },
    /// The index of the newest benchmarked member of its family (`models::family`).
    Inherited { index: f32, from: String },
    /// No benchmark: the context-length and paid/free proxy.
    Estimate,
}
```

and inside `pub struct ModelCapabilities`, directly after the `intelligence_index` field:

```rust
    /// Quality prior stamped by the registry (`ModelRegistry::register` / `apply_routing_reference`).
    /// `None` on a spec that was never registered; scoring then uses the fixed-constant formula.
    #[serde(default)]
    pub quality_prior: Option<QualityPrior>,
```

Add `quality_prior: None,` to both `ModelCapabilities { … }` literals in `virtual_models.rs` (after
`intelligence_index: None,`). If `cargo check -p vox-orchestrator -p vox-research-shim` names any other literal,
STOP and list it.

- [ ] **Step 4: `tiering.rs`.** Replace the body of `derive_tier` with a call, and add `derive_tier_with` below it:

```rust
pub fn derive_tier(is_free: bool, cost_per_1k_output: f64) -> ModelTier {
    derive_tier_with(
        is_free,
        cost_per_1k_output,
        ELITE_MIN_OUTPUT_USD_PER_1K,
        PRO_MIN_OUTPUT_USD_PER_1K,
    )
}

/// [`derive_tier`] with explicit bands (USD per 1k output tokens), e.g. a registry's derived
/// `RoutingReference`.
#[must_use]
pub fn derive_tier_with(
    is_free: bool,
    cost_per_1k_output: f64,
    elite_min_out: f64,
    pro_min_out: f64,
) -> ModelTier {
    if is_free {
        return ModelTier::Free;
    }
    if !cost_per_1k_output.is_finite() || cost_per_1k_output <= 0.0 {
        return ModelTier::Unknown;
    }
    if cost_per_1k_output >= elite_min_out {
        ModelTier::Elite
    } else if cost_per_1k_output >= pro_min_out {
        ModelTier::Pro
    } else {
        ModelTier::Fast
    }
}
```

(keep `derive_tier`'s doc comment and `#[must_use]`; delete its old `ponytail:` line, which this plan resolves).

- [ ] **Step 5: `scoring.rs` — add `proxy_quality` only.** <!-- AMENDED: R2 — the fallback-equivalence test must
  compare against the *old* `quality_score`, so `quality_score` is not touched until Step 6c. --> Add, directly after
  `quality_score` (leave `quality_score` unchanged for now):

```rust
/// The unscaled context-length + paid/free proxy used for a model with no benchmark.
#[must_use]
pub(super) fn proxy_quality(m: &ModelSpec) -> f64 {
    let token_component = (m.max_tokens as f64).log10().clamp(1.0, 7.0) / 7.0;
    let paid_component = if m.is_free {
        QUALITY_FREE_PAID_COMPONENT
    } else {
        QUALITY_PAID_COMPONENT
    };
    (token_component * QUALITY_TOKEN_WEIGHT) + (paid_component * QUALITY_PAID_WEIGHT)
}
```

Update the doc comment of `QUALITY_INDEX_REFERENCE` to "Fallback for `RoutingReference::quality_reference` …" and
delete its `ponytail:` line; update `UNBENCHMARKED_QUALITY_SCALE`'s first line to "Fallback for
`RoutingReference::unbenchmarked_scale`: scale applied …".

- [ ] **Step 6: Implement `reference.rs`** (above the test module):

```rust
//! Routing scales derived from the live catalog, so a benchmark rescale or a market-wide price move
//! changes routing without a code edit. Each value falls back to the fixed constant it replaces when
//! the catalog is too small to derive from (offline bootstrap, small test registries). The registry
//! holds the current reference and stamps each spec's `QualityPrior` from it.

use serde::{Deserialize, Serialize};

use super::spec::{QualityPrior, QualitySource};
use super::{ModelSpec, ProviderType};

/// Fewest benchmarked (or priced) cloud models a value is derived from.
pub const MIN_DERIVE_SAMPLE: usize = 20;
/// Paid output-price quantiles for the Elite and Pro bands. On the 2026-09-28 OpenRouter catalog
/// (343 paid models) they reproduce the former fixed bands exactly: $20/M at 0.93, $4/M at 0.70.
pub const ELITE_PRICE_QUANTILE: f64 = 0.93;
pub const PRO_PRICE_QUANTILE: f64 = 0.70;
/// Weight on an index inherited from an older family member.
// ponytail: flat discount; learn it from outcomes once the family-keyed scoreboard lands.
pub const INHERITED_INDEX_DISCOUNT: f32 = 0.95;
const UNBENCHMARKED_SCALE_MIN: f64 = 0.3;
const UNBENCHMARKED_SCALE_MAX: f64 = 1.0;

/// Whether a value was derived from the catalog or is the built-in fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSource {
    Derived,
    Fallback,
}

/// The catalog-derived scales routing scores against.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RoutingReference {
    /// Intelligence index treated as quality 1.0.
    pub quality_reference: f64,
    /// Scale on the proxy for unbenchmarked OpenRouter models.
    pub unbenchmarked_scale: f64,
    pub quality_source: ReferenceSource,
    /// USD per 1k output tokens at or above which a paid model is Elite.
    pub elite_min_out: f64,
    /// USD per 1k output tokens at or above which a paid model is Pro.
    pub pro_min_out: f64,
    pub bands_source: ReferenceSource,
    /// Benchmarked OpenRouter models the quality values came from (0 on fallback).
    pub benchmarked: usize,
    /// Paid, priced cloud models the bands came from (0 on fallback).
    pub priced: usize,
}

impl Default for RoutingReference {
    fn default() -> Self {
        Self::FALLBACK
    }
}

impl RoutingReference {
    /// The fixed constants this module replaces.
    pub const FALLBACK: Self = Self {
        quality_reference: super::scoring::QUALITY_INDEX_REFERENCE,
        unbenchmarked_scale: super::scoring::UNBENCHMARKED_QUALITY_SCALE,
        quality_source: ReferenceSource::Fallback,
        elite_min_out: super::tiering::ELITE_MIN_OUTPUT_USD_PER_1K,
        pro_min_out: super::tiering::PRO_MIN_OUTPUT_USD_PER_1K,
        bands_source: ReferenceSource::Fallback,
        benchmarked: 0,
        priced: 0,
    };

    /// Derive from registered specs. Local models never shape the reference; free models
    /// shape the quality scale but not the price bands.
    #[must_use]
    pub fn derive<'a>(specs: impl IntoIterator<Item = &'a ModelSpec>) -> Self {
        let mut out = Self::FALLBACK;
        let mut indexed: Vec<(f64, f64)> = Vec::new();
        let mut paid: Vec<f64> = Vec::new();
        for m in specs {
            if is_local(m) {
                continue;
            }
            if let Some(i) = m.capabilities.intelligence_index.filter(|i| i.is_finite() && *i > 0.0) {
                if m.provider_type == ProviderType::OpenRouter {
                    indexed.push((f64::from(i), super::scoring::proxy_quality(m)));
                }
            }
            if !m.is_free && m.cost_per_1k_output.is_finite() && m.cost_per_1k_output > 0.0 {
                paid.push(m.cost_per_1k_output);
            }
        }
        if indexed.len() >= MIN_DERIVE_SAMPLE {
            let top = indexed.iter().map(|(i, _)| *i).fold(f64::MIN, f64::max);
            let norm = median(indexed.iter().map(|(i, _)| i / top).collect());
            let proxy = median(indexed.iter().map(|(_, p)| *p).collect());
            out.quality_reference = top;
            if proxy > 0.0 {
                out.unbenchmarked_scale =
                    (norm / proxy).clamp(UNBENCHMARKED_SCALE_MIN, UNBENCHMARKED_SCALE_MAX);
            }
            out.quality_source = ReferenceSource::Derived;
            out.benchmarked = indexed.len();
        }
        if paid.len() >= MIN_DERIVE_SAMPLE {
            paid.sort_by(f64::total_cmp);
            out.elite_min_out = quantile(&paid, ELITE_PRICE_QUANTILE);
            out.pro_min_out = quantile(&paid, PRO_PRICE_QUANTILE);
            out.bands_source = ReferenceSource::Derived;
            out.priced = paid.len();
        }
        out
    }

    /// Quality prior for `m`. `inherited` is its family's benchmark `(index, from_id)`, used only
    /// when `m` has no index of its own.
    #[must_use]
    pub fn quality_prior(&self, m: &ModelSpec, inherited: Option<(f32, &str)>) -> QualityPrior {
        if let Some(index) = m.capabilities.intelligence_index {
            return QualityPrior { value: self.normalise(f64::from(index)), source: QualitySource::Benchmark { index } };
        }
        if let Some((index, from)) = inherited {
            return QualityPrior {
                value: self.normalise(f64::from(index * INHERITED_INDEX_DISCOUNT)),
                source: QualitySource::Inherited { index, from: from.to_string() },
            };
        }
        let scale = if m.provider_type == ProviderType::OpenRouter { self.unbenchmarked_scale } else { 1.0 };
        QualityPrior {
            value: (super::scoring::proxy_quality(m) * scale).clamp(0.0, 1.0),
            source: QualitySource::Estimate,
        }
    }

    fn normalise(&self, index: f64) -> f64 {
        (index / self.quality_reference).clamp(0.0, 1.0)
    }
}

fn is_local(m: &ModelSpec) -> bool {
    crate::route_policy::is_local_http_provider(&m.provider_type)
}

/// Value at `floor(q * (n - 1))` of an ascending, non-empty slice.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[((q * (sorted.len() - 1) as f64).floor() as usize).min(sorted.len() - 1)]
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}
```

`QUALITY_INDEX_REFERENCE` and `UNBENCHMARKED_QUALITY_SCALE` are `pub(super)` in `scoring.rs`, visible here. If
`median` of the benchmark test gives `29.5/49` only to within 1e-15 that is expected (the test allows 1e-12).
<!-- AMENDED: R2/C3 — `is_local` below reuses the canonical `route_policy::is_local_http_provider`. -->

- [ ] **Step 6b: Prove the fallback prior equals the old formula** (still before switching):
  `timeout 1500s cargo test -p vox-orchestrator --lib models::reference::tests::the_fallback_prior_equals_the_unstamped_formula > target/routing-sm-t1a-equivalence.txt 2>&1; tail -5 target/routing-sm-t1a-equivalence.txt`.
  Expected: PASS against the unchanged `quality_score`. If it fails, STOP and report the spec and both values.
- [ ] **Step 6c: `quality_score` reads the stamp, else the fallback prior** (one formula, not two). Replace the body of
  `quality_score` with:

```rust
    m.capabilities.quality_prior.as_ref().map_or_else(
        || super::reference::RoutingReference::FALLBACK.quality_prior(m, None).value,
        |prior| prior.value,
    )
```

- [ ] **Step 7: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::reference models::tiering models::scoring 2>&1 | tail -15 && timeout 1500s cargo check -p vox-research-shim 2>&1 | tail -3 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5`
Expected: all pass (the fallback-equivalence test proves the `quality_score` refactor changed nothing for
unstamped specs).

- [ ] **Step 8 (Claude): mutation proofs** — each must fail the named test, then restore:
  (a) `ELITE_PRICE_QUANTILE = 0.93` → `0.90` → `bands_are_the_configured_quantiles_of_paid_prices`;
  (b) delete `if is_local(m) { continue; }` → `free_and_local_models_do_not_shape_the_reference`;
  (c) in `quality_prior`, `self.unbenchmarked_scale` → `1.0` → `quality_prior_says_where_it_came_from`;
  (d) in `quality_score`, always use the fallback (ignore the stamp) → `quality_score_reads_a_stamped_prior`;
  (e) `indexed.len() >= MIN_DERIVE_SAMPLE` → `> 0` → `a_small_catalog_uses_the_fallback_constants`.

- [ ] **Step 9: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/reference.rs crates/vox-orchestrator/src/models/spec.rs crates/vox-orchestrator/src/models/scoring.rs crates/vox-orchestrator/src/models/tiering.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-research-shim/src/selection/virtual_models.rs
git commit -m "feat(models): derive routing scales and price bands from the live catalog"
```

---

### Task 1b: The registry holds the reference, stamps priors, and re-derives after every catalog merge

**Files:** Modify `models/registry.rs` (struct `ModelRegistry`, its three `ModelRegistry { … }` literals in
`maybe_refresh_catalogs`, `from_cache` and `new`, `register`, `from_cache`'s and `new`'s tails, and new methods
`routing_reference` / `apply_routing_reference` placed directly after `register`),
`orchestrator/catalog_refresh.rs` (`refresh_once`, `run_unified_catalog_refresh`, a key-pin test module). Test:
`models/tests.rs` (new module at the end). <!-- AMENDED: R4 — telemetry.rs is no longer touched. -->

**Interfaces — produces:** `ModelRegistry::routing_reference(&self) -> RoutingReference`,
`ModelRegistry::apply_routing_reference(&mut self) -> RoutingReference`.

- [ ] **Step 1: Write the failing tests** — append to `models/tests.rs`:

```rust
#[cfg(test)]
mod routing_reference_tests {
    use crate::models::reference::{ReferenceSource, RoutingReference};
    use crate::models::spec::{PricingSource, QualitySource};
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType, StrengthTag};

    fn spec(id: &str, provider_type: ProviderType, out: f64, index: Option<f32>, source: PricingSource) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 128_000,
            cost_per_1k: out,
            cost_per_1k_input: out / 4.0,
            cost_per_1k_output: out,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities { intelligence_index: index, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: source,
            supported_parameters: vec![],
        }
    }

    /// 40 paid, benchmarked OpenRouter models (derived bands: Elite 0.0365, Pro 0.0275 per 1k).
    fn catalog_registry() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        for i in 0..40 {
            r.register(spec(
                &format!("acme/m-{i}"),
                ProviderType::OpenRouter,
                0.0005 + 0.001 * f64::from(i),
                Some((10 + i) as f32),
                PricingSource::OpenRouter,
            ));
        }
        r
    }

    #[test]
    fn a_registry_starts_on_the_fallback_reference() {
        assert_eq!(ModelRegistry::default().routing_reference(), RoutingReference::FALLBACK);
    }

    #[test]
    fn register_stamps_a_quality_prior() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/b", ProviderType::OpenRouter, 0.01, Some(30.0), PricingSource::OpenRouter));
        let prior = r.get("acme/b").unwrap().capabilities.quality_prior.expect("stamped");
        assert_eq!(prior.source, QualitySource::Benchmark { index: 30.0 });
        assert!((prior.value - 0.5).abs() < 1e-12, "fallback reference is 60");
    }

    #[test]
    fn apply_derives_the_reference_and_restamps_priors() {
        let mut r = catalog_registry();
        let before = r.get("acme/m-39").unwrap().capabilities.quality_prior.unwrap().value;
        let reference = r.apply_routing_reference();
        assert_eq!(reference.quality_source, ReferenceSource::Derived);
        assert_eq!(r.routing_reference(), reference);
        let after = r.get("acme/m-39").unwrap().capabilities.quality_prior.unwrap().value;
        assert!((before - 49.0 / 60.0).abs() < 1e-12);
        assert!((after - 1.0).abs() < 1e-12, "the top benchmark is 1.0 under the derived reference");
    }

    #[test]
    fn apply_restamps_priced_tiers_and_nothing_else() {
        // <!-- AMENDED: G3 — no production path yields UserConfig for models.toml entries (serde
        // defaults them to Bootstrap), LiteLLM relabels non-Telemetry specs (registry.rs:350), and
        // Telemetry is the highest-trust price: the tier follows the best-known price. -->
        let mut r = catalog_registry();
        let mut hand = spec("acme/hand-tiered", ProviderType::OpenRouter, 0.0385, None, PricingSource::Bootstrap);
        hand.capabilities.tier = ModelTier::Pro;
        let mut unpriced = spec("acme/direct-flagship", ProviderType::Anthropic, 0.0, None, PricingSource::AnthropicDirect);
        unpriced.capabilities.tier = ModelTier::Elite;
        let mut stale = spec("acme/m-stale", ProviderType::OpenRouter, 0.0385, None, PricingSource::OpenRouter);
        stale.capabilities.tier = ModelTier::Fast;
        let mut observed = spec("acme/observed", ProviderType::OpenRouter, 0.0385, None, PricingSource::Telemetry);
        observed.capabilities.tier = ModelTier::Fast;
        r.register(hand);
        r.register(unpriced);
        r.register(stale);
        r.register(observed);
        r.apply_routing_reference();
        assert_eq!(r.get("acme/hand-tiered").unwrap().capabilities.tier, ModelTier::Pro, "an entry no catalog prices keeps its tier");
        assert_eq!(r.get("acme/direct-flagship").unwrap().capabilities.tier, ModelTier::Elite, "an unknown price keeps the tier");
        assert_eq!(r.get("acme/m-stale").unwrap().capabilities.tier, ModelTier::Elite, "a catalog price decides the tier");
        assert_eq!(r.get("acme/observed").unwrap().capabilities.tier, ModelTier::Elite, "an observed price decides the tier");
    }

    // <!-- AMENDED: R4 — observed prices re-band inside inject_pricing_catalog. -->
    /// A high-confidence observed price (`ModelPricingCatalogRow` derives no `Default`).
    fn observed_row(model_id: &str, per_1k: f64) -> vox_db::store::types::ModelPricingCatalogRow {
        vox_db::store::types::ModelPricingCatalogRow {
            model_id: model_id.into(),
            provider: "acme".into(),
            observed_blended_per_1k: Some(per_1k),
            observed_input_per_1k: Some(per_1k / 4.0),
            observed_output_per_1k: Some(per_1k),
            catalog_input_per_1k: 0.00025,
            catalog_output_per_1k: 0.001,
            n_provider_reported: 10,
            n_estimated: 0,
            n_free: 0,
            confidence: "high".into(),
            last_observed_at_ms: Some(0),
            updated_at_ms: 0,
        }
    }
    #[test]
    fn injecting_observed_prices_re_derives_the_tier() {
        let mut r = catalog_registry();
        r.register(spec("acme/listed", ProviderType::OpenRouter, 0.001, None, PricingSource::OpenRouter));
        r.apply_routing_reference();
        assert_eq!(r.get("acme/listed").unwrap().capabilities.tier, ModelTier::Fast);
        r.inject_pricing_catalog(vec![observed_row("acme/listed", 0.05)]);
        assert_eq!(r.get("acme/listed").unwrap().capabilities.tier, ModelTier::Elite, "the observed price decides");
    }

    #[test]
    fn a_model_registered_after_apply_uses_the_derived_bands() {
        let mut r = catalog_registry();
        r.apply_routing_reference();
        r.register(spec("acme/new", ProviderType::OpenRouter, 0.025, None, PricingSource::OpenRouter));
        assert_eq!(crate::models::tiering::derive_tier(false, 0.025), ModelTier::Elite, "fixed bands say Elite");
        assert_eq!(r.get("acme/new").unwrap().capabilities.tier, ModelTier::Fast, "derived bands say Fast");
    }
}
```

(With the 40 catalog prices plus the three priced additions at 0.0385 — `hand-tiered` counts toward the bands even
though its tier is not restamped — there are 43 paid prices; index `floor(0.93 × 42)` = 39 holds 0.0385, so Elite.)

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::tests::routing_reference_tests > target/routing-sm-t1b-red.txt 2>&1; tail -20 target/routing-sm-t1b-red.txt`
Expected: FAIL to compile — `no method named routing_reference` / `apply_routing_reference`.

- [ ] **Step 3: Implement in `registry.rs`.**
  1. In `pub struct ModelRegistry`, after `penalty_map`, add:
     ```rust
         /// Catalog-derived scales (`models::reference`); re-derived by [`Self::apply_routing_reference`].
         reference: super::reference::RoutingReference,
     ```
     and add `reference: super::reference::RoutingReference::FALLBACK,` after `penalty_map: HashMap::new(),` in
     each of the three `ModelRegistry { … }` literals (`maybe_refresh_catalogs`, `from_cache`, `new`).
  2. In `register`, replace
     `super::tiering::derive_tier(spec.is_free, spec.cost_per_1k_output);` with
     ```rust
                 super::tiering::derive_tier_with(
                     spec.is_free,
                     spec.cost_per_1k_output,
                     self.reference.elite_min_out,
                     self.reference.pro_min_out,
                 );
     ```
     and directly after the closing `}` of that `if` add:
     ```rust
             spec.capabilities.quality_prior = Some(self.reference.quality_prior(&spec, None));
     ```
  3. Directly after `register`, add:
     ```rust
         /// The routing reference this registry scores against.
         #[must_use]
         pub fn routing_reference(&self) -> super::reference::RoutingReference {
             self.reference
         }

         /// Re-derive the routing reference from the registered models, restamp every model's
         /// quality prior, and re-derive the tier of every model whose price came from a live
         /// catalog or from observed spend (an entry no catalog prices, and an unknown price, keep
         /// their tier). Call after every catalog merge and after injecting observed prices.
         pub fn apply_routing_reference(&mut self) -> super::reference::RoutingReference {
             use super::spec::PricingSource;
             let reference = super::reference::RoutingReference::derive(self.models.values());
             self.reference = reference;
             for spec in self.models.values_mut() {
                 let prior = reference.quality_prior(spec, None);
                 spec.capabilities.quality_prior = Some(prior);
                 // <!-- AMENDED: G3 — Telemetry (observed price) joins the re-derive set. -->
                 let priced = matches!(
                     spec.pricing_source,
                     PricingSource::OpenRouter
                         | PricingSource::LiteLLM
                         | PricingSource::AnthropicDirect
                         | PricingSource::Telemetry
                 );
                 let local = matches!(
                     spec.provider_type,
                     ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal
                 );
                 if priced && !local {
                     let tier = super::tiering::derive_tier_with(
                         spec.is_free,
                         spec.cost_per_1k_output,
                         reference.elite_min_out,
                         reference.pro_min_out,
                     );
                     if tier != super::ModelTier::Unknown {
                         spec.capabilities.tier = tier;
                     }
                 }
             }
             reference
         }
     ```
  4. In `from_cache`, directly after `registry.register_mens_local_candidates();` add
     `registry.apply_routing_reference();`. In `new`, directly after its `registry.register_mens_local_candidates();`
     add:
     ```rust
             // Test builds stay on the fixed constants (hermetic, like `maybe_refresh_catalogs`).
             #[cfg(not(test))]
             registry.apply_routing_reference();
     ```
- [ ] **Step 4: Wire the refresh paths in `catalog_refresh.rs`.** In `refresh_once`, inside the write-lock block,
  directly after the `if !litellm_entries.is_empty() { registry.apply_litellm_pricing(&litellm_entries); }` block add
  `registry.apply_routing_reference();`. In `run_unified_catalog_refresh`, directly before
  `let mut snapshot = registry.list_models();` add `registry.apply_routing_reference();`.
  <!-- AMENDED: G13, R4 — observed prices must re-band too; the re-derive goes inside `inject_pricing_catalog`
  (registry.rs, which has tests) rather than in `telemetry.rs` (pub fns, no tests: tdd-guard would block it), so the
  orchestrator and the GUI registry both get it. --> In `registry.rs#inject_pricing_catalog`, after its `for` loop add
  `self.apply_routing_reference();`.
  <!-- AMENDED: R9 — specs promoted to Telemetry are re-registered after the merge and would lose an inherited prior. -->
  In `refresh_once`, directly after the `for m in &snapshot { … registry.register(m.clone()); … }` promotion loop
  (inside `Ok(count) if count > 0 => { … }`), add `registry.apply_routing_reference();`; in
  `run_unified_catalog_refresh`, directly after its `for m in &snapshot { … registry.register(m.clone()); … }` loop,
  add `registry.apply_routing_reference();`.
  <!-- AMENDED: R4 — catalog_refresh.rs has pub fns and no in-file test; tdd-guard scans whole staged files. --> At the end
  of `catalog_refresh.rs` add:

```rust
#[cfg(test)]
mod tests {
    /// `vox doctor` and the startup refresh read this key; the doctor once read a different one.
    #[test]
    fn the_refresh_timestamp_key_is_stable() {
        assert_eq!(super::MODEL_CATALOG_LAST_REFRESH_KEY, "model_catalog_last_refresh");
    }
}
```
- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::tests::routing_reference_tests 2>&1 | tail -12 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5`
Expected: all pass. `rg -c "apply_routing_reference\(\);" crates/vox-orchestrator/src/orchestrator/catalog_refresh.rs`
prints `4` (two merges, two promotion loops). Before the agent run, Claude runs the lefthook tdd-guard command
(`lefthook.yml` `tdd-guard`) on `catalog_refresh.rs` and `registry.rs` and confirms both pass with the new tests.

- [ ] **Step 6 (Claude): mutation proofs** — (a) in `register`, `self.reference.elite_min_out` →
  `super::tiering::ELITE_MIN_OUTPUT_USD_PER_1K` → `a_model_registered_after_apply_uses_the_derived_bands`;
  (b) delete `spec.capabilities.quality_prior = Some(prior);` → `apply_derives_the_reference_and_restamps_priors`;
  (c) `priced && !local` → `!local` → `apply_restamps_priced_tiers_and_nothing_else` (hand-tiered case);
  (d) delete `if tier != super::ModelTier::Unknown` guard (assign unconditionally) → same test (unknown price);
  (e) remove `| PricingSource::Telemetry` → same test (observed case). <!-- AMENDED: G3 -->
  (f) delete `self.apply_routing_reference();` from `inject_pricing_catalog` → `injecting_observed_prices_re_derives_the_tier`. <!-- AMENDED: R4 -->
  Then run `timeout 3000s cargo test -p vox-orchestrator --tests` once.
- [ ] **Step 6b (Claude): live pick comparison.** <!-- AMENDED: G4 — the quality side moves production picks from
  this task on; check them now, not only at Task 12. --> Build the registry the way production does — `ModelRegistry::from_cache()` after a real
  `vox model discover` (so bootstrap, AnthropicDirect/LiteLLM-priced and supplemental catalogs are in the population,
  not only the OpenRouter snapshot; <!-- AMENDED: R15/A15 -->) — and record the derived reference and the Efficient / Balanced / Genius picks at complexity 2, 5 and 10 next to the
  2026-09-29 record (Efficient cx10 `xiaomi/mimo-v2.6-pro`, Genius cx10 the Opus line). A **regression** is a Spec
  Feature 2 failure (an Elite pick in Efficient/Balanced while a non-Elite candidate fits) or a health-invariant
  failure; a changed pick is allowed, but its cause (which prior moved, by how much) goes in the commit body. The fix
  for a regression is a one-line constant in `reference.rs` (quantiles, clamp, discount), never a model name.
- [ ] **Step 7: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/orchestrator/catalog_refresh.rs crates/vox-orchestrator/src/models/tests.rs
git commit -m "feat(models): the registry re-derives its routing reference after every catalog merge"
```

---

### Task 2: New releases inherit their family's benchmark; alias targets are never superseded

**Files:** Modify `models/spec.rs` (field `is_alias_target`), `virtual_models.rs` (both literals get
`is_alias_target: false,`), `catalog.rs` (`OpenRouterModelData`, `specs_from_openrouter_json_at`, one test),
`models/family.rs` (`join_key`, `family_benchmarks`, `is_superseded`, tests), `models/registry.rs`
(`apply_routing_reference` only). Test: `models/tests.rs` (new module).

**Interfaces — produces:** `ModelCapabilities::is_alias_target: bool`; `family::join_key(&ModelSpec) -> String`;
`family::family_benchmarks(specs) -> HashMap<String, (f32, String)>`.

- [ ] **Step 1: Write the failing tests.** Append to `family.rs`'s `mod tests`:

```rust
    #[test]
    fn an_alias_target_is_never_superseded() {
        let mut old = dated("acme/widget-4.8", Some(1_700_000_000));
        old.capabilities.is_alias_target = true;
        let new = dated("acme/widget-5.5", Some(1_760_000_000));
        let newest = newest_per_family([&old, &new]);
        assert!(!is_superseded(&old, &newest));
    }

    #[test]
    fn join_key_uses_the_canonical_slug_for_direct_ids_and_ignores_free() {
        let mut direct = dated("widget-5-5-20260101", Some(1));
        direct.canonical_slug = "acme/widget-5-5-20260101".into();
        assert_eq!(join_key(&direct), "acme/widget");
        assert_eq!(join_key(&dated("acme/widget-5.5:free", None)), "acme/widget");
        assert_eq!(join_key(&dated("acme/widget-4.8", None)), "acme/widget");
    }

    #[test]
    fn family_benchmarks_pick_the_newest_benchmarked_member() {
        let mut a = dated("acme/widget-4.8", Some(1_700_000_000));
        a.capabilities.intelligence_index = Some(30.0);
        let mut b = dated("acme/widget-5.0", Some(1_750_000_000));
        b.capabilities.intelligence_index = Some(38.0);
        let c = dated("acme/widget-5.5", Some(1_760_000_000));
        let fb = family_benchmarks([&a, &b, &c]);
        assert_eq!(fb.get("acme/widget"), Some(&(38.0, "acme/widget-5.0".to_string())));
    }
```

Append to `catalog.rs`'s `mod tests`:

```rust
    #[test]
    fn alias_targets_are_flagged() {
        let json = r#"{"data":[
          {"id":"~acme/widget-latest","created":1780000000,"pricing":{"prompt":"0.000001","completion":"0.000002"},
           "context_length":1000,"alias_target":{"name":"Widget","slug":"acme/widget-4.8"}},
          {"id":"acme/widget-4.8","created":1700000000,"pricing":{"prompt":"0.000001","completion":"0.000002"},"context_length":1000},
          {"id":"acme/widget-5.5","created":1760000000,"pricing":{"prompt":"0.000001","completion":"0.000002"},"context_length":1000}
        ]}"#;
        let specs = specs_from_openrouter_json_at(json, "2026-09-29").expect("parse");
        let flag = |id: &str| specs.iter().find(|s| s.id == id).unwrap().capabilities.is_alias_target;
        assert!(flag("acme/widget-4.8"));
        assert!(!flag("acme/widget-5.5"));
        assert_eq!(specs.len(), 2, "the alias entry itself is still skipped");
    }
```

Append to `models/tests.rs`:

```rust
#[cfg(test)]
mod family_inheritance_tests {
    use crate::models::reference::RoutingReference;
    use crate::models::spec::{PricingSource, QualitySource};
    use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ProviderType, StrengthTag};

    fn spec(id: &str, provider_type: ProviderType, index: Option<f32>, released_at: Option<u64>) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 128_000,
            cost_per_1k: 0.01,
            cost_per_1k_input: 0.0025,
            cost_per_1k_output: 0.01,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities { intelligence_index: index, released_at, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::OpenRouter,
            supported_parameters: vec![],
        }
    }

    fn prior_source(r: &ModelRegistry, id: &str) -> QualitySource {
        r.get(id).unwrap().capabilities.quality_prior.unwrap().source
    }

    fn registry_with_benchmarked_5_0() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/widget-5.0", ProviderType::OpenRouter, Some(38.0), Some(1_750_000_000)));
        r
    }

    #[test]
    fn a_new_release_inherits_its_familys_benchmark() {
        let mut r = registry_with_benchmarked_5_0();
        r.register(spec("acme/widget-5.5", ProviderType::OpenRouter, None, Some(1_760_000_000)));
        r.apply_routing_reference();
        assert_eq!(
            prior_source(&r, "acme/widget-5.5"),
            QualitySource::Inherited { index: 38.0, from: "acme/widget-5.0".into() }
        );
        assert_eq!(prior_source(&r, "acme/widget-5.0"), QualitySource::Benchmark { index: 38.0 });
    }

    #[test]
    fn a_direct_provider_model_joins_its_openrouter_family() {
        let mut r = registry_with_benchmarked_5_0();
        let mut direct = spec("widget-5-5-20260101", ProviderType::Anthropic, None, None);
        direct.canonical_slug = "acme/widget-5-5-20260101".into();
        direct.pricing_source = PricingSource::AnthropicDirect;
        r.register(direct);
        r.apply_routing_reference();
        assert_eq!(
            prior_source(&r, "widget-5-5-20260101"),
            QualitySource::Inherited { index: 38.0, from: "acme/widget-5.0".into() }
        );
    }

    #[test]
    fn local_models_never_inherit() {
        let mut r = registry_with_benchmarked_5_0();
        r.register(spec("acme/widget-5.9", ProviderType::Ollama, None, Some(1_770_000_000)));
        r.apply_routing_reference();
        assert_eq!(prior_source(&r, "acme/widget-5.9"), QualitySource::Estimate);
    }

    #[test]
    fn an_openrouter_inherited_prior_outranks_its_estimate() {
        let mut r = registry_with_benchmarked_5_0();
        let fresh = spec("acme/widget-5.5", ProviderType::OpenRouter, None, Some(1_760_000_000));
        let estimate = RoutingReference::FALLBACK.quality_prior(&fresh, None).value;
        r.register(fresh);
        r.apply_routing_reference();
        let inherited = r.get("acme/widget-5.5").unwrap().capabilities.quality_prior.unwrap().value;
        assert!(inherited > estimate, "{inherited} vs {estimate}");
    }

    // <!-- AMENDED: G5 — the invariant is "same line, same prior, whichever key the user has"; for a
    // direct-provider model inheritance can LOWER the prior (its unscaled context proxy is ~0.83). -->
    #[test]
    fn a_direct_model_scores_like_its_openrouter_sibling() {
        let mut r = registry_with_benchmarked_5_0();
        r.register(spec("acme/widget-5.5", ProviderType::OpenRouter, None, Some(1_760_000_000)));
        let mut direct = spec("widget-5-5-20260101", ProviderType::Anthropic, None, None);
        direct.canonical_slug = "acme/widget-5-5-20260101".into();
        direct.pricing_source = PricingSource::AnthropicDirect;
        let direct_estimate = RoutingReference::FALLBACK.quality_prior(&direct, None).value;
        r.register(direct);
        r.apply_routing_reference();
        let prior = |id: &str| r.get(id).unwrap().capabilities.quality_prior.unwrap().value;
        assert_eq!(prior("widget-5-5-20260101"), prior("acme/widget-5.5"));
        assert!(prior("widget-5-5-20260101") < direct_estimate, "the family's measured index replaces the proxy");
    }
}
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::family catalog::tests::alias_targets models::tests::family_inheritance_tests > target/routing-sm-t2-red.txt 2>&1; tail -20 target/routing-sm-t2-red.txt`
Expected: FAIL to compile — no field `is_alias_target`, no function `join_key` / `family_benchmarks`.

- [ ] **Step 3: `spec.rs` + `virtual_models.rs`.** Directly after the `quality_prior` field add:

```rust
    /// OpenRouter lists this model as the current target of a `~vendor/…-latest` alias. An alias
    /// target is never treated as superseded, whatever `models::family::family_key` infers.
    #[serde(default)]
    pub is_alias_target: bool,
```

and add `is_alias_target: false,` to both literals in `virtual_models.rs`.

- [ ] **Step 4: `catalog.rs`.** Above `struct OpenRouterModelData` add
  `#[derive(serde::Deserialize)] struct OpenRouterAliasTarget { slug: String }`; in `OpenRouterModelData` after
  `benchmarks` add `#[serde(default)] alias_target: Option<OpenRouterAliasTarget>,`. In
  `specs_from_openrouter_json_at`, directly after `let body: OpenRouterModelsResponse = serde_json::from_str(json)?;`:

```rust
    // `~vendor/…-latest` entries are skipped as models, but they name each line's current model.
    let alias_targets: std::collections::HashSet<String> = body
        .data
        .iter()
        .filter(|m| m.id.starts_with('~'))
        .filter_map(|m| m.alias_target.as_ref().map(|t| t.slug.clone()))
        .collect();
```

and in the loop's `ModelCapabilities { … }` literal, after `intelligence_index: …,` add
`is_alias_target: alias_targets.contains(&m.id),`. Change nothing else.

- [ ] **Step 5: `family.rs`.** First statement of `is_superseded`:
  `if m.capabilities.is_alias_target { return false; }`. Add after `is_superseded`:

```rust
/// Family key for joining a model to its family's benchmark: a direct-provider id has no `org/`
/// prefix (`widget-5-5-…`), so its dated `canonical_slug` is used; a `:free` variant is the same
/// weights as its paid sibling, so it shares the paid family's benchmark.
#[must_use]
pub fn join_key(m: &ModelSpec) -> String {
    let slug = if m.id.contains('/') || m.canonical_slug.is_empty() { &m.id } else { &m.canonical_slug };
    family_key(slug).trim_end_matches(":free").to_string()
}

/// The newest benchmarked member of each family (by [`join_key`]): `(index, id)`.
#[must_use]
pub fn family_benchmarks<'a>(specs: impl IntoIterator<Item = &'a ModelSpec>) -> HashMap<String, (f32, String)> {
    let mut best: HashMap<String, ((u64, Vec<u32>), f32, String)> = HashMap::new();
    for m in specs {
        let Some(index) = m.capabilities.intelligence_index.filter(|i| i.is_finite()) else {
            continue;
        };
        let key = join_key(m);
        let rank = (m.capabilities.released_at.unwrap_or(0), version_tuple(&m.id));
        match best.get(&key) {
            Some((cur, _, _)) if *cur >= rank => {}
            _ => {
                best.insert(key, (rank, index, m.id.clone()));
            }
        }
    }
    best.into_iter().map(|(k, (_, index, id))| (k, (index, id))).collect()
}
```

- [ ] **Step 6: `registry.rs` `apply_routing_reference`.** Directly after `self.reference = reference;` add
  `let family = super::family::family_benchmarks(self.models.values());` and replace
  `let prior = reference.quality_prior(spec, None);` with:

```rust
            let local = matches!(
                spec.provider_type,
                ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal
            );
            let inherited = if local || spec.capabilities.intelligence_index.is_some() {
                None
            } else {
                family
                    .get(&super::family::join_key(spec))
                    .filter(|(_, from)| *from != spec.id)
                    .map(|(index, from)| (*index, from.as_str()))
            };
            let prior = reference.quality_prior(spec, inherited);
```

and delete the second, now duplicate, `let local = …;` further down in the loop (keep one binding).

- [ ] **Step 7: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::family catalog models::tests::family_inheritance_tests models::tests::routing_reference_tests 2>&1 | tail -15 && timeout 1500s cargo check -p vox-research-shim 2>&1 | tail -3 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5`
Expected: all pass.

- [ ] **Step 8 (Claude): mutation proofs** — (a) delete the `is_alias_target` early return →
  `an_alias_target_is_never_superseded`; (b) `trim_end_matches(":free")` removed →
  `join_key_uses_the_canonical_slug_for_direct_ids_and_ignores_free`; (c) `inherited` always `None` →
  `a_new_release_inherits_its_familys_benchmark`; (d) `local ||` removed → `local_models_never_inherit`;
  (e) `alias_targets.contains(&m.id)` → `false` → `alias_targets_are_flagged`;
  (f) `local ||` → `local || spec.provider_type != ProviderType::OpenRouter` →
  `a_direct_model_scores_like_its_openrouter_sibling`. <!-- AMENDED: G5 -->
- [ ] **Step 8b (Claude): live pick comparison** — repeat Task 1b Step 6b after inheritance; also record how many
  specs inherited (expect ≥ 7 on the 2026-09-29 snapshot). <!-- AMENDED: G4 -->
- [ ] **Step 9: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/spec.rs crates/vox-research-shim/src/selection/virtual_models.rs crates/vox-orchestrator/src/catalog.rs crates/vox-orchestrator/src/models/family.rs crates/vox-orchestrator/src/models/registry.rs crates/vox-orchestrator/src/models/tests.rs
git commit -m "feat(models): new releases inherit their family's benchmark; alias targets are never superseded"
```

---

### Task 3: `ScoreParts` — every term of the routing score, from the scorer itself

**Files:** Modify `models/scoring.rs` only (`auto_score_model`; new `AxisPart`, `ScoreParts`, `auto_score_parts`;
tests in `mod tests`).

**Interfaces — produces:** `scoring::{AxisPart, ScoreParts, auto_score_parts}` (all `pub`);
`ScoreParts::weighted(&self) -> f64`. `auto_score_model` keeps its signature and returns
`auto_score_parts(..).total`.

- [ ] **Step 1: Write the failing tests** — append inside `mod tests` in `scoring.rs` (not in
  `axes_override_tests` or the `populi-transport`-gated `donations_vox_wiring_tests`):

```rust
    // <!-- AMENDED: R10 — two scorer calls read the global base weights, which
    // `install_base_routing_priority_is_observed_then_cleared` (#[serial]) rewrites. -->
    #[test]
    #[serial_test::serial]
    fn score_parts_add_up_to_the_total() {
        let cases = [
            (make_spec(ProviderType::OpenRouter, 1.0, false), 2, CostPreference::Economy),
            (make_spec(ProviderType::Ollama, 0.0, true), 5, CostPreference::Economy),
            (make_spec(ProviderType::Anthropic, 0.045, false), 9, CostPreference::Performance),
        ];
        for (m, complexity, pref) in cases {
            let p = auto_score_parts(&m, complexity, false, None, pref, None, None);
            let recomputed = p.weighted() + p.fill_in_middle + p.free_bonus + p.off_peak_bonus + p.telemetry + p.vram;
            assert_eq!(p.total.to_bits(), recomputed.to_bits(), "{:?}", m.provider_type);
            assert_eq!(p.total, auto_score_model(&m, complexity, false, None, pref, None, None));
            assert_eq!(p.quality.value, quality_score(&m));
            assert_eq!(p.efficiency.value, efficiency_score(&m));
            assert!(!p.rate_limited);
        }
    }

    #[test]
    fn a_free_model_below_the_high_cutoff_shows_its_bonus() {
        let free = make_spec(ProviderType::Ollama, 0.0, true);
        assert_eq!(auto_score_parts(&free, 5, false, None, CostPreference::Economy, None, None).free_bonus, ZERO_COST_BASE_BONUS);
        assert_eq!(auto_score_parts(&free, COMPLEXITY_HIGH_CUTOFF, false, None, CostPreference::Economy, None, None).free_bonus, 0.0);
    }

    #[test]
    fn a_rate_limited_model_reports_the_floor_and_why() {
        let spec = make_spec(ProviderType::OpenRouter, 0.01, false);
        let hints = vec![crate::usage::RemainingBudget {
            provider: "openrouter".into(),
            model: "test/model".into(),
            calls_used: 50,
            daily_limit: 100,
            remaining: 50,
            cost_today: 0.5,
            rate_limited: true,
        }];
        let p = auto_score_parts(&spec, 5, false, None, CostPreference::Economy, Some(&hints), None);
        assert!(p.rate_limited);
        assert_eq!(p.total, RATE_LIMITED_SCORE_FLOOR);
        assert_eq!(p.quality.weight, 0);
    }
```

(The hint fixture is copied from the existing `rate_limited_model_floors_to_negative`, which proves `make_spec`'s
usage key matches `openrouter` / `test/model`.)

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::scoring::tests > target/routing-sm-t3-red.txt 2>&1; tail -15 target/routing-sm-t3-red.txt`
Expected: FAIL to compile — `cannot find function auto_score_parts`.

- [ ] **Step 3: Implement.** Directly above `auto_score_model` add:

```rust
/// One weighted axis of [`ScoreParts`]: its weight and its 0–1 value.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct AxisPart {
    pub weight: u8,
    pub value: f64,
}

const ZERO_AXIS: AxisPart = AxisPart { weight: 0, value: 0.0 };

/// Every term of [`auto_score_model`], so an explanation shows why a model scored what it did
/// without re-implementing the formula. [`auto_score_model`] returns `total`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct ScoreParts {
    pub total: f64,
    /// A provider budget hint marked the model rate-limited: `total` is the floor and the parts are all zero,
    /// so this is the one case where the parts do not add up to `total`. <!-- AMENDED: A11 -->
    pub rate_limited: bool,
    pub efficiency: AxisPart,
    pub quality: AxisPart,
    pub latency: AxisPart,
    pub availability: AxisPart,
    pub balance: AxisPart,
    pub mobile: AxisPart,
    /// Sum of the six weights (at least 1); the weighted axes are divided by it.
    pub weight_sum: f64,
    pub fill_in_middle: f64,
    pub free_bonus: f64,
    pub off_peak_bonus: f64,
    pub telemetry: f64,
    pub vram: f64,
}

impl ScoreParts {
    /// The weighted-axis share of `total`, summed in the scorer's own order.
    #[must_use]
    pub fn weighted(&self) -> f64 {
        (f64::from(self.efficiency.weight) * self.efficiency.value
            + f64::from(self.quality.weight) * self.quality.value
            + f64::from(self.latency.weight) * self.latency.value
            + f64::from(self.availability.weight) * self.availability.value
            + f64::from(self.balance.weight) * self.balance.value
            + f64::from(self.mobile.weight) * self.mobile.value)
            / self.weight_sum
    }
}
```

Then rename `pub fn auto_score_model(` to `pub fn auto_score_parts(` with return type `-> ScoreParts`, and make
exactly these edits inside it:
  1. `return RATE_LIMITED_SCORE_FLOOR;` becomes
     `return ScoreParts { total: RATE_LIMITED_SCORE_FLOOR, rate_limited: true, efficiency: ZERO_AXIS, quality: ZERO_AXIS, latency: ZERO_AXIS, availability: ZERO_AXIS, balance: ZERO_AXIS, mobile: ZERO_AXIS, weight_sum: 1.0, fill_in_middle: 0.0, free_bonus: 0.0, off_peak_bonus: 0.0, telemetry: 0.0, vram: 0.0 };`
  2. Before `let score = f64::from(w.efficiency) * efficiency_score(m)` add
     `let (eff, qual, bal) = (efficiency_score(m), quality_score(m), balance_bias);` and
     `let mob = mobile_score(m);`, and change the `score` expression to use `eff`, `qual`, `live_latency`,
     `availability_score`, `bal`, `mob` in the same order (same arithmetic, named values).
  3. Replace the final expression
     `(score / total_w) + fim_bias + mens_bonus + off_peak_bonus + telemetry_boost + vram_penalty` with:

```rust
    let parts = ScoreParts {
        total: 0.0,
        rate_limited: false,
        efficiency: AxisPart { weight: w.efficiency, value: eff },
        quality: AxisPart { weight: w.precision, value: qual },
        latency: AxisPart { weight: w.latency, value: live_latency },
        availability: AxisPart { weight: w.availability, value: availability_score },
        balance: AxisPart { weight: w.balance, value: bal },
        mobile: AxisPart { weight: w.mobile, value: mob },
        weight_sum: total_w,
        fill_in_middle: fim_bias,
        free_bonus: mens_bonus,
        off_peak_bonus,
        telemetry: telemetry_boost,
        vram: vram_penalty,
    };
    ScoreParts {
        total: (score / total_w) + fim_bias + mens_bonus + off_peak_bonus + telemetry_boost + vram_penalty,
        ..parts
    }
```

Finally add the thin wrapper with `auto_score_model`'s original doc comment and attributes:

```rust
#[must_use]
pub fn auto_score_model(
    m: &ModelSpec,
    complexity: u8,
    free_tier_fill_in_middle: bool,
    context_fill_ratio: Option<f32>,
    preference: CostPreference,
    hints: Option<&[RemainingBudget]>,
    scoreboard: Option<&super::registry::ModelScore>,
) -> f64 {
    auto_score_parts(m, complexity, free_tier_fill_in_middle, context_fill_ratio, preference, hints, scoreboard).total
}
```

(`mens_bonus` keeps its `#[cfg_attr(not(feature = "populi-transport"), allow(unused_mut))]` and its feature-gated
`+= 0.15` block unchanged.)

- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::scoring 2>&1 | tail -10 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5`
Expected: all pass (the existing scoring suite pins the formula's behaviour).

- [ ] **Step 6 (Claude): mutation proofs** — (a) `weight_sum: total_w` → `total_w + 1.0` →
  `score_parts_add_up_to_the_total`; (b) `quality: AxisPart { weight: w.precision, value: qual }` → `value: eff` →
  same test; (c) `free_bonus: mens_bonus` → `0.0` → `a_free_model_below_the_high_cutoff_shows_its_bonus`.
- [ ] **Step 7: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/scoring.rs
git commit -m "feat(models): the routing scorer reports every term of its score"
```

---

### Task 4: `Ranking` — the selector's own ranked list, with a reason for every exclusion

**Files:** Create `crates/vox-orchestrator/src/models/ranking.rs`; modify `models/mod.rs` (`pub mod ranking;`),
`models/registry.rs` (`matches_strength` becomes `pub(crate)`; bodies of `best_for_internal` and
`best_for_task_with_filter` delegate to the ranking; the old `max_by` comparator is deleted — it moves).

**Interfaces — produces:** `ranking::{Exclusion, RankedModel, Ranking}`; `Ranking::chosen(&self) ->
Option<&ModelSpec>`; `ModelRegistry::{rank_with_filter, rank_task_with_filter}` (`pub`), `rank_pass` (`pub(crate)`).

**Behaviour change (intended, documented):** exact score ties now resolve by model id ascending. Before, `max_by`
over a `HashMap` picked an arbitrary one of the tied models.

- [ ] **Step 1: Write the failing tests** — create `ranking.rs` with only this module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, StrengthTag};

    fn spec(id: &str, provider_type: ProviderType, cost: f64, is_free: bool, strengths: Vec<StrengthTag>) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 64_000,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free,
            observed_cost_per_1k: None,
            strengths,
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    fn reason(r: &Ranking, id: &str) -> Option<Exclusion> {
        r.excluded.iter().find(|(x, _)| x == id).map(|(_, e)| e.clone())
    }

    // <!-- AMENDED: R11 — both calls read the global privacy override other tests flip. -->
    #[test]
    #[serial_test::file_serial]
    fn the_ranking_agrees_with_best_for_on_the_bootstrap_registry() {
        let reg = ModelRegistry::new();
        for task in [TaskCategory::CodeGen, TaskCategory::Research, TaskCategory::General] {
            for pref in [CostPreference::Economy, CostPreference::Performance] {
                for complexity in [2u8, 5, 9] {
                    let chosen = reg.best_for_with_filter(task, complexity, pref, false, |_| true, None);
                    let ranking = reg.rank_with_filter(task, complexity, pref, false, |_| None, None);
                    let top = ranking.ranked.first().map(|r| r.parts.total);
                    let chosen_score = chosen.as_ref().map(|m| {
                        crate::models::scoring::auto_score_model(m, complexity, false, None, pref, None, reg.scoreboard_snapshot().get(&m.id))
                    });
                    assert_eq!(chosen_score, top, "{task:?} {pref:?} cx{complexity}");
                }
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn ranked_models_are_in_descending_score_order_ties_by_id() {
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/b-twin", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]));
        reg.register(spec("acme/a-twin", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]));
        reg.register(spec("acme/pricey", ProviderType::Ollama, 0.9, false, vec![StrengthTag::Generalist]));
        let r = reg.rank_with_filter(TaskCategory::CodeGen, 5, CostPreference::Economy, false, |_| None, None);
        for w in r.ranked.windows(2) {
            assert!(w[0].parts.total >= w[1].parts.total);
        }
        let ids: Vec<&str> = r.ranked.iter().map(|m| m.spec.id.as_str()).collect();
        assert_eq!(&ids[..2], &["acme/a-twin", "acme/b-twin"], "exact ties resolve by id");
    }

    #[test]
    fn every_model_is_ranked_or_excluded_exactly_once() {
        let reg = ModelRegistry::new();
        let r = reg.rank_with_filter(TaskCategory::CodeGen, 5, CostPreference::Performance, false, |_| None, None);
        let mut seen: Vec<String> = r.ranked.iter().map(|m| m.spec.id.clone()).chain(r.excluded.iter().map(|(id, _)| id.clone())).collect();
        seen.sort();
        let mut all: Vec<String> = reg.list_models().into_iter().map(|m| m.id).collect();
        all.sort();
        assert_eq!(seen, all);
    }

    #[test]
    fn exclusions_name_their_reason() {
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/ok", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]));
        reg.register(spec("acme/free", ProviderType::Ollama, 0.0, true, vec![StrengthTag::Generalist]));
        reg.register(spec("acme/vision-only", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Vision]));
        reg.register(spec("acme/huge", ProviderType::Ollama, 1.0e6, false, vec![StrengthTag::Generalist]));
        reg.register(spec("acme/dropped", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]));
        reg.record_penalty("acme/ok-2".into(), TaskCategory::CodeGen, std::time::Duration::from_secs(600));
        reg.register(spec("acme/ok-2", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]));
        let r = reg.rank_with_filter(TaskCategory::CodeGen, 5, CostPreference::Performance, false,
            |m| (m.id == "acme/dropped").then_some(Exclusion::Filtered), None);
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/ok"));
        assert_eq!(reason(&r, "acme/free"), Some(Exclusion::FreeInPerformanceMode));
        assert_eq!(reason(&r, "acme/vision-only"), Some(Exclusion::StrengthMismatch));
        assert_eq!(reason(&r, "acme/huge"), Some(Exclusion::OverRequestCostCap));
        assert_eq!(reason(&r, "acme/dropped"), Some(Exclusion::Filtered));
        assert_eq!(reason(&r, "acme/ok-2"), Some(Exclusion::Penalized));
    }

    #[test]
    fn a_superseded_member_names_its_successor() {
        let mut reg = ModelRegistry::default();
        let mut old = spec("acme/widget-4.8", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]);
        old.capabilities.released_at = Some(1_700_000_000);
        let mut new = spec("acme/widget-5.5", ProviderType::Ollama, 0.02, false, vec![StrengthTag::Generalist]);
        new.capabilities.released_at = Some(1_760_000_000);
        reg.register(old);
        reg.register(new);
        let r = reg.rank_with_filter(TaskCategory::CodeGen, 5, CostPreference::Economy, false, |_| None, None);
        assert_eq!(reason(&r, "acme/widget-4.8"), Some(Exclusion::Superseded { by: "acme/widget-5.5".into() }));
    }

    // <!-- AMENDED: G1 — HashMap RandomState + a stable sort let a tie-break mutant survive by chance;
    // compare two exact twins directly so the mutant fails on every run. -->
    #[test]
    fn exact_ties_resolve_by_id_ascending() {
        let reg = ModelRegistry::default();
        // One scorer call: a concurrent weights change cannot split the twins. <!-- AMENDED: R11 -->
        let parts = auto_score_parts(
            &spec("acme/any", ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]),
            5, false, None, CostPreference::Economy, None, None,
        );
        let twin = |id: &str| RankedModel {
            spec: spec(id, ProviderType::Ollama, 0.01, false, vec![StrengthTag::Generalist]),
            parts,
        };
        let (a, b) = (twin("acme/a-twin"), twin("acme/b-twin"));
        assert_eq!(reg.rank_order(&a, &b), std::cmp::Ordering::Greater, "the smaller id is chosen");
        assert_eq!(reg.rank_order(&b, &a), std::cmp::Ordering::Less);
    }

    // <!-- AMENDED: G2 — the bootstrap registry is undated and the first equivalence test passes no
    // filter, task or penalty, so these four gates get their own old-vs-new comparison. -->
    #[test]
    fn the_ranking_agrees_with_best_for_on_every_gate() {
        use crate::types::{AgentTask, Budget, TaskId, TaskPriority};
        fn agree(reg: &ModelRegistry, pref: CostPreference, reject: &str, task: Option<&AgentTask>) -> Ranking {
            let chosen = reg.best_for_with_filter(TaskCategory::CodeGen, 5, pref, false, |m| m.id != reject, task);
            let ranking = reg.rank_with_filter(TaskCategory::CodeGen, 5, pref, false,
                |m| (m.id == reject).then_some(Exclusion::Filtered), task);
            assert_eq!(chosen.map(|m| m.id), ranking.chosen().map(|m| m.id.clone()));
            ranking
        }
        let g = || vec![StrengthTag::Generalist];

        // 1. Supersession: the older member is cheaper, so it would win on cost.
        let mut reg = ModelRegistry::default();
        let mut old = spec("acme/widget-4.8", ProviderType::Ollama, 0.001, false, g());
        old.capabilities.released_at = Some(1_700_000_000);
        let mut new = spec("acme/widget-5.5", ProviderType::Ollama, 0.02, false, g());
        new.capabilities.released_at = Some(1_760_000_000);
        reg.register(old);
        reg.register(new);
        let r = agree(&reg, CostPreference::Economy, "", None);
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/widget-5.5"));

        // 2. The caller's filter rejects the model that would win.
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/cheap", ProviderType::Ollama, 0.001, false, g()));
        reg.register(spec("acme/dear", ProviderType::Ollama, 0.02, false, g()));
        let r = agree(&reg, CostPreference::Economy, "acme/cheap", None);
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/dear"));
        assert_eq!(reason(&r, "acme/cheap"), Some(Exclusion::Filtered));

        // 3. The task budget excludes the pricier model.
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/cheap", ProviderType::Ollama, 0.0001, false, g()));
        reg.register(spec("acme/big", ProviderType::Ollama, 0.5, false, g()));
        let mut task = AgentTask::new(TaskId(1), "budgeted", TaskPriority::Normal, vec![]);
        task.budget = Some(Budget { max_cost_usd: Some(0.001), max_latency_ms: None });
        let r = agree(&reg, CostPreference::Performance, "", Some(&task));
        assert_eq!(reason(&r, "acme/big"), Some(Exclusion::OverTaskBudget));

        // 4. Every model is penalised: the second pass must still choose.
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/only", ProviderType::Ollama, 0.01, false, g()));
        reg.record_penalty("acme/only".into(), TaskCategory::CodeGen, std::time::Duration::from_secs(600));
        let r = agree(&reg, CostPreference::Economy, "", None);
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/only"));
    }
}
```

(If `StrengthTag::Vision` does not exist, use any strength variant other than `Codegen` and `Generalist` —
`rg -n "pub enum StrengthTag" -A30 crates/vox-orchestrator/src/models/generated.rs` — and say which. If `Budget` is
not re-exported at `crate::types`, use the path `rg -n "pub struct Budget" crates/vox-orchestrator/src` gives.)

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::ranking > target/routing-sm-t4-red.txt 2>&1; tail -15 target/routing-sm-t4-red.txt`
Expected: FAIL to compile — `cannot find type Ranking` / `Exclusion`, no method `rank_with_filter`.

- [ ] **Step 3: Implement `ranking.rs`** (above the tests). Do **not** touch `registry.rs` yet:

```rust
//! The selector's ranked, explained candidate list: the same filter chain and ordering as
//! `ModelRegistry::best_for_with_filter`, recording why each excluded model was excluded and
//! each candidate's `ScoreParts`. `best_for_with_filter` returns this ranking's first entry, so
//! an explanation cannot disagree with the choice.

use super::scoring::{ScoreParts, auto_score_parts};
use super::spec::task_category_strength;
use super::{ModelRegistry, ModelSpec, ProviderType, StrengthTag, TaskCategory};
use crate::config::CostPreference;
use crate::types::AgentTask;

/// Why a registered model was not a candidate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Exclusion {
    /// Recently abstained on this task category.
    Penalized,
    /// Free models are skipped in performance mode.
    FreeInPerformanceMode,
    /// Estimated request cost is above the configured per-request safety cap.
    OverRequestCostCap,
    /// Estimated cost is above the task's own budget.
    OverTaskBudget,
    /// `VOX_ROUTE_*` policy excludes it.
    RoutePolicy,
    /// Local-only privacy mode excludes cloud models.
    PrivacyLocalOnly,
    /// Not suited to this task category.
    StrengthMismatch,
    /// The caller's own filter.
    Filtered,
    /// No resolvable key for its provider.
    NoProviderKey,
    /// The mode keeps flagships out while another model fits.
    FlagshipExcludedByMode,
    /// Free mode: only zero-cost models (`mode_select::DispatchGate`).
    NotFree,
    /// Today's exploration budget is spent, so unpriced models are skipped (`DispatchGate`).
    ExplorationBudgetSpent,
    /// Its provider has no usage budget left today (`DispatchGate`).
    ProviderBudgetExhausted,
    /// A newer member of its family is a candidate.
    Superseded { by: String },
}

/// A candidate and the parts of its routing score.
#[derive(Debug, Clone)]
pub struct RankedModel {
    pub spec: ModelSpec,
    pub parts: ScoreParts,
}

/// Candidates best-first, and every excluded model with its reason.
#[derive(Debug, Clone, Default)]
pub struct Ranking {
    pub ranked: Vec<RankedModel>,
    pub excluded: Vec<(String, Exclusion)>,
}

impl Ranking {
    /// The model routing picks: the first ranked candidate.
    #[must_use]
    pub fn chosen(&self) -> Option<&ModelSpec> {
        self.ranked.first().map(|r| &r.spec)
    }
}

impl ModelRegistry {
    /// The ranking behind [`Self::best_for_with_filter`], with the same two passes: penalties
    /// are respected unless that leaves nothing.
    pub fn rank_with_filter(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
        allow_free_in_performance_mode: bool,
        mut filter: impl FnMut(&ModelSpec) -> Option<Exclusion>,
        task: Option<&AgentTask>,
    ) -> Ranking {
        let strength = task_category_strength(task_type);
        let first = self.rank_pass(task_type, strength, complexity, preference, allow_free_in_performance_mode, &mut filter, true, task);
        if !first.ranked.is_empty() {
            return first;
        }
        self.rank_pass(task_type, strength, complexity, preference, allow_free_in_performance_mode, &mut filter, false, task)
    }

    /// [`Self::rank_with_filter`] for a task, with [`Self::best_for_task_with_filter`]'s
    /// adjustments (research hints route as Research; two or more tool hints raise complexity to 7).
    pub fn rank_task_with_filter(
        &self,
        task: &AgentTask,
        preference: CostPreference,
        filter: impl FnMut(&ModelSpec) -> Option<Exclusion>,
    ) -> Ranking {
        let mut complexity = task.estimated_complexity;
        let mut task_type = task.task_category;
        if !task.research_hints.is_empty() && task_type != TaskCategory::Research {
            task_type = TaskCategory::Research;
        }
        if task.tool_hints.len() >= 2 && complexity < 7 {
            complexity = 7;
        }
        self.rank_with_filter(task_type, complexity, preference, false, filter, Some(task))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn rank_pass(
        &self,
        task_type: TaskCategory,
        strength: StrengthTag,
        complexity: u8,
        preference: CostPreference,
        allow_free_in_performance_mode: bool,
        filter: &mut dyn FnMut(&ModelSpec) -> Option<Exclusion>,
        respect_penalties: bool,
        task: Option<&AgentTask>,
    ) -> Ranking {
        let safety_cap = vox_config::load_model_routing_config().safety.max_cost_usd_per_request;
        let privacy_local_only = crate::route_policy::inference_privacy_local_only_from_env();
        let scoreboard = self.scoreboard_snapshot();
        let mut excluded: Vec<(String, Exclusion)> = Vec::new();
        let mut candidates: Vec<&ModelSpec> = Vec::new();
        for m in self.models_iter() {
            let why = if respect_penalties && self.is_penalized(&m.id, task_type) {
                Some(Exclusion::Penalized)
            } else if preference == CostPreference::Performance && m.is_free && !allow_free_in_performance_mode {
                Some(Exclusion::FreeInPerformanceMode)
            } else if request_cost(m, task) > safety_cap {
                Some(Exclusion::OverRequestCostCap)
            } else if over_task_budget(m, task, scoreboard) {
                Some(Exclusion::OverTaskBudget)
            } else if !crate::route_policy::route_policy_allows_model(m) {
                Some(Exclusion::RoutePolicy)
            } else if !crate::route_policy::privacy_allows_model_for_mode(m, privacy_local_only) {
                Some(Exclusion::PrivacyLocalOnly)
            } else if !Self::matches_strength(m, strength) {
                Some(Exclusion::StrengthMismatch)
            } else {
                filter(m)
            };
            match why {
                Some(e) => excluded.push((m.id.clone(), e)),
                None => candidates.push(m),
            }
        }
        let newest = super::family::newest_per_family(candidates.iter().copied());
        let mut ranked: Vec<RankedModel> = Vec::new();
        for m in &candidates {
            if super::family::is_superseded(m, &newest) {
                let key = super::family::family_key(&m.id);
                let by = candidates
                    .iter()
                    .find(|c| {
                        super::family::family_key(&c.id) == key
                            && c.capabilities.released_at.map(|at| (at, super::family::version_tuple(&c.id)))
                                == newest.get(&key).cloned()
                    })
                    .map_or_else(String::new, |c| c.id.clone());
                excluded.push((m.id.clone(), Exclusion::Superseded { by }));
            } else {
                ranked.push(RankedModel {
                    spec: (*m).clone(),
                    parts: auto_score_parts(m, complexity, false, None, preference, None, scoreboard.get(&m.id)),
                });
            }
        }
        ranked.sort_by(|a, b| self.rank_order(b, a));
        Ranking { ranked, excluded }
    }

    /// `Greater` when `a` should be chosen over `b`: score, then cheaper, then higher observed
    /// success, then lower observed latency, then mesh when mesh is preferred, then smaller id.
    fn rank_order(&self, a: &RankedModel, b: &RankedModel) -> std::cmp::Ordering {
        let sb = self.scoreboard_snapshot();
        let success = |m: &ModelSpec| sb.get(&m.id).map(|s| s.success_rate).unwrap_or(0.5);
        let latency = |m: &ModelSpec| sb.get(&m.id).and_then(|s| s.p50_latency_ms).unwrap_or(2000);
        a.parts
            .total
            .total_cmp(&b.parts.total)
            .then_with(|| b.spec.cost_per_1k.total_cmp(&a.spec.cost_per_1k))
            .then_with(|| success(&a.spec).total_cmp(&success(&b.spec)))
            .then_with(|| latency(&b.spec).cmp(&latency(&a.spec)))
            .then_with(|| {
                let prefer_mesh = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxRoutingPreferMesh)
                    .expose()
                    .map(|s: &str| s.trim() == "true")
                    .unwrap_or(false);
                if prefer_mesh {
                    (a.spec.provider_type == ProviderType::PopuliMesh)
                        .cmp(&(b.spec.provider_type == ProviderType::PopuliMesh))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| b.spec.id.cmp(&a.spec.id))
    }
}

fn request_cost(m: &ModelSpec, task: Option<&AgentTask>) -> f64 {
    let est_tokens = task.map(|t| t.estimated_token_count()).unwrap_or(1024) as f64;
    let per_1k = if m.cost_per_1k_input > 0.0 || m.cost_per_1k_output > 0.0 {
        (m.cost_per_1k_input + m.cost_per_1k_output) / 2.0
    } else {
        m.cost_per_1k
    };
    (est_tokens / 1000.0) * per_1k
}

fn over_task_budget(
    m: &ModelSpec,
    task: Option<&AgentTask>,
    scoreboard: &std::collections::HashMap<String, super::ModelScore>,
) -> bool {
    let Some(t) = task else { return false };
    let Some(max) = t.budget.as_ref().and_then(|b| b.max_cost_usd) else { return false };
    let basis = scoreboard.get(&m.id).and_then(|s| s.cost_per_success_usd).unwrap_or(m.cost_per_1k);
    (t.estimated_token_count() as f64 / 1000.0) * basis > max
}
```

In `registry.rs` change `fn matches_strength` to `pub(crate) fn matches_strength`. If an import path above does not
resolve (`super::ModelScore`, `StrengthTag`, `TaskCategory` re-exports), fix only the `use` line and say so.

- [ ] **Step 4: Prove equivalence against the old selector before switching it.** Run
  `timeout 1500s cargo test -p vox-orchestrator --lib models::ranking > target/routing-sm-t4-equivalence.txt 2>&1; tail -12 target/routing-sm-t4-equivalence.txt`.
  Expected: all pass **while `best_for_internal` still has its own `max_by`** — this is the independent proof that
  the ranking chooses what the old selector chose, on the bootstrap registry and on every gate. If
  `the_ranking_agrees_with_best_for_on_the_bootstrap_registry` or `the_ranking_agrees_with_best_for_on_every_gate`
  fails, STOP and report the case and both results. <!-- AMENDED: G2 -->
- [ ] **Step 5: Switch the selector to the ranking** <!-- AMENDED: R19/C1 — one two-pass selector, not two. -->. In
  `registry.rs`, replace the whole body of `best_for_with_filter` (the identity `effective_pref`, both
  `best_for_internal` passes) with:

```rust
        let mut pred = pred;
        self.rank_with_filter(
            task_type,
            complexity,
            preference,
            allow_free_in_performance_mode,
            |m| (!pred(m)).then_some(super::ranking::Exclusion::Filtered),
            task,
        )
        .ranked
        .into_iter()
        .next()
        .map(|r| r.spec)
```

  (its signature keeps `mut pred: impl FnMut(&ModelSpec) -> bool`; drop the now-redundant `let mut pred = pred;` if
  the parameter is already `mut`). Then delete `fn best_for_internal` entirely (its logic moved to `rank_pass`), and
  replace the doc-comment mentions of `best_for_internal` (`rg -n "best_for_internal" crates/vox-orchestrator/src`)
  with `rank_pass`.

  and replace the body of `best_for_task_with_filter` with:

```rust
        let mut pred = pred;
        self.rank_task_with_filter(task, preference, |m| {
            (!pred(m)).then_some(super::ranking::Exclusion::Filtered)
        })
        .chosen()
        .cloned()
```

  (its parameter stays `pred: impl FnMut(&ModelSpec) -> bool`). Remove imports that become unused.
- [ ] **Step 6: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models:: runtime 2>&1 | tail -10 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5`
Expected: all pass. A pre-existing test that fails only on an exact-tie choice is a STOP: report it (the tie rule
is the documented behaviour change).
- [ ] **Step 7 (Claude): mutation proofs** — (a) `ranked.sort_by(|a, b| self.rank_order(b, a))` →
  `self.rank_order(a, b)` → `ranked_models_are_in_descending_score_order_ties_by_id`; (b) swap the
  `OverRequestCostCap` and `OverTaskBudget` arms' reasons → `exclusions_name_their_reason`; (c) `Superseded { by }`
  → `Superseded { by: String::new() }` → `a_superseded_member_names_its_successor`; (d) drop
  `.then_with(|| b.spec.id.cmp(&a.spec.id))` → `exact_ties_resolve_by_id_ascending` (fails every run);
  (e) make `over_task_budget` return `false` → `the_ranking_agrees_with_best_for_on_every_gate`. <!-- AMENDED: G1, G2 --> Then `timeout 3000s cargo test -p vox-orchestrator --tests`.
- [ ] **Step 8: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/ranking.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/registry.rs
git commit -m "feat(models): selection returns its own ranking, with a reason for every excluded model"
```

---

### Task 5: The mode selector and `decide()` return the ranking; dispatch's own rules become one shared gate

<!-- AMENDED: G6, G9, G10 — a key-agnostic variant for health; the free pool / exploration cut-off / provider
allowlist move from runtime.rs's closures into a `DispatchGate` the explainer reuses; the decide() test gets a
fixture on which decide() actually returns. -->

**Files:** Modify `models/mode_select.rs` (`ModeSelection`, `best_for_task_in_mode`, new
`best_for_task_in_mode_keyed`, new `DispatchGate` and `provider_budget_key`, tests), `models/select.rs`
(`ModelSelectionDecision`, `decide`; tests in its test module), `crates/vox-orchestrator/src/runtime.rs` (the two
`best_for_task_in_mode` closures in `AiTaskProcessor::process` collapse to one gate call).

**Interfaces — produces:** `ModeSelection { spec, only_candidate, ranking: Ranking }`;
`ModelRegistry::best_for_task_in_mode_keyed(&self, task, preference, clutch, key_ok: &dyn Fn(&ProviderType) -> bool,
filter: impl FnMut(&ModelSpec) -> Option<Exclusion>) -> Option<ModeSelection>`;
`ModelRegistry::best_for_task_under_gate(&self, task, preference, clutch, &DispatchGate) -> Option<ModeSelection>`;
`mode_select::DispatchGate { force_free_pool, unknown_price_blocked, allowed_providers }` with
`exclusion(&self, &ModelSpec) -> Option<Exclusion>`; `mode_select::provider_budget_key(&ProviderType) -> &'static str`;
`decide()`'s `alternatives` are the scorer's ranked runners-up under chat's own flagship rule.
<!-- AMENDED: R7/C4 — no `ranking` field: its only reader was the chat link, deferred by G8. -->

- [ ] **Step 1: Write the failing tests.** Append to `mode_select.rs`'s `mod tests`:

```rust
    #[test]
    #[serial_test::file_serial] // <!-- AMENDED: R12 — OpenRouter fixture; the privacy override is global. -->
    fn the_mode_ranking_explains_its_exclusions() {
        let mut r = registry();
        r.register(spec("acme/keyless-9", ProviderType::OpenRouter, ModelTier::Fast, 0.0001));
        set_test_key_availability(Some(vec![]));
        let pick = r.best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |_| true);
        set_test_key_availability(None);
        let pick = pick.expect("a candidate");
        assert_eq!(pick.ranking.chosen().map(|m| m.id.clone()), Some(pick.spec.id.clone()));
        let why = |id: &str| pick.ranking.excluded.iter().find(|(x, _)| x == id).map(|(_, e)| e.clone());
        assert_eq!(why("acme/flagship-9"), Some(crate::models::ranking::Exclusion::FlagshipExcludedByMode));
        assert_eq!(why("acme/keyless-9"), Some(crate::models::ranking::Exclusion::NoProviderKey));
    }

    #[test]
    fn the_dispatch_gate_names_each_rule() {
        use crate::models::ranking::Exclusion;
        use crate::models::spec::PricingSource;
        let mut paid = spec("acme/paid", ProviderType::OpenRouter, ModelTier::Fast, 0.01);
        let mut unpriced = spec("acme/unpriced", ProviderType::OpenRouter, ModelTier::Fast, 0.01);
        unpriced.pricing_source = PricingSource::Unknown;
        let free_only = super::DispatchGate { force_free_pool: true, ..Default::default() };
        assert_eq!(free_only.exclusion(&paid), Some(Exclusion::NotFree));
        let spent = super::DispatchGate { unknown_price_blocked: true, ..Default::default() };
        assert_eq!(spent.exclusion(&unpriced), Some(Exclusion::ExplorationBudgetSpent));
        let budgets = super::DispatchGate {
            allowed_providers: Some(["deepseek".to_string()].into_iter().collect()),
            ..Default::default()
        };
        assert_eq!(budgets.exclusion(&paid), Some(Exclusion::ProviderBudgetExhausted));
        let deepseek = spec("acme/ds", ProviderType::DeepSeek, ModelTier::Fast, 0.01);
        assert_eq!(budgets.exclusion(&deepseek), None);
        paid.is_free = true;
        assert_eq!(free_only.exclusion(&paid), None);
        assert_eq!(super::DispatchGate::default().exclusion(&unpriced), None);
    }

    #[test]
    fn free_mode_through_the_gate_never_chooses_a_paid_model() {
        let mut r = registry();
        let mut free = spec("acme/free-9", ProviderType::Ollama, ModelTier::Fast, 0.0);
        free.is_free = true;
        r.register(free);
        let gate = super::DispatchGate { force_free_pool: true, ..Default::default() };
        let pick = r
            .best_for_task_in_mode_keyed(&hard_task(), CostPreference::Economy, ClutchProfile::Free, &|_| true, |m| gate.exclusion(m))
            .expect("the free model");
        assert_eq!(pick.spec.id, "acme/free-9");
        assert!(pick.ranking.excluded.iter().any(|(id, e)| {
            id == "acme/workhorse-9" && *e == crate::models::ranking::Exclusion::NotFree
        }));
    }

    #[test]
    #[serial_test::file_serial] // <!-- AMENDED: R12 -->
    fn the_keyed_variant_can_ignore_this_processs_keys() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/cloud-9", ProviderType::OpenRouter, ModelTier::Fast, 0.01));
        set_test_key_availability(Some(vec![]));
        let catalog = r.best_for_task_in_mode_keyed(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, &|_| true, |_| None);
        let this_process = r.best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |_| true);
        set_test_key_availability(None);
        assert_eq!(catalog.map(|s| s.spec.id), Some("acme/cloud-9".to_string()));
        assert!(this_process.is_none());
    }

    #[test]
    fn the_fallback_pass_returns_its_own_ranking() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/flagship-9", ProviderType::Ollama, ModelTier::Elite, 0.0005));
        let pick = r
            .best_for_task_in_mode(&hard_task(), CostPreference::Economy, ClutchProfile::Efficiency, |_| true)
            .expect("the only candidate");
        assert!(pick.only_candidate);
        assert_eq!(pick.ranking.chosen().map(|m| m.id.as_str()), Some("acme/flagship-9"));
    }
```

Append to `select.rs`'s `mod tests` (the one that imports `file_serial`):

```rust
    // <!-- AMENDED: G10 — on the bootstrap registry every model is Shadowed and decide() returns None, so an
    // `if let Some` body never runs; UserConfig models are Confirmed (the pattern of this module's UserConfig
    // fixture), and a superseded pair makes the HashMap-order mutant fail on every run. -->
    #[test]
    #[file_serial]
    fn decide_alternatives_follow_the_ranking() {
        fn confirmed(id: &str, cost: f64, released_at: Option<u64>) -> crate::models::ModelSpec {
            crate::models::ModelSpec {
                id: id.into(),
                canonical_slug: id.into(),
                provider: "test".into(),
                provider_type: crate::models::ProviderType::Ollama,
                max_tokens: 64_000,
                cost_per_1k: cost,
                cost_per_1k_input: cost,
                cost_per_1k_output: cost,
                is_free: false,
                observed_cost_per_1k: None,
                strengths: vec![crate::models::StrengthTag::Codegen, crate::models::StrengthTag::Generalist],
                capabilities: crate::models::ModelCapabilities { released_at, ..Default::default() },
                cache_creation_cost_per_1k: 0.0,
                cache_read_cost_per_1k: 0.0,
                supports_prompt_caching: false,
                pricing_source: crate::models::spec::PricingSource::UserConfig,
                supported_parameters: vec![],
            }
        }
        let mut registry = ModelRegistry::default();
        registry.register(confirmed("acme/widget-4.8", 0.001, Some(1_700_000_000)));
        registry.register(confirmed("acme/widget-5.5", 0.02, Some(1_760_000_000)));
        registry.register(confirmed("acme/gadget", 0.005, None));
        registry.register(confirmed("acme/gizmo", 0.01, None));
        let req = ModelSelectionRequest::from_intent(SelectionIntent::for_task(TaskCategory::CodeGen));
        let decision = decide(&req, &registry).expect("confirmed local candidates");
        assert!(
            !decision.alternatives.contains(&"acme/widget-4.8".to_string()),
            "a superseded model is not an alternative: {:?}",
            decision.alternatives
        );
        assert!(!decision.alternatives.contains(&decision.selected_model));
        assert_eq!(decision.alternatives.len(), 2, "{:?}", decision.alternatives);
    }
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::mode_select models::select::tests::decide_alternatives > target/routing-sm-t5-red.txt 2>&1; tail -15 target/routing-sm-t5-red.txt`
Expected: FAIL to compile — no field `ranking`, no `DispatchGate`, no `best_for_task_in_mode_keyed`.

- [ ] **Step 3: `mode_select.rs`.** Add `pub ranking: crate::models::ranking::Ranking,` to `ModeSelection`. Replace
  the body of `best_for_task_in_mode` with a delegation, and add the keyed variant directly after it (inside the same
  `impl ModelRegistry`):

```rust
        self.best_for_task_in_mode_keyed(task, preference, clutch, &selection_key_available, |m| {
            (!pred(m)).then_some(crate::models::ranking::Exclusion::Filtered)
        })
    }

    /// [`Self::best_for_task_in_mode`] with an explicit provider-key check and a reason-giving filter.
    /// Routing health passes `&|_| true` to judge the catalog rather than this process's keys; the
    /// routing explainer passes a [`DispatchGate`] so it applies dispatch's own rules.
    pub fn best_for_task_in_mode_keyed(
        &self,
        task: &AgentTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        key_ok: &dyn Fn(&crate::models::ProviderType) -> bool,
        mut filter: impl FnMut(&ModelSpec) -> Option<crate::models::ranking::Exclusion>,
    ) -> Option<ModeSelection> {
        use crate::models::ranking::Exclusion;
        let exclude_elite = excludes_elite(clutch);
        let preferred = self.rank_task_with_filter(task, preference, |m| {
            if exclude_elite && m.capabilities.tier == ModelTier::Elite {
                Some(Exclusion::FlagshipExcludedByMode)
            } else if !key_ok(&m.provider_type) {
                Some(Exclusion::NoProviderKey)
            } else {
                filter(m)
            }
        });
        if let Some(spec) = preferred.chosen().cloned() {
            return Some(ModeSelection { spec, only_candidate: false, ranking: preferred });
        }
        if !exclude_elite {
            return None;
        }
        let fallback = self.rank_task_with_filter(task, preference, |m| {
            if !key_ok(&m.provider_type) { Some(Exclusion::NoProviderKey) } else { filter(m) }
        });
        fallback.chosen().cloned().map(|spec| ModeSelection { spec, only_candidate: true, ranking: fallback })
    }

    /// What task dispatch chooses under `gate`, with this process's provider keys. `runtime.rs` and the
    /// routing explainer both call this, so the explanation cannot drift from dispatch.
    pub fn best_for_task_under_gate(
        &self,
        task: &AgentTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        gate: &DispatchGate,
    ) -> Option<ModeSelection> {
        self.best_for_task_in_mode_keyed(task, preference, clutch, &selection_key_available, |m| gate.exclusion(m))
```

  (keep the existing signature line of `best_for_task_in_mode` and its closing `}` is now the one shown above; the
  check order — tier, key, caller — matches the old two-closure version, so the caller's filter is still called only
  for models that passed the other checks). Then add, after the `impl ModelRegistry` block:

```rust
/// Caller-side rules task dispatch applies on top of the mode (`runtime.rs`), shared with the routing
/// explainer so the explanation and the dispatch cannot drift apart.
#[derive(Debug, Clone, Default)]
pub struct DispatchGate {
    /// Free mode: only zero-cost models.
    pub force_free_pool: bool,
    /// Today's exploration budget is spent: unpriced (`PricingSource::Unknown`) models are skipped.
    pub unknown_price_blocked: bool,
    /// Usage budgets exist: only these provider keys ([`provider_budget_key`]) have budget left.
    pub allowed_providers: Option<std::collections::HashSet<String>>,
}

impl DispatchGate {
    /// Why this gate keeps `m` out, if it does (same order as dispatch's old closure).
    #[must_use]
    pub fn exclusion(&self, m: &ModelSpec) -> Option<crate::models::ranking::Exclusion> {
        use crate::models::ranking::Exclusion;
        if self.unknown_price_blocked && m.pricing_source == crate::models::spec::PricingSource::Unknown {
            return Some(Exclusion::ExplorationBudgetSpent);
        }
        if self.force_free_pool && !m.is_free {
            return Some(Exclusion::NotFree);
        }
        match &self.allowed_providers {
            Some(allowed) if !allowed.contains(provider_budget_key(&m.provider_type)) => {
                Some(Exclusion::ProviderBudgetExhausted)
            }
            _ => None,
        }
    }
}

/// The provider key usage budgets are recorded under (moved verbatim from `runtime.rs`).
#[must_use]
pub fn provider_budget_key(p: &crate::models::ProviderType) -> &'static str {
    use crate::models::ProviderType;
    match p {
        ProviderType::OpenRouter => "openrouter",
        ProviderType::Ollama => "ollama",
        ProviderType::GoogleDirect => "google",
        ProviderType::Groq => "groq",
        ProviderType::Cerebras => "cerebras",
        ProviderType::Mistral => "mistral",
        ProviderType::DeepSeek => "deepseek",
        ProviderType::SambaNova => "sambanova",
        ProviderType::Anthropic => "anthropic",
        ProviderType::PopuliMesh => "populimesh",
        ProviderType::HuggingFaceRouter => "huggingface",
        ProviderType::Custom(_) => "custom",
        ProviderType::VoxLocal => "vox_local",
    }
}
```

- [ ] **Step 3b: `runtime.rs`.** In `AiTaskProcessor::process`, replace the whole
  `if allowed_providers.is_empty() { registry.best_for_task_in_mode(…) } else { registry.best_for_task_in_mode(…) }`
  expression (both closures, including the provider-string `match`) with:

```rust
            let gate = crate::models::mode_select::DispatchGate {
                force_free_pool,
                unknown_price_blocked: exploration_spent >= exploration_limit,
                allowed_providers: (!allowed_providers.is_empty()).then_some(allowed_providers),
            };
            registry.best_for_task_under_gate(&task, cost_pref, clutch, &gate)
```

  Change nothing else in `process` (the `.map(|sel| …)` after the block stays). If the compiler reports
  `allowed_providers` used after the move, STOP and report the line.

- [ ] **Step 4: `select.rs`.** In `decide`, replace the `let alternatives: Vec<String> = candidates … .collect();`
  statement with:

```rust
    // The candidates in the scorer's own order, under this request's axes and chat's flagship rule
    // (the same guard and rule as `select_via_scorer`), so "alternatives" are the real runners-up
    // rather than HashMap order. <!-- AMENDED: R7 -->
    let scored = {
        let _axes = crate::models::scoring::AxesOverrideGuard::set(
            intent.axes.to_routing_priority(intent.prefer_local),
        );
        registry.rank_with_filter(
            intent.task,
            intent.complexity,
            intent.axes.to_cost_preference(),
            intent.allow_free_in_performance_mode,
            |m| {
                if !candidate_ids.contains(&m.id) {
                    Some(super::ranking::Exclusion::Filtered)
                } else if intent.axes.intelligence < 50 && m.capabilities.tier == super::ModelTier::Elite {
                    Some(super::ranking::Exclusion::FlagshipExcludedByMode)
                } else {
                    None
                }
            },
            None,
        )
    };
    let alternatives: Vec<String> = scored
        .ranked
        .iter()
        .filter(|r| r.spec.id != selected.model_id)
        .take(5)
        .map(|r| r.spec.id.clone())
        .collect();
```

  `ModelSelectionDecision` is unchanged.
- [ ] **Step 5: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib -- models::mode_select models::select 2>&1 | tail -10 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5 && timeout 1500s cargo check -p vox-gui -p vox-cli -p vox-orchestrator-mcp 2>&1 | tail -3`
Expected: all pass; the dependents compile.
- [ ] **Step 6 (Claude): mutation proofs** — (a) `Some(Exclusion::FlagshipExcludedByMode)` → `Some(Exclusion::Filtered)`
  → `the_mode_ranking_explains_its_exclusions`; (b) `ranking: fallback` → `ranking: Default::default()` →
  `the_fallback_pass_returns_its_own_ranking`; (c) `alternatives` built from `candidates.iter()` again →
  `decide_alternatives_follow_the_ranking` (fails every run: `acme/widget-4.8` is a candidate);
  (d) delete `if self.force_free_pool && !m.is_free { … }` → `the_dispatch_gate_names_each_rule` and
  `free_mode_through_the_gate_never_chooses_a_paid_model`; (e) in the keyed variant, `key_ok(&m.provider_type)` →
  `selection_key_available(&m.provider_type)` → `the_keyed_variant_can_ignore_this_processs_keys`.
  Then `timeout 3000s cargo test -p vox-orchestrator --tests` (dispatch runs through `runtime.rs`).
- [ ] **Step 7: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/mode_select.rs crates/vox-orchestrator/src/models/select.rs crates/vox-orchestrator/src/runtime.rs
git commit -m "feat(models): mode selection and decide() return the scorer's ranking; dispatch rules become one gate"
```

---

### Task 6: Routing health on every refresh, and a `vox doctor` that can pass

**Files:** Create `crates/vox-orchestrator/src/models/health.rs`; modify `models/mod.rs` (`pub mod health;`),
`orchestrator/catalog_refresh.rs` (`refresh_once`, `run_unified_catalog_refresh`, new
`persist_routing_health`; its `MODEL_CATALOG_LAST_REFRESH_KEY` becomes a re-export),
`models/registry.rs` (its private `MODEL_CATALOG_LAST_REFRESH_KEY` const is replaced by `use`),
`crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/model_catalog.rs`.

**Interfaces — produces:** `health::{RoutingHealth, Violation, check_routing_health, MODEL_CATALOG_LAST_REFRESH_KEY,
ROUTING_HEALTH_KEY, ROUTING_HEALTH_SCHEMA_VERSION}`.

- [ ] **Step 1: Write the failing tests.** Create `health.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelSpec, ModelTier, ProviderType, StrengthTag};

    fn spec(id: &str, out: f64, tier: ModelTier) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 64_000,
            cost_per_1k: out,
            cost_per_1k_input: out,
            cost_per_1k_output: out,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities { tier, ..Default::default() },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn a_healthy_registry_has_no_violations() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/flagship", 0.0005, ModelTier::Elite));
        r.register(spec("acme/workhorse", 0.01, ModelTier::Pro)); // below the $20/M fallback Elite band (R5)
        let h = check_routing_health(&r, 1_790_000_000);
        assert_eq!(h.schema_version, ROUTING_HEALTH_SCHEMA_VERSION);
        assert_eq!(h.models, 2);
        assert!(h.violations.is_empty(), "{:?}", h.violations);
    }

    #[test]
    fn many_cloud_models_without_a_tier_are_a_violation() {
        let mut r = ModelRegistry::default();
        for i in 0..10 {
            let mut m = spec(&format!("acme/cloud-{i}"), 0.0, ModelTier::Unknown);
            m.provider_type = ProviderType::OpenRouter;
            r.register(m);
        }
        let h = check_routing_health(&r, 0);
        assert_eq!(h.unknown_tier_cloud, 10);
        assert!(h.violations.iter().any(|v| v.invariant == "tiers_known"), "{:?}", h.violations);
    }

    #[test]
    fn a_catalog_this_large_with_no_prices_is_a_violation() {
        let mut r = ModelRegistry::default();
        for i in 0..crate::models::reference::MIN_DERIVE_SAMPLE {
            let mut m = spec(&format!("acme/cloud-{i}"), 0.0, ModelTier::Pro);
            m.provider_type = ProviderType::OpenRouter;
            r.register(m);
        }
        r.apply_routing_reference();
        let h = check_routing_health(&r, 0);
        assert!(h.violations.iter().any(|v| v.invariant == "prices_known"), "{:?}", h.violations);
    }

    // <!-- AMENDED: R5 — a flagship-priced model with a stale tier label is caught. -->
    #[test]
    fn a_flagship_priced_pick_is_a_violation_whatever_its_label() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/mislabelled", 0.05, ModelTier::Pro));
        let h = check_routing_health(&r, 0);
        assert!(h.violations.iter().any(|v| v.invariant == "efficient_never_flagship"), "{:?}", h.violations);
    }

    // <!-- AMENDED: G6 — health judges the catalog, not this process's keys. -->
    #[test]
    #[serial_test::file_serial] // <!-- AMENDED: R12 -->
    fn health_judges_the_catalog_not_this_processs_keys() {
        let mut r = ModelRegistry::default();
        let mut cloud = spec("acme/cloud", 0.01, ModelTier::Pro);
        cloud.provider_type = ProviderType::OpenRouter;
        r.register(cloud);
        crate::models::key_guard::set_test_key_availability(Some(vec![]));
        let h = check_routing_health(&r, 0);
        crate::models::key_guard::set_test_key_availability(None);
        assert_eq!(h.efficient_pick.as_deref(), Some("acme/cloud"));
    }

    #[test]
    fn health_round_trips_as_json() {
        let r = ModelRegistry::default();
        let h = check_routing_health(&r, 42);
        let back: RoutingHealth = serde_json::from_str(&serde_json::to_string(&h).unwrap()).unwrap();
        assert_eq!(back, h);
    }
}
```

In `model_catalog.rs` add at the end:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_health_passes_without_violations() {
        let json = r#"{"schema_version":1,"checked_at_unix":0,"models":9,"cloud_models":7,"benchmarked":3,
          "inherited":1,"unknown_tier_cloud":0,"quality_scale":"derived","price_bands":"fallback","violations":[]}"#;
        let c = routing_health_check(Some(json));
        assert!(c.pass, "{}", c.detail);
        assert!(c.detail.contains("3 of 7 cloud models benchmarked"), "{}", c.detail);
        assert!(c.detail.contains("price bands: built-in fallback"), "{}", c.detail);
    }

    #[test]
    fn routing_health_fails_and_names_each_violation() {
        let json = r#"{"schema_version":1,"checked_at_unix":0,"models":9,"cloud_models":7,"benchmarked":3,
          "inherited":1,"unknown_tier_cloud":5,"quality_scale":"derived","price_bands":"derived",
          "violations":[{"invariant":"tiers_known","detail":"5 of 7 cloud models have no tier"}]}"#;
        let c = routing_health_check(Some(json));
        assert!(!c.pass);
        assert!(c.detail.contains("tiers_known: 5 of 7 cloud models have no tier"), "{}", c.detail);
    }

    #[test]
    fn routing_health_missing_or_unreadable_fails_with_the_fix() {
        assert!(!routing_health_check(None).pass);
        assert!(routing_health_check(None).detail.contains("vox model discover"));
        assert!(!routing_health_check(Some("not json")).pass);
    }
}
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::health > target/routing-sm-t6-red.txt 2>&1; tail -12 target/routing-sm-t6-red.txt; timeout 1500s cargo test -p vox-cli --lib model_catalog >> target/routing-sm-t6-red.txt 2>&1; tail -8 target/routing-sm-t6-red.txt`
Expected: both FAIL to compile (`check_routing_health`, `routing_health_check` missing).

- [ ] **Step 3: Implement `health.rs`** (above the tests):

```rust
//! Routing health: invariants of the live registry, checked after every catalog refresh, logged,
//! persisted for `vox doctor`, and shown in the GUI. A violation means "the routing logic or its
//! data drifted — retune", never "edit a model name".

use serde::{Deserialize, Serialize};

use super::reference::{MIN_DERIVE_SAMPLE, ReferenceSource};
use super::spec::QualitySource;
use super::{ModelRegistry, ModelTier};
use crate::mode::ClutchProfile;

pub const ROUTING_HEALTH_SCHEMA_VERSION: u32 = 1;
/// User-preference key (scope `global`) holding the last [`RoutingHealth`] as JSON.
pub const ROUTING_HEALTH_KEY: &str = "model_routing_health";
/// User-preference key (scope `global`) holding the last catalog refresh time (unix seconds).
pub const MODEL_CATALOG_LAST_REFRESH_KEY: &str = "model_catalog_last_refresh";
/// Largest share of cloud models allowed to have no tier.
pub const MAX_UNKNOWN_TIER_SHARE: f64 = 0.2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub invariant: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingHealth {
    pub schema_version: u32,
    pub checked_at_unix: u64,
    pub models: usize,
    pub cloud_models: usize,
    pub benchmarked: usize,
    pub inherited: usize,
    pub unknown_tier_cloud: usize,
    pub quality_scale: ReferenceSource,
    pub price_bands: ReferenceSource,
    /// What Efficient picks at complexity 10 over the whole catalog (keys ignored).
    #[serde(default)]
    pub efficient_pick: Option<String>,
    pub violations: Vec<Violation>,
}

/// Check the registry's routing invariants now.
#[must_use]
pub fn check_routing_health(registry: &ModelRegistry, now_unix: u64) -> RoutingHealth {
    let models = registry.list_models();
    let cloud: Vec<_> = models
        .iter()
        .filter(|m| !crate::route_policy::is_local_http_provider(&m.provider_type))
        .collect();
    let prior_kind = |m: &super::ModelSpec| m.capabilities.quality_prior.as_ref().map(|p| p.source.clone());
    let benchmarked = cloud.iter().filter(|m| matches!(prior_kind(m), Some(QualitySource::Benchmark { .. }))).count();
    let inherited = cloud.iter().filter(|m| matches!(prior_kind(m), Some(QualitySource::Inherited { .. }))).count();
    let unknown_tier_cloud = cloud.iter().filter(|m| m.capabilities.tier == ModelTier::Unknown).count();
    let reference = registry.routing_reference();
    let mut violations = Vec::new();

    let mut task = crate::types::AgentTask::new(
        crate::types::TaskId(0),
        "routing health probe",
        crate::types::TaskPriority::Normal,
        vec![],
    );
    task.task_category = crate::types::TaskCategory::CodeGen;
    task.estimated_complexity = 10;
    // Probe the catalog, not this process's keys (`&|_| true`): health is about the routing logic and its
    // data, and two refreshers with different keys must write the same verdict.
    let mut efficient_pick = None;
    for clutch in [ClutchProfile::Efficiency, ClutchProfile::Balanced] {
        if let Some(sel) =
            registry.best_for_task_in_mode_keyed(&task, clutch.resolve().cost_preference, clutch, &|_| true, |_| None)
        {
            if clutch == ClutchProfile::Efficiency {
                efficient_pick = Some(sel.spec.id.clone());
            }
            // Judged by price, not by the tier label: the mode already drops `Elite`-labelled models,
            // so a flagship that slipped through is one whose label is wrong (stale bands, a hand-set
            // tier). <!-- AMENDED: R5 — a tier check here could never fire. -->
            if !sel.only_candidate && sel.spec.cost_per_1k_output >= reference.elite_min_out {
                violations.push(Violation {
                    invariant: "efficient_never_flagship".into(),
                    detail: format!(
                        "{clutch:?} picked {} at a flagship price while others fit",
                        sel.spec.id
                    ),
                });
            }
        }
    }
    if !cloud.is_empty() && unknown_tier_cloud as f64 / cloud.len() as f64 > MAX_UNKNOWN_TIER_SHARE {
        violations.push(Violation {
            invariant: "tiers_known".into(),
            detail: format!("{unknown_tier_cloud} of {} cloud models have no tier", cloud.len()),
        });
    }
    if cloud.len() >= MIN_DERIVE_SAMPLE && reference.bands_source == ReferenceSource::Fallback {
        violations.push(Violation {
            invariant: "prices_known".into(),
            detail: format!("{} cloud models but fewer than {MIN_DERIVE_SAMPLE} have a price", cloud.len()),
        });
    }

    RoutingHealth {
        schema_version: ROUTING_HEALTH_SCHEMA_VERSION,
        checked_at_unix: now_unix,
        models: models.len(),
        cloud_models: cloud.len(),
        benchmarked,
        inherited,
        unknown_tier_cloud,
        quality_scale: reference.quality_source,
        price_bands: reference.bands_source,
        efficient_pick,
        violations,
    }
}
```

If `AgentTask::new`'s signature or the `types` paths differ, adapt only the probe construction (copy
`mode_select.rs`'s `hard_task()`), and say so. <!-- AMENDED: G7 — the Genius invariant and GENIUS_QUALITY_FLOOR are
removed: no source for 0.9, and Genius legitimately trades quality against the other scored terms. -->

- [ ] **Step 4: One key constant.** In `registry.rs` replace
  `const MODEL_CATALOG_LAST_REFRESH_KEY: &str = "model_catalog_last_refresh";` with
  `use super::health::MODEL_CATALOG_LAST_REFRESH_KEY;`. In `catalog_refresh.rs` replace
  `pub const MODEL_CATALOG_LAST_REFRESH_KEY: &str = "model_catalog_last_refresh";` with
  `pub use crate::models::health::MODEL_CATALOG_LAST_REFRESH_KEY;`.
- [ ] **Step 5: Check and persist on refresh (`catalog_refresh.rs`).** Add after `persist_catalog_refresh_timestamp`:

```rust
/// Log each routing-health violation and persist the report for `vox doctor` and the GUI.
async fn persist_routing_health(health: &crate::models::health::RoutingHealth) {
    for v in &health.violations {
        tracing::warn!(
            target: "vox.orchestrator.catalog_refresh",
            invariant = %v.invariant,
            detail = %v.detail,
            "model routing health violation"
        );
    }
    if let (Ok(json), Ok(cfg)) = (serde_json::to_string(health), vox_db::DbConfig::resolve_canonical())
        && let Ok(db) = vox_db::VoxDb::connect(cfg).await
    {
        let _ = db.set_user_preference("global", crate::models::health::ROUTING_HEALTH_KEY, &json).await;
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
```

  In `refresh_once`, directly after the `tracing::info!(… "background catalog refresh applied")` call, add
  (under a **read** lock, not the merge's write lock — the check runs several rankings; <!-- AMENDED: R16 -->):

```rust
    let health = {
        let registry = orch.models.read().unwrap();
        crate::models::health::check_routing_health(&registry, unix_now())
    };
    persist_routing_health(&health).await;
    persist_catalog_refresh_timestamp().await; // <!-- AMENDED: R18 — the background loop never wrote it, so doctor's freshness check failed on a long-running daemon -->
```
  In `run_unified_catalog_refresh`, directly after `persist_catalog_refresh_timestamp().await;` add
  `persist_routing_health(&crate::models::health::check_routing_health(&registry, unix_now())).await;`.
  If `unix_now` already exists in the file under another name, reuse it and say so.
- [ ] **Step 6: `vox doctor` (`model_catalog.rs`).** Replace the literal `"catalog_refresh"` with
  `vox_orchestrator::models::health::MODEL_CATALOG_LAST_REFRESH_KEY`. Add:

```rust
/// The "Model routing" doctor row from the persisted [`RoutingHealth`] JSON.
pub(crate) fn routing_health_check(json: Option<&str>) -> Check {
    use vox_orchestrator::models::health::RoutingHealth;
    use vox_orchestrator::models::reference::ReferenceSource;
    const NAME: &str = "Model routing";
    let Some(json) = json else {
        return Check::fail(NAME, "No routing health recorded yet. Run `vox model discover`.");
    };
    let Ok(h) = serde_json::from_str::<RoutingHealth>(json) else {
        return Check::fail(NAME, "Routing health record is unreadable. Run `vox model discover`.");
    };
    let source = |s: ReferenceSource| if s == ReferenceSource::Derived { "live catalog" } else { "built-in fallback" };
    let summary = format!(
        "{} of {} cloud models benchmarked ({} inherited); quality scale: {}; price bands: {}; Efficient picks {}",
        h.benchmarked,
        h.cloud_models,
        h.inherited,
        source(h.quality_scale),
        source(h.price_bands),
        h.efficient_pick.as_deref().unwrap_or("nothing")
    );
    if h.violations.is_empty() {
        Check::pass(NAME, summary)
    } else {
        let problems: Vec<String> = h.violations.iter().map(|v| format!("{}: {}", v.invariant, v.detail)).collect();
        Check::fail(NAME, format!("{summary}. Problems: {}", problems.join("; ")))
    }
}
```

  and in `run`, inside `if let Some(db) = db_opt { … }` after the freshness `match`, add:
  `checks.push(routing_health_check(db.get_user_preference("global", vox_orchestrator::models::health::ROUTING_HEALTH_KEY).await.ok().flatten().as_deref()));`.
  (`db` is moved into the `match` today only by reference; if the borrow checker objects, bind the preference before
  the `match` and say so.)
- [ ] **Step 7: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-orchestrator --lib models::health 2>&1 | tail -8 && timeout 1500s cargo test -p vox-cli --lib model_catalog 2>&1 | tail -8 && timeout 1500s cargo test -p vox-orchestrator --lib 2>&1 | tail -5`
Expected: all pass. `rg -n '"catalog_refresh"' crates/vox-cli` prints nothing.
- [ ] **Step 8 (Claude): mutation proofs** — (a0) `sel.spec.cost_per_1k_output >= reference.elite_min_out` →
  `sel.spec.capabilities.tier == ModelTier::Elite` → `a_flagship_priced_pick_is_a_violation_whatever_its_label`;
  (a) `> MAX_UNKNOWN_TIER_SHARE` → `> 1.0` →
  `many_cloud_models_without_a_tier_are_a_violation`; (b) delete the `prices_known` push →
  `a_catalog_this_large_with_no_prices_is_a_violation`; (c) `if h.violations.is_empty()` → `if true` →
  `routing_health_fails_and_names_each_violation`; (c2) `&|_| true` → `&crate::models::key_guard::selection_key_available`
  in `check_routing_health` → `health_judges_the_catalog_not_this_processs_keys` <!-- AMENDED: G6 -->; (d) restore the `"catalog_refresh"` literal and confirm by reading
  that nothing else writes it (no test can cover the DB key; record the grep).
- [ ] **Step 9: Commit (Claude Code)**

```bash
git add crates/vox-orchestrator/src/models/health.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/orchestrator/catalog_refresh.rs crates/vox-orchestrator/src/models/registry.rs crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/model_catalog.rs
git commit -m "feat(models): check routing health on every catalog refresh; vox doctor reads the right key"
```

---

### Task 7: Tauri commands that explain routing from the ranking

**Files:** Create `crates/vox-gui/src/commands/routing_explain.rs`; modify `crates/vox-gui/src/commands/mod.rs`
(`pub mod routing_explain;`), `crates/vox-gui/src/main.rs` (two entries in `tauri::generate_handler![…]`, next to
`commands::models::suggest_model_for_task`), `crates/vox-gui/src/commands/models.rs` (`async fn
registry_with_scoreboard` → `pub(crate) async fn registry_with_scoreboard`, and the telemetry mirroring below;
nothing else), `crates/vox-gui/Cargo.toml` (`[dev-dependencies]` gains
`vox-orchestrator = { workspace = true, features = ["test-support"] }` — an existing dependency, test builds only, so
the tests can control provider-key availability; <!-- AMENDED: R6 -->),
`crates/vox-gui/ui/src/types/tauri.ts` (append the TS interfaces).

**Interfaces — produces (wire shapes, snake_case):**
`RouteExplanationDto { mode, task, complexity, chosen: string|null, only_candidate, total_models,
candidates: CandidateDto[], excluded: ExclusionGroupDto[] }`;
`CandidateDto { id, provider, tier, is_free, price_out_per_m: number|null, score, quality: QualityDto,
parts: PartsDto }`; `QualityDto { value, source: 'benchmark'|'inherited'|'estimate'|'unknown', index: number|null,
inherited_from: string|null }`; `PartsDto { quality, efficiency, latency, other, bonuses }` (each the weighted share
of `score`); `ExclusionGroupDto { reason, count, examples }` (`reason` is the `Exclusion` `kind`); commands
`explain_routing(task, mode, complexity)` and `get_routing_health()` (returns `RoutingHealth`).

- [ ] **Step 1: Write the failing tests** — create `routing_explain.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator::models::ranking::{Exclusion, RankedModel, Ranking};
    use vox_orchestrator::models::scoring::auto_score_parts;
    use vox_orchestrator::models::spec::{PricingSource, QualityPrior, QualitySource};
    use vox_orchestrator::models::{ModelCapabilities, ModelSpec, ModelTier, ProviderType, StrengthTag};

    fn spec(id: &str, out_per_1k: f64, source: QualitySource) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "acme".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 64_000,
            cost_per_1k: out_per_1k,
            cost_per_1k_input: out_per_1k,
            cost_per_1k_output: out_per_1k,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                tier: ModelTier::Pro,
                quality_prior: Some(QualityPrior { value: 0.5, source }),
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::OpenRouter,
            supported_parameters: vec![],
        }
    }

    fn ranked(m: ModelSpec) -> RankedModel {
        let parts = auto_score_parts(&m, 5, false, None, vox_orchestrator::config::CostPreference::Economy, None, None);
        RankedModel { spec: m, parts }
    }

    fn ranking() -> Ranking {
        let mut ranked_models: Vec<RankedModel> = (0..12)
            .map(|i| ranked(spec(&format!("acme/m-{i}"), 0.001, QualitySource::Estimate)))
            .collect();
        ranked_models[0] = ranked(spec("acme/bench", 0.0009, QualitySource::Benchmark { index: 46.3 }));
        ranked_models[1] = ranked(spec("acme/heir", 0.0, QualitySource::Inherited { index: 38.2, from: "acme/widget-5.0".into() }));
        Ranking {
            ranked: ranked_models,
            // Superseded first, so an unsorted grouping comes out in the wrong order (R13).
            excluded: vec![
                ("acme/old".into(), Exclusion::Superseded { by: "acme/new".into() }),
                ("acme/x1".into(), Exclusion::NoProviderKey),
                ("acme/x2".into(), Exclusion::NoProviderKey),
                ("acme/x3".into(), Exclusion::NoProviderKey),
                ("acme/x4".into(), Exclusion::NoProviderKey),
            ],
        }
    }

    #[test]
    fn the_explanation_is_built_from_the_ranking() {
        let dto = explanation_dto(ClutchProfile::Efficiency, TaskCategory::CodeGen, 7, 99, &ranking(), false);
        assert_eq!(dto.mode, "efficiency");
        assert_eq!(dto.task, "codegen");
        assert_eq!(dto.chosen.as_deref(), Some("acme/bench"));
        assert_eq!(dto.candidates.len(), MAX_CANDIDATES);
        assert_eq!(dto.total_models, 99);
    }

    #[test]
    fn every_number_carries_its_provenance() {
        let dto = explanation_dto(ClutchProfile::Balanced, TaskCategory::General, 5, 12, &ranking(), false);
        let bench = &dto.candidates[0];
        assert_eq!(bench.quality.source, "benchmark");
        assert_eq!(bench.quality.index, Some(46.3));
        assert!((bench.price_out_per_m.unwrap() - 0.9).abs() < 1e-9);
        let heir = &dto.candidates[1];
        assert_eq!(heir.quality.source, "inherited");
        assert_eq!(heir.quality.inherited_from.as_deref(), Some("acme/widget-5.0"));
        assert_eq!(heir.price_out_per_m, None, "an unknown paid price is None, never 0");
        assert_eq!(dto.candidates[2].quality.source, "estimate");
    }

    #[test]
    fn the_parts_add_up_to_the_score() {
        let dto = explanation_dto(ClutchProfile::Genius, TaskCategory::CodeGen, 5, 12, &ranking(), false);
        for c in &dto.candidates {
            let sum = c.parts.quality + c.parts.efficiency + c.parts.latency + c.parts.other + c.parts.bonuses;
            assert!((sum - c.score).abs() < 1e-9, "{}: {sum} vs {}", c.id, c.score);
        }
    }

    #[test]
    fn exclusions_are_grouped_by_reason_largest_first_with_few_examples() {
        let dto = explanation_dto(ClutchProfile::Efficiency, TaskCategory::CodeGen, 5, 12, &ranking(), false);
        assert_eq!(dto.excluded[0].reason, "no_provider_key");
        assert_eq!(dto.excluded[0].count, 4);
        assert_eq!(dto.excluded[0].examples.len(), MAX_EXAMPLES);
        assert_eq!(dto.excluded[1].reason, "superseded");
        assert_eq!(dto.excluded[1].examples, vec!["acme/old".to_string()]);
    }

    #[test]
    fn free_mode_explains_a_free_choice_and_why_paid_models_are_out() {
        let mut reg = vox_orchestrator::models::ModelRegistry::default();
        let mut paid = spec("acme/paid", 0.001, QualitySource::Estimate);
        paid.provider_type = ProviderType::Ollama;
        let mut free = spec("acme/free", 0.0, QualitySource::Estimate);
        free.provider_type = ProviderType::Ollama;
        free.is_free = true;
        reg.register(paid);
        reg.register(free);
        let dto = explain(&reg, ClutchProfile::Free, TaskCategory::General, 5);
        assert_eq!(dto.chosen.as_deref(), Some("acme/free"));
        assert!(
            dto.excluded.iter().any(|g| g.reason == "not_free" && g.examples == vec!["acme/paid".to_string()]),
            "{:?}",
            dto.excluded
        );
    }

    // <!-- AMENDED: R6 -->
    #[test]
    fn when_dispatch_would_choose_nothing_no_model_is_named() {
        let mut reg = vox_orchestrator::models::ModelRegistry::default();
        reg.register(spec("acme/keyless", 0.001, QualitySource::Estimate)); // OpenRouter
        vox_orchestrator::models::key_guard::set_test_key_availability(Some(vec![]));
        let dto = explain(&reg, ClutchProfile::Efficiency, TaskCategory::General, 5);
        vox_orchestrator::models::key_guard::set_test_key_availability(None);
        assert_eq!(dto.chosen, None);
        assert_eq!(dto.candidates.first().map(|c| c.id.as_str()), Some("acme/keyless"), "still shown, not chosen");
    }

    #[test]
    fn labels_parse_and_unknown_labels_are_rejected() {
        assert_eq!(task_category_from_label("CodeGen"), Some(TaskCategory::CodeGen));
        assert_eq!(task_category_from_label("general"), Some(TaskCategory::General));
        assert_eq!(task_category_from_label("nope"), None);
    }
}
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-gui --bin vox-gui routing_explain > target/routing-sm-t7-red.txt 2>&1; tail -15 target/routing-sm-t7-red.txt`
Expected: FAIL to compile (`explanation_dto` missing). If `cargo test -p vox-gui` fails inside `tauri-build`
("resource path … doesn't exist" or missing `ui/dist`), that is the known fresh-worktree sidecar issue in AGENTS.md
(Perennial Bug Patterns): STOP and report; do not "fix" it.

- [ ] **Step 3: Implement** (above the tests):

```rust
//! Tauri commands that explain model routing from the selector's own ranking
//! (`vox_orchestrator::models::ranking`). Nothing here recomputes a score.

use serde::Serialize;
use vox_orchestrator::mode::ClutchProfile;
use vox_orchestrator::models::health::{RoutingHealth, check_routing_health};
use vox_orchestrator::models::ranking::{Exclusion, Ranking};
use vox_orchestrator::models::spec::QualitySource;
use vox_orchestrator::models::TaskCategory;

/// Candidates shown, best first.
pub const MAX_CANDIDATES: usize = 10;
/// Example ids shown per exclusion reason.
pub const MAX_EXAMPLES: usize = 3;

#[derive(Debug, Serialize, PartialEq)]
pub struct QualityDto {
    pub value: f64,
    /// `benchmark` | `inherited` | `estimate` | `unknown`.
    pub source: &'static str,
    pub index: Option<f32>,
    pub inherited_from: Option<String>,
}

/// Each field is that group's weighted share of `score`; together they sum to it.
#[derive(Debug, Serialize, PartialEq)]
pub struct PartsDto {
    pub quality: f64,
    pub efficiency: f64,
    pub latency: f64,
    /// Availability + context balance + mobile.
    pub other: f64,
    /// Free, off-peak, telemetry, fill-in-middle and VRAM adjustments.
    pub bonuses: f64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CandidateDto {
    pub id: String,
    pub provider: String,
    pub tier: String,
    pub is_free: bool,
    /// USD per million output tokens; `None` when a paid model's price is unknown.
    pub price_out_per_m: Option<f64>,
    pub score: f64,
    pub quality: QualityDto,
    pub parts: PartsDto,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ExclusionGroupDto {
    /// The `Exclusion` kind (`no_provider_key`, `superseded`, …).
    pub reason: String,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RouteExplanationDto {
    pub mode: String,
    pub task: String,
    pub complexity: u8,
    pub chosen: Option<String>,
    /// The mode's preferred models were all unavailable; the choice came from the fallback pass.
    pub only_candidate: bool,
    pub total_models: usize,
    pub candidates: Vec<CandidateDto>,
    pub excluded: Vec<ExclusionGroupDto>,
}

/// `codegen` | `research` | `review` | `general` (case-insensitive).
#[must_use]
pub fn task_category_from_label(label: &str) -> Option<TaskCategory> {
    match label.trim().to_ascii_lowercase().as_str() {
        "codegen" => Some(TaskCategory::CodeGen),
        "research" => Some(TaskCategory::Research),
        "review" => Some(TaskCategory::Review),
        "general" => Some(TaskCategory::General),
        _ => None,
    }
}

fn exclusion_kind(e: &Exclusion) -> String {
    serde_json::to_value(e)
        .ok()
        .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(str::to_string))
        .unwrap_or_else(|| "unknown".into())
}

fn quality_dto(m: &vox_orchestrator::models::ModelSpec, value: f64) -> QualityDto {
    match m.capabilities.quality_prior.as_ref().map(|p| &p.source) {
        Some(QualitySource::Benchmark { index }) => QualityDto { value, source: "benchmark", index: Some(*index), inherited_from: None },
        Some(QualitySource::Inherited { index, from }) => {
            QualityDto { value, source: "inherited", index: Some(*index), inherited_from: Some(from.clone()) }
        }
        Some(QualitySource::Estimate) => QualityDto { value, source: "estimate", index: None, inherited_from: None },
        None => QualityDto { value, source: "unknown", index: None, inherited_from: None },
    }
}

/// Build the explanation from a ranking. Pure: no registry, no I/O.
#[must_use]
pub fn explanation_dto(
    mode: ClutchProfile,
    task: TaskCategory,
    complexity: u8,
    total_models: usize,
    ranking: &Ranking,
    only_candidate: bool,
) -> RouteExplanationDto {
    let candidates = ranking
        .ranked
        .iter()
        .take(MAX_CANDIDATES)
        .map(|r| {
            let p = &r.parts;
            let share = |a: &vox_orchestrator::models::scoring::AxisPart| f64::from(a.weight) * a.value / p.weight_sum;
            let price = r.spec.cost_per_1k_output;
            CandidateDto {
                id: r.spec.id.clone(),
                provider: r.spec.provider.clone(),
                tier: format!("{:?}", r.spec.capabilities.tier),
                is_free: r.spec.is_free,
                price_out_per_m: if r.spec.is_free { Some(0.0) } else if price.is_finite() && price > 0.0 { Some(price * 1000.0) } else { None },
                score: p.total,
                quality: quality_dto(&r.spec, p.quality.value),
                parts: PartsDto {
                    quality: share(&p.quality),
                    efficiency: share(&p.efficiency),
                    latency: share(&p.latency),
                    other: share(&p.availability) + share(&p.balance) + share(&p.mobile),
                    bonuses: p.fill_in_middle + p.free_bonus + p.off_peak_bonus + p.telemetry + p.vram,
                },
            }
        })
        .collect();
    let mut groups: Vec<ExclusionGroupDto> = Vec::new();
    for (id, e) in &ranking.excluded {
        let kind = exclusion_kind(e);
        match groups.iter_mut().find(|g| g.reason == kind) {
            Some(g) => {
                g.count += 1;
                if g.examples.len() < MAX_EXAMPLES {
                    g.examples.push(id.clone());
                }
            }
            None => groups.push(ExclusionGroupDto { reason: kind, count: 1, examples: vec![id.clone()] }),
        }
    }
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));
    RouteExplanationDto {
        mode: serde_json::to_value(mode).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default(),
        task: format!("{task:?}").to_ascii_lowercase(),
        complexity,
        chosen: ranking.chosen().map(|m| m.id.clone()),
        only_candidate,
        total_models,
        candidates,
        excluded: groups,
    }
}

/// The explanation for `registry` (pure: no DB, no I/O). <!-- AMENDED: G8, G9 -->
#[must_use]
pub fn explain(
    registry: &vox_orchestrator::models::ModelRegistry,
    clutch: ClutchProfile,
    task_category: TaskCategory,
    complexity: u8,
) -> RouteExplanationDto {
    let mut probe = vox_orchestrator::types::AgentTask::new(
        vox_orchestrator::types::TaskId(0),
        "routing explanation",
        vox_orchestrator::types::TaskPriority::Normal,
        vec![],
    );
    probe.task_category = task_category;
    probe.estimated_complexity = complexity;
    let resolved = clutch.resolve();
    // Dispatch's own rules (runtime.rs), minus the two that need the running orchestrator's live state
    // (exploration spend, provider usage); the panel says so.
    let gate = vox_orchestrator::models::mode_select::DispatchGate {
        force_free_pool: resolved.force_free_pool,
        ..Default::default()
    };
    match registry.best_for_task_under_gate(&probe, resolved.cost_preference, clutch, &gate) {
        Some(sel) => explanation_dto(clutch, task_category, complexity, registry.list_models().len(), &sel.ranking, sel.only_candidate),
        None => {
            // Dispatch would choose nothing (no key, or every model gated): show why, but name no choice.
            // <!-- AMENDED: R6 — the fallback ranking has no key check, so its first entry is not a choice. -->
            let ranking = registry.rank_task_with_filter(&probe, resolved.cost_preference, |m| gate.exclusion(m));
            let mut dto = explanation_dto(clutch, task_category, complexity, registry.list_models().len(), &ranking, false);
            dto.chosen = None;
            dto
        }
    }
}

/// How task dispatch would choose now for `mode` / `task` / `complexity` (1–10). Not chat: chat resolves
/// through `models::select::decide` (premium aliases, confidence gating, per-request axes).
#[tauri::command]
pub async fn explain_routing(task: String, mode: String, complexity: u8) -> Result<RouteExplanationDto, String> {
    let clutch = ClutchProfile::from_label(&mode).ok_or("unknown mode")?;
    let task_category = task_category_from_label(&task).ok_or("unknown task")?;
    let registry = super::models::registry_with_scoreboard().await;
    Ok(explain(&registry, clutch, task_category, complexity.clamp(1, 10)))
}

/// The registry's routing health, checked now.
#[tauri::command]
pub async fn get_routing_health() -> Result<RoutingHealth, String> {
    let registry = super::models::registry_with_scoreboard().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(check_routing_health(&registry, now))
}
```

<!-- AMENDED: G13 — "the registry" is rebuilt from the same inputs dispatch uses (orchestrator/core/telemetry.rs):
observed latency and observed prices, then the derived reference. --> In `models.rs#registry_with_scoreboard`, inside
`if let Ok(rows) = db.get_model_scoreboard(7).await {`, add `reg.inject_scoreboard_latency(&rows);` as the first
statement (before `rows` is consumed); after that `if` block add:

```rust
    if let Ok(pricing) = db.get_pricing_catalog().await {
        reg.inject_pricing_catalog(pricing);
    }
    reg.apply_routing_reference();
```

This edit has no unit test (it needs a workspace DB); Task 12's live check covers it — it is this plan's one
admitted unguarded edit, recorded in the commit body.

The `TaskCategory` `Debug` names give `codegen`, `research`, `review`, `general` when lower-cased; if any variant's
`Debug` differs, map it explicitly and say so. Register both commands in `main.rs`'s `generate_handler!` directly
after `commands::models::suggest_model_for_task,`. Append to `tauri.ts`:

```ts
export interface RouteQuality {
  value: number;
  source: 'benchmark' | 'inherited' | 'estimate' | 'unknown';
  index: number | null;
  inherited_from: string | null;
}
export interface RouteScoreParts { quality: number; efficiency: number; latency: number; other: number; bonuses: number }
export interface RouteCandidate {
  id: string; provider: string; tier: string; is_free: boolean;
  price_out_per_m: number | null; score: number; quality: RouteQuality; parts: RouteScoreParts;
}
export interface RouteExclusionGroup { reason: string; count: number; examples: string[] }
export interface RouteExplanation {
  mode: string; task: string; complexity: number; chosen: string | null; only_candidate: boolean;
  total_models: number; candidates: RouteCandidate[]; excluded: RouteExclusionGroup[];
}
export interface RoutingHealthViolation { invariant: string; detail: string }
export interface RoutingHealth {
  schema_version: number; checked_at_unix: number; models: number; cloud_models: number;
  benchmarked: number; inherited: number; unknown_tier_cloud: number;
  quality_scale: 'derived' | 'fallback'; price_bands: 'derived' | 'fallback';
  efficient_pick: string | null; // <!-- AMENDED: A13 -->
  violations: RoutingHealthViolation[];
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 1500s cargo test -p vox-gui --bin vox-gui routing_explain 2>&1 | tail -10 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -3`
Expected: all pass; typecheck clean.
- [ ] **Step 5 (Claude): mutation proofs** — (a) `.take(MAX_CANDIDATES)` removed →
  `the_explanation_is_built_from_the_ranking`; (b) `else { None }` in `price_out_per_m` → `Some(0.0)` →
  `every_number_carries_its_provenance`; (c) `other: share(&p.availability) + …` → `share(&p.availability)` →
  `the_parts_add_up_to_the_score`; (d) `groups.sort_by(…)` removed → the grouping test; (e) `force_free_pool:
  resolved.force_free_pool` → `false` → `free_mode_explains_a_free_choice_and_why_paid_models_are_out`; (f) delete `dto.chosen = None;` →
  `when_dispatch_would_choose_nothing_no_model_is_named`. Confirm with
  `rg -n "explain_routing|get_routing_health" crates/vox-gui/src/main.rs` (two lines).
- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-gui/src/commands/routing_explain.rs crates/vox-gui/src/commands/mod.rs crates/vox-gui/src/main.rs crates/vox-gui/src/commands/models.rs crates/vox-gui/Cargo.toml crates/vox-gui/ui/src/types/tauri.ts
git commit -m "feat(gui): Tauri commands that explain routing from the selector's ranking"
```

---

### Task 8: `RoutingExplainer` — the panel, its labels and its hook

**Pre-flight** <!-- AMENDED: G12 — trace T5 and T6 also edit turnEvents.ts; wait for the whole trace plan. -->
(any failure is a STOP):
1. `git log --oneline -- crates/vox-gui/ui/src/lib/turnEvents.ts` shows the trace plan's Task 4, 5 and 6 commits,
   and every task of `2026-09-28-chat-turn-trace.md` is committed.
2. `git status --porcelain crates/vox-gui/ui/src/lib/turnEvents.ts crates/vox-gui/ui/src/lib/turnEvents.test.ts`
   prints nothing.
3. `rg -n "export const MODE_NAMES|export function modeLabel" crates/vox-gui/ui/src/lib/turnEvents.ts` prints two
   lines.

**Files:** Create `lib/routingLabels.ts` + `.test.ts`, `hooks/useRoutingExplanation.ts` + `.test.ts`,
`components/surfaces/Models/RoutingExplainer.tsx` + `.test.tsx` (all under `crates/vox-gui/ui/src/`). Modify
`lib/turnEvents.ts` (append `MODE_OBJECTIVES` and `modeObjective`; nothing else) and `transport.ts` (add `explainRouting(mode, task, complexity)` and `getRoutingHealth()` to the transport class). <!-- AMENDED: the hook calls `voxTransport`, never `invoke`: `src/guards/ipcBoundaries.test.ts` allows `invoke` only in `transport.ts`, and the hook test mocks `../transport`. -->

- [ ] **Step 1: Write the failing tests.**

`lib/routingLabels.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { exclusionLabel, qualityLabel, priceLabel } from './routingLabels';

describe('routingLabels', () => {
  it('names every exclusion reason in plain English and never shows a code name', () => {
    for (const kind of ['penalized', 'free_in_performance_mode', 'over_request_cost_cap', 'over_task_budget',
      'route_policy', 'privacy_local_only', 'strength_mismatch', 'filtered', 'no_provider_key',
      'flagship_excluded_by_mode', 'not_free', 'exploration_budget_spent', 'provider_budget_exhausted',
      'superseded']) {
      const label = exclusionLabel(kind);
      expect(label).not.toContain('_');
      expect(label.length).toBeGreaterThan(3);
    }
    expect(exclusionLabel('something_new')).toBe('Other');
  });

  it('says where a quality number came from', () => {
    expect(qualityLabel({ value: 0.8, source: 'benchmark', index: 46.3, inherited_from: null })).toBe('benchmark 46.3');
    expect(qualityLabel({ value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' }))
      .toBe('from acme/widget-5.0 (38.2)');
    expect(qualityLabel({ value: 0.4, source: 'estimate', index: null, inherited_from: null })).toBe('estimate');
    expect(qualityLabel({ value: 0, source: 'unknown', index: null, inherited_from: null })).toBe('—');
  });

  it('formats price per million and never shows 0 for an unknown price', () => {
    expect(priceLabel(0.9, false)).toBe('$0.90/M');
    expect(priceLabel(20, false)).toBe('$20.00/M');
    expect(priceLabel(0, true)).toBe('free');
    expect(priceLabel(null, false)).toBe('—');
  });
});
```

`components/surfaces/Models/RoutingExplainer.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import { RoutingExplainer } from './RoutingExplainer';
import type { RouteExplanation, RoutingHealth } from '../../../types/tauri';

const parts = { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 };
const explanation: RouteExplanation = {
  mode: 'efficiency', task: 'codegen', complexity: 7, chosen: 'acme/widget-5.5', only_candidate: false,
  total_models: 120,
  candidates: [
    { id: 'acme/widget-5.5', provider: 'acme', tier: 'Fast', is_free: false, price_out_per_m: 0.9, score: 0.65,
      quality: { value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' }, parts },
    { id: 'acme/gadget-2', provider: 'acme', tier: 'Pro', is_free: false, price_out_per_m: null, score: 0.61,
      quality: { value: 0.8, source: 'benchmark', index: 46.3, inherited_from: null }, parts },
  ],
  excluded: [
    { reason: 'flagship_excluded_by_mode', count: 4, examples: ['acme/flagship-9', 'acme/flagship-8', 'acme/flagship-7'] },
    { reason: 'superseded', count: 1, examples: ['acme/widget-4.8'] },
  ],
};
const healthy: RoutingHealth = {
  schema_version: 1, checked_at_unix: 0, models: 120, cloud_models: 100, benchmarked: 40, inherited: 7,
  unknown_tier_cloud: 0, quality_scale: 'derived', price_bands: 'derived', violations: [],
};

describe('RoutingExplainer', () => {
  it('leads with the chosen model and the mode in plain language', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    expect(screen.getByRole('heading', { name: /why acme\/widget-5\.5/i })).toBeTruthy();
    expect(screen.getByText(/^Efficient/)).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/intel=|eff=|lat=|_/);
  });

  // <!-- AMENDED: G8, G9, G13 — the panel explains task dispatch (not chat) and says what it cannot see. -->
  it('says what it explains and what it does not see', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    expect(screen.getByText(/How task dispatch would choose now/)).toBeTruthy();
    const scope = screen.getByTestId('routing-scope-note').textContent ?? '';
    expect(scope).toMatch(/exploration budget or provider usage limits/);
    expect(scope).toMatch(/last scoreboard refresh/);
  });

  it('ranks candidates in a real table and marks the choice in text', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    const table = screen.getByRole('table', { name: /candidates ranked by routing score/i });
    const rows = within(table).getAllByRole('row').slice(1);
    expect(rows).toHaveLength(2);
    expect(within(rows[0]).getByText('Chosen')).toBeTruthy();
    expect(within(rows[0]).getByText('from acme/widget-5.0 (38.2)')).toBeTruthy();
    expect(within(rows[1]).getByText('benchmark 46.3')).toBeTruthy();
    expect(within(rows[1]).getByText('—')).toBeTruthy();
    expect(within(table).getAllByRole('columnheader').every(h => h.getAttribute('scope') === 'col')).toBe(true);
  });

  it('keeps the excluded models behind a collapsed disclosure with counts', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    const details = screen.getByTestId('routing-excluded') as HTMLDetailsElement;
    expect(details.open).toBe(false);
    expect(within(details).getByText(/not considered \(5\)/i)).toBeTruthy();
    expect(within(details).getByText(/flagship.*\(4\)/i)).toBeTruthy();
  });

  it('says so when only a fallback model fits', () => {
    render(<RoutingExplainer explanation={{ ...explanation, only_candidate: true }} health={healthy} />);
    expect(screen.getByRole('note').textContent).toMatch(/only/i);
  });

  it('draws attention to health only when something is wrong', () => {
    const { rerender } = render(<RoutingExplainer explanation={explanation} health={healthy} />);
    expect(screen.getByTestId('routing-health').getAttribute('data-state')).toBe('ok');
    rerender(<RoutingExplainer explanation={explanation} health={{ ...healthy, price_bands: 'fallback',
      violations: [{ invariant: 'tiers_known', detail: '30 of 100 cloud models have no tier' }] }} />);
    const footer = screen.getByTestId('routing-health');
    expect(footer.getAttribute('data-state')).toBe('warn');
    expect(footer.textContent).toContain('30 of 100 cloud models have no tier');
    expect(footer.textContent).toMatch(/built-in fallback/);
  });

  it('renders a placeholder only before the first explanation, and keeps content while reloading', () => {
    const { rerender } = render(<RoutingExplainer explanation={null} health={null} loading />);
    expect(screen.getByTestId('routing-explainer-loading').getAttribute('aria-busy')).toBe('true');
    rerender(<RoutingExplainer explanation={explanation} health={healthy} loading />);
    expect(screen.queryByTestId('routing-explainer-loading')).toBeNull();
    expect(screen.getByTestId('routing-explainer').getAttribute('aria-busy')).toBe('true');
  });
});
```

`hooks/useRoutingExplanation.test.ts`:

```ts
// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invokeMock(...a) }));

import { useRoutingExplanation } from './useRoutingExplanation';

beforeEach(() => invokeMock.mockReset());

describe('useRoutingExplanation', () => {
  it('asks for the explanation and the health for the given mode, task and complexity', async () => {
    invokeMock.mockImplementation((cmd: string) =>
      Promise.resolve(cmd === 'explain_routing' ? { chosen: 'acme/widget-5.5', candidates: [], excluded: [] } : { violations: [] }));
    const { result } = renderHook(() => useRoutingExplanation('balanced', 'codegen', 7));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(invokeMock).toHaveBeenCalledWith('explain_routing', { mode: 'balanced', task: 'codegen', complexity: 7 });
    expect(invokeMock).toHaveBeenCalledWith('get_routing_health');
    expect(result.current.explanation?.chosen).toBe('acme/widget-5.5');
    expect(result.current.error).toBeNull();
  });

  it('treats a null or failed response as unavailable, not as a crash', async () => {
    invokeMock.mockImplementation((cmd: string) => (cmd === 'explain_routing' ? Promise.reject(new Error('x')) : Promise.resolve(null)));
    const { result } = renderHook(() => useRoutingExplanation('efficiency', 'general', 5));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.explanation).toBeNull();
    expect(result.current.health).toBeNull();
    expect(result.current.error).toBe('Routing explanation unavailable');
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/routingLabels.test.ts src/components/surfaces/Models/RoutingExplainer.test.tsx src/hooks/useRoutingExplanation.test.ts > target/routing-sm-t8-red.txt 2>&1; tail -20 target/routing-sm-t8-red.txt`
Expected: FAIL (modules not found).

- [ ] **Step 3: Implement.**

`lib/turnEvents.ts` — append (the objectives are the promises the model-routing plans made true):

```ts
/** One-sentence objective per mode, keyed by the `MODE_NAMES` wire value. */
export const MODE_OBJECTIVES: Readonly<Record<string, string>> = Object.freeze({
  free: 'free models only',
  efficiency: 'best quality per dollar; flagships only when nothing else fits',
  balanced: 'quality, cost and speed weighted evenly; flagships only when nothing else fits',
  genius: 'highest quality regardless of cost',
});

export function modeObjective(mode: string): string {
  return MODE_OBJECTIVES[mode] ?? '';
}
```

`lib/routingLabels.ts`:

```ts
import type { RouteQuality } from '../types/tauri';

/** Plain-English reason for each `Exclusion` kind (Rust: `models::ranking::Exclusion`). */
const EXCLUSION_LABELS: Readonly<Record<string, string>> = Object.freeze({
  penalized: 'Recently failed this kind of task',
  free_in_performance_mode: 'Free model skipped in this mode',
  over_request_cost_cap: 'Over the per-request cost cap',
  over_task_budget: 'Over this task’s budget',
  route_policy: 'Excluded by routing policy',
  privacy_local_only: 'Cloud model, local-only privacy is on',
  strength_mismatch: 'Not suited to this task',
  filtered: 'Excluded by this request',
  no_provider_key: 'No API key for its provider',
  flagship_excluded_by_mode: 'Flagship, kept out by this mode',
  not_free: 'Paid model, Free mode is on',
  exploration_budget_spent: 'Unpriced, today’s exploration budget is spent',
  provider_budget_exhausted: 'Its provider’s usage budget is spent',
  superseded: 'A newer version is available',
});

export function exclusionLabel(kind: string): string {
  return EXCLUSION_LABELS[kind] ?? 'Other';
}

export function qualityLabel(q: RouteQuality): string {
  switch (q.source) {
    case 'benchmark': return `benchmark ${q.index?.toFixed(1)}`;
    case 'inherited': return `from ${q.inherited_from} (${q.index?.toFixed(1)})`;
    case 'estimate': return 'estimate';
    default: return '—';
  }
}

export function priceLabel(perMillion: number | null, isFree: boolean): string {
  if (isFree) return 'free';
  if (perMillion == null) return '—';
  return `$${perMillion.toFixed(2)}/M`;
}
```

`hooks/useRoutingExplanation.ts`:

```ts
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { RouteExplanation, RoutingHealth } from '../types/tauri';

export interface RoutingExplanationState {
  explanation: RouteExplanation | null;
  health: RoutingHealth | null;
  loading: boolean;
  error: string | null;
}

/** How routing would choose now for a mode, task and complexity, plus routing health. */
export function useRoutingExplanation(mode: string, task: string, complexity: number): RoutingExplanationState {
  const [state, setState] = useState<RoutingExplanationState>({ explanation: null, health: null, loading: true, error: null });
  useEffect(() => {
    let live = true;
    setState(s => ({ ...s, loading: true }));
    Promise.allSettled([
      invoke<RouteExplanation | null>('explain_routing', { mode, task, complexity }),
      invoke<RoutingHealth | null>('get_routing_health'),
    ]).then(([exp, health]) => {
      if (!live) return;
      const explanation = exp.status === 'fulfilled' ? exp.value ?? null : null;
      setState({
        explanation,
        health: health.status === 'fulfilled' ? health.value ?? null : null,
        loading: false,
        error: explanation ? null : 'Routing explanation unavailable',
      });
    });
    return () => { live = false; };
  }, [mode, task, complexity]);
  return state;
}
```

`components/surfaces/Models/RoutingExplainer.tsx`:

```tsx
import React from 'react';
import type { RouteExplanation, RoutingHealth } from '../../../types/tauri';
import { modeLabel, modeObjective } from '../../../lib/turnEvents';
import { exclusionLabel, priceLabel, qualityLabel } from '../../../lib/routingLabels';

interface Props {
  explanation: RouteExplanation | null;
  health: RoutingHealth | null;
  loading?: boolean;
}

const SEGMENTS: Array<{ key: 'quality' | 'efficiency' | 'latency' | 'other'; color: string }> = [
  { key: 'quality', color: 'var(--color-accent-default)' },
  { key: 'efficiency', color: 'var(--color-accent-secondary)' },
  { key: 'latency', color: 'var(--color-status-info)' },
  { key: 'other', color: 'var(--color-neutral-400)' },
];

export function RoutingExplainer({ explanation, health, loading = false }: Props) {
  // <!-- AMENDED: R8 — skeleton only before the first explanation; reloads keep the old content (no layout
  // shift); `aria-busy` follows `loading`. -->
  if (!explanation) {
    return (
      <div data-testid="routing-explainer-loading" className="min-h-72 rounded-lg bg-overlay-subtle" aria-busy={loading}>
        {!loading && <p className="p-4 text-xs text-text-muted">Routing explanation unavailable</p>}
      </div>
    );
  }
  const excludedTotal = explanation.excluded.reduce((n, g) => n + g.count, 0);
  const problems = health?.violations ?? [];
  const fallback = health && (health.quality_scale === 'fallback' || health.price_bands === 'fallback');
  const healthState = problems.length > 0 ? 'warn' : 'ok';
  return (
    <section data-testid="routing-explainer" aria-labelledby="routing-explainer-title" aria-busy={loading}
      className="flex min-h-72 min-w-0 flex-col gap-3">
      <header>
        <h3 id="routing-explainer-title" className="text-sm text-text-primary">
          Why {explanation.chosen ?? 'no model'}
        </h3>
        <p className="text-xs text-text-muted">How task dispatch would choose now</p>
        <p className="text-xs text-text-muted">
          {modeLabel(explanation.mode)}: {modeObjective(explanation.mode)}
        </p>
        {explanation.only_candidate && (
          <p role="note" className="text-xs" style={{ color: 'var(--color-status-warn)' }}>
            Only this model fits; every model this mode prefers is unavailable.
          </p>
        )}
      </header>
      <div className="overflow-x-auto">
      <table className="w-full table-fixed text-xs tabular-nums">
        <caption className="sr-only">Candidates ranked by routing score</caption>
        <thead>
          <tr className="text-left text-text-muted">
            <th scope="col" className="w-8">#</th>
            <th scope="col">Model</th>
            <th scope="col" className="hidden w-16 sm:table-cell">Tier</th>
            <th scope="col" className="w-28 sm:w-48">Quality</th>
            <th scope="col" className="w-20">Price</th>
            <th scope="col" className="w-16 sm:w-40">Score</th>
          </tr>
        </thead>
        <tbody>
          {explanation.candidates.map((c, i) => {
            const chosen = c.id === explanation.chosen;
            return (
              <tr key={c.id} data-chosen={chosen ? 'true' : undefined}>
                <td>{i + 1}</td>
                <td className="truncate font-mono" title={c.id}>
                  {c.id} {chosen && <span className="ml-1 text-text-primary">Chosen</span>}
                </td>
                <td className="hidden sm:table-cell">{c.tier}</td>
                <td className="truncate whitespace-nowrap" title={qualityLabel(c.quality)}>
                  {(c.quality.value * 100).toFixed(0)} · <span className="text-text-muted">{qualityLabel(c.quality)}</span>
                </td>
                <td title={c.price_out_per_m == null && !c.is_free ? 'price not published' : undefined}>
                  {priceLabel(c.price_out_per_m, c.is_free)}
                </td>
                <td>
                  <span className="mr-2">{c.score.toFixed(2)}</span>
                  <span aria-hidden="true" className="hidden h-1.5 w-24 overflow-hidden rounded align-middle sm:inline-flex">
                    {SEGMENTS.map(s => (
                      <span key={s.key} style={{ width: `${Math.max(0, c.parts[s.key]) * 100}%`, background: s.color }} />
                    ))}
                  </span>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      </div>
      <details data-testid="routing-excluded" className="text-xs">
        <summary className="cursor-pointer text-text-secondary">Not considered ({excludedTotal})</summary>
        <ul className="mt-1 flex flex-col gap-1">
          {explanation.excluded.map(g => (
            <li key={g.reason}>
              {exclusionLabel(g.reason)} ({g.count})
              <span className="ml-1 font-mono text-text-muted">{g.examples.join(', ')}</span>
            </li>
          ))}
        </ul>
      </details>
      <p data-testid="routing-scope-note" className="text-xs text-text-muted">
        Doesn’t reflect today’s exploration budget or provider usage limits. Telemetry as of the last scoreboard
        refresh.
      </p>
      {health && (
        <footer data-testid="routing-health" data-state={healthState} className="text-xs text-text-muted"
          style={healthState === 'warn' ? { color: 'var(--color-status-warn)' } : undefined}>
          {health.benchmarked} of {health.cloud_models} cloud models benchmarked, {health.inherited} inherited
          {fallback ? ' · using built-in fallback scales' : ' · scales from the live catalog'}
          {problems.map(v => <div key={v.invariant}>{v.detail}</div>)}
        </footer>
      )}
    </section>
  );
}
```

If `modeLabel` returns something other than the English mode name for `efficiency` (e.g. a code name), STOP and
report: the label owner decides, this component does not.

- [ ] **Step 4: Run to verify pass**

Run: `cd /Users/brbrainerd/dev/vox && timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/routingLabels.test.ts src/components/surfaces/Models/RoutingExplainer.test.tsx src/hooks/useRoutingExplanation.test.ts src/lib/turnEvents.test.ts 2>&1 | tail -10 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -3`
Expected: all pass. (If `src/lib/turnEvents.test.ts` does not exist, drop it from the command.)
- [ ] **Step 5 (Claude): mutation proofs** — (a) `priceLabel`: `if (perMillion == null) return '—'` removed → the price
  test; (b) `details` gets `open` → the disclosure test; (c) `healthState` always `'ok'` → the health test;
  (d) `scope="col"` removed from one header → the table test; (e) delete the `routing-scope-note` paragraph → the
  scope test. Before `git add`, run `git diff -- crates/vox-gui/ui/src/lib/turnEvents.ts` and confirm the only hunk
  is the `MODE_OBJECTIVES` / `modeObjective` append. <!-- AMENDED: G12 -->
- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-gui/ui/src/lib/routingLabels.ts crates/vox-gui/ui/src/lib/routingLabels.test.ts crates/vox-gui/ui/src/lib/turnEvents.ts crates/vox-gui/ui/src/hooks/useRoutingExplanation.ts crates/vox-gui/ui/src/hooks/useRoutingExplanation.test.ts crates/vox-gui/ui/src/components/surfaces/Models/RoutingExplainer.tsx crates/vox-gui/ui/src/components/surfaces/Models/RoutingExplainer.test.tsx
git commit -m "feat(gui): a routing explainer that shows why a model was chosen and why others were not"
```

---

### Task 9: The Models surface explains routing instead of printing jargon

**Files:** Modify `crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx` (+ `.test.tsx`),
`crates/vox-gui/ui/e2e/lib/tauriMock.ts` (default responses for `explain_routing` / `get_routing_health`).

- [ ] **Step 1: Write the failing tests** <!-- AMENDED: G11 — exact code; the existing fixture has
  `decision_preview: null`, so a Decision Preview mutant can only fail against a local non-null preview; the $0.02
  card lives only in this block so the existing listitem-count test is untouched. --> In `ModelsView.test.tsx`
  make exactly two sanctioned edits to existing lines: (1) the import lines become
  `import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';` and
  `import { render, screen, cleanup, waitFor, fireEvent } from '@testing-library/react';`; (2) the mock function
  is hoisted unchanged into a named constant — replace `const invokeMock = vi.fn((cmd: string) => {` … `});` with:

```tsx
const baseImpl = (cmd: string) => {
  if (cmd === 'list_model_cards') return Promise.resolve(CARDS);
  if (cmd === 'get_routing_summary_live') return Promise.resolve(SUMMARY);
  if (cmd === 'get_active_model') return Promise.resolve('openai/gpt-x');
  if (cmd === 'set_active_model') return Promise.resolve(null);
  return Promise.resolve(null);
};
const invokeMock = vi.fn(baseImpl);
```

  Then append at the end of the file:

```tsx
const EXPLANATION = {
  mode: 'efficiency', task: 'codegen', complexity: 7, chosen: 'acme/widget-5.5', only_candidate: false,
  total_models: 3,
  candidates: [{
    id: 'acme/widget-5.5', provider: 'acme', tier: 'Fast', is_free: false, price_out_per_m: 0.9, score: 0.65,
    quality: { value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' },
    parts: { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 },
  }],
  excluded: [],
};
const HEALTHY = {
  schema_version: 1, checked_at_unix: 0, models: 3, cloud_models: 2, benchmarked: 1, inherited: 1,
  unknown_tier_cloud: 0, quality_scale: 'derived', price_bands: 'derived', efficient_pick: 'acme/widget-5.5',
  violations: [],
};
const PREVIEW = {
  selected_model: 'acme/widget-5.5', discovery_state: 'confirmed', intelligence_score: 0.8,
  efficiency_score: 0.5, latency_score: 0.4, alternatives: ['acme/widget-4.8'], rejection_reasons: [],
};
const ROUTING_CARDS = [...CARDS, {
  id: 'acme/widget-9', provider: 'acme', tier: 'Pro', cost_per_1k: 0.02, max_tokens: 64000, is_free: false,
  latency_p50_ms: 500,
}];
const routingAware = (cmd: string) => {
  if (cmd === 'list_model_cards') return Promise.resolve(ROUTING_CARDS);
  if (cmd === 'get_routing_summary_live') return Promise.resolve({ ...SUMMARY, decision_preview: PREVIEW });
  if (cmd === 'explain_routing') return Promise.resolve(EXPLANATION);
  if (cmd === 'get_routing_health') return Promise.resolve(HEALTHY);
  return baseImpl(cmd);
};

describe('ModelsView routing panel', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockClear();
    invokeMock.mockImplementation(routingAware);
  });
  afterEach(() => {
    invokeMock.mockImplementation(baseImpl);
  });

  it('renders the routing explainer instead of the decision preview', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await screen.findByTestId('routing-explainer');
    expect(screen.queryByText(/Decision Preview/i)).toBeNull();
    expect(document.body.textContent).not.toMatch(/intel=|eff=|lat=/);
  });

  it('changing the mode asks routing again for that mode', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await screen.findByTestId('routing-explainer');
    fireEvent.change(screen.getByRole('combobox', { name: 'Routing mode' }), { target: { value: 'genius' } });
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('explain_routing', { mode: 'genius', task: 'codegen', complexity: 7 }),
    );
  });

  it('prices are per million output tokens', async () => {
    render(<ModelsView pushToast={vi.fn()} />);
    await screen.findByText('$20.00/M');
    expect(document.body.textContent).not.toContain('$/1k');
  });
});
```

- [ ] **Step 2: Run to verify failure; save the output** —
  `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Models/ModelsView.test.tsx > target/routing-sm-t9-red.txt 2>&1; tail -20 target/routing-sm-t9-red.txt`. Expected: the three new tests fail.
- [ ] **Step 3: Implement.** In `ModelsView.tsx`: delete the `{summary?.decision_preview && ( … )}` block and the
  `decision_preview` field of the local `RoutingSummary` interface; in its place render

```tsx
      <RoutingPanel />
```

  and add, below `ModelsView`:

```tsx
const MODES = ['efficiency', 'balanced', 'genius', 'free'] as const;
const TASKS = ['codegen', 'research', 'review', 'general'] as const;

function RoutingPanel() {
  const [mode, setMode] = useState<string>('efficiency');
  const [task, setTask] = useState<string>('codegen');
  const { explanation, health, loading } = useRoutingExplanation(mode, task, 7);
  return (
    <Glass className="p-4 flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3 text-xs">
        <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">Routing</div>
        <label className="flex items-center gap-1">
          Routing mode
          <select aria-label="Routing mode" value={mode} onChange={e => setMode(e.target.value)}>
            {MODES.map(m => <option key={m} value={m}>{modeLabel(m)}</option>)}
          </select>
        </label>
        <label className="flex items-center gap-1">
          Task
          <select aria-label="Task" value={task} onChange={e => setTask(e.target.value)}>
            {TASKS.map(t => <option key={t} value={t}>{t}</option>)}
          </select>
        </label>
      </div>
      <RoutingExplainer explanation={explanation} health={health} loading={loading} />
    </Glass>
  );
}
```

  with imports `import { RoutingExplainer } from './RoutingExplainer';`,
  `import { useRoutingExplanation } from '../../../hooks/useRoutingExplanation';`,
  `import { modeLabel } from '../../../lib/turnEvents';`. In `ModelGrid`, replace
  `<div><span className="text-text-muted">$/1k</span> {m.cost_per_1k.toFixed(4)}</div>` with
  `<div>{priceLabel(m.is_free ? 0 : m.cost_per_1k > 0 ? m.cost_per_1k * 1000 : null, m.is_free)}</div>` (import
  `priceLabel` from `../../../lib/routingLabels`; an unknown paid price shows "—", never `$0.00/M`), and replace the
  `free` badge's `text-emerald-400` with `style={{ color: 'var(--color-status-pass)' }}`. <!-- AMENDED: R8/C-UI --> In `e2e/lib/tauriMock.ts`, next to `get_routing_summary_live`, add default
  responses for `explain_routing` (a two-candidate explanation using `acme/widget-*` ids) and `get_routing_health`
  (no violations). Change nothing else.
- [ ] **Step 4: Run to verify pass** —
  `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Models 2>&1 | tail -8 && timeout 300s pnpm --dir crates/vox-gui/ui typecheck 2>&1 | tail -3 && timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/models.spec.ts --project=chromium --reporter=line 2>&1 | tail -5`.
  Expected: all pass (the existing `models.spec.ts` must stay green).
- [ ] **Step 5 (Claude): mutation proof** — keep the old Decision Preview block alongside the panel →
  `renders the routing explainer instead of the decision preview` fails (the local `PREVIEW` is non-null, so the old
  block renders "Decision Preview" and `intel=`). <!-- AMENDED: G11 -->
- [ ] **Step 6: Commit (Claude Code)**

```bash
git add crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.test.tsx crates/vox-gui/ui/e2e/lib/tauriMock.ts
git commit -m "feat(gui): the Models surface explains routing instead of printing internal scores"
```

---

### Task 10: The Routing status card shows routing health (re-anchor before driving)

<!-- AMENDED: G8 — the chat "Why this model?" link is Deferred: the explainer explains task dispatch, and chat
resolves through `decide()` (premium aliases, confidence gating, per-request axes). -->

**Owner of the re-anchor: Claude.** The Routing status card is created by
`2026-09-28-chat-surfaces-consolidation.md` Task 2. Before driving, Claude reads it as committed and rewrites this
task with exact tests and insertion points. The decided behaviour:

- The card calls `get_routing_health` with the same refresh cadence as its other data, and shows a single status dot
  (`--color-status-warn`) with `aria-label="Routing health: N problems"` only when `violations` is non-empty or a
  scale is `fallback`. Healthy shows nothing extra (no dot, no text).
- Activating the card (click or Enter) navigates to the Models surface and scrolls the Routing panel into view; its
  accessible name says so ("Open routing details").
- The chat transcript gets nothing from this plan. Routing facts live on the status card, the rail (surfaces plan)
  and the Models surface.
- Playwright coverage: the re-anchored task adds a sixth test to `e2e/routing-explainer.spec.ts` (a health
  violation shows the dot; a healthy response shows none) and changes Task 11's expected count to 6. <!-- AMENDED: A12 -->

---

### Task 11: Playwright visual verification of the explainer

**Files:** Create `crates/vox-gui/ui/e2e/routing-explainer.spec.ts`.

- [ ] **Step 1: Write the spec** <!-- AMENDED: R14 — exact code; a self-contained mock in the `models.spec.ts` style
  (`get_initial_view` → `'models'`), not a helper imported from another spec; the narrow check measures the panel. -->

```ts
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');
const PARTS = { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 };
const EXPLANATION = {
  mode: 'efficiency', task: 'codegen', complexity: 7, chosen: 'acme/widget-5.5', only_candidate: false, total_models: 120,
  candidates: [
    { id: 'acme/widget-5.5', provider: 'acme', tier: 'Fast', is_free: false, price_out_per_m: 0.9, score: 0.65,
      quality: { value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' }, parts: PARTS },
    { id: 'acme/gadget-2', provider: 'acme', tier: 'Pro', is_free: false, price_out_per_m: null, score: 0.61,
      quality: { value: 0.8, source: 'benchmark', index: 46.3, inherited_from: null }, parts: PARTS },
  ],
  excluded: [
    { reason: 'flagship_excluded_by_mode', count: 4, examples: ['acme/flagship-9', 'acme/flagship-8', 'acme/flagship-7'] },
    { reason: 'superseded', count: 1, examples: ['acme/widget-4.8'] },
  ],
};
const HEALTHY = {
  schema_version: 1, checked_at_unix: 0, models: 120, cloud_models: 100, benchmarked: 40, inherited: 7,
  unknown_tier_cloud: 0, quality_scale: 'derived', price_bands: 'derived', efficient_pick: 'acme/widget-5.5',
  violations: [] as { invariant: string; detail: string }[],
};

async function openModels(page: Page, health = HEALTHY) {
  await page.addInitScript(({ explanation, health }) => {
    localStorage.setItem('vox_sidebar_mode', 'default');
    localStorage.setItem('vox_onboarding_dismissed', 'true');
    (window as any).__explainCalls = [];
    (window as any).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args?: any) => {
        if (cmd === 'get_initial_view') return 'models';
        if (cmd === 'list_model_cards') return [];
        if (cmd === 'get_routing_summary_live') return { decision_preview: null };
        if (cmd === 'get_active_model') return null;
        if (cmd === 'inference_provider_status') return [];
        if (cmd === 'explain_routing') { (window as any).__explainCalls.push(args); return explanation; }
        if (cmd === 'get_routing_health') return health;
        return null;
      },
    };
  }, { explanation: EXPLANATION, health });
  await page.goto('/');
  await expect(page.getByTestId('routing-explainer')).toBeVisible();
}

async function shot(page: Page, name: string) {
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, name) });
}

test.describe('Routing explainer', () => {
  test('explainer shows the chosen model, ranked candidates and provenance', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page);
    const panel = page.getByTestId('routing-explainer');
    await expect(panel.getByRole('heading', { name: 'Why acme/widget-5.5' })).toBeVisible();
    await expect(panel.getByText('How task dispatch would choose now')).toBeVisible();
    const rows = panel.getByRole('table', { name: 'Candidates ranked by routing score' }).getByRole('row');
    await expect(rows.nth(1)).toContainText('Chosen');
    await expect(rows.nth(1)).toContainText('from acme/widget-5.0 (38.2)');
    await expect(rows.nth(2)).toContainText('—');
    await shot(page, 'routing-explainer.png');
  });

  test('excluded models open from the disclosure', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page);
    await page.getByText('Not considered (5)').click();
    await expect(page.getByText('Flagship, kept out by this mode (4)')).toBeVisible();
    await shot(page, 'routing-explainer-excluded.png');
  });

  test('health problems are visible, and a healthy panel is quiet', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page, { ...HEALTHY, price_bands: 'fallback',
      violations: [{ invariant: 'tiers_known', detail: '30 of 100 cloud models have no tier' }] });
    const footer = page.getByTestId('routing-health');
    await expect(footer).toHaveAttribute('data-state', 'warn');
    await expect(footer).toContainText('30 of 100 cloud models have no tier');
    await shot(page, 'routing-explainer-health-warn.png');
  });

  test('the panel fits a narrow window', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openModels(page);
    const box = await page.getByTestId('routing-explainer').boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x + box!.width).toBeLessThanOrEqual(390);
    await shot(page, 'routing-explainer-narrow.png');
  });

  test('switching mode asks routing again', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page);
    await page.getByRole('combobox', { name: 'Routing mode' }).selectOption('genius');
    await expect
      .poll(() => page.evaluate(() => (window as any).__explainCalls.some((a: any) => a?.mode === 'genius')))
      .toBe(true);
  });
});
```

- [ ] **Step 2: Run** — `timeout 300s pnpm --dir crates/vox-gui/ui exec playwright test e2e/routing-explainer.spec.ts --project=chromium --reporter=line 2>&1 | tail -10`. Expected: 5 passed; four screenshots written.
- [ ] **Step 3 (Claude):** open each screenshot and check it against the GUI design rules; a failed rule is a fix,
  not a pass.
- [ ] **Step 4: Commit (Claude Code)** — `git add crates/vox-gui/ui/e2e/routing-explainer.spec.ts` and commit
  "test(gui): visual verification of the routing explainer".

---

### Task 12: Verification sweep — Owner: Claude

- [ ] `cargo test -p vox-orchestrator --lib`, `--tests`; `cargo test -p vox-cli --lib model_catalog`;
  `cargo test -p vox-gui --bin vox-gui routing_explain`; `cargo clippy -p vox-orchestrator -p vox-research-shim -p vox-cli -p vox-gui --all-targets -- -D warnings`.
- [ ] UI: `vitest run`, `typecheck`, the explainer and models Playwright specs.
- [ ] Live check (no key, no fee): build a registry from the public OpenRouter catalog snapshot, call
  `apply_routing_reference`, and record: the derived reference (expect `quality_reference` = top index,
  `elite_min_out` ≈ 0.020, `pro_min_out` ≈ 0.004, `unbenchmarked_scale` ≈ 0.5), how many specs inherited an index
  (expect ≥ 7), Efficient/Balanced/Genius picks at complexity 2/5/10 (no Elite in Efficient/Balanced), and
  `check_routing_health` (no violations). Compare the picks with the 2026-09-29 record in this plan; any change is
  explained or fixed.
- [ ] `vox doctor` on this machine shows "Model routing" and a Model Catalog freshness that can pass.
- [ ] `where-things-live.md`: rows for "Routing reference / quality priors" (`models/reference.rs`), "Selection
  ranking and exclusion reasons" (`models/ranking.rs`), "Routing health" (`models/health.rs`), "Routing explainer
  (GUI)" (`commands/routing_explain.rs`, `Models/RoutingExplainer.tsx`). Doc lint.
- [ ] `vox ci pre-push --complete` (or the documented standalone fallback when the 25-minute budget is exceeded);
  regenerate and commit any inventory it names.

---

## Decisions (resolved 2026-09-29; the user delegated open decisions to Claude)

1. **No globals for the reference.** The registry holds it and stamps a `QualityPrior` onto each spec. A process
   global would leak between tests sharing a worker thread and between registries in one process (the GUI builds
   several).
2. **Quantiles calibrated to today.** 0.93 / 0.70 reproduce $20/M / $4/M on the 2026-09-28 catalog, so the change
   is behaviour-neutral on day one and only moves when the market does.
3. **Quality reference = top benchmark** (not a floor of 60): the top model scores 1.0 and ordering is preserved
   under any rescale.
4. **Aliases protect, never override.** They are finer than the heuristic; overriding `family_key` would split
   families and stop older members being superseded.
5. **The tier follows the best-known price** at each merge and after observed prices are injected (OpenRouter,
   LiteLLM, AnthropicDirect and Telemetry sources); an entry no catalog prices, and an unknown price, keep their tier.
   No production path marks a `models.toml` entry as user-set (serde defaults it to `Bootstrap`), so "the user's tier
   is kept" is not claimed. This retires `AnthropicDirectCatalog`'s `id.contains("opus")` rule once the model is
   priced. <!-- AMENDED: G3 -->
6. **Health is checked at refresh, not in CI.** Every user's catalog refresh is the canary; a network-dependent CI
   job is deferred.
7. **The explainer explains task dispatch, not chat** (G8), through the same `best_for_task_under_gate` call
   dispatch makes (G9); the panel names the live gates it cannot see and its telemetry age (G13).
10. **Routing health judges the catalog**, not the refreshing process's keys (G6), so two refreshers write the same
    verdict; per-user key effects are shown by the explainer (`NoProviderKey`).
11. **A regression** is a Spec Feature 2 failure or a health-invariant failure; recorded picks may change with a
    written cause, checked at Tasks 1b and 2 (G4).
8. **`models/pareto.rs` stays reporting-only** (ADR-046).
9. **Exact ties resolve by id** (documented behaviour change in Task 4).

## Deferred

- **Explain chat routing** (was Task 10's chat link, G8): a `decide()`-based command; a premium-alias pick renders as
  the chosen line with reason "premium alias for {task}" and the ranking labelled "scorer's order (not used for this
  pick)"; the chat trace's routing step then links to it. Pairs with "Per-turn ranking in `routing_decision`" below.
- **A Genius quality invariant** (G7): add only with a measured threshold, alongside "Modes as objectives".
- **`decide_populates_non_placeholder_fields`** (`select.rs` tests) has the same always-skipped `if let Some` body as
  G10 found; fix it with the Task 5 fixture in its own change.
- **Hot-path cost of `rank_pass`**: it clones each candidate and sorts instead of one `max_by`; measure dispatch
  latency before optimising (keep the ranking lazy if it shows).

- **`vox model explain`** still uses `ModelRegistry::explain_selection`, a separate filter whose Task 2.1b
  semantics (free models shown under Performance at low complexity) are pinned by
  `explain_selection_complexity_tests`. Route it through `rank_with_filter` once those semantics are re-decided.
- **Thompson fallback, `best_free_for*`, `cheapest*`** (`registry_model_resolve.rs`) still skip recency, the key
  gate and the mode exclusion.
- **The DeepSeek off-peak bonus** keys on `"deepseek"` / `"r1"` in the id (`scoring.rs`); derive it from provider
  pricing windows or drop it.
- **Outcome feedback by family** (a bounded update of the quality prior from the scoreboard, keyed by `join_key` so
  history survives a version bump) — after the family-keyed scoreboard plan.
- **Modes as objectives** (quality-per-dollar above a complexity floor) instead of the Elite exclusion rule; the
  invariants in `health.rs` are the acceptance test for that change.
- **Network CI canary** running `check_routing_health` on the public catalog nightly.
- **Offline bootstrap refresh** (`vox model discover --write-bootstrap`); the runtime cache already refreshes.
- **Per-turn ranking in `routing_decision`** once the chat resolver returns `decide()`'s ranking.
- **Latency from the catalog**: OpenRouter's `/models` has none; responsiveness comes from measured latency only.

## Deferred Minor Issues

<!-- From the 3-track review; none blocks driving. -->
- A9: every unbenchmarked member inherits, not only the newest (older undated members could receive a newer member's
  index); restrict to members newer than the source if it shows in the live check.
- A10: `family_benchmarks` picks the first of exact ties (same date and version, e.g. `x` and `x:free`) in HashMap
  order; add an id tie-break, and apply the `derive` filters (index > 0, not local).
- A16: `vox doctor` does not flag an old routing-health record; the catalog freshness check covers staleness today.
- A14: the `latency_score` id-substring fallback (`llama-3`, `groq`, `cerebras`) is hand-kept; replace with measured
  latency when the scoreboard carries it per family.
- C5/C6/C7: `RoutingReference.benchmarked/priced`, `RoutingHealth.schema_version/models` and `ScoreParts::weighted()`
  are read only by tests or the doctor summary; keep while the health record is new, cut if unused after a release.
- C9: `task_category_from_label` duplicates the generated `TaskCategory: FromStr`, which maps unknown labels to
  `General` instead of rejecting them; kept strict on purpose.
- C-b: `decide()` now ranks the confirmed candidates once more per chat turn (to order `alternatives`); measure before
  optimising.
- A13 (second half): tiers are shown as their Rust names (`Elite`, `Pro`); give them plain labels with the visual plan's
  vocabulary.

## Execution Order

- **Sequential (shared files):** 1a → 1b → 2 (`spec.rs`, `registry.rs`, `virtual_models.rs`); 3 → 4 → 5
  (`scoring.rs`, `registry.rs`, `mode_select.rs`); 5 → 6 (`health.rs` uses `best_for_task_in_mode_keyed`); 5 → 7
  (`best_for_task_under_gate`, `DispatchGate`); trace T3 → 7 → 8 → 9 → 11; the whole trace plan → 8 (G12);
  surfaces T8 → 9; surfaces T2 and visual-language T1, T4–T6 → 10 → 11. <!-- AMENDED: R15 --> Rust tasks never run concurrently with other Rust plans (shared working tree and build).
- **Batch candidates:** none; each Rust task ends with a full `--lib` run.
- **Build loop (added 2026-09-29):** the agent runs only the filtered tests; Claude runs the crate suite once and clippy once per batch, and
  applies mutants that fail different named tests in one build. A producer task and the task that wires it (trace 2a and 2b) are the one
  case where clippy `dead_code` is expected between them. After Task 6, the [build-loop plan](2026-09-29-rust-build-loop-and-models-crate.md)
  measures whether extracting `models/` into its own crate is worth doing (Tasks A1–A4); its Part B moves `models/` and must not start while
  any task here is uncommitted. **Measured 2026-10-03: NO-GO** (an edit costs 37 CPU-s, 150 needed; see `docs/src/architecture/rust-build-loop-ssot-2026.md`), so Part B is not started and nothing here waits on it.
- **Pre-flight per task:** working tree clean for the task's files; HEAD on `main`; chat-lane Task 1 committed
  before Task 5; trace plan Task 4 committed before Task 8.
- **SDD ledger:**
  - D1 registry-held reference, stamped priors, no globals — ruling: settled
  - D2 quantiles 0.93 / 0.70 calibrated to the 2026-09-28 catalog — ruling: settled
  - D3 quality reference = top benchmark — ruling: settled
  - D4 aliases protect their target, never override `family_key` — ruling: settled
  - D5 catalog price decides tier at merge; user tier and unknown price kept — ruling: settled
  - D6 health at refresh + doctor; CI canary deferred — ruling: settled
  - D7 chat link explains "now", not the historical turn — ruling: settled
  - D8 pareto.rs not used for selection (ADR-046) — ruling: settled
  - D9 exact ties by id — ruling: settled
  - G1 tie-break proven by a direct comparator test — ruling: settled
  - G2 gate-equivalence test before the selector switch — ruling: settled
  - G3 tier follows the best-known price incl. Telemetry — ruling: settled
  - G4 regression = SF2 or invariant; live picks checked at 1b and 2 — ruling: settled
  - G5 same line, same prior regardless of provider key — ruling: settled
  - G6 health judges the catalog (key-agnostic probe, `efficient_pick`) — ruling: settled
  - G7 no Genius quality invariant — ruling: settled
  - G8 explainer explains task dispatch; chat link deferred — ruling: settled
  - G9 `DispatchGate` + `best_for_task_under_gate` shared by runtime and explainer — ruling: settled
  - G10 decide() test on a confirmed UserConfig fixture with `.expect` — ruling: settled
  - G11 Task 9 tests as exact code with local fixtures — ruling: settled
  - G12 Task 8 waits for the whole trace plan; git log/status pre-flight — ruling: settled
  - G13 GUI registry mirrors dispatch's telemetry inputs — ruling: settled (amended by R4: the re-derive lives in `inject_pricing_catalog`)
  - R1 f32/f64 in the inherited-prior assertion — ruling: settled
  - R2 fallback equivalence proven against the old formula before `quality_score` delegates — ruling: settled
  - R3 vox-gui tests run with `--bin vox-gui` (no library target) — ruling: settled
  - R4 re-derive inside `inject_pricing_catalog`; key-pin test in catalog_refresh.rs for tdd-guard — ruling: settled
  - R5 flagship health judged by price, not tier label — ruling: settled
  - R6 when dispatch chooses nothing, the explainer names no model (test-support dev feature) — ruling: settled
  - R7 decide() alternatives follow chat's flagship rule; no `ranking` field — ruling: settled
  - R8 explainer fits 390 px, no layout shift, honest `aria-busy`; price "—" not $0.00 — ruling: settled
  - R9 re-derive after promotion re-registers — ruling: settled
  - R10–R12 serial/file_serial on tests sharing global weights or the privacy override — ruling: settled
  - R13 grouping fixture ordered so the sort mutant dies — ruling: settled
  - R14 Task 11 Playwright spec as exact code — ruling: settled
  - R15 extra prerequisites: trace T3 → 7, surfaces T8 → 9, visual-language T1/T4–T6 → 10 — sequential — settled
  - R16 routing health computed under a read lock — ruling: settled
  - R18 the background refresh writes the catalog timestamp — ruling: settled
  - R19 one two-pass selector (`best_for_internal` removed) — ruling: settled
  - `spec.rs` / `virtual_models.rs` 1a → 2; `registry.rs` 1b → 2 → 4 → 6; `scoring.rs` 1a → 3; `mode_select.rs` 5 → 6; `select.rs` chat-lane T1 → 5; `runtime.rs` 5 only; `lib/turnEvents.ts` whole trace plan → 8 — sequential — settled
