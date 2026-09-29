# Model Routing That Keeps Itself Current — Design and Task Outline

> **Status: design outline, not yet drive-ready.** Each task below must be expanded to the
> `writing-plans` level (exact tests, exact insertion points, mutation proof) and put through the
> 3-track review before it goes to `agy`. Order and interfaces are decided; test bodies are not.
> Follows [`2026-09-28-model-routing-latest-and-honest.md`](2026-09-28-model-routing-latest-and-honest.md)
> (Tasks 1–6 committed) and [`2026-09-28-model-routing-chat-lane.md`](2026-09-28-model-routing-chat-lane.md).

**Goal:** A new model release, a price cut, or a benchmark rescale must change routing behaviour with
**no code edit and no hand-kept list**, and drift that the data cannot absorb must be *detected*,
not discovered by a user picking an old or absurdly expensive model.

## What is hand-kept today, and how it goes stale

Measured on the 2026-09-28 catalog (369 models) after Tasks 1–6: Efficient at complexity 10 picks
`xiaomi/mimo-v2.6-pro` (index 46.3, $0.9/M output); Genius picks `anthropic/claude-opus-5.5`
(index 57.6, $20/M); no Opus-class model appears in any Efficient/Balanced pick. The behaviour is right; these are
the constants and heuristics it rests on:

| Hand-kept piece | Where | How it rots |
|---|---|---|
| `QUALITY_INDEX_REFERENCE = 60.0` | `models/scoring.rs` | Artificial Analysis rescales or a model exceeds 60: every score saturates at 1.0 and the ordering flattens. |
| Tier bands `$20/M` Elite, `$4/M` Pro | `models/tiering.rs` | Prices fall (today's flagship band is next year's mid band) or a new price class appears; "Elite" stops meaning flagship. |
| `UNBENCHMARKED_QUALITY_SCALE = 0.6` | `models/scoring.rs` | An arbitrary factor; right for one catalog snapshot only. |
| `family_key` slug parsing | `models/family.rs` | A vendor changes its naming scheme; two lines merge or one line splits, and supersession silently misfires. |
| Index only from OpenRouter | `catalog.rs` | A direct-provider model (Anthropic key, no OpenRouter) is permanently "unbenchmarked". |
| `excludes_elite(clutch)` as a special case | `models/mode_select.rs` | Each new mode needs another hand-written rule. |
| No outcome feedback in the prior | `scoreboard`, Thompson arms | A brand-new model has no history, so it is scored only by a benchmark that may not exist yet. |
| Tests name models | various | Fixtures rot; nothing detects that live routing has drifted. |

## Design principles

1. **Derive from the live catalog; keep constants only as floors/fallbacks.** Every derived value is recomputed on catalog refresh and carries its provenance (derived / fallback).
2. **A new model inherits its family's prior** until it has its own benchmark and then its own outcomes.
3. **Modes are objectives, not special cases.** Efficient = maximize quality per dollar above a complexity-scaled quality floor; Genius = maximize quality; Responsive = minimize measured latency above a floor.
4. **Invariants, not names, are tested.** A canary asserts properties of the live ranking; it does not name a model.
5. **Drift is loud.** When a derived value falls back, or the catalog is old, `vox doctor` and the GUI routing surface say so.

## Tasks (each becomes its own drive-ready plan section)

### Task A — Live-derived scale and tier bands
- `RoutingReference { quality_reference: f64, elite_min_out: f64, pro_min_out: f64, source: Derived|Fallback }` computed from the registry at refresh: `quality_reference` = a high percentile (p99) of `intelligence_index` among benchmarked models, floored at the current 60.0; Elite band = output price at or above a multiple of the median price of the top-decile-by-index models, floored at today's `$20/M` only if fewer than N benchmarked models exist.
- `derive_tier` and `quality_score` read the registry's current `RoutingReference`, not constants. The constants remain as the `Fallback`.
- Tests: rescale the fixture index ×1.5 and prices ×0.5 → tiers and ordering unchanged in shape; empty/tiny catalog → `Fallback` and the current constants; determinism.
- Files: `models/tiering.rs`, `models/scoring.rs`, `models/registry.rs`, new `models/reference.rs`.

