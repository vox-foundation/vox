# Phase 339: Model cards v2 - Context

**Gathered:** 2026-11-06
**Status:** Ready for planning
**Source:** PRD Express Path (docs/superpowers/specs/2026-11-06-mens-hub-cards-v2-design.md, commit 36edebb02)

<domain>
## Phase Boundary

Replace the phase-336 generated Hugging Face cards (`crates/vox-populi/src/mens/hub_card.rs`, ~1,100
lines of `format!` prose) with: one typed facts layer, askama markdown templates, a per-size data file,
one shared voxlang.org page (`docs/src/reference/mens-models.md`), real before/after Vox examples from
stored eval checkpoints, and lints that forbid duplication. Ends with a human side-by-side review of
every rendered card; only after approval does `vox mens hub sync` push cards plus the index.

Out of scope: Hub commit messages and CLI output (D3); the phase-340 selector itself (index uses a
static rule until it lands); re-gating any size (phases 337/338).

</domain>

<decisions>
## Implementation Decisions

### Locked decisions D1-D5 (user-approved 2026-11-06; do NOT re-open)
- **D1** All shared explanation lives on one voxlang.org page, `docs/src/reference/mens-models.md`. The
  Hub index has a one-paragraph summary that links to it. Model cards carry only size-specific content.
- **D2** Before/after examples are chosen by a fixed, stated rule: 2 wins plus 1 loss. Each card states
  the rule and the totals.
- **D3** "Hub sync text" means generated card prose, not commit messages or CLI output.
- **D4** Structure: typed `CardFacts` layer, askama markdown templates, data file with per-size wording.
- **D5** State is three independent facts (trained, spoke, gate); each card states each fact exactly once.

### State model (spec §3)
- Trained: `trained` | `not trained` — adapter weights present + `train-record.json`.
- Spoke: `vox-lang` for all current ladder adapters (`train_mixed.jsonl` holds only `vox_codegen` +
  `vox_docs_qa`); planned: `harness: tool-selection`, `harness: argument-generation`, `rust`.
- Gate: `passed` | `needs re-gate` | `failed` | `not evaluated` — from `gate_receipt.json` +
  `collateral_damage_report.json`, checked against the current policy.
- `needs re-gate`: report passed under a non-current rule. Current = slice pins (`slice_blake3`),
  paired rule, alpha, and `grader` id all match `eval-gates-ladder.yaml` and the running grader.
- The check is phase 338 wave 1's `vox_eval::collateral_evidence::check_collateral_report`. Until 338
  lands, cards v2 calls a stand-in with the SAME signature; 338 replaces it with no other change.
- `failed`: a current-rule evaluation measured the adapter and it did not pass.
- `not evaluated`: trained but a required current-rule evaluation is missing.
- Expected today: 1.7B/4B/8B trained, vox-lang, needs re-gate; 14B trained, vox-lang, not evaluated;
  32B not trained; 0.6B whatever its 128-item re-gate (running now) produces.
- Each fact appears once per card in a status line under the title, linking to the docs page's state
  legend. The gate part names what blocks it. A model card never states another size's state. The index
  grid is the only all-size state listing.

### Model card content, in order (spec §4)
1. Frontmatter: valid HF metadata — base model + revision, tags, spoke and gate-state tags.
   `model-index` holds current-rule metrics only.
2. Title + status line.
3. Why this size exists: 2-3 sentences from `mens/config/hub-cards.yaml` (hardware with measured
   memory, suited work, when to step up/down).
4. What it does, shown (D2): wins = the 2 shortest held-out items (prompt length, then item id) the
   adapter passes and base fails; loss = shortest item the adapter still fails (if none, say so). Each
   shows prompt, base output, adapter output, each with a one-line compiler verdict. One line states the
   rule and totals ("of 52 items the adapter fixes 15 and breaks 1").
5. Use it: pull, serve, one request, that request's real recorded output. Vox in that block must pass
   `vox check`.
6. Results: one table — held-out pass@1 base vs adapter; collateral per slice base vs adapter with the
   McNemar p-value. Column headers link to the docs page; one interpreting sentence per size. Removed:
   second reasoning table, invalid-results table, raw gate-message rows. Receipt file stays linked.
7. Training: only fields that differ between sizes (steps, tokens, losses, wall time, peak memory,
   commit). Shared fields move to the docs page.
8. Files: one line.
9. Footer: one line linking index + docs page; the only sentence allowed on more than one card.
- Removed from model cards: "What this is / is not", other-sizes list, PEFT explanation, links block.
- The 32B repo keeps its hand-written card under the existing `ensure_ladder_repos` rule.

### Docs page and index (spec §5)
- `docs/src/reference/mens-models.md`: full repo frontmatter, topic/domain/authorship facets
  (`authorship: llm-assisted` + `authorship_model`), kb:summary; one anchored section per concept:
  What MENS is; Hub and spoke (a `vox-diagram` fence, never mermaid; vox-lang trained, harness
  tool-selection/argument-generation planned with suites in 338, rust planned); The harness; Gating
  (held-out pass@1, collateral, why McNemar, why pinned slices, state legend); Prompt protocol and
  training method (`qwen3_non_thinking_codegen_v1`, shared method, why PEFT/transformers cannot load the
  adapters); Links to existing training/serving/data-contract pages (no copying).
- Index (`vox-mens-index`): (1) one paragraph linking into docs; (2) size × spoke grid, one state badge
  per cell, harness/rust columns "planned" with link to the plan — the only all-size state listing;
  (3) Which model should I use — rule over memory and task + pull command; while re-gates pending, names
  only gate-current sizes and says so; replaced by the 340 selector when it lands; (4) What's next, built
  from records (sizes needing re-gate, spokes with no adapter, estimated eval time each); (5) one held-out
  pass@1 comparison across sizes, current-rule only; (6) links.

