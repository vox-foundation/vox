# Phase 321 — Harness before/after (HARN-01, BENCH-04)

**Host:** Apple Silicon (`silicon-gpu-metal`)  
**Branch:** `gsd/mens-compare-316`  
**Decision (2026-10-29):** Document & close — collateral `status: fail` on both trained sizes; do **not** invent a serve pass. Gate refusals recorded in `evidence/serve-smoke.json`.

**Machine rollup:** [`mens/runs/ladder_before_after.json`](../../../mens/runs/ladder_before_after.json) (`mens_size_ladder_before_after.v1`, `gate_input: false`)

## BENCH-04 — Trained sizes (real scores)

### 0.6B (`mens/runs/qwen3-0.6b`)

| Slice | Base acc | Adapter acc | Think init (base/adapter) | n |
|-------|----------|-------------|---------------------------|---|
| gsm8k_think | 0.500 | 0.406 | 1.0 / 1.0 | 32 |
| math500_think | 0.156 | 0.188 | 1.0 / 1.0 | 32 |
| vox_think | 0.219 | 0.219 | 1.0 / 1.0 | 32 |

| Intelligence (heldout_bench) | pass@1 | n |
|------------------------------|--------|---|
| Base snapshot | 0.00 | 52 |
| Adapter | 0.038 | 52 |

Artifacts: `mens/runs/reasoning_compare/Qwen3-0.6B-c1899de289a0.json`, `mens/runs/qwen3-0.6b/evidence/heldout_{base,adapter}.json`.

**Collateral:** `fail` on `gsm8k_think` (pre 0.50 → post 0.41, relative drop ~18.8% vs 5% bar). Report: `mens/runs/qwen3-0.6b/collateral_damage_report.json`.

### 1.7B (`mens/runs/qwen3-1.7b`)

| Slice | Base acc | Adapter acc | Think init (base/adapter) | n |
|-------|----------|-------------|---------------------------|---|
| gsm8k_think | 0.594 | 0.656 | 1.0 / 1.0 | 32 |
| math500_think | 0.188 | 0.281 | 1.0 / 1.0 | 32 |
| vox_think | 0.656 | 0.594 | 1.0 / 1.0 | 32 |

| Intelligence (heldout_bench) | pass@1 | n |
|------------------------------|--------|---|
| Base snapshot | 0.00 | 52 |
| Adapter | 0.00 | 52 |

Artifacts: `mens/runs/reasoning_compare/Qwen3-1.7B-70d244cc86cc.json`, `mens/runs/qwen3-1.7b/evidence/heldout_{base,adapter}.json`.

**Collateral:** `fail` on `vox_think` (pre 0.66 → post 0.59, relative drop ~9.5% vs 5% bar). Report: `mens/runs/qwen3-1.7b/collateral_damage_report.json`.

## HARN-01 — Serve gate (enforcement, not live serve)

| Attempt | Result |
|---------|--------|
| `--model` = adapter **file** | **Refused** (D-321-10 / `resolve_serve_run_dir`) |
| `--model` = run **directory** (0.6B) | **Refused** — collateral `fail` (“cannot be served”) |
| Live HTTP serve | **Not opened** (document-close decision) |

Evidence: `.planning/phases/321-harness-before-after-harn-01/evidence/serve-smoke.json`.  
`gate_receipt.json` was **not** produced (eval-gate not run after collateral fail).

## not_trained sizes (no numeric scores)

| Size | Status | Reason (from Phase 320 ladder) |
|------|--------|--------------------------------|
| 4B | not_trained | S3 probe watchdog — does not fit |
| 8B | not_trained | Above non-fitting 4B (D-06) |
| 14B | not_trained | Above non-fitting 4B (D-06) |
| 32B | not_trained | Above non-fitting 4B (D-06) |

## Wall time (Metal scoring, from checkpoint)

| Size | Reasoning compare | Heldout base | Heldout adapter | Collateral |
|------|-------------------|--------------|-----------------|------------|
| 0.6B | ~5.6 h | ~15 min | ~15 min | ~2.5 h (fail) |
| 1.7B | ~8.5 h | ~23 min | ~18 min | ~7.5 h (fail) |

## Findings

1. **BENCH-04 met:** two trained sizes with real before/after reasoning (n=32) and intelligence (n=52).
2. **Collateral bar blocked serve:** both adapters exceed the 5% relative degradation rule on at least one slice; serve correctly refuses.
3. **Heldout pass@1 remains near zero** on this Metal pack — intelligence signal is weak; reasoning slices are the stronger before/after signal.
4. **Native `vox run` of `mens-ladder-before-after.vox`** still fails compile on the native lane; prefer `--interp` / direct CLI until fixed.
