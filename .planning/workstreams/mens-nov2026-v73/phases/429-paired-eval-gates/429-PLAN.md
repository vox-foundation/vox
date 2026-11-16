---
phase: 429-paired-eval-gates
plan: 01
type: execute
wave: 5
depends_on: [428-01]
files_modified:
  - mens/runs/qwen3-0.6b/
  - mens/runs/qwen3.5-2b/
  - mens/runs/qwen3.5-4b/
  - mens/runs/qwen3.5-9b/
  - mens/runs/qwen3.8-27b/
  - mens/runs/reasoning_compare/
autonomous: true
requirements:
  - MENS-NOV26-EVAL-01
  - MENS-NOV26-EVAL-02
  - MENS-NOV26-EVAL-03
---

# Phase 429-01: Paired Held-Out, Reasoning Compare & Collateral Gate Evaluation

## Objective
Execute paired base-vs-candidate `eval-local` (`mens/data/heldout_bench`), hash-pinned `eval-reasoning-compare` (`gsm8k_think`, `math500_think`, `vox_think`), `baseline-from-compare`, `eval-collateral-damage` (`--no-fail-fast`), and `eval-gate` (`mens/config/eval-gates-ladder.yaml`) across the Nov 2026 MENS ladder (`0.6B`, `2B`, `4B`, `9B`, `27B`), generating `gate_receipt.json` (`vox_mens_gate_receipt_v2`) and `mens/runs/reasoning_compare/*.json` consumed dynamically by `hub_card::facts`.

> **Retired Subcommand Note:** `vox mens size-train` was retired in commit `9a429892f` and is no longer a CLI subcommand on `PopuliAction`. Do **not** invoke `size-train`; `hub_card::facts` consumes `gate_receipt.json` and `mens/runs/reasoning_compare/*.json` directly.

## Execution Invariants & Hardware Guardrails
- **Release Binary:** `./target/release/vox-ml-cli` (built with `cargo build --release -p vox-ml-cli --features gpu,mens-candle-metal,mens-hf-hub` after Phase 428 / Phase 454–457.1 land).
- **Single-GPU Strict Serial Execution:** All Metal GPU inference/evaluation tasks (`eval-local`, `eval-reasoning-compare`, `eval-collateral-damage`) are `[SEQUENTIAL]`. Never run two Metal workloads concurrently on the 128 GiB Apple M5 Max unified-memory host.
- **CPU Tasks Parallel-Safe:** Pure CPU post-processing and policy checks (`baseline-from-compare`, `eval-gate --no-hf-sync`, JSON/receipt schema checks) are `[PARALLEL-SAFE]` across rungs once their upstream Metal inference outputs exist.
- **Defer Hub Push to Phase 458:** Pass `--no-hf-sync` on `eval-gate` in Phase 429 so Hub publishing waits until Phase 458 completes native BF16 causal-LM `merge-qlora` + `peft/` packaging.
- **Resumable Checkpoints:** `eval-local`, `eval-reasoning-compare`, and `eval-collateral-damage` write per-item `.ckpt.jsonl` / `mens/runs/reasoning_compare/checkpoints/` files and resume automatically unless `--force-restart` is passed.

## Disk Baseline & Rung Matrix

| Rung | Run Dir | Pinned Base & Snapshot Path (`~/.cache/huggingface/hub/...`) | Current Disk State |
| :--- | :--- | :--- | :--- |
| `0.6B` | `mens/runs/qwen3-0.6b` | `Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca` (`models--Qwen--Qwen3-0.6B/snapshots/c1899de289a04d12100db370d81485cdf75e47ca`) | `candle_qlora_adapter.safetensors`, `eval_local_report.json`, `collateral_damage_report.json`, `gate_receipt.json`, `mens/runs/reasoning_compare/Qwen3-0.6B-c1899de289a0.json` present |
| `2B` | `mens/runs/qwen3.5-2b` | `Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc` (`models--Qwen--Qwen3.5-2B/snapshots/15852e8c16360a2fea060d615a32b45270f8a8fc`) | `candle_qlora_adapter.safetensors`, `adapter_manifest.json`, `run_summary.json` present; no Phase 429 evals yet |
| `4B` | `mens/runs/qwen3.5-4b` | `Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` (`models--Qwen--Qwen3.5-4B/snapshots/851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a`) | `candle_qlora_adapter.safetensors`, `adapter_manifest.json`, `run_summary.json` present; no Phase 429 evals yet |
| `9B` | `mens/runs/qwen3.5-9b` | `Qwen/Qwen3.5-9B@c202236235762e1c871ad0ccb60c8ee5ba337b9a` (`models--Qwen--Qwen3.5-9B/snapshots/c202236235762e1c871ad0ccb60c8ee5ba337b9a`) | `candle_qlora_adapter.safetensors`, `adapter_manifest.json`, `run_summary.json` present; no Phase 429 evals yet |
| `27B` | `mens/runs/qwen3.8-27b` | `Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` (`models--Qwen--Qwen3.8-27B/snapshots/37561816e66b5b9153b1b64c10968c18c1e6f4a0`) | Training in `task-22621`; Phase 429 starts only after `run_summary.json` (`capped: false`) and `candle_qlora_adapter.safetensors` land |

