---
title: "CI runner contract"
description: "Official documentation for CI runner contract for the Vox language. Detailed technical reference, architecture guides, and implementation"
category: "reference"
last_updated: "2026-05-05"
training_eligible: true

schema_type: "TechArticle"
---

# CI runner contract

## Self-hosted labels (default)

| Profile | `runs-on` |
|---------|-----------|
| Basic Linux | `[self-hosted, linux, x64]` |
| Docker / Buildx | `[self-hosted, linux, x64, docker]` |
| Playwright / browser | `[self-hosted, linux, x64, browser]` |

## GitHub-hosted exceptions

Use `ubuntu-latest`, `windows-latest`, or `macos-latest` only where documented — see [GitHub-hosted exceptions](github-hosted-exceptions.md).

## Workspace root manifest (fix forward)

Do **not** depend on git history to recover the root `Cargo.toml`. SSOT and repair steps: [workspace root manifest](workspace-root-manifest.md). Verify resolution with **`vox ci manifest`** (CI runs this via `cargo run -p vox-cli --quiet -- ci manifest`).

## Agent / local terminal vs CI shell

- **CI jobs** in this repository are largely **Linux self-hosted** and use **`bash`** for workflow steps unless a job sets `shell: pwsh` (see individual workflows). That is a runner convenience, not a contradiction of contributor policy.
- **Local work and coding agents** should prefer **[PowerShell 7 (`pwsh`)](https://github.com/PowerShell/PowerShell)** on **any OS** when it is installed, consistent with [`AGENTS.md`](../../../AGENTS.md) and machine-checked terminal policy (`vox shell check`, [`contracts/terminal/exec-policy.v1.yaml`](../../../contracts/terminal/exec-policy.v1.yaml)).

## Canonical `vox ci` vs shell scripts

Guard logic lives in **`vox ci`** (`crates/vox-cli/src/commands/ci`). Shell scripts under `scripts/` are **optional thin delegates** for local POSIX ergonomics; **prefer `vox ci …`** when the `vox` binary is on `PATH`. Mapping table: [scripts/README.md](../adr/index.md). Machine-readable registry: [`docs/agents/script-registry.json`](../../agents/script-registry.json).

## Pre-push validation (Linux CI mirror)

Use **`vox ci pre-push`** to run the merge-blocking subset locally
(fmt-check, clippy, ssot-drift, line-endings, doc-inventory verify, scoped
TOESTUB). See [local CI parity](../contributors/local-ci-pre-push.md).

## Line endings (cross-platform)

- **Policy:** LF for tracked source/docs/config (see root [`.gitattributes`](../../../.gitattributes) and [`.editorconfig`](../../../.editorconfig)). **`*.ps1`** uses CRLF on checkout / in editors that respect EditorConfig.
- **CI gate:** **`vox ci line-endings`** — forward-only by default (diff vs `GITHUB_BASE_SHA`…`GITHUB_SHA` in GitHub Actions, else `HEAD~1`…`HEAD` locally). Audit whole tree with **`--all`**. Override base with **`VOX_LINE_ENDINGS_BASE`** or **`--base <ref>`** (optional **`VOX_LINE_ENDINGS_HEAD`**, default `HEAD`).
- **TOESTUB:** rule id **`cross-platform/line-endings`** / finding **`cross-platform/crlf`** (warning) on scanned languages — see [governance](../../agents/governance.md).

**ML / repo hygiene (Rust, not shell):**

- **`vox ci grammar-export-check`** — wired in the default **`.github/workflows/ci.yml`** Linux job after the CLI feature matrix; asserts grammar exports are non-empty (EBNF/GBNF/Lark/JSON-Schema).
- **`vox ci grammar-drift`** — SHA-256 of the EBNF export vs `mens/data/grammar_fingerprint.txt` (and Populi twin); updates the file when drift is detected. The **`ml_data_extraction.yml`** workflow runs this with **`--emit github`**. Use **`--emit github`** (stdout: `drift=true|false` only, for `GITHUB_OUTPUT`) or **`--emit gitlab`** (writes `drift.env` in the repo root) when wiring other pipelines.
- **`vox ci repo-guards`** — replaces ad-hoc `grep`/`find` blocks: no `TypeVar(0)` in **`vox-codegen-rust` / `vox-codegen-ts` sources** (typechecker uses that sentinel legitimately), filtered `opencode` references under `crates/`, and no stray root clutter files (same policy as the former GitLab `guards` job).

## Build timings (wall-clock `cargo check`)

**Canonical:** **`vox ci build-timings`** — prints duration for `cargo check -p vox-cli` (default features) and `cargo check -p vox-cli --features gpu,mens-qlora,stub-check`, plus an optional CUDA lane when `nvcc` is available (**`PATH`** or **`CUDA_PATH`** / **`CUDA_HOME`** pointing at the toolkit root; same skip rules as `cuda-features`). Use **`--json`** for one JSON object per line. **`--crates`** adds isolated `cargo check` lanes for `vox-cli --no-default-features`, `vox-db`, `vox-oratio`, `vox-populi --features mens-train`, and **`vox-cli --features oratio`** (see [crate-build-lanes migration](../archive/research-2026-q1/crate-build-lanes-migration.md)). Soft budgets: `docs/ci/build-timings/budgets.json`; optional env **`VOX_BUILD_TIMINGS_BUDGET_WARN=1`** (stderr when a lane exceeds its soft max) and **`VOX_BUILD_TIMINGS_BUDGET_FAIL=1`** (fail the command after successful checks — use only with tuned budgets). Pair committed **`latest.jsonl`** with **`docs/ci/build-timings/snapshot-metadata.json`** (`rustc` / host / CUDA / cache note). Skip CUDA lane when **`SKIP_CUDA_FEATURE_CHECK=1`**. GitHub `ci.yml` runs **`build-timings --crates`**. See [vox-cli build feature inventory](../archive/research-2026-q1/vox-cli-build-feature-inventory.md).

## Optional CUDA compile gate

**Canonical:** **`vox ci cuda-features`** (wired in GitHub `ci.yml`). It **no-ops** when `nvcc` is absent (common on CPU-only self-hosted runners). When `nvcc` is on `PATH`, it runs:

- `cargo check -p vox-oratio --features cuda` — typechecks Oratio's `#[cfg(feature = "cuda")]` paths.
- `cargo check -p vox-cli --features gpu,mens-candle-cuda` — typechecks Mens Candle qlora with CUDA.

Thin delegate: `scripts/check_cuda_feature_builds.sh` (optional POSIX wrapper around the same checks). Local escape hatch (e.g. Windows with CUDA installed but no MSVC host for `nvcc`): `SKIP_CUDA_FEATURE_CHECK=1 vox ci cuda-features` or the same env with `bash scripts/check_cuda_feature_builds.sh`. On PowerShell, use `bash -c 'export SKIP_CUDA_FEATURE_CHECK=1; ./scripts/check_cuda_feature_builds.sh'` so the variable reaches Bash.

## GPU / CUDA runner profile

Workflow jobs that run **`vox ci cuda-features`** or compile with **`nvcc`** should use the **Docker** self-hosted profile (`[self-hosted, linux, x64, docker]`) when the job image must supply CUDA toolchains. CPU-only `cargo check` lanes stay on the basic Linux profile (`[self-hosted, linux, x64]`). Keep workflow `runs-on` **explicit per job** (do not hide runner choice behind reusable-only defaults).

## Optional: strict parse for all examples

Set **`VOX_EXAMPLES_STRICT_PARSE=1`** when running **`cargo test -p vox-parser --test parity_test`** to require every `examples/**/*.vox` to parse. Default CI keeps the **golden-only** gate. Status: [`examples/PARSE_STATUS.md`](../../../examples/PARSE_STATUS.md). Delegates: [`scripts/examples_strict_parse.sh`](../../../scripts/verify_workspace_manifest.sh), [`scripts/examples_strict_parse.ps1`](../../../scripts/check_docs_ssot.ps1).

## Test hangs: `cargo test` vs `cargo nextest`

Rust’s built-in harness (**`cargo test`**) does **not** enforce per-test timeouts. After ~60 seconds it may print *“has been running for over 60 seconds”* — that is only a **warning**; the test keeps running until it finishes or you interrupt it.

**`cargo nextest run`** (used in GitHub `ci.yml` and `.gitlab-ci.yml`) reads **`.config/nextest.toml`**. There, **`slow-timeout`** marks slow tests and, with **`terminate-after`**, ends a stuck test after roughly **`terminate-after × period`** wall time (see [nextest slow tests](https://nexte.st/docs/features/slow-tests/)). The **`global-timeout`** setting caps the **entire** test run duration for a binary, not each case.

For local debugging of a single crate, prefer:

```bash
cargo nextest run -p vox-mcp --profile ci
```

Individual async tests can still wrap work in **`tokio::time::timeout`** so plain **`cargo test`** fails instead of hanging indefinitely.

## Workflow list

See [workflow enumeration](workflow-enumeration.md).

