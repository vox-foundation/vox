---
gsd_state_version: "1.0"
milestone: v9.5
current_phase: 553
current_phase_name: "first of 12: Phases 553–564"
status: planning
stopped_at: Roadmap revised; next step `/gsd-plan-phase 553`
last_updated: "2026-11-20T12:57:51.459Z"
last_activity: 2026-11-20
last_activity_desc: Roadmap revised on owner feedback (12 phases, 67/67 requirements mapped)
state_head: 66593b6b801bc1589c6f09e2643d23f7ad526817
progress:
  total_phases: 12
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
milestone_name: MENS Flywheel, Truthful Lanes & Trustworthy Hub Releases
---

# Project State

## Project Reference

See: `.planning/PROJECT.md`; workstream roadmap `ROADMAP.md`, requirements `REQUIREMENTS.md`, research `research/`.

**Core value:** Every claim about a MENS hub or spoke — in a report, a model card, or the Space — is backed by a
reproducible, paired before/after measurement, and nothing reaches the Hugging Face Hub without it.
**Current focus:** Phase 553 — Clean Base, GPU Lock, Frozen Contracts & Proof Manifest

## Current Position

Phase: 553 (first of 12: Phases 553–564)
Plan: —
Status: Ready to plan
Last activity: 2026-11-20 — Roadmap revised on owner feedback (12 phases, 67/67 requirements mapped)

Progress: [░░░░░░░░░░] 0%

## Execution Waves

W0 553 → W1 554 · 555 · 556 · 557 → W2 558 · 559 · 560 · 561 → W3 562 + 564 code groups → W4 563 (564 still open)
→ W5 564 closes with the 27B verdict

GPU queue (one lock, strictly serial, nothing before `PID 34157` exits): 556 Spike B → 559 Metal prefix-cache parity →
559 "before" evals (published 0.8B, 2B, 4B, 9B adapters and bases) → 559 extension-slice evals → 560 live agent episodes
→ 562 Tier-1 canary (0.8B, 2B) → 562 rungs 0.8B → 2B → 4B → 9B → 563 27B preflight → 563 CAP-27B-04 (last training
job) → 564 27B `hub pull` smoke test. Short browser checks (561) and 0.8B–9B smoke tests (564) fit gaps between training
jobs. Rung durations are measured into `contracts/reports/mens-v95-ladder-queue.json`, never assumed.

## Accumulated Context

### Decisions

- Owner (2026-11-20): HF namespace stays personal; the weightless 27B repo is kept with a first-line disclosure; Space
  scope is the honest Space plus merged-ONNX in-browser inference for 0.8B; paid distillation capped at 12 USD after a
  0-USD local harvest.
- Owner (2026-11-20, roadmap feedback): every size is retrained with the new recipe and corpus, in ascending order
  0.8B → 2B → 4B → 9B → 27B, each gated on the one below; the 27B is the last training job. Metrics must separate
  adjacent sizes at an affordable cost, with an eval type fitted to each hub size, spoke and auxiliary model.
- Roadmap: numbering starts at 553; 551–552 are retired and not reused.
- Roadmap: discrimination (DISC-01, 02, 03, 04, 05) and the eval-only "before" (LADR-01) sit in Phase 559, before any
  retrain; DISC-06 (all sizes on one table) sits with the proof reports in Phase 564.
- Roadmap: the ladder is split: Phase 562 holds the canary and 0.8B–9B rungs, Phase 563 holds the 27B preflight and
  CAP-27B-04, so GPU-free work (561, 564's code groups) continues during the long jobs.
- Roadmap: the Improved / Neutral / Regressed verdict rule is pre-registered in Phase 557's `proof_stats.rs`, so the
  ladder gate (562) and the release reports (564) use one rule.
- Roadmap: CAP-27B-04 (Phase 563) trains on the canonical vox-lang mix after Phase 558's data lands, not `--fast-corpus`.
- Roadmap: LANE-06 sits with the card work in Phase 561 so `hub_card/**` has one owner per wave (Phase 556 also edits it).
- Roadmap: `train_record.rs` has one owner, so FUNL-04 (training health fields) sits with run stamping in Phase 555.
- Execution model: plan, plan-check and verify on Opus; execute on Sonnet 5.5.

### Constraints

- `PID 34157` (`eval-reasoning-compare`, Qwen3.8-27B) was still running at 2026-11-20T11:50Z (elapsed 12:27:03). Never
  rebuild `target/release` or reinstall the Metal plugin dylib while it runs.
- Never change request JSON or checkpoint header inputs; never edit `eval-gates-ladder.yaml` in place; new metric fields
  are optional.
- No train time is assumed for the retrained sizes; each is measured as its rung completes (LADR-03).

### Blockers

- All GPU work (556 Spike B, 559, 560, 561 browser checks, 562, 563, 564 smoke tests) waits for `PID 34157` to exit.

### Pending Todos

None yet.

### Roadmap Evolution

- Phase 553 edited: 27B v4 collateral failure + dogfood_v5 audit (PROOF-08, DATA-06)
- Phase 557 edited: 27B v4 collateral failure + dogfood_v5 audit (PROOF-08, DATA-06)
- Phase 558 edited: 27B v4 collateral failure + dogfood_v5 audit (PROOF-08, DATA-06)
- Phase 563 edited: 27B v4 collateral failure + dogfood_v5 audit (PROOF-08, DATA-06)
- Phase 564 edited: 27B v4 collateral failure + dogfood_v5 audit (PROOF-08, DATA-06)

## Session Continuity

**Stopped At:** Roadmap revised; next step `/gsd-plan-phase 553`
**Resume File:** None
