---
phase: 06-model-routing-transparency-ml-dependency-health
plan: 03
subsystem: ci
tags: [cuda, nightly, ml-health, fail-closed]
requires: [06-02]
provides:
  - Fail-closed `ml-cuda-health` nightly job on GitHub-hosted `ubuntu-latest`
  - Workflow contract test pinning the job's compile-only, fail-closed shape
affects: [06-04]
completed: 2026-09-30
status: complete
---

# Phase 6 Plan 03 Summary

## Accomplishments

- Added the `ml-cuda-health` job to `.github/workflows/nightly.yml`: `ubuntu-latest`, 180-minute cap, `Jimver/cuda-toolkit@v0.2.36` with the CUDA 12.6.3 toolkit sub-package (12.4 has no Ubuntu 24.04 apt package; first hosted run failed on `cuda-12-4`), `CUDA_COMPUTE_CAP=80`, `nvcc --version` gate, then `cargo test --no-run -p vox-plugin-mens-candle-cuda --features cuda`, `vox ci cuda-features`, and `vox ci cuda-release-build`.
- Added `ml_cuda_health_is_hosted_compile_only_and_fail_closed` to `crates/vox-cli/tests/ci_workflow_contract.rs`, rejecting `continue-on-error`, `VOX_CI_ALLOW_CUDA_SKIP`, `--features cpu`, `if: false`, and `self-hosted` inside the job.
- The job is documented in-line as compile-only: it does not exercise a CUDA device or runtime kernels.

## Remote Evidence

- PASS: `ML CUDA dependency health` succeeded in https://github.com/vox-foundation/vox/actions/runs/36821415478 at head SHA `bc3214504f981e31817da3d9b4dbeae1b6cb0892`. Compile-only: no CUDA device or runtime kernels were exercised.

## Deviations

- The pre-existing `vox-corpus` incremental cache deadlocked rustc locally; verification ran with `CARGO_INCREMENTAL=0`.
