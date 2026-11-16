# Phase 427 Verification: Metal Memory Model Slope Calibration & Ladder Floor Measurement

**Status:** PASSED (`3/3` requirements verified)

## Requirement Traceability

| Requirement | Description | Evidence | Result |
|---|---|---|---|
| `MENS-NOV26-CAL-01` | Fit `candle-metal` `act_bytes_per_lht` from two distinct `tokens_per_step` runs on `Qwen3.5-2B` via `fit_a_lane` | `target/mens-cal/2b-s512/run_summary.json` (`9,633,126,196` B @ `512` tok) & `target/mens-cal/2b-s2048/run_summary.json` (`21,900,136,876` B @ `2048` tok) -> `162.48240311940512` B/LHT in `contracts/mens/memory-model.v1.yaml` and `calibration::tests::qwen35_candle_metal_calibration_fits_the_shipped_contract_slope` | PASS |
| `MENS-NOV26-CAL-02` | Record empirical `peak_mb`, safe `floor_mb`, `floor_measured: true`, and `measured_on: silicon-gpu-metal` across `2B`, `4B`, `9B`, `27B` | `mens/config/gpu-specs.yaml` (`19500`, `34000`, `36000`, `74000` MiB), `target/mens-cal/{2b,4b,9b}-preset` & `27b-s1024`, `./target/release/vox-ml-cli mens probe --detailed` (`verdict: FITS` on `Qwen3.5-2B` & `Qwen3.8-27B`) | PASS |
| `MENS-NOV26-CAL-03` | Store machine-portable measurement artifact & doc under `docs/src/architecture/measurements/` with relative paths only | `docs/src/architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.{json,md}`, `vox-doc-pipeline --lint-only` (`exit 0`), machine-specific user/tmp path scan zero matches | PASS |

## Regression Suite
- `vox-populi --features mens --lib`: `270 passed; 0 failed`
- `vox-populi --features mens --test training_presets_yaml_contract`: `4 passed; 0 failed`
- `vox-ml-cli --features mens-candle-metal --lib`: `495 passed; 0 failed`