### Task B — Family inheritance and the direct↔OpenRouter benchmark join
- `family_key` gains an authoritative source: OpenRouter `~vendor/…-latest` alias entries (`alias_target.slug`) map a family to its current model; where one exists it overrides the heuristic. Alias entries are parsed into a `FamilyAliases` map instead of being skipped outright (Task 1 of the first plan skips them as models; they remain skipped as models).
- A model with no `intelligence_index` inherits the **latest benchmarked member of its family** (discounted by a small recency-gap factor), so a fresh release is not scored as an unknown long-tail model.
- A direct-provider model (Anthropic/OpenAI/… key) takes the index of the same family's OpenRouter entry.
- Tests: alias overrides a heuristic that would split the family; unbenchmarked new member inherits; direct model joins; a family with no benchmark stays "unbenchmarked" and keeps the OpenRouter-only scale.

### Task C — Calibrated proxy for genuinely unbenchmarked models
- Replace the fixed `0.6` with a fit of index against the proxy (context length, paid/free) over benchmarked models, recomputed at refresh, with shrinkage toward the mean when few points exist. Falls back to `0.6`.
- Test: synthetic catalog where the proxy is a known linear function of the index → recovered within tolerance.

### Task D — Modes as objectives
- Replace `excludes_elite` with a per-clutch objective over the eligible set: Efficiency/Balanced maximize `quality / price` above a floor that rises with complexity; Genius maximizes quality; a future Responsive minimizes measured latency above a floor. The Elite exclusion becomes an emergent property (Elite fails the value objective) with the explicit rule kept as a tested invariant, not a code path.
- Reuse `models/pareto.rs`'s frontier (ADR-046) instead of adding a second frontier implementation.
- Tests: the invariants below on generated catalogs; Genius unchanged.

### Task E — Outcome feedback and measured latency
- Feed scoreboard successes/latency/cost into the quality prior keyed by **family** (so history carries across versions), as a bounded Bayesian update of the benchmark prior. OpenRouter's `/models` reports no latency, so responsiveness comes from measured latency only.
- Depends on the family-keyed scoreboard plan; do not start before it lands.

### Task F — Canary, freshness and visibility (do this first)
- `vox ci model-routing-canary`: fetch the public catalog (no key, no fee), build the registry, and assert **invariants**: (1) Efficiency/Balanced top-5 contain no Elite while ≥ 5 non-Elite candidates exist; (2) no keyless provider is ever picked; (3) no superseded member is picked; (4) Genius's pick has an index within ε of the maximum; (5) every family with ≥ 2 members has exactly one non-superseded newest; (6) the number of `Unknown`-tier cloud models is below a threshold; (7) the derived `RoutingReference` is `Derived`, not `Fallback`. Emits findings JSONL with `schema_version`.
- Non-blocking in PR CI, blocking nightly; a failure means "retune", never "edit a model name".
- `vox doctor` reports catalog age and `RoutingReference.source`; the GUI routing surface shows the same one-line provenance next to the chosen model (feeds the chat-trace plan).
- Companion: `scripts/refresh-model-catalog.vox` regenerates `model-catalog.bootstrap.v1.json` from the live catalog so offline runs are not stale.

## Invariants (the canary and the property tests share these)

1. Efficiency/Balanced never pick Elite while a non-Elite candidate fits.
2. No provider without a resolvable key is picked.
3. No superseded family member is picked among eligible candidates.
4. Genius's pick has the highest (or within ε of the highest) intelligence index among eligible models.
5. Scaling every price by a constant does not change the ordering shape of the tiers.
6. Adding a model that is dominated (lower index, higher price) never changes the pick.

## Order

F (canary first: it measures every later change) → A → B → C → D → E. Rust only; sequential, shared files
(`scoring.rs`, `tiering.rs`, `registry.rs`). Not concurrent with other Rust plans.

## Constraints

- No new crate, edge, or dependency. `contracts/orchestration/model-pins.v1.yaml` (council-ratified) is not edited; tuning that must be policy goes in a new, separate contract only if a value cannot be derived.
- The canary fetches only the public catalog endpoint; no credential is read or sent.
- Every derived value has a tested fallback; no task may make routing depend on the network being up.
