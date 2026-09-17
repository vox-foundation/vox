# Chat Composer & Model-Priority Implementation Plan (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the chat composer's model knobs honest, minimal, and actually load-bearing: one ranking control over efficiency/responsiveness/intelligence that genuinely changes which model runs, on every lane; delete the redundant and inert controls around it; make mesh work; and label the send button from the real keybinding.

**Architecture:** The rank **is** the clutch. `clutch` is already an `Option<String>` on the wire, already schema-validated, already carried by both the sync and background lanes — so extending its vocabulary from 4 presets to 4 presets + 6 rank permutations adds zero plumbing and reaches every lane for free. Before that means anything, the shared ranking function must stop discarding the axes (Phase 0), and the "intelligence" axis must stop being a proxy for context-window size (Phase 2).

**Tech Stack:** Rust (`vox-orchestrator`, `vox-orchestrator-mcp`, `vox-gui`), TypeScript/React + Tailwind + `@dnd-kit` (already a dependency), Vitest + Testing Library, `cargo nextest`.

**Spec:** [`docs/src/architecture/chat-ui-model-knobs-audit-2026-09-10.md`](../../src/architecture/chat-ui-model-knobs-audit-2026-09-10.md) — **read its CORRECTION blocks first**; the first pass of that audit had five wrong conclusions and this plan is built on the corrected version.

## Why v2 replaced v1

v1 proposed a new `PriorityAxis` type, a new `priority_rank` wire field, a new `priorityRank.ts` module, and hand-rolled HTML5 drag-and-drop. An eight-track verification pass killed all four:

- **Axes are discarded on the primary selection path.** `select_via_scorer` installs a thread-local axes override, then calls `best_for_with_filter` → `best_for_internal` (`registry.rs:813`), which never calls `auto_score_model`. v1 would have shipped a higher-resolution version of an existing no-op.
- **A new wire field was unnecessary** — and `clutch` already reaches the background lane that v1 admitted it could not cover.
- **v1's `[10,30,60]` weights don't separate the permutations.** Total weight is 125; a 10→60 swing moves 0.4 normalized. Efficiency swings ~0.24–0.35, intelligence ~0.14, responsiveness **~0.072 — less than two existing hardcoded bonuses**. Model choice actually flips on two *thresholds* (`intelligence >= 50` premium alias at `select.rs:699`; `to_cost_preference()` dropping free models at `registry.rs:831`), and only the top slot crosses either — so six orderings collapse to three outcomes.
- **HTML5 DnD cannot work in this app.** `crates/vox-gui/tauri.conf.json` has no `dragDropEnabled` key, so it defaults to `true`, meaning Tauri's native handler owns drags and DOM drag events never reach the page. The identical broken pattern already ships in `Settings/PriorityChainEditor.tsx:186-203`, whose drag handle is dead. `@dnd-kit/core` + `@dnd-kit/sortable` are already dependencies, pointer-based, and proven in `DashboardGrid.tsx`.

## Global Constraints

- **"Green" means `cd crates/vox-gui/ui && pnpm typecheck && pnpm test`, run manually.** No hook and no CI workflow runs the frontend vitest suite; pre-commit runs only `surfaceHonesty.guard.test.ts`, pre-push only `vox-drift-check`. A red frontend test is invisible to automation — every task must verify by hand.
- Frontend commands are `pnpm`-based from `crates/vox-gui/ui`, never `npx`: `pnpm vitest run <file>`, `pnpm test`, `pnpm typecheck`.
- Rust: plain `cargo` on PATH (build broker shim). Single test: `cargo nextest run -p <crate> -E 'test(<name>)'`. Format with `vox run scripts/fmt.vox`, never `cargo fmt --all`.
- A `honesty-guard` pre-commit hook fires on `crates/vox-gui/ui/src/components/surfaces/**/*.tsx` — i.e. on Loquela and DriveConsole. `docs/agents/gui-honesty-manifest.json` pins `DriveConsole.tsx:47` by file **and line**; regenerate with `crates/vox-gui/ui/scripts/inventory.mjs` when those files move.
- `tdd-guard` checks for a test in the same **file**; `mode.rs` already satisfies it trivially, so write real tests by judgment, not to satisfy the gate.
- Every new `pub fn` in `crates/*/src/**` needs a `#[test]` in the same file.
- Commit messages: imperative subject <72 chars, body explains why. Do not push or open a PR unless asked.

## Deferred, with reasons (not silently dropped)

- **External benchmark ingestion** (LMSYS/Arena/MMLU). Phase 2 wires the *existing* `vox model eval-corpus` pass@1 evidence instead. Revisit only if that corpus proves too thin.
- **Policy changes that alter what the machine dials or broadcasts**: defaulting `VOX_MESH_ENABLED=1` (writes a host/GPU fingerprint to disk), auto-dialing with no configured control URL, enabling `mesh-discovery-publish`. Phase 5 fixes only what is broken for an operator who has *already* configured mesh.
- **`throughput_score`'s 0.1 default** makes the Responsiveness axis nearly inert (swing ~0.072). Phase 1 ships the ranking anyway because efficiency and intelligence do separate; Task 1.4 records the limitation honestly rather than pretending otherwise.

---

## PHASE 0 — Make the axes real (gate: user go/no-go)

> **STOP.** Phase 0 changes the primary model-selection path for *every* caller in the product, not just chat. It is the difference between shipping a working knob and shipping theatre, but its blast radius is the whole router. Get explicit approval before executing, and run Task 0.1 first — if 0.1 passes before any change, this plan's premise is wrong and everything downstream should be re-examined.

### Task 0.1: The proving test — six orderings must pick different models

**Files:**
- Create: `crates/vox-orchestrator/tests/axis_ranking_separates_models.rs`

**Interfaces:**
- Consumes: `vox_orchestrator::models::{decide, ModelSelectionRequest, SelectionIntent, SelectionAxes, ModelSpec, CandidateScope}`.
- Produces: the acceptance gate for Phases 0–2. Nothing imports it.

This test is the honesty gate for the entire program. It must fail now and pass at the end of Phase 1.

- [ ] **Step 1: Write the failing test**

