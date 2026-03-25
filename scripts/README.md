# Scripts

Policy: **`vox ci`** is canonical; scripts here are optional thin delegates — see **`docs/src/ci/runner-contract.md`** and [`docs/agents/script-registry.json`](../docs/agents/script-registry.json).

| Script | Prefer |
|--------|--------|
| `check_docs_ssot.sh` / `.ps1` | `vox ci check-docs-ssot` |
| `check_codex_ssot.sh` / `.ps1` | `vox ci check-codex-ssot` |
| `verify_workspace_manifest.sh` | `vox ci manifest` |
| `check_vox_cli_feature_matrix.sh` | `vox ci feature-matrix` |
| `check_vox_cli_no_vox_dei.sh` | `vox ci no-vox-dei-import` |
| `check_cuda_feature_builds.sh` | `vox ci cuda-features` |
| `examples_strict_parse.sh` / `.ps1` | Optional: `VOX_EXAMPLES_STRICT_PARSE=1 cargo test -p vox-parser --test parity_test` (see `examples/PARSE_STATUS.md`) |
| _(no shell twin)_ | `vox ci line-endings` (forward-only LF policy; Rust implementation) |
| `quality/toestub_scoped.sh` | `vox ci toestub-scoped [ROOT]` |
| `populi_release_gate.sh` / `.ps1` (legacy wrappers) | **`vox ci mens-gate --profile m1m4`** |
| `mens/release_training_gate.sh` / `.ps1` | `vox ci mens-gate --profile training` |
| `run_qwen25_qlora_real_4080.ps1` | Optional **operator** helper: CUDA (or CPU Candle) build + background **`vox schola train --backend qlora …`**; canonical command is still **`vox schola train`** (see `docs/src/architecture/mens-training-ssot.md`). |

Mens gate steps live in **`scripts/mens/gates.yaml`**. **`ci_full`** (run in GitHub `ci.yml` after `cargo test --workspace`) is the broad CI profile: **`vox ci mens-gate --profile ci_full`**. **`m1m4`** / **`training`** are narrower release/training profiles used by the shell wrappers above.

**Doc inventory:** `vox ci doc-inventory generate` / `verify` — see **`docs/src/ci/doc-inventory-ssot.md`**.
