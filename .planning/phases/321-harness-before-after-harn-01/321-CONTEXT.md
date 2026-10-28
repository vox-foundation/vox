---
phase: 321-harness-before-after-harn-01
title: "Harness Before/After — locked decisions"
status: ready-for-planning
authorship: llm-assisted
updated: 2026-10-28
---

# Phase 321 CONTEXT

Autonomous discuss (2026-10-28): grey areas locked to RESEARCH.md recommendations. Phase 320 closed with SIZE-03 PASS; trained adapters at `mens/runs/qwen3-0.6b` and `mens/runs/qwen3-1.7b`.

## Locked decisions

| ID | Decision |
|----|----------|
| D-321-01 | **Harness = MENS operator path** (`vox mens serve` / eval-compare / eval-local), not agent `vox harness eval`. |
| D-321-02 | **Intelligence bench = `mens/data/heldout_bench`** via `vox mens eval-local` (codegen pass@1 + **n**). |
| D-321-03 | **Aggregate report schema** `mens_size_ladder_before_after.v1` (new); live under `mens/runs/` (or phase `evidence/`) plus a human `321-BEFORE-AFTER-REPORT.md`. |
| D-321-04 | **HARN-01 serve smoke** required for each **trained adapter directory** behind collateral gate. Base side scored via eval path; optional base HTTP smoke not required. |
| D-321-05 | **Per-size baselines**: before scores from each size's pinned base snapshot (same pin as training_manifest). |
| D-321-06 | **heldout_bench**: run full set; record true **n**. Wall-clock may be multi-hour on Metal — plans must use durable run records and sparse progress (no silent timeouts). |
| D-321-07 | **Orchestration**: prefer `scripts/mens-ladder-before-after.vox` (VoxLang-first); thin CLI glue only if a tracer needs a single entry. |
| D-321-08 | **Collateral first**: before serve, produce `collateral_damage_report.json` + `gate_receipt.json` in each trained run dir (Phase 320 gap). |
| D-321-09 | **not_trained sizes** (4B–32B): listed in the report with status only — **no fabricated scores**. |
| D-321-10 | **Serve model arg**: always the **run directory**, never the adapter file (bypass hazard from Phase 320). |

## Must-haves (roadmap)

1. Harness serves a chosen trained size's adapter through existing serve path behind collateral gate.
2. One report: for 0.6B and 1.7B, base vs adapter — reasoning accuracy + think-initiation on three slices, plus intelligence on heldout_bench, each with **n**.
3. At least two sizes with real trained scores; other sizes `not_trained` with no numbers.

## Inputs

- `.planning/phases/321-harness-before-after-harn-01/321-RESEARCH.md`
- `.planning/phases/320-train-the-sizes-size-03/320-LADDER-REPORT.md`
- `mens/runs/qwen3-0.6b/`, `mens/runs/qwen3-1.7b/`

## Out of scope

- Phase 319 mesh selection (still deferred)
- Training additional sizes
- Changing gpu-specs floors