```rust
//! Acceptance gate: a user's axis ranking must actually change which model is
//! selected. If this passes with the scorer bypassed, the ranking UI is
//! theatre — see docs/superpowers/plans/2026-09-10-chat-composer-and-model-priority.md.
use std::collections::BTreeSet;

use vox_orchestrator::models::{
    CandidateScope, ModelSelectionRequest, SelectionAxes, SelectionIntent, decide,
};
use vox_orchestrator::types::TaskCategory;

/// Four models chosen so each axis has a distinct winner:
/// free-small (cheapest), cheap-paid, fast-paid (lowest latency), frontier
/// (largest context => highest heuristic quality).
fn fixture_registry() -> vox_orchestrator::models::ModelRegistry {
    // Build via the registry's public test constructor; see
    // crates/vox-orchestrator/src/models/registry.rs for `ModelRegistry::from_specs`
    // (if that constructor does not exist, add it in this task, with a test).
    todo_replace_with_registry_from_specs()
}

#[test]
fn six_axis_orderings_do_not_collapse_to_one_model() {
    let registry = fixture_registry();
    let orderings: [(&str, SelectionAxes); 6] = [
        ("e>r>i", SelectionAxes { cost: 70, responsiveness: 25, intelligence: 5 }),
        ("e>i>r", SelectionAxes { cost: 70, responsiveness: 5, intelligence: 25 }),
        ("r>e>i", SelectionAxes { cost: 25, responsiveness: 70, intelligence: 5 }),
        ("r>i>e", SelectionAxes { cost: 5, responsiveness: 70, intelligence: 25 }),
        ("i>e>r", SelectionAxes { cost: 25, responsiveness: 5, intelligence: 70 }),
        ("i>r>e", SelectionAxes { cost: 5, responsiveness: 25, intelligence: 70 }),
    ];

    let mut picked: Vec<(&str, String)> = Vec::new();
    for (label, axes) in orderings {
        let mut intent = SelectionIntent::for_task(TaskCategory::CodeGen);
        intent.axes = axes;
        let req = ModelSelectionRequest {
            intent,
            required_capabilities: vec![],
            candidate_scope: CandidateScope::AllProviders,
        };
        let decision = decide(&req, &registry).expect("a model is selected");
        picked.push((label, decision.outcome.model_spec.model_id.clone()));
    }

    let distinct: BTreeSet<&String> = picked.iter().map(|(_, id)| id).collect();
    assert!(
        distinct.len() >= 3,
        "axis ranking does not change model choice — the knob is theatre. Picks: {picked:?}"
    );

    // The three single-axis winners must be genuinely different models.
    let by = |l: &str| picked.iter().find(|(k, _)| *k == l).map(|(_, v)| v.clone()).unwrap();
    assert_ne!(by("e>r>i"), by("i>r>e"), "cost-first and intelligence-first must differ");
    assert_ne!(by("e>r>i"), by("r>i>e"), "cost-first and responsiveness-first must differ");
}
```

Replace `todo_replace_with_registry_from_specs()` with a real fixture as the first act of implementation: four `ModelSpec`s differing in `cost_per_1k` (0.0 / 0.0005 / 0.003 / 0.015), `capabilities.latency_p50_ms` (2000 / 1200 / 300 / 1800), `capabilities.max_context` (8_192 / 32_768 / 32_768 / 200_000) and `is_free` (true / false / false / false). Read `crates/vox-orchestrator/src/models/spec.rs:37-168` for the exact field set and construct them literally — do not add a builder.

- [ ] **Step 2: Run it — it must FAIL**

Run: `cargo nextest run -p vox-orchestrator -E 'test(six_axis_orderings)'`
Expected: FAIL. Per the audit, `decide` → `best_for_internal` ignores the axes, so all six orderings pick the same model. **If this passes before any other change, stop and report — the plan's premise is wrong.**

- [ ] **Step 3: Commit the failing test**

