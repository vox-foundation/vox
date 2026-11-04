# Phase 321 — Harness before/after (HARN-01, BENCH-04)

**Host:** Apple Silicon (`silicon-gpu-metal`)  
**Branch:** `gsd/mens-compare-316`  
**Decision (2026-11-01):** Close ladder at 4B — three sizes trained and scored (`0.6B`, `1.7B`, `4B`). `4B` passes the 5% reasoning collateral gate across all three slices (`vox_think` +27.8%, `gsm8k_think` 0.0% drop, `math500_think` 0.0% drop); `0.6B` and `1.7B` fail collateral. `8B` refused at 82,103 MB free vs 122,876 MB estimate floor (`mens/runs/qwen3-8b/train-record.json`).

**Machine rollup:** [`mens/runs/ladder_before_after.json`](../../../mens/runs/ladder_before_after.json) (`mens_size_ladder_before_after.v1`, `gate_input: false`)

## BENCH-04 — Trained sizes (real scores)

### 0.6B (`mens/runs/qwen3-0.6b`)

| Slice | Base acc | Adapter acc | Think init (base/adapter) | n |
|-------|----------|-------------|---------------------------|---|
| gsm8k_think | 0.500 | 0.406 | 1.0 / 1.0 | 32 |
| math500_think | 0.156 | 0.188 | 1.0 / 1.0 | 32 |
| vox_think | 0.219 | 0.219 | 1.0 / 1.0 | 32 |

| Intelligence (`heldout_bench`, `qwen3_non_thinking_codegen_v1`, 1024 tok) | pass@1 | n |
|---------------------------------------------------------------------------|--------|---|
| Base snapshot | 0.115 (6/52) | 52 |
| Adapter | 0.365 (19/52) | 52 |

Artifacts: `mens/runs/reasoning_compare/Qwen3-0.6B-c1899de289a0.json`, `mens/runs/qwen3-0.6b/evidence/heldout_{base,adapter}_nothink.json`.

**Collateral:** `fail` on `gsm8k_think` (pre 0.500 → post 0.406, relative drop 18.75% vs 5% bar). Report: `mens/runs/qwen3-0.6b/collateral_damage_report.json`.

### 1.7B (`mens/runs/qwen3-1.7b`)

| Slice | Base acc | Adapter acc | Think init (base/adapter) | n |
|-------|----------|-------------|---------------------------|---|
| gsm8k_think | 0.594 | 0.656 | 1.0 / 1.0 | 32 |
| math500_think | 0.188 | 0.281 | 1.0 / 1.0 | 32 |
| vox_think | 0.656 | 0.594 | 1.0 / 1.0 | 32 |

| Intelligence (`heldout_bench`, `qwen3_non_thinking_codegen_v1`, 1024 tok) | pass@1 | n |
|---------------------------------------------------------------------------|--------|---|
| Base snapshot | 0.058 (3/52) | 52 |
| Adapter | 0.327 (17/52) | 52 |

Artifacts: `mens/runs/reasoning_compare/Qwen3-1.7B-70d244cc86cc.json`, `mens/runs/qwen3-1.7b/evidence/heldout_{base,adapter}_nothink.json`.

**Collateral:** `fail` on `vox_think` (pre 0.656 → post 0.594, relative drop 9.52% vs 5% bar). Report: `mens/runs/qwen3-1.7b/collateral_damage_report.json`.

### 4B (`mens/runs/qwen3-4b`)

| Slice | Base acc | Adapter acc | Think init (base/adapter) | n |
|-------|----------|-------------|---------------------------|---|
| gsm8k_think | 0.625 | 0.625 | 1.0 / 1.0 | 32 |
| math500_think | 0.219 | 0.219 | 1.0 / 1.0 | 32 |
| vox_think | 0.5625 | 0.71875 | 1.0 / 1.0 | 32 |

| Intelligence (`heldout_bench`, `qwen3_non_thinking_codegen_v1`, 1024 tok) | pass@1 | n |
|---------------------------------------------------------------------------|--------|---|
| Base snapshot | 0.077 (4/52) | 52 |
| Adapter | 0.250 (13/52) | 52 |

Artifacts: `mens/runs/reasoning_compare/Qwen3-4B-1cfa9a720891.json`, `mens/runs/qwen3-4b/evidence/heldout_{base,adapter}_nothink.json`.

**Collateral:** **`pass`** across all three reasoning slices (`vox_think` +27.8% relative gain, `gsm8k_think` 0.0% degradation, `math500_think` 0.0% degradation). Report: `mens/runs/qwen3-4b/collateral_damage_report.json`.

## HARN-01 — Serve gate (enforcement)

| Attempt | Result |
|---------|--------|
| `--model` = adapter **file** (`0.6B`, `4B`) | **Refused** (D-321-10 / `resolve_serve_run_dir`) |
| `--model` = run **directory** (`0.6B`, `1.7B`) | **Refused** at `assert_collateral_gate` — collateral `fail` (“cannot be served”) |
| `--model` = run **directory** (`4B`) | **Passes `assert_collateral_gate`** (`status: pass` across all 3 slices); refused at `assert_serve_preconditions` (`eval-gate` blocking gates not met) |
| Live HTTP serve | **Not opened** (document-close decision) |

Evidence: `.planning/phases/321-harness-before-after-harn-01/evidence/serve-smoke.json`.

## not_trained sizes (no numeric scores)

| Size | Status | Reason (from Phase 320 ladder / train-record) |
|------|--------|-----------------------------------------------|
| 8B | not_trained | Explicit size 8B needs 122876 MB but 82103 MB is free (Metal working set); refusing rather than shrinking |
| 14B | not_trained | Estimate floor (195382 MB) exceeds live free memory |
| 32B | not_trained | Estimate floor (295822 MB) exceeds live free memory |

## Findings

1. **BENCH-04 exceeded:** three trained sizes (`0.6B`, `1.7B`, `4B`) with real before/after reasoning (`n=32` per slice) and non-thinking codegen intelligence (`n=52`).
2. **Non-thinking codegen protocol (`448df0192`) unlocked heldout `pass@1`:** switching `eval-local` to `qwen3_non_thinking_codegen_v1` with a 1024-token budget raised heldout `pass@1` from `0.0%` (truncated inside `<think>` at 128 tokens) to **36.5%** on `0.6B` (vs 11.5% base), **32.7%** on `1.7B` (vs 5.8% base), and **25.0%** on `4B` (vs 7.7% base).
3. **4B clears reasoning collateral:** `Qwen3-4B` preserves `gsm8k_think` (`62.5%`) and `math500_think` (`21.9%`) with zero degradation while improving `vox_think` from `56.25%` to `71.875%` (+27.8% relative), passing `assert_collateral_gate`.
