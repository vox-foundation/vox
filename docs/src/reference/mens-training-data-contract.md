# Mens training data (JSONL) contract

## Preflight (`preflight_train_jsonl`)

Before loading, native Candle QLoRA training runs `preflight_train_jsonl`:

- **No blank lines** — empty lines are errors (fail fast).
- **Line length cap** — default large cap (bytes); oversize lines error.
- **Non-empty file** required.

## Loading (`vox_tensor::data::load_all_with_policy`)

| Policy | Env | Behavior |
|--------|-----|----------|
| **Skip** (default) | (default) | Non-empty lines that are not valid `TrainingPair` JSON are **silently skipped** (`vox_tensor::data`). |
| **Fail fast** | `VOX_MENS_TRAIN_JSONL_STRICT=1` | First malformed non-empty line aborts with `InvalidData` and line context. |

Use **strict** in CI or when preparing golden corpora so silent data loss is visible.

## Mix / filter semantics

- **`min_rating`**: pairs below rating threshold are excluded after parse.
- **`--context-filter`**: retains only rows whose category contains the needle; **empty result** errors (`No training pairs found`).
- **In-loop skips** (short sequences, curriculum, etc.) are counted in training logs/telemetry; see Candle QLoRA training loop.
- **Lane metadata contract** (backward compatible):
  - optional `lane` (`vox_codegen`, `vox_docs_qa`, `vox_tooling`, `vox_speech`),
  - optional `response_mode` (`code_only`, `prose_only`),
  - optional `task_family` (freeform short tag).
  Missing fields are backfilled by corpus mix before write.
- **Default production lane policy**: code-only by default (`include_lanes: [vox_codegen]` in `mens/config/mix.yaml`).
  Docs QA/prose rows are excluded unless operators explicitly opt in.

## Related

- `docs/src/reference/mens-training.md` — tooling overview.
- `docs/src/operations/voxdb-cutover-runbook.md` — DB + telemetry sidecar rollout.