### Architecture (spec §6)
- `hub_card/facts.rs` — reads run records, returns `CardFacts`; pure; holds no wording; only reader of
  the records. Depends on the vox_eval collateral check (stand-in until 338-01).
- `hub_card/examples.rs` — D2 rule over base and adapter checkpoints + reports joined by item id;
  deterministic; a missing output renders "no recorded output", never skipped silently.
- `hub_card/render.rs` — passes `CardFacts` to askama templates.
- `crates/vox-populi/templates/hub/` — `model.md`, `index.md`, partials; all wording lives here.
- `mens/config/hub-cards.yaml` — per-size blurbs + spoke descriptions; validated on load (unknown size or
  missing blurb is an error).
- `hub_sync.rs` — unchanged interface; `render_model_card` and `render_index_readme` keep signatures.
- askama: already in Cargo.lock; new direct dep of vox-populi; must add no workspace crate edge.
- Data flow: run dir → CardFacts → render → `mens/runs/hub_cards/<repo>/README.md` → user reviews side by
  side → `vox mens hub sync` pushes. Nothing pushed before review.

### Testing (spec §7)
- Goldens: one per state (passed, needs re-gate, failed, not evaluated, not trained), one with examples,
  one without, and the index; each from a fixture run dir.
- Lints as tests over every rendered card: no duplicate sentence of ≥4 words across two cards (footer
  excepted); every `voxlang.org/reference/mens-models/#…` anchor exists as a heading in mens-models.md;
  frontmatter validates against HF model-card metadata rules via a local fixture (no network); every
  `vox` block passes `vox check`.
- Mutation proofs: stale pass mapped to `passed` fails a test; dropping the loss example fails the
  honesty test; stating gate state twice fails the duplication lint.
- Docs: `vox-doc-pipeline --lint-only --paths docs/src/reference/mens-models.md` clean.

### Push rules (orchestrator brief)
- Final plan ends at a human checkpoint: side-by-side review of every rendered card before any push.
- After approval: `vox mens hub sync` pushes cards for all repos + the index; card-only for every size
  except one whose weights are gated-current; never `--force-ungated`; 32B card untouched.

### Process constraints
- Test-first; mutation proofs per §7; no hardcoded counts (totals, item counts derive from data);
  per-crate cargo only (no workspace-wide clippy/test); no new crate edge without user authorization.
- Coordinate with phase 335 (editing crates/vox-eval, vox-ml-cli eval files) and 338-01 (adds
  crates/vox-eval/src/collateral_evidence.rs, edits hub_sync.rs and hub_card.rs).

### Claude's Discretion
- Exact `CardFacts` field shapes, template partial breakdown, golden fixture layout.
- Where the `vox check` lint runs (vox-populi test vs a crate that already depends on both) — must not
  add a crate edge.
- The HF metadata fixture format and which rules it encodes.
- Static "which model" rule wording and the What's-next eval-time estimate source.
- Wave split and ordering vs 338-01.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spec and roadmap
- `docs/superpowers/specs/2026-11-06-mens-hub-cards-v2-design.md` — the approved design (this PRD)
- `.planning/ROADMAP.md` §Phase 339 (and 336, 338, 340) — goals and dependencies

### Coordination
- `.planning/phases/338-spoke-aware-gating-and-hub-repos/338-01-PLAN.md` — defines
  `check_collateral_report`, `CollateralRequirement`, `CollateralRefusal`, `CardStatus::NeedsReGate`,
  `RunRecords.collateral_req`; edits hub_card.rs / hub_sync.rs
- `.planning/phases/338-spoke-aware-gating-and-hub-repos/338-02-PLAN.md`, `338-06-PLAN.md` — receipt v2,
  spoke recorded on cards
- `.planning/phases/338-spoke-aware-gating-and-hub-repos/338-RESEARCH.md`, `338-CONTEXT.md`

### Code
- `crates/vox-populi/src/mens/hub_card.rs`, `hub_card_tests.rs`, `crates/vox-populi/tests/golden/hub_cards/`
- `crates/vox-populi/src/mens/hub_sync.rs` (callers of `render_model_card` / `render_index_readme`)
- `mens/config/eval-gates-ladder.yaml`
- `mens/runs/qwen3-*/evidence/heldout_{base,adapter}_nothink.json` and matching `*.ckpt.jsonl`
- `mens/data/heldout_bench/manifest.json`

### Repo policy
- `AGENTS.md` (Dependency Discipline, Authored Markdown Frontmatter, VoxDiagram, Test-First)
- `docs/src/contributors/documentation-governance.md` §Frontmatter, `contracts/documentation/kb-taxonomy.v1.yaml`
- `docs/src/reference/vox-diagram.md`
- `contracts/ci/crate-edges.allow.v1.json`, `docs/src/architecture/layers.toml`

</canonical_refs>

<specifics>
## Specific Ideas

- Example verdict line: "base: does not parse — `let mut` is not Vox".
- Example totals line: "of 52 items the adapter fixes 15 and breaks 1" (numbers derived, never hardcoded).
- Example gate blocker: "Needs re-gate: passed the retired 32-item collateral rule; the 128-item paired
  re-run has not been done."
- 0.6B: the gated push now in progress (phase 335) goes out with the current card; v2 replaces it later.

</specifics>

<deferred>
## Deferred Ideas

- Phase-340 selector output replacing the index's static "which model" rule.
- Hub commit messages and CLI output wording (D3).
- Re-gating 1.7B/4B/8B/14B (phases 337/338).

</deferred>

---

*Phase: 339-model-cards-v2*
*Context gathered: 2026-11-06 via PRD Express Path*