---

## Tasks

### Task 429.0 — Verify `0.6B` Baseline & Pre-Eval Release Binary `[PARALLEL-SAFE]`
- **Inputs:** `mens/runs/qwen3-0.6b/{gate_receipt.json,eval_local_report.json,collateral_damage_report.json}`, `mens/runs/reasoning_compare/Qwen3-0.6B-c1899de289a0.json`, `./target/release/vox-ml-cli`.
- **Actions:**
  1. Verify `./target/release/vox-ml-cli` exists and supports `eval-local`, `eval-reasoning-compare`, `baseline-from-compare`, `eval-collateral-damage`, and `eval-gate`.
  2. Re-run CPU-only `eval-gate` on `0.6B` to confirm its `gate_receipt.json` policy hash matches current `mens/config/eval-gates-ladder.yaml`:
     ```sh
     ./target/release/vox-ml-cli mens eval-gate --run-dir mens/runs/qwen3-0.6b --policy mens/config/eval-gates-ladder.yaml --no-hf-sync
     ```
- **Expected Output:** Exit `0`; `mens/runs/qwen3-0.6b/gate_receipt.json` shows `"overall_passed": true` with `"policy": "mens/config/eval-gates-ladder.yaml"`.

### Task 429.1 — `Qwen3.5-2B` Paired Eval & Gate (`mens/runs/qwen3.5-2b`) `[SEQUENTIAL]`
- **Precondition:** No active Metal job (`task-22621` idle/complete).
- **Commands (run strictly one at a time):**
  1. Base held-out baseline:
     ```sh
     ./target/release/vox-ml-cli mens eval-local --base ~/.cache/huggingface/hub/models--Qwen--Qwen3.5-2B/snapshots/15852e8c16360a2fea060d615a32b45270f8a8fc --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.5-2b/baseline_passatk.json
     ```
  2. Candidate held-out eval (writes `eval_local_report.json` + `benchmark_passatk.json`):
     ```sh
     ./target/release/vox-ml-cli mens eval-local --model mens/runs/qwen3.5-2b/candle_qlora_adapter.safetensors --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.5-2b/eval_local_report.json
     ```
  3. Hash-pinned reasoning compare (`gsm8k_think`, `math500_think`, `vox_think`):
     ```sh
     ./target/release/vox-ml-cli mens eval-reasoning-compare --base Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc --adapter mens/runs/qwen3.5-2b --bench-root mens/data --out mens/runs/reasoning_compare/Qwen3.5-2B-15852e8c1636.json
     ```
  4. Build collateral baseline (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens baseline-from-compare --compare mens/runs/reasoning_compare/Qwen3.5-2B-15852e8c1636.json --out mens/runs/qwen3.5-2b/collateral_baseline.json
     ```
  5. Paired collateral damage McNemar check:
     ```sh
     ./target/release/vox-ml-cli mens eval-collateral-damage --pre-score mens/runs/qwen3.5-2b/collateral_baseline.json --post mens/runs/qwen3.5-2b/candle_qlora_adapter.safetensors --no-fail-fast
     ```
  6. Gate evaluation (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens eval-gate --run-dir mens/runs/qwen3.5-2b --policy mens/config/eval-gates-ladder.yaml --no-hf-sync
     ```
- **Outputs:** `mens/runs/qwen3.5-2b/{baseline_passatk.json,benchmark_passatk.json,eval_local_report.json,collateral_baseline.json,collateral_damage_report.json,gate_receipt.json}` and `mens/runs/reasoning_compare/Qwen3.5-2B-15852e8c1636.json`.

### Task 429.2 — `Qwen3.5-4B` Paired Eval & Gate (`mens/runs/qwen3.5-4b`) `[SEQUENTIAL]`
- **Precondition:** Task 429.1 Metal steps finished.
- **Commands (run strictly one at a time):**
  1. Base held-out baseline:
     ```sh
     ./target/release/vox-ml-cli mens eval-local --base ~/.cache/huggingface/hub/models--Qwen--Qwen3.5-4B/snapshots/851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.5-4b/baseline_passatk.json
     ```
  2. Candidate held-out eval:
     ```sh
     ./target/release/vox-ml-cli mens eval-local --model mens/runs/qwen3.5-4b/candle_qlora_adapter.safetensors --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.5-4b/eval_local_report.json
     ```
  3. Hash-pinned reasoning compare:
     ```sh
     ./target/release/vox-ml-cli mens eval-reasoning-compare --base Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a --adapter mens/runs/qwen3.5-4b --bench-root mens/data --out mens/runs/reasoning_compare/Qwen3.5-4B-851bf6e806ef.json
     ```
  4. Build collateral baseline (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens baseline-from-compare --compare mens/runs/reasoning_compare/Qwen3.5-4B-851bf6e806ef.json --out mens/runs/qwen3.5-4b/collateral_baseline.json
     ```
  5. Paired collateral damage McNemar check:
     ```sh
     ./target/release/vox-ml-cli mens eval-collateral-damage --pre-score mens/runs/qwen3.5-4b/collateral_baseline.json --post mens/runs/qwen3.5-4b/candle_qlora_adapter.safetensors --no-fail-fast
     ```
  6. Gate evaluation (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens eval-gate --run-dir mens/runs/qwen3.5-4b --policy mens/config/eval-gates-ladder.yaml --no-hf-sync
     ```
- **Outputs:** `mens/runs/qwen3.5-4b/{baseline_passatk.json,benchmark_passatk.json,eval_local_report.json,collateral_baseline.json,collateral_damage_report.json,gate_receipt.json}` and `mens/runs/reasoning_compare/Qwen3.5-4B-851bf6e806ef.json`.

### Task 429.3 — `Qwen3.5-9B` Paired Eval & Gate (`mens/runs/qwen3.5-9b`) `[SEQUENTIAL]`
- **Precondition:** Task 429.2 Metal steps finished.
- **Commands (run strictly one at a time):**
  1. Base held-out baseline:
     ```sh
     ./target/release/vox-ml-cli mens eval-local --base ~/.cache/huggingface/hub/models--Qwen--Qwen3.5-9B/snapshots/c202236235762e1c871ad0ccb60c8ee5ba337b9a --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.5-9b/baseline_passatk.json
     ```
  2. Candidate held-out eval:
     ```sh
     ./target/release/vox-ml-cli mens eval-local --model mens/runs/qwen3.5-9b/candle_qlora_adapter.safetensors --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.5-9b/eval_local_report.json
     ```
  3. Hash-pinned reasoning compare:
     ```sh
     ./target/release/vox-ml-cli mens eval-reasoning-compare --base Qwen/Qwen3.5-9B@c202236235762e1c871ad0ccb60c8ee5ba337b9a --adapter mens/runs/qwen3.5-9b --bench-root mens/data --out mens/runs/reasoning_compare/Qwen3.5-9B-c20223623576.json
     ```
  4. Build collateral baseline (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens baseline-from-compare --compare mens/runs/reasoning_compare/Qwen3.5-9B-c20223623576.json --out mens/runs/qwen3.5-9b/collateral_baseline.json
     ```
  5. Paired collateral damage McNemar check:
     ```sh
     ./target/release/vox-ml-cli mens eval-collateral-damage --pre-score mens/runs/qwen3.5-9b/collateral_baseline.json --post mens/runs/qwen3.5-9b/candle_qlora_adapter.safetensors --no-fail-fast
     ```
  6. Gate evaluation (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens eval-gate --run-dir mens/runs/qwen3.5-9b --policy mens/config/eval-gates-ladder.yaml --no-hf-sync
     ```
- **Outputs:** `mens/runs/qwen3.5-9b/{baseline_passatk.json,benchmark_passatk.json,eval_local_report.json,collateral_baseline.json,collateral_damage_report.json,gate_receipt.json}` and `mens/runs/reasoning_compare/Qwen3.5-9B-c20223623576.json`.

### Task 429.4 — `Qwen3.8-27B` Paired Eval & Gate (`mens/runs/qwen3.8-27b`) `[SEQUENTIAL]`
- **Precondition:** Task 429.3 Metal steps finished **and** Phase 428 `task-22621` completed (`mens/runs/qwen3.8-27b/run_summary.json` present with `"capped": false` and `candle_qlora_adapter.safetensors` present).
- **Commands (run strictly one at a time):**
  1. Base held-out baseline:
     ```sh
     ./target/release/vox-ml-cli mens eval-local --base ~/.cache/huggingface/hub/models--Qwen--Qwen3.8-27B/snapshots/37561816e66b5b9153b1b64c10968c18c1e6f4a0 --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.8-27b/baseline_passatk.json
     ```
  2. Candidate held-out eval:
     ```sh
     ./target/release/vox-ml-cli mens eval-local --model mens/runs/qwen3.8-27b/candle_qlora_adapter.safetensors --bench mens/data/heldout_bench --temperature 0.0 --samples 1 --k 1 --output mens/runs/qwen3.8-27b/eval_local_report.json
     ```
  3. Hash-pinned reasoning compare:
     ```sh
     ./target/release/vox-ml-cli mens eval-reasoning-compare --base Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0 --adapter mens/runs/qwen3.8-27b --bench-root mens/data --out mens/runs/reasoning_compare/Qwen3.8-27B-37561816e66b.json
     ```
  4. Build collateral baseline (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens baseline-from-compare --compare mens/runs/reasoning_compare/Qwen3.8-27B-37561816e66b.json --out mens/runs/qwen3.8-27b/collateral_baseline.json
     ```
  5. Paired collateral damage McNemar check:
     ```sh
     ./target/release/vox-ml-cli mens eval-collateral-damage --pre-score mens/runs/qwen3.8-27b/collateral_baseline.json --post mens/runs/qwen3.8-27b/candle_qlora_adapter.safetensors --no-fail-fast
     ```
  6. Gate evaluation (`[PARALLEL-SAFE]` CPU):
     ```sh
     ./target/release/vox-ml-cli mens eval-gate --run-dir mens/runs/qwen3.8-27b --policy mens/config/eval-gates-ladder.yaml --no-hf-sync
     ```
- **Outputs:** `mens/runs/qwen3.8-27b/{baseline_passatk.json,benchmark_passatk.json,eval_local_report.json,collateral_baseline.json,collateral_damage_report.json,gate_receipt.json}` and `mens/runs/reasoning_compare/Qwen3.8-27B-37561816e66b.json`.

### Task 429.5 — Cross-Ladder Receipt & Facts Ingestion Check `[PARALLEL-SAFE]`
- **Inputs:** All 5 `mens/runs/<rung>/gate_receipt.json` files and `mens/runs/reasoning_compare/*.json`.
- **Verification:**
  1. Confirm each `gate_receipt.json` has `"schema": "vox_mens_gate_receipt_v2"`, `"policy": "mens/config/eval-gates-ladder.yaml"`, and all 5 gates (`leakage`, `supervised_ratio`, `anti_stub`, `pass_at_k`, `collateral_damage`) recorded.
  2. Dry-run card generation (`./target/release/vox-ml-cli mens hub sync --namespace brbrainerd`, no `--push`) to verify `hub_card::facts` dynamically ingests all 5 `gate_receipt.json` and `reasoning_compare/` JSONs without fallback errors.
  3. Write `.planning/workstreams/mens-nov2026-v73/phases/429-paired-eval-gates/429-SUMMARY.md`.

---

## Expected Gate Thresholds (`mens/config/eval-gates-ladder.yaml`)
- `leakage` (`block: true`): zero held-out answer / corpus-completion n-gram overlap.
- `pass_at_k` (`block: true`): `min_pass_rate_at_1 >= 0.25`, `max_regression_drop <= 0.03` vs `baseline_passatk.json`.
- `anti_stub` (`block: true`): `min_pass_rate >= 0.92`, `max_placeholder_event_rate <= 0.08`, `max_trivial_placeholder_event_rate <= 0.08`, `min_construct_richness_mean >= 0.125`.
- `collateral_damage` (`block: true`): paired McNemar `alpha = 0.05` across all 3 pinned slices (`gsm8k_think`, `math500_think`, `vox_think`) with zero statistically significant regression.
- `supervised_ratio` (`block: false`): `>= 10.0%` from `telemetry.jsonl`.

## Abort & Rollback Triggers
- **Leakage Failure (`leakage.passed == false`):** Hard stop for that rung; do **not** promote or push to Hub; quarantine corpus overlap before retraining.
- **Gate Failure (`overall_passed == false` on `anti_stub`, `pass_at_k`, or `collateral_damage`):** Mark rung `gate_failed` in Phase 430; exclude from default Hub promotion (or only allow explicit `--force-ungated` if user-approved, never default `main`); preserve existing `.ckpt.jsonl` for root-cause inspection.
- **Metal OOM / Command Hang:** Abort immediately; do not lower precision or skip slices silently; preserve per-item `.ckpt.jsonl` and resume after verifying zero concurrent GPU processes.
