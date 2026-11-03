# Task 7 report

## Status

Implemented the host-aware Metal/CUDA serve-worker dispatch, Metal
characterization test, 100-pair local ChatML corpus, training contract,
macOS Metal how-to, and Darwin-only train-to-serve automation.

## Verification

- `cargo check -p vox-ml-cli --features gpu` — passed.
- `vox check scripts/mens-macos-metal-e2e.vox` — passed.
- JSONL validation — 100 valid `prompt`/`response` pairs.
- `cargo test -p vox-ml-cli --features 'gpu,execution-api' metal_capability_selects_metal_serve_plugin` — blocked by the pre-existing `handlers.rs:261` Axum type-inference error.

## Commit

Commit message: `feat(mens): Metal serve host dispatch and macos metal e2e scaffolding`

Commit SHA: recorded by `git rev-parse HEAD` after commit.

## Concerns

- The physical train/serve flow requires Apple Silicon, the installed
  `mens-candle-metal` plugin, a downloaded hub model, and a baseline path via
  `VOX_MENS_BASELINE` unless a collateral-pass report already exists.
- The ignored JSONL corpus is intentionally force-added because the repository
  ignores `*.jsonl`; it must remain named `dogfood-metal-e2e.jsonl` rather than
  `train.jsonl`.
- The worker test is gated on `execution-api`; the package-wide test command is
  currently blocked before test execution by the unrelated Axum error.
