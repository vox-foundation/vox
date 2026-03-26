# Scripts

Policy: **`vox ci`** is canonical; scripts here are optional thin delegates — see **`docs/src/ci/runner-contract.md`** and [`docs/agents/script-registry.json`](../docs/agents/script-registry.json).

**Mens gate (SSOT):** `vox ci mens-gate --profile <training|ci_full|m1m4>`. Use **`--isolated-runner`** (optional **`--gate-log-file`**) when the in-use `vox` binary would block the gate; **`scripts/populi/mens_gate_safe.ps1`** / **`mens_gate_safe.sh`** delegate to that and support **detach** for long agent runs. **CUDA release build + log:** `vox ci cuda-release-build` or **`scripts/populi/cursor_background_cuda_build.ps1`**.

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
| `populi/release_training_gate.sh` / `.ps1`, `populi/release_ci_full_gate.ps1`, `populi/mens_gate_safe.ps1`, `populi/mens_gate_safe.sh` | `vox ci mens-gate --profile training` / `ci_full` (use **`mens_gate_safe.* --detach`** when the gate would exceed tool timeouts) |
| `mens/release_training_gate.sh` / `.ps1` | Legacy forwards to `populi/release_training_gate.*` |
| `run_qwen25_qlora_real_4080.ps1` | Optional **operator** helper: CUDA (or CPU Candle) build + background **`vox mens train --backend qlora …`**. Canonical train path in scripts is **`vox mens train`**; **`vox schola train`** is the thin Schola entry (see `docs/src/architecture/mens-training-ssot.md`). |
| `populi/dogfood_qlora_cuda.ps1` | Dogfood QLoRA preset (**`vox mens train --background`** + `--log-dir`). |

Full inventory: **`docs/src/architecture/script-surface-audit.md`**.

Mens gate steps live in **`scripts/populi/gates.yaml`** (legacy `scripts/mens/gates.yaml`). **`ci_full`** is the broad CI profile: **`vox ci mens-gate --profile ci_full`**. **`m1m4`** / **`training`** are narrower profiles; PowerShell uses **`mens_gate_safe.ps1`** (isolated build + temp `vox.exe` on Windows).

**Doc inventory:** `vox ci doc-inventory generate` / `verify` — see **`docs/src/ci/doc-inventory-ssot.md`**.
