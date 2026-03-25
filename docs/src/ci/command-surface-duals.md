---
title: "Command surface duals (intentional)"
description: "Official documentation for Command surface duals (intentional) for the Vox language. Detailed technical reference, architecture guides, a"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Command surface duals (intentional)

Some behaviors exist in more than one place by design:

| Surface | Notes |
|---------|--------|
| **`vox ci cuda-features`** vs `scripts/check_cuda_feature_builds.sh` | Rust command is canonical; shell script is an optional thin delegate. |
| **`vox ci build-timings`** | Wall-clock **`cargo check`** for default `vox-cli`, GPU+stub, optional CUDA (when `nvcc` on `PATH` or via `CUDA_PATH`/`CUDA_HOME`), and with **`--crates`** extra per-crate lanes (`--json` supported). Soft budgets: `docs/ci/build-timings/budgets.json`; **`VOX_BUILD_TIMINGS_BUDGET_WARN`** / **`VOX_BUILD_TIMINGS_BUDGET_FAIL`**; pair **`latest.jsonl`** with **`snapshot-metadata.json`**. GitHub **`ci.yml`** runs **`build-timings --crates`**; no shell dual required. |
| **`vox ci toestub-scoped`** vs `vox stub-check`** vs `toestub` binary | CI uses **`vox ci toestub-scoped`** (fixed default root). **`vox stub-check`** is the interactive / full-flag path. The **`toestub`** crate binary remains for embedding. |
| **`vox run --mode script`** vs **`vox script`** | Same script runner; `vox script` exposes sandbox / cache / isolation flags explicitly. |
| **`vox schola train`** vs **`vox train`** | Canonical native training is **`vox schola train`**. **`vox train --provider local`** bails with the exact **`vox schola train --backend qlora …`** command (no `train_qlora.vox`). **`vox train --native`** remains a legacy Burn scratch path when built with **`mens-dei`**. |
| **`vox schola train-uv`** vs **`vox schola train --backend qlora`** | **`train-uv`** is **retired** (bails). Canonical QLoRA is **`vox schola train`**. |
| **`vox fabrica` / `vox mens` / `vox ars` / `vox recensio`** vs flat **`build`**, **`doctor`**, **`snippet`**, **`review`**, … | Same dispatch as the legacy top-level verbs; Latin names are **discoverability aliases** (see [`cli-design-rules.md`](../reference/cli.md)). |
| **`vox completions <shell>`** | Shell completion output (bash/zsh/fish/powershell/elvish); no script dual required. |

There is **no** `vox clean` subcommand; benchmarks and docs must not assume one — clear caches by deleting the relevant dirs (e.g. `~/.vox/script-cache*`) or use feature-specific tooling.