```bash
git add crates/vox-orchestrator/tests/axis_ranking_separates_models.rs
git commit -m "$(cat <<'EOF'
test(vox-orchestrator): add the axis-ranking acceptance gate (currently red)

Pins the claim that a user's axis ranking changes which model is picked.
It fails today because best_for_internal never consults the axis scorer —
that is the bug the next commits fix. Committed red on purpose so the fix
has a witness.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

### Task 0.2: Route the shared ranking through the axis-aware scorer

**Files:**
- Modify: `crates/vox-orchestrator/src/models/select.rs:801-820` (`select_via_scorer`)
- Modify: `crates/vox-orchestrator/src/models/registry.rs:1004-1025` (extract the ranking `explain_selection` already uses)

**Interfaces:**
- Consumes: `auto_score_model` (`scoring.rs:321`), the `AxesOverrideGuard` thread-local (`select.rs:812`, `scoring.rs:269-318`).
- Produces: `ModelRegistry::best_by_axis_score(&self, filter) -> Option<ModelSpec>` — the axis-aware ranking, used by both `explain_selection` and `select_via_scorer`.

`explain_selection` (`registry.rs:1007`) already ranks by `auto_score_model` — the axis-aware path exists and is exercised by `vox model explain`. This task extracts it and makes the production path use it, so `vox model explain` stops being a liar about what the router will do.

- [ ] **Step 1: Read both call sites before changing anything**

Read `registry.rs:813-941` (`best_for_internal`, the current cost-first ranking) and `registry.rs:1004-1025` (`explain_selection`'s axis ranking). Write down, in the commit message later, the one behavioral difference you are introducing: selection stops being "cheapest cost-per-success" and becomes "highest weighted axis score."

- [ ] **Step 2: Extract the axis ranking into a shared method**

In `registry.rs`, add next to `best_for_internal`:

```rust
    /// Axis-aware ranking: the highest `auto_score_model` candidate under the
    /// currently-installed `SelectionAxes` (see `scoring::AxesOverrideGuard`).
    /// This is the same ranking `explain_selection` reports, extracted so the
    /// production path and the explain path cannot diverge — before this,
    /// `vox model explain` described a ranking `best_for_internal` never used.
    pub fn best_by_axis_score(
        &self,
        filter: &dyn Fn(&ModelSpec) -> bool,
    ) -> Option<ModelSpec> {
        self.list_models()
            .into_iter()
            .filter(|m| filter(m))
            .map(|m| {
                let score = crate::models::scoring::auto_score_model(&m);
                (m, score)
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(m, _)| m)
    }
```

Then rewrite `explain_selection`'s ranking to call it, so there is exactly one implementation. (Read its current body first — it also builds a per-candidate breakdown for display; keep that, but source the winner from `best_by_axis_score`.)

- [ ] **Step 3: Point `select_via_scorer` at it**

In `select.rs:801-820`, replace the `registry.best_for_with_filter(...)` call with `registry.best_by_axis_score(...)`, keeping the existing filter closure and keeping `best_for_with_filter` as the fallback when `best_by_axis_score` returns `None`. Keep the `AxesOverrideGuard` — it now becomes load-bearing instead of inert.

- [ ] **Step 4: Run the gate and the existing suites**

Run: `cargo nextest run -p vox-orchestrator -E 'test(six_axis_orderings)'` → expected PASS (this is the moment the knob becomes real).
Run: `cargo nextest run -p vox-orchestrator` and `cargo nextest run -p vox-orchestrator-mcp` → expected PASS. **Any pre-existing test that now fails is a genuine behavior change; investigate and report each one rather than editing the assertion to match.** Selection changing from cost-first to axis-weighted will move some picks; that is the point, but each moved pick must be explainable.

- [ ] **Step 5: Format and commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-orchestrator/src/models/select.rs crates/vox-orchestrator/src/models/registry.rs
git commit -m "$(cat <<'EOF'
fix(vox-orchestrator): make selection honor the axes it already installs

select_via_scorer installed a SelectionAxes thread-local and then called
best_for_internal, which ranks purely by cost-per-success and never reads
it — so every axis knob in the product (clutch, Settings routing priority,
research quality targets) was inert except via two threshold side effects.
Both the sync and background lanes converge here, so one fix serves all
lanes. The axis ranking itself is not new: it is what `vox model explain`
has been reporting all along, now extracted so explain and production
cannot diverge.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

### Task 0.3: Install axes on the background lane

**Files:**
- Modify: `crates/vox-orchestrator/src/runtime.rs:683-750`

**Interfaces:**
- Consumes: `AgentTask.clutch_profile` / `AgentTask.risk_posture` (already parsed by `apply_hints`, `types/tasks.rs:971-980`), `mode::effective_axes`.
- Produces: background tasks reaching `best_by_axis_score` with the user's axes installed.

The background lane already *parses* clutch/risk into `AgentTask` and then throws the nuance away in `resolve_task_cost_policy`, which returns only `(CostPreference, force_free_pool, RiskPosture)`. Since Task 0.2 made the shared ranking axis-aware, the background lane only needs to install the guard.

- [ ] **Step 1: Write the failing test**

Add to `runtime.rs`'s test module (or create one if absent — check first):

```rust
    #[test]
    fn background_task_installs_axes_from_its_clutch() {
        // A background task carrying an intelligence-first clutch must reach
        // selection with intelligence-weighted axes, not just CostPreference.
        let mut task = crate::types::AgentTask::new(
            crate::types::TaskId(1),
            "refactor the parser",
            crate::types::TaskPriority::Normal,
            vec![],
        );
        task.clutch_profile = Some(crate::mode::ClutchProfile::Genius);
        task.risk_posture = Some(crate::mode::RiskPosture::Moderate);

        let axes = axes_for_task(&task);
        assert_eq!(axes, Some((15, 15, 70)));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo nextest run -p vox-orchestrator -E 'test(background_task_installs_axes)'`
Expected: FAIL — `axes_for_task` does not exist.

- [ ] **Step 3: Implement**

Add to `runtime.rs`:

```rust
/// The axes a background task's clutch/risk imply, or `None` when the task
/// carries neither (leaving the process-wide base priority in charge).
/// Mirrors the sync lane's `build_selection_request` so the two lanes cannot
/// drift — see crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs.
pub fn axes_for_task(task: &crate::types::AgentTask) -> Option<(u8, u8, u8)> {
    let clutch = task.clutch_profile?;
    let risk = task.risk_posture.unwrap_or(crate::mode::RiskPosture::Moderate);
    Some(crate::mode::effective_axes(clutch, risk))
}
```

Then in the dispatch path at `runtime.rs:713-750`, wrap the selection call in the axes guard when `axes_for_task` returns `Some` — read `select.rs:804-812` for the exact guard construction and mirror it. Leave `resolve_task_cost_policy` alone; it still supplies `force_free_pool` and the cost preference.

- [ ] **Step 4: Verify**

Run: `cargo nextest run -p vox-orchestrator` → PASS.

- [ ] **Step 5: Format and commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-orchestrator/src/runtime.rs
git commit -m "$(cat <<'EOF'
fix(vox-orchestrator): background tasks honor their clutch axes

apply_hints already parsed clutch/risk onto AgentTask, then runtime threw
the nuance away by collapsing to CostPreference. Now that the shared
ranking is axis-aware, the background lane installs the same axes the sync
lane does — so "Background task" and "Quick chat" route consistently.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## PHASE 1 — The rank IS the clutch

### Task 1.1: Extend `ClutchProfile` with six rank permutations

**Files:**
- Modify: `crates/vox-orchestrator/src/mode.rs`
- Modify: `contracts/gui/drive-console.v1.yaml`
- Modify: `crates/vox-orchestrator/src/control/mod.rs` (parity test data)
- Modify: `crates/vox-orchestrator-mcp/src/input_schemas.rs` (the `clutch` enum literal)

**Interfaces:**
- Produces: six new `ClutchProfile` variants with wire ids `"i>r>e"`, `"i>e>r"`, `"r>i>e"`, `"r>e>i"`, `"e>i>r"`, `"e>r>i"` (**most-important axis first**), each resolving to a `[5,25,70]` positional axes triple and to quality/cost-preference/budget-gate keyed off the top axis.
- Consumed by: Task 1.2 (risk floor), Task 3.x (GUI emits these ids).

The four legacy ids stay. `free` in particular is **not** a ranking — it is a filter (`force_free_pool`), a conflation this task documents and Task 3.2 splits in the UI.

- [ ] **Step 1: Write the failing tests**

Add to `mode.rs`:

```rust
#[cfg(test)]
mod rank_clutch_tests {
    use super::*;

    #[test]
    fn rank_ids_round_trip_through_from_label() {
        for id in ["i>r>e", "i>e>r", "r>i>e", "r>e>i", "e>i>r", "e>r>i"] {
            let parsed = ClutchProfile::from_label(id).unwrap_or_else(|| panic!("{id} parses"));
            assert_eq!(parsed.label(), id, "label() must round-trip {id}");
        }
    }

    #[test]
    fn rank_axes_use_the_positional_table_top_first() {
        // "i>r>e": intelligence 70, responsiveness 25, efficiency 5.
        // resolve().axes is (cost, responsiveness, intelligence).
        assert_eq!(ClutchProfile::from_label("i>r>e").unwrap().resolve().axes, (5, 25, 70));
        assert_eq!(ClutchProfile::from_label("e>r>i").unwrap().resolve().axes, (70, 25, 5));
        assert_eq!(ClutchProfile::from_label("r>e>i").unwrap().resolve().axes, (25, 70, 5));
    }

    #[test]
    fn top_axis_drives_quality_cost_and_budget() {
        let intel = ClutchProfile::from_label("i>r>e").unwrap().resolve();
        assert_eq!(intel.quality, QualityLevel::Premium);
        assert_eq!(intel.cost_preference, CostPreference::Performance);
        assert_eq!(intel.budget_gate, BudgetAggressiveness::Relaxed);

        let eff = ClutchProfile::from_label("e>r>i").unwrap().resolve();
        assert_eq!(eff.quality, QualityLevel::Flash);
        assert_eq!(eff.cost_preference, CostPreference::Economy);

        let resp = ClutchProfile::from_label("r>i>e").unwrap().resolve();
        assert_eq!(resp.quality, QualityLevel::Balanced);
    }

    #[test]
    fn no_rank_variant_forces_the_free_pool() {
        // `free` is a FILTER, not a ranking — no ordering may imply it.
        for id in ["i>r>e", "i>e>r", "r>i>e", "r>e>i", "e>i>r", "e>r>i"] {
            assert!(!ClutchProfile::from_label(id).unwrap().resolve().force_free_pool);
        }
        assert!(ClutchProfile::Free.resolve().force_free_pool);
    }

    #[test]
    fn legacy_labels_still_parse_unchanged() {
        assert_eq!(ClutchProfile::from_label("genius").unwrap().resolve().axes, (15, 15, 70));
        assert_eq!(ClutchProfile::from_label("efficiency").unwrap().resolve().axes, (70, 15, 15));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p vox-orchestrator -E 'test(rank_clutch_tests)'`
Expected: FAIL — the rank ids don't parse and `label()` doesn't exist.

- [ ] **Step 3: Implement in `mode.rs`**

Add the six variants to `ClutchProfile` (keeping the four existing ones), with explicit serde renames:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClutchProfile {
    Free,
    #[default]
    Efficiency,
    Balanced,
    Genius,
    // ── Rank permutations: the user's explicit axis ordering, MOST IMPORTANT
    // FIRST. The GUI stores its chips left-to-right least→most (rightmost
    // wins) and reverses once at the wire boundary — see
    // PriorityRankControl.tsx. Keep that inversion in exactly those two places.
    #[serde(rename = "i>r>e")] RankIRE,
    #[serde(rename = "i>e>r")] RankIER,
    #[serde(rename = "r>i>e")] RankRIE,
    #[serde(rename = "r>e>i")] RankREI,
    #[serde(rename = "e>i>r")] RankEIR,
    #[serde(rename = "e>r>i")] RankERI,
}

/// Weight for rank position, most-important first. Chosen so the top slot
/// clears BOTH thresholds that actually flip model choice — the
/// `intelligence >= 50` premium-alias branch (models/select.rs) and
/// `to_cost_preference()`'s Economy/Performance split — while the middle slot
/// still separates orderings that share a top axis. A flatter table
/// (e.g. 10/30/60) collapses the six orderings into three outcomes; see
/// tests/axis_ranking_separates_models.rs.
const RANK_WEIGHTS: [u8; 3] = [70, 25, 5];
```

Add `pub fn label(self) -> &'static str` returning the wire id for every variant, extend `from_label` with the six ids, and extend `resolve()` so each rank variant returns:
- `axes`: the `(cost, responsiveness, intelligence)` triple built by placing `RANK_WEIGHTS[position]` on each axis according to its position in the id.
- `quality` / `cost_preference` / `budget_gate` keyed off the **top** axis: Intelligence → `Premium`/`Performance`/`Relaxed`; Efficiency → `Flash`/`Economy`/`Default`; Responsiveness → `Balanced`/`Economy`/`Default`.
- `force_free_pool: false`, `always_delegate_free: false`, `delegate_free_when_simple` = `true` only when the top axis is Efficiency.

- [ ] **Step 4: Update the contract and its parity gate**

Add the six rows to `contracts/gui/drive-console.v1.yaml`'s `clutch:` list, each with `id`, `quality`, `axes`, `force_free_pool` — matching `resolve()` exactly. The parity test at `crates/vox-orchestrator/src/control/mod.rs:23` (`parity_tests::code_matches_contract`) iterates a fixed list of variants: extend that list with the six new ones so the gate actually covers them.

- [ ] **Step 5: Update the wire schema**

In `crates/vox-orchestrator-mcp/src/input_schemas.rs:627`, extend the `"clutch"` property's `"enum"` array with the six ids. **This is load-bearing and ungated:** there is no test asserting the schema literal matches `ChatMessageParams`, and `vox_chat_message` tolerates unknown keys but validates enum members — an id missing here is silently rejected at runtime. Add an assertion to the schema's existing test block (`input_schemas.rs:899-1065`) that the clutch enum contains all ten ids.

- [ ] **Step 6: Verify**

Run: `cargo nextest run -p vox-orchestrator -E 'test(rank_clutch_tests)'` → PASS
Run: `cargo nextest run -p vox-orchestrator -E 'test(parity)'` → PASS
Run: `cargo nextest run -p vox-orchestrator -p vox-orchestrator-mcp` → PASS

- [ ] **Step 7: Format and commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-orchestrator/src/mode.rs contracts/gui/drive-console.v1.yaml crates/vox-orchestrator/src/control/mod.rs crates/vox-orchestrator-mcp/src/input_schemas.rs
git commit -m "$(cat <<'EOF'
feat(vox-orchestrator): clutch vocabulary carries a full axis ranking

Adds six rank permutations (i>r>e etc.) alongside the four presets rather
than inventing a parallel wire field: `clutch` is already schema-validated
and already flows down both the sync and background lanes, so the ranking
reaches every lane with no new plumbing. Weights are 70/25/5 so the top
slot clears the two thresholds that actually flip model choice; a flatter
table collapses six orderings into three outcomes.

`free` stays a filter, not a ranking — no ordering implies force_free_pool.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

### Task 1.2: Risk=Low floors intelligence instead of erasing the ranking

**Files:**
- Modify: `crates/vox-orchestrator/src/mode.rs:299-311` (`effective_axes`)
- Modify: `crates/vox-orchestrator/src/mode.rs:426-447` (`interaction_tests`)
- Modify: `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs:634-647`

**Interfaces:** unchanged signature; changed semantics.

Today a `Low` risk posture hard-returns `(15,15,70)`, discarding whatever the user ranked. Defensible for a 4-preset dial; silently destructive once the user has expressed an explicit ordering.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn low_risk_floors_intelligence_without_erasing_the_ranking() {
        // e>r>i under Low risk: intelligence is raised to the floor, but the
        // user's efficiency-over-responsiveness preference survives.
        let clutch = ClutchProfile::from_label("e>r>i").expect("parses");
        let (cost, responsiveness, intelligence) = effective_axes(clutch, RiskPosture::Low);
        assert_eq!(intelligence, 70, "Low risk floors intelligence");
        assert_eq!((cost, responsiveness), (70, 25), "the user's ordering survives");
        assert!(cost > responsiveness, "relative order preserved");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo nextest run -p vox-orchestrator -E 'test(low_risk_floors)'`
Expected: FAIL — currently returns `(15, 15, 70)`.

- [ ] **Step 3: Implement**

```rust
/// The (cost, responsiveness, intelligence) triple the scorer should use after
/// risk adjusts the clutch. A `Low` posture's `ModelLean::Intelligence` raises
/// intelligence to a floor rather than overwriting the whole triple: the
/// scorer normalizes by total weight, so flooring still trips the
/// intelligence-threshold branch while preserving the ordering the user
/// explicitly chose. Overwriting silently discarded that choice.
#[must_use]
pub fn effective_axes(clutch: ClutchProfile, risk: RiskPosture) -> (u8, u8, u8) {
    let (cost, responsiveness, intelligence) = clutch.resolve().axes;
    match risk.resolve().model_lean {
        ModelLean::Intelligence => (cost, responsiveness, intelligence.max(70)),
        ModelLean::Neutral => (cost, responsiveness, intelligence),
    }
}
```

- [ ] **Step 4: Update the three assertions that pinned the old behavior**

`mode.rs:431-446` (`low_risk_overrides_efficiency_cheap_pick`, `genius_already_intelligent_unchanged_by_low_risk`) and `resolve.rs:634-647`. Each must now assert the floor semantics. **Update them deliberately, one at a time, stating in the commit why each changed** — these are the tests that encoded the old contract, so silently rewriting them would hide the behavior change.

- [ ] **Step 5: Verify and commit**

Run: `cargo nextest run -p vox-orchestrator -p vox-orchestrator-mcp` → PASS

```bash
vox run scripts/fmt.vox
git add crates/vox-orchestrator/src/mode.rs crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs
git commit -m "$(cat <<'EOF'
fix(vox-orchestrator): Low risk floors intelligence, no longer erases the rank

effective_axes hard-returned (15,15,70) for any Low-risk turn, silently
discarding a ranking the user had just set by hand. Flooring intelligence
at 70 still trips the intelligence threshold and still biases toward
capable models, while preserving the relative order of the other two axes
(the scorer normalizes by total weight, so magnitude is not the point).

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

### Task 1.3: Delete dead `VOX_MODEL_AXES` plumbing

**Files:**
- Modify: `crates/vox-orchestrator/src/models/select.rs:339-364` and its tests

- [ ] **Step 1: Confirm it is dead**

```bash
rg -n "from_env|VOX_MODEL_AXES" --type rust -g '!target' crates/
```
Expected: matches only in `select.rs` itself and its own tests. If any production caller exists, skip this task and say so.

- [ ] **Step 2: Delete `SelectionAxes::from_env` and its tests, and remove `VOX_MODEL_AXES` from any docs that reference it**

```bash
rg -rn "VOX_MODEL_AXES" docs/ contracts/
```
Remove stale references found there in the same commit.

- [ ] **Step 3: Verify and commit**

Run: `cargo nextest run -p vox-orchestrator` → PASS

```bash
vox run scripts/fmt.vox
git add -u
git commit -m "$(cat <<'EOF'
refactor(vox-orchestrator): delete unused SelectionAxes::from_env

No production caller — only its own tests. It also overlapped the new rank
vocabulary conceptually, offering a third way to express the same axes.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

### Task 1.4: Record the Responsiveness limitation honestly

**Files:**
- Modify: `crates/vox-orchestrator/src/models/scoring.rs` (doc comment on `live_latency`/`throughput_score`)
- Modify: `docs/src/architecture/chat-ui-model-knobs-audit-2026-09-10.md`

Ranking Responsiveness top moves the composite score by only ~0.072 — less than `ZERO_COST_BASE_BONUS` (0.1) — because `throughput_score` defaults to `20/200 = 0.1` for models with no measured throughput. The axis is wired but weak. Do not ship a UI that implies otherwise without saying so.

- [ ] **Step 1: Add a doc comment at the `throughput_score` default** naming the measured swing (~0.072), the two bonuses that exceed it, and what would fix it (populating throughput from `model_scoreboard.goodput_tokens_per_sec`, which the table already carries).
- [ ] **Step 2: Add a "Known limitation" note** to the audit doc's §4 with the same numbers.
- [ ] **Step 3: Commit** (docs-only; run `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/chat-ui-model-knobs-audit-2026-09-10.md` first).

---

## PHASE 2 — Make "Intelligence" mean something

### Task 2.1: Fix the scoreboard cache key collision

**Files:**
- Modify: `crates/vox-orchestrator/src/models/telemetry.rs:107-120` (`refresh_model_scoreboard`)

`refresh_model_scoreboard` keys its in-memory map by `model_id` while the table's PK is `(model_id, task_category, strength_tag, window_days)` — so rows overwrite each other last-write-wins with no `ORDER BY`, and a measured eval row can be clobbered by a telemetry row. Fix this before Task 2.2 reads from it, or Task 2.2 reads garbage.

- [ ] **Step 1: Write a failing test** proving two rows for the same `model_id` with different `task_category` both survive the refresh.
- [ ] **Step 2: Run it** — expected FAIL.
- [ ] **Step 3: Re-key the map** to `(model_id, task_category)` and add a deterministic `ORDER BY` to the query so the selected row is stable. Update every reader of the map accordingly (`rg -n "scoreboard" crates/vox-orchestrator/src/models/`).
- [ ] **Step 4: Verify** `cargo nextest run -p vox-orchestrator` → PASS. **Commit.**

### Task 2.2: Blend measured eval quality into the intelligence axis

**Files:**
- Modify: `crates/vox-orchestrator/src/models/scoring.rs:149-162` (`quality_score`), `:388-406` (the weighted sum)
- Modify: `crates/vox-orchestrator/src/models/select.rs:60-72` (`ScoreBreakdown`)

**Interfaces:**
- Produces: `intelligence_term(spec, scoreboard) -> (f64, QualityProvenance)` where `QualityProvenance` is `Measured { n: u32 } | Estimated`.

Today `quality_score` is `log10(max_tokens)/7 × 0.6 + (is_free ? 0.35 : 0.95) × 0.4` — context-window size and a price constant. Real measured quality exists (`vox model eval-corpus` writes pass@1 into `model_scoreboard`) but is never read at selection time. The telemetry rollup's own `quality_score` is **flat 1.0 for every model** (empty `llm_feedback`, callers hardcoding `Some(1.0)`) and must be excluded, or every model becomes perfectly intelligent.

- [ ] **Step 1: Write the failing tests** — three cases: (a) a model with an eval-sourced scoreboard row and `n_calls >= MIN` uses the measured value and reports `Measured`; (b) a model whose only row came from the telemetry rollup is **ignored** (flat-1.0 guard) and reports `Estimated`; (c) a model with no rows falls back to the heuristic shrunk toward the median of observed models — not 0 (unfairly buried) and not 1.0 (unfairly promoted).
- [ ] **Step 2: Run** — expected FAIL.
- [ ] **Step 3: Implement `intelligence_term`**, distinguishing eval rows from telemetry rows by `strength_tag`/`task_category` (`eval.rs:180`, `eval_corpus.rs:296` set these — read both and match on what they actually write, not on what seems likely). Replace `w.precision * quality_score(m)` at `scoring.rs:402` with the new term.
- [ ] **Step 4: Carry provenance into `ScoreBreakdown`**, which already has both `intelligence_score` and `telemetry_quality_score` fields — add the enum tag so the GUI can say "measured pass@1 over N fixtures" vs "estimated from context window."
- [ ] **Step 5: Verify** the Phase 0 gate still passes (`six_axis_orderings`) plus the full orchestrator suite. **Commit.**

### Task 2.3: Let discovered models earn their evidence

**Files:**
- Modify: `crates/vox-orchestrator/src/models/select.rs:265-271`

Cold start is a closed circuit: `resolve_eligibility(m, None, 0.0)` is called with no scoreboard, discovered models sit at `Shadowed`, `eligible_for_routing()` is `Confirmed`-only — so an OpenRouter model can never be selected and therefore never earns the rows that would promote it. `VOX_ROUTING_ENABLE_EXPLORATION=1` is the only escape and is off by default.

- [ ] **Step 1: Write a failing test** proving a `Shadowed` model with sufficient scoreboard evidence becomes routable without the env var.
- [ ] **Step 2: Run** — expected FAIL.
- [ ] **Step 3: Pass the real scoreboard row into `resolve_eligibility`** at `select.rs:271` (the plumbing exists and is already tested), so promotion is driven by evidence rather than by an env flag.
- [ ] **Step 4: Decide and document the exploration policy.** Evidence-driven promotion still requires *some* traffic to a shadowed model. Either (a) keep exploration opt-in and state plainly in the audit doc that new models require `vox model eval-corpus` to become routable, or (b) propose a bounded exploration budget. **Do not enable unbounded exploration silently — surface the choice.**
- [ ] **Step 5: Verify and commit.**

---

## PHASE 3 — The GUI control

### Task 3.1: `PriorityRankControl` — dnd-kit drag + click-promote + arrow buttons

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Loquela/PriorityRankControl.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Loquela/PriorityRankControl.test.tsx`

**Interfaces:**
- Consumes: `@dnd-kit/core` + `@dnd-kit/sortable` (already dependencies — follow the `DndContext`/`SortableContext`/`useSortable` + `PointerSensor{distance:6}` shape proven in `components/dashboard/DashboardGrid.tsx:231,314`).
- Produces: `<PriorityRankControl rank={PriorityRank} onChange={(next: PriorityRank) => void} />` where `PriorityRank` is `[AxisId, AxisId, AxisId]` in **display order, least→most (rightmost wins)**.

**Do not use HTML5 drag-and-drop.** `tauri.conf.json` leaves `dragDropEnabled` at its `true` default, so DOM drag events never reach the page; `Settings/PriorityChainEditor.tsx:186-203` already ships that exact dead pattern. dnd-kit is pointer-based and immune.

- [ ] **Step 1: Check for an existing home for the axis metadata.** `src/lib/taskPriority.ts` already exists with a similar wire-value/label shape — read it and either extend it or justify a new module in the commit message. Do not create a parallel file by default.
- [ ] **Step 2: Write the failing tests** covering: renders three abbreviations in display order; clicking an axis promotes it to rightmost; the ← and → buttons move an axis one slot and are disabled at the ends; the rightmost chip is visually distinct; an `aria-live="polite"` region announces the new order after a change; and a property test that repeated promotion reaches **all six** permutations (it does, in ≤2 clicks — verified).
- [ ] **Step 3: Run** — expected FAIL.
- [ ] **Step 4: Implement** with `role="group"` and `aria-label="Priority order: …"` naming the full order (do **not** put "priority 1 of 3" in each chip's accessible name — it churns on every reorder). Each chip is a `<button>` with `aria-label` = the axis name. The ← → buttons are the keyboard story; no arrow-key handler needed. Follow `Settings/HudTilesEditor.tsx:53-67` for the move-button pattern already used in this codebase.
- [ ] **Step 5: Verify** `cd crates/vox-gui/ui && pnpm vitest run src/components/surfaces/Loquela/PriorityRankControl.test.tsx && pnpm typecheck` → PASS.
- [ ] **Step 6: Manual verification in the real app is REQUIRED.** No automated lane can catch a webview-only drag failure: Playwright runs against the Vite dev server in Chromium, which is the one engine that tolerates the bugs at issue. Launch the app, drag a chip, confirm it reorders. Record the result in the commit message. If drag does not work, ship click + arrows and open a follow-up — do not claim drag works on the strength of green tests.
- [ ] **Step 7: Commit.**

### Task 3.2: Rewire DriveConsole + Loquela (one commit — the shape change cannot be atomic in pieces)

**Files:**
- Modify: `crates/vox-gui/ui/src/lib/driveConsole.ts`, `.../Loquela/DriveConsole.tsx`, `DriveConsole.test.tsx`, `Loquela.tsx`, `Loquela.test.tsx`, `src/lib/driveConsole.test.ts`, `src/components/surfaces/Settings/TaskPolicySection.tsx`, `src/types/tauri.ts`, `src/App.tsx`

`ControlState.clutch → priorityRank` plus deleting `CLUTCH_DETENTS` touches every consumer at once; splitting it is exactly what created v1's red commits. **`TaskPolicySection.tsx` also imports `CLUTCH_DETENTS`** — it was missed in v1.

- [ ] **Step 1: Enumerate consumers before editing**

```bash
rg -n "CLUTCH_DETENTS|ClutchId|control\.clutch|clutch:" crates/vox-gui/ui/src
```

- [ ] **Step 2: Write/adjust the failing tests** — `DriveConsole.test.tsx` rewritten for the ranking control (drop the four-detent and cost assertions), `driveConsole.test.ts:6`'s `CLUTCH_DETENTS` assertion replaced, and `Loquela.test.tsx`'s three `resolve_default_task_policy` tests re-pointed at the ranking control's rendered order.
- [ ] **Step 3: Run** — expected FAIL.
- [ ] **Step 4: Implement.** `ControlState` carries `priorityRank` + `risk` + a separate `freeOnly: boolean` (preserving the `free` filter the old detent provided — it is a filter, not a ranking, per Task 1.1). `Loquela.send()` emits `clutch: rankToClutchId(control.priorityRank, control.freeOnly)` — which returns `"free"` when `freeOnly`, else the reversed-and-joined rank id. **The display-order → wire-order reversal happens here and only here**; test it explicitly in both directions. `TaskPolicySection.tsx` keeps offering the four legacy presets (it configures per-category policy, not the composer) — point it at a static list rather than the deleted export.
- [ ] **Step 5: Verify** `pnpm typecheck && pnpm test` → PASS, then regenerate the honesty manifest (`node crates/vox-gui/ui/scripts/inventory.mjs`) since `DriveConsole.tsx:47` is pinned by line.
- [ ] **Step 6: Commit.**

### Task 3.3: Surface the routing decision inline

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ModelBadge.tsx`, `ChatTranscript.tsx:60-67`

This is the "visually reflected" half of the request, and it is nearly free: `selectionReason` is already plumbed end-to-end (`transport.ts:1036` → `chatSend.ts:30` → `App.tsx:1011,1206`) but hidden behind a click, and `ModelBadge`'s `selection` prop — with human labels "Your pick" / "Auto-routed" / "Fell back" — has **zero call sites**.

- [ ] **Step 1: Write the failing test** — the badge renders the selection reason inline (not only in the popover) and renders the `selection` label when supplied.
- [ ] **Step 2: Run** — expected FAIL.
- [ ] **Step 3: Implement**, and pass the already-available `selection` data from `ChatTranscript`. If Task 2.2's provenance tag is available by now, show "measured" vs "estimated" here — that is what makes the intelligence ranking honest to the user.
- [ ] **Step 4: Verify and commit.**

---

## PHASE 4 — Delete the confusion

Each task is independently green and independently revertible.

### Task 4.1: Delete dead composer controls
- [ ] **Queue chip** (`Loquela.tsx:702-711`) — never renders (App passes no `queueDepth`) and duplicates the status-bar Queue tile. Delete it and the two props.
- [ ] **Dry-run** — `setDryRun` is never called, so the "Dry-run" label is unreachable and `dry_run` is unconsumed on the sync path. Delete the state and the dead branch; leave the wire field (background path reads it).
- [ ] Write a test asserting neither renders, run it red, delete, run green, commit.

### Task 4.2: Budget lives in the status bar, with the burn rate it never had
- [ ] Delete DriveConsole's cost block and Loquela's `formatSessionBudget` readout + the `sessionBudget` prop chain (`App.tsx:1627-1631`). Note `burnPerMin` was never passed and `source` was always ignored, so nothing real is lost but a progress bar.
- [ ] Add `delta` (already in `kpis.budgetBurn`) to the status-bar tile, and replace the bare `—` cap with "cap unknown (no daemon)".
- [ ] Delete the per-draft cost estimate **with** the tier picker in 4.3 (its `tierObj.cost` is null unless the backfill ran).
- [ ] Test, commit.

### Task 4.3: One model menu
- [ ] Remove the tier-picker model backfill (`Loquela.tsx:274-312`) — it lists concrete model ids the backend cannot honor (`resolution_for_tier` understands only `local|mesh|cloud`, warns, and treats the rest as auto), so the control lies.
- [ ] Fold the three real location semantics into `ChatModelPicker` as top entries: `auto · local-only · cloud · <model…>`.
- [ ] Test that selecting `local-only` still sends `tier: "local"`, commit.

### Task 4.4: Settings routing becomes honest
- [ ] Make Settings → Routing read-only, relabelled "default for background/automated tasks", since the composer's clutch always wins for interactive turns (and the daemon only reads the pref at startup).
- [ ] Delete the 6-axis advanced block (`SettingsView.tsx:1497-1521`) — `availability`/`balance`/`mobile` have no user-facing meaning.
- [ ] Keep `PriorityChainEditor` and `TaskPolicySection` (genuinely per-category), but **fix `PriorityChainEditor.tsx:186-203`'s dead HTML5 drag** while you are there — same root cause as Task 3.1, currently a `title="Drag to reorder"` handle that does nothing. Either port it to dnd-kit or remove the affordance so the UI stops lying.
- [ ] Test, commit.

---

## PHASE 5 — Make mesh work (independent of all other phases)

### Task 5.1: Compile mesh into the shipped binaries
- [ ] Add `populi-transport` to `vox-orchestrator-d`'s two dependency entries (`Cargo.toml:15,17`) and `vox-gui`'s (`Cargo.toml:22,24`). **This is the single change everything else is downstream of** — without it `discover_populi_mesh_models` returns `Ok(vec![])` by `#[cfg]` and no mesh model can exist.
- [ ] Verify the daemon and GUI still build and that no new build-toolchain dependency appears (AGENTS.md's clean-clone invariant: no cmake/nasm/Go/perl/libclang).
- [ ] Commit.

### Task 5.2: Implement or delete the stubs
- [ ] `ServerState::spawn_populi_federation_poller` (`server_state.rs:619-621`) is an **empty body** with a comment where the implementation should be, and its two siblings at `:622-627` match. Either delegate to `vox_orchestrator::mesh_federation_poll` or delete them and their call site at `:335`. A stub wearing a working component's name is worse than an honest absence.
- [ ] Make `discover_populi_mesh_models` (`catalog.rs:517-530`) read `OrchestratorConfig::populi_control_url`, not only the two secrets.
- [ ] Map `VOX_MESH_CONTROL_ADDR` into `populi_control_url` in `impl_env.rs:240` so the poller and the MCP tool agree on one URL.
- [ ] Wire `populi_inference_base_url` or delete it — nothing reads it today.
- [ ] Test each, commit.

### Task 5.3: Recover from a control plane that was down
- [ ] `populi_http_join_best_effort` (`http_lifecycle.rs:132-196`) spawns its heartbeat loop only inside the `Ok` arm, so a control plane down at startup is never retried. Add bounded retry/backoff and spawn the heartbeat on the failure path too.
- [ ] This is safe: it only re-dials a URL the operator already configured. **Do not** default `VOX_MESH_ENABLED=1` or auto-dial without a configured URL — those need explicit sign-off and are out of scope.
- [ ] Test with a simulated unreachable-then-reachable control plane, commit.

### Task 5.4: Tell the truth in the status bar
- [ ] Switch `BottomStatusBar` from `useMeshNodes` to `useMeshNodesFull` (already exists) and render three distinct states: `local only` (no control plane configured), `unreachable` (configured, `control_plane_error` present), `N/M online`. Today all three read as "0/0 online," indistinguishable from broken.
- [ ] Tests for all three states; the two existing mesh tests must still pass unchanged. Commit.

---

## PHASE 6 — The send button (independent of all other phases)

### Task 6.1: Label the button from the real keybinding

**Files:**
- Modify: `crates/vox-gui/ui/src/lib/keybinds.ts`, `Loquela.tsx`, `App.tsx`, `Loquela.test.tsx`

A keybinding SSOT already exists — `keybinds.ts` defines `dispatch-intent` ("Dispatch intent (in composer)") bound to `Mod+Enter`, rebindable in Settings → Keybinds and persisted via `gui.keybinds`. Loquela ignores it and hardcodes both handlers, and the button hardcodes `⌘↵` while its aria-label says "(Enter)". Hardcoding a glyph would be the **third** divergent copy of one fact.

- [ ] **Step 1: Write the failing tests** — the button's label reflects the *bound* chord; rebinding `dispatch-intent` changes the label; on a non-Mac platform it renders `Ctrl` rather than `⌘`; the button is square (`w-9 h-9`), centered (`items-center justify-center`), and contains no separate `<kbd>`.
- [ ] **Step 2: Run** — expected FAIL (note `Loquela.test.tsx:207-217` currently asserts the literal `'⌘↵'`; that assertion is being replaced, deliberately).
- [ ] **Step 3: Add `formatChord(chord: string): string` to `keybinds.ts`** — the inverse of the existing `chordFromEvent`, with platform detection (none exists anywhere in the frontend today; add one helper, not one per call site).
- [ ] **Step 4: Pass `bindings` into Loquela** (App already holds the parsed state at `App.tsx:370-374`) and have `onKey` consult the bound chord for `dispatch-intent` instead of hardcoding `Meta/Ctrl+Enter`. Keep bare-Enter submit as-is — it is unrepresented in the registry and changing it is a separate decision.
- [ ] **Step 5: Restyle** the button to a square keycap showing `formatChord(binding)`, `aria-label` retaining the word "Run" (several existing tests match `/run/i`).
- [ ] **Step 6:** Leave Stop and Resume alone — Stop's `↵` is *truthful* (Enter does interrupt while a task runs) and Resume has no binding at all.
- [ ] **Step 7: Verify and commit.** Consider also applying `formatChord` to `Sidebar.tsx:220`'s hardcoded `⌘K` and `SettingsView.tsx:1600`'s raw `"Mod+Enter"` — but in a **separate** commit, as unrelated cleanup.

### Task 6.2: Fix the composer growth cap mismatch
- [ ] `Loquela.tsx:226` grows the textarea to `min(200, scrollHeight)` while `:563`'s className caps it at `max-h-[160px]`; `max-height` wins, so the last 40px becomes internal scrolling. Pick one number (160 to match the visual, or 200 to match the intent) and make both agree. The `expanded` branch (360/`max-h-[360px]`) is already consistent — match its pattern.
- [ ] Add a test pinning the two numbers together so they cannot drift again. Commit.

---

## Parallelization

Three independent streams — safe to run as concurrent subagents or worktrees:

- **Stream A (Rust router):** Phase 0 → Phase 1 → Phase 2, strictly sequential. Highest risk, gated on user approval.
- **Stream B (GUI):** Phase 3 depends on Task 1.1's wire ids existing; Phase 4 is independent of everything and can start immediately.
- **Stream C:** Phase 5 (mesh) and Phase 6 (send button) share no files with A or B and can run from the start.

Stream A's Task 0.1 gate should land before Stream B invests in the control — if the axes cannot be made to separate models, the ranking UI should not ship at all.

## Self-Review Notes

- **Every demand is mapped:** run button → 6.1/6.2; centering/single-line → 6.1/6.2; hot-key label → 6.1 (from the SSOT, not hardcoded); enter-key shape/far-right → 6.1; drag+click ranking → 3.1; rightmost-wins → 3.1/3.2; knobs actually wired → Phase 0 (the precondition v1 missed); benchmarks/dynamics incl. OpenRouter → Phase 2; visually reflected → 3.3; budget to status bar → 4.2; condense services → Phase 4; mesh auto-online → Phase 5; audit/advise → the corrected audit doc.
- **Known-and-stated gaps:** the Tauri IPC hop (`chat_turn.rs` → `ChatMessageParams`) has no test and matches by field name only; no lane can catch a webview-only drag failure, hence the mandatory manual step in 3.1; Responsiveness is weak until `throughput_score` is populated (1.4).
- **Net line count should be negative** across Phases 1, 3 and 4 — deletions (`from_env`, `CLUTCH_DETENTS`, `MiniSlider`, queue chip, dry-run, tier backfill, Settings 6-axis block, est-cost readout) exceed additions. If it ends up positive, the "fewer knobs" goal was missed.
