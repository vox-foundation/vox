# 337-13 SUMMARY

**Completed:** 2026-11-13
**Requirements:** THRU-09, THRU-10

## What shipped

- Closed Phase 337 throughput instrumentation, dense inference collapse (`QuantizedLinear::Dense`), batched decode parity across CPU & Metal (`F32` + `Bf16` padded protocols), and `337-MEASUREMENTS.md`.
- Note: Full end-to-end live-model GPU throughput wall-clock benchmarks are left for explicit operator hardware approval (`live_metal_tests::*`).

## Verification

- `timeout 900s cargo test -p vox-plugin-mens-candle-metal --lib`: `89 passed; 0 failed; 3 ignored` (`15.05s`).
