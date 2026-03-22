# Reference: `vox` CLI (minimal compiler binary)

The **`vox`** executable is built from `crates/vox-cli` (repository root). This page documents the **commands that exist in that crate today**. Other markdown pages may describe a **broader future or workspace-wide toolchain** (Populi, review, MCP, etc.) — those are not necessarily linked into this binary yet.

## Global flags, completions, Latin groupings

- **Global (before subcommand):** **`--color auto|always|never`** (see `NO_COLOR`), **`--json`** (sets `VOX_CLI_GLOBAL_JSON` for subcommands that support machine JSON), **`--verbose` / `-v`** (if `RUST_LOG` is unset, tracing uses `debug`), **`--quiet` / `-q`** (`VOX_CLI_QUIET`).
- **Completions:** **`vox completions bash`** | **`zsh`** | **`fish`** | **`powershell`** | **`elvish`** — print to stdout and install per your shell (e.g. bash: `vox completions bash > /path/to/bash_completion.d/vox`).
- **Latin aliases (same behavior as flat commands):** **`vox fabrica`** (`fab`) — build/check/test/run/dev/bundle/fmt/script; **`vox mens`** — doctor, architect, stub-check; **`vox ars`** — snippet, share, skill, openclaw, ludus; **`vox recensio`** (`rec`, feature **`coderabbit`**) — same as **`vox review`**.

Design rules and registry parity: [`cli-design-rules-ssot.md`](../architecture/cli-design-rules-ssot.md), [`command-compliance-ssot.md`](../ci/command-compliance-ssot.md).

**Environment variables:** canonical names and precedence — [`reference/env-vars-ssot.md`](reference/env-vars-ssot.md) (alias: [`ref/env-vars-ssot.md`](ref/env-vars-ssot.md)).

## Build & run

### `vox build <file>`

Compile a `.vox` source file.

| Flag | Default | Description |
|------|---------|-------------|
| `-o`, `--out-dir` | `dist` | Directory for generated **TypeScript** (and related frontend files) |
| _(positional)_ | — | Path to the `.vox` file |

**Also writes** generated **Rust** under `target/generated/` (backend crate). If the module declares `@v0` UI components and output files are missing, the CLI may call **v0.dev** when `V0_API_KEY` is set.

### `vox island …` (feature `island`)

**Not in default builds.** `cargo build -p vox-cli --features island` (often add default stack: e.g. `--features island,populi-base` if you used `--no-default-features`).

| Subcommand | Role |
|------------|------|
| `generate <NAME> --prompt '…'` | Calls v0.dev (needs **`V0_API_KEY`**), writes `islands/src/<NAME>/<NAME>.component.tsx`, prints or injects an `@island` stub (`--target file.vox`). Cache: `~/.vox/island-cache/`; `--force` bypasses cache. |
| `upgrade <NAME> --prompt '…'` | Re-generates from existing TSX + instructions (always hits API). |
| `list` | Scans `islands/src/` and `Vox.toml [islands]` (`--json`). |
| `add <component>` | Runs `npx shadcn@latest add` in `islands/` (optional `--from` `.vox` path for `@shadcn` line). Kebab-case registry names get a **PascalCase** import alias (e.g. `dropdown-menu` → `DropdownMenu`). |
| `cache list \| clear \| remove <NAME>` | Manage the local island cache. |

**First run:** if **`islands/package.json`** is missing, `generate`, `upgrade`, `add`, and the build step **bootstrap** a minimal Vite + React tree under **`islands/`** (then **`pnpm install`** / **`pnpm run build`**). Requires **pnpm** on `PATH` (same as `vox run`’s frontend step). Use **`--no-build`** on generate/upgrade to skip the Vite build.

### `vox run <file> [-- <args>…]`

1. Runs the same pipeline as `build` (output to `dist/`).
2. If `.tsx` files are present under `dist/`, scaffolds a Vite app, runs **`pnpm install`** / **`pnpm run build`**, and copies assets into `target/generated/public/`.
3. Runs `cargo run -- <args>` in `target/generated`.

| Flag | Default | Description |
|------|---------|-------------|
| `--port` | _(from `VOX_PORT` or 3000)_ | Sets `VOX_PORT` for the generated Axum server and Vite `/api` proxy |
| `--mode` | `auto` | `app` = always generated server; `script` = `fn main()` script lane (**needs** `cargo build -p vox-cli --features script-execution`); `auto` = script lane when the file has no `@page` and the binary was built with `script-execution`. |

Backend listens on the port from **`VOX_PORT`** (or **3000**) — same variable the generated `main.rs` reads.

**pnpm workspace (repo root):** when the scaffold wrote **`pnpm-workspace.yaml`** at the repository root (for example **`islands/`** plus **`dist/.../app`**), run **`pnpm install`** once from that root so workspace packages link correctly, then use per-package **`pnpm run build`** / **`pnpm run dev`** as needed. See [tanstack-web-backlog.md](architecture/tanstack-web-backlog.md) Phase 3.

### `vox script <file> [-- <args>…]` (feature `script-execution`)

**Not in default builds.** Same script runner as `vox run --mode script`, with explicit flags: `--sandbox`, `--no-cache`, `--isolation`, `--trust-class`. Build: `cargo build -p vox-cli --features script-execution`.

When **`VOX_MESH_ENABLED=1`** and the binary is built with **`--features mesh`** (optionally combine with **`script-execution`**), `vox script` / script-mode `vox run` **best-effort** publishes a node record to the local registry file (see [mesh SSOT](../architecture/mesh-ssot.md)).

### `vox mesh …` (feature `mesh`)

**Not in default builds.** Local mesh registry introspection and a minimal HTTP control plane (join / list / heartbeat). Build: `cargo build -p vox-cli --features mesh`.

| Subcommand | Role |
|------------|------|
| `vox mesh status` | Print mesh env, on-disk registry path + nodes, and probed capabilities for this process (`--registry` override; `--json`). |
| `vox mesh serve` | Bind HTTP (`--bind 127.0.0.1:9847`); optional `--registry` seeds in-memory state from a JSON file. |

Interpreted **`vox workflow run`** / **`vox populi workflow run`** (journal + `mesh_*` activity hooks) requires **`--features workflow-runtime`** (implies `populi-dei` + `vox-workflow-runtime`). The runtime emits **`ActivityStarted` / `ActivityCompleted`** rows with **`activity_id`** (from `with { activity_id: … }` or a generated id). Mesh steps use **env-derived** `VOX_MESH_CONTROL_ADDR` / `Vox.toml` `[mesh]` only — use `with { mesh: "noop" | "join" | "snapshot" | "heartbeat" }` on `mesh_*` calls; see **`examples/mesh/workflow_mesh_demo.vox`**. Codex append is opt-in via **`VOX_WORKFLOW_JOURNAL_CODEX`** ([orchestration SSOT](../architecture/orchestration-unified-ssot.md)).

### `vox ci …`

Repository guards (manifest lockfile, docs/Codex SSOT, `vox-cli` feature matrix, doc inventory, workflow `scripts/` allowlist, Populi gate matrix, TOESTUB scoped scan, optional CUDA checks). **Canonical:** **`vox ci <subcommand>`** when `vox` is on `PATH`. **CI/bootstrap:** `cargo run -p vox-cli --quiet -- ci <subcommand>` from the repo root (same code path).

| Subcommand | Role |
|------------|------|
| `manifest` | `cargo metadata --locked` |
| `check-docs-ssot` / `check-codex-ssot` | Required doc / Codex files + inventory / OpenAPI checks |
| `doc-inventory generate \| verify` | Regenerate or verify `docs/agents/doc-inventory.json` (Rust; replaces retired Python scripts) |
| `feature-matrix` / `no-vox-dei-import` | `vox-cli` compile matrix + import guard |
| `workflow-scripts` | Fail if `.github/workflows/*.yml` references `scripts/…` not in `docs/agents/workflow-script-allowlist.txt` |
| `line-endings` | Forward-only: changed LF-policy files must not contain CR/CRLF (`*.ps1` exempt). Env: `GITHUB_BASE_SHA` / `GITHUB_SHA`, or `VOX_LINE_ENDINGS_BASE` (+ optional `VOX_LINE_ENDINGS_HEAD`). Flags: `--all`, `--base <ref>` |
| `populi-gate --profile ci_full \| m1m4 \| training` | Runs `scripts/populi/gates.yaml` steps |
| `toestub-scoped` | Default scan `crates/vox-repository` |
| `cuda-features` | Optional CUDA compile checks when `nvcc` exists |
| `build-timings` | Wall-clock `cargo check` lanes: default `vox-cli`, GPU+stub, optional CUDA when `nvcc` is on `PATH` or under `CUDA_PATH`/`CUDA_HOME`; **`--json`** one object per line; **`--crates`** adds `vox-cli --no-default-features`, `vox-db`, `vox-oratio`, `vox-populi --features train`, `vox-cli --features populi-oratio`. Budgets: `docs/ci/build-timings/budgets.json`; env `VOX_BUILD_TIMINGS_BUDGET_WARN` / `VOX_BUILD_TIMINGS_BUDGET_FAIL`; `SKIP_CUDA_FEATURE_CHECK=1` skips CUDA lane. |
| `grammar-drift` | Compare/update grammar fingerprint; `--emit github` / `--emit gitlab` for CI |
| `repo-guards` | TypeVar / `opencode` / stray-root file guards (GitLab parity) |
| `command-compliance` | Validates `contracts/cli/command-registry.yaml` (and schema) against `vox-cli` top-level commands, `ref-cli.md`, reachability SSOT, compilerd/dei RPC names, MCP tool registry, and script duals — blocks orphan CLI drift |

**Diagnostics:** `vox lock-report` remains separate (lock telemetry); it is **not** part of the `vox ci` surface.

### `vox dev <file>`

Watch mode: spawns **`vox-compilerd`** (JSON lines on stdio; one `DispatchRequest` per process), sends a `dev` request with `file`, `out_dir`, `port`, and `open`, then streams daemon output until exit or Ctrl+C. Resolve the daemon the same way as other compilerd tools: sibling to the `vox` executable, then `PATH`.

Build the daemon from this repo: `cargo build -p vox-cli --bin vox-compilerd` → `target/debug/vox-compilerd(.exe)` (install next to `vox` or add to `PATH`).

| Flag | Default | Description |
|------|---------|-------------|
| `-o`, `--out-dir` | `dist` | Build artifact directory |
| `--port` | `3000` | Dev server port (when applicable) |
| `--open` | `false` | Open browser when the daemon reports a URL |

### `vox live`

Terminal dashboard subscribed to an in-process `vox-orchestrator` event bus (demo / local use). **Not in default builds:** `cargo build -p vox-cli --features live` then run `vox live`.

Set **`VOX_ORCHESTRATOR_EVENT_LOG`** to a file path to tail the same JSONL stream **`vox-mcp`** appends when that variable is set (shared runtime view across MCP and CLI).

### `vox bundle <file>`

End-to-end **shipping** flow: build → scaffold `dist/app` (Vite + React) → `npm install` + `npm run build` → copy static assets → `cargo build` on the backend → copy the resulting binary into `dist/<stem>` (plus `.exe` on Windows when applicable).

| Flag | Default | Description |
|------|---------|-------------|
| `-o`, `--out-dir` | `dist` | TS/frontend codegen output (same as `build`) |
| `--target` | _(host)_ | Optional Rust target triple for cross-compile (`rustup target add` attempted) |
| `--release` | `true` | Release vs debug backend build |

If no TSX components are detected after build, stops after codegen (“backend-only”).

## Quality

### `vox check <file>`

Lex, parse, and type-check only. Prints diagnostics to stderr; exits with error if any **error**-severity diagnostic exists.

- `--emit-training-jsonl <PATH>`: append successful frontend records to JSONL for training corpus generation.

### `vox test <file>`

Runs `build`, then **`cargo test`** in `target/generated`.

### `vox fmt <file>`

**Placeholder.** The formatter is not wired to the current AST; the command performs a no-op read/write path check. Stderr notes this unless `VOX_SILENT_STUB_FMT=1`. Use `vox-fmt` directly in development if you are working on the formatter crate.

### `vox doctor`

Development environment checks (Rust/Cargo, Node/pnpm, Git, optional Docker/Podman, `Vox.toml`, Codex workspace registration, API keys, etc.).

| Build | Flags |
|-------|--------|
| **Default** | `--auto-heal`, `--test-health` |
| **`--features codex`** | Also `--build-perf`, `--scope`, `--json` (extended doctor in `commands::diagnostics::doctor`) |

Build: `cargo build -p vox-cli --features codex` for the extended path.

## Tooling

### `vox install <package_name>`

**Not implemented** in the shipped binary: exits with an error. Registry install is tracked for **`vox-pm`**.

### `vox db`

Local **VoxDB** inspection and research helpers (`crates/vox-cli/src/commands/db.rs`, `db_cli.rs`). Uses the same connection resolution as Codex (`VOX_DB_*`, compatibility `VOX_TURSO_*`, legacy `TURSO_*`, or local path).

Common subcommands: `status`, `schema`, `sample`, `migrate`, `export` / `import`, `vacuum`, `pref-get` / `pref-set` / `pref-list`, plus research flows (`research-ingest-url`, `research-list`, `capability-list`, …). Run `vox db --help` for the full tree.

### `vox scientia`

**Vox Scientia** — thin facade over the same Codex research / capability-map handlers as `vox db` (`capability-list`, `research-list`, `research-map-list`, `retrieval-status`, `research-refresh`). Use `vox scientia --help` for flags; connection resolution matches `vox db` (`VOX_DB_*`, …).

### `vox codex`

**Codex** (Turso / Arca) utilities backed by `vox-db`.

| Subcommand | Description |
|------------|-------------|
| `verify` | Prints `schema_version` (baseline **1**), manifest-derived reactivity table check, and legacy-chain flag |
| `export-legacy -o <file>` | Writes JSONL for legacy table set (see `vox_db::codex_legacy::LEGACY_EXPORT_TABLES`) |
| `import-legacy -i <file>` | Restores rows from that JSONL |
| `socrates-metrics [--repository-id <id>] [--limit N]` | Prints `SocratesSurfaceAggregate` JSON from recent `socrates_surface` `research_metrics` rows |
| `socrates-eval-snapshot --eval-id <id> [--repository-id <id>] [--limit N]` | Writes one `eval_runs` row via `VoxDb::record_socrates_eval_summary` (errors if no `socrates_surface` rows in window) |

Connection uses `DbConfig::resolve_standalone()` (`VOX_DB_*`, `VOX_TURSO_*`, legacy `TURSO_*`, or local path).

### `vox snippet` / `vox share`

Always available in the minimal binary. **`vox snippet`** — `save`, `search`, and `export` use the local Arca `CodeStore` (`VOX_TURSO_URL` / `VOX_TURSO_TOKEN` or `.vox/store.db`). **`vox share`** — `publish`, `search`, `list`, `review` against the same index.

### `vox skill` (feature `ars`)

**Not in default builds.** `cargo build -p vox-cli --features ars`. Subcommands mirror the ARS helpers: `list`, `install`, `uninstall`, `search`, `info`, `create`, `eval-task`, `promote`, `run`, `context-assemble`, `discover` (see `commands::extras::ars`).

### `vox ludus` (feature `extras-ludus`)

**Not in default builds.** `cargo build -p vox-cli --features extras-ludus`. Companions, quests, shop, arena, collegium, etc. (`commands::extras::ludus`).

### `vox stub-check` (feature `stub-check`)

**Not in default builds.** `cargo build -p vox-cli --features stub-check`. Runs **TOESTUB** (`vox-toestub`) over a directory tree, with optional **Codex** persistence (baselines, task queue, suppressions) and **Ludus** rewards on a clean run (`vox-ludus`).

| Argument / flag | Description |
|-----------------|-------------|
| `[PATH]` | Positional scan root (default `.` if omitted) |
| `-p`, `--path <PATH>` | Same as positional; mutually exclusive with `[PATH]` |
| `-f`, `--format <FMT>` | Output format (e.g. `terminal`, `json`, `markdown`) |
| `-s`, `--severity <LVL>` | Minimum severity: `info`, `warning`, `error`, `critical` |
| `--suggest-fixes` | Emit fix suggestions / task queue (default `true`) |
| `--rules <LIST>` | Comma-separated rule id prefixes |
| `--excludes <PATH>` | Repeatable exclude globs/paths |
| `--langs <LIST>` | Comma-separated languages (`rust`, `ts`, …) |
| `--baseline <NAME or FILE>` | Named baseline in VoxDB or path to a JSON file |
| `--save-baseline <NAME>` | Store current findings as a named baseline |
| `--task-list` | Print last saved task queue from VoxDB and exit |
| `--import-suppressions` | Import `toestub.toml` suppressions into VoxDB |
| `--ingest-findings <FILE>` | Ingest findings JSON into VoxDB task queue |
| `--fix-pipeline` / `--fix-pipeline-apply` | Staged doc/unwired fixes (apply = write) |
| `--gate <MODE>` / `--gate-budget-path <PATH>` | CI warning budget / ratchet |
| `--verify-impacted`, `--max-escalation`, `--self-heal-safe-mode` | Reserved / advanced hooks |

**CI / parity:** prefer **`vox ci toestub-scoped`** (default scan root `crates/vox-repository`) — same policy surface as GitHub Actions. Use **`vox stub-check …`** for interactive or repo-wide scans when you need clap flags (format, baselines, Ludus, etc.). Optional thin shell: `scripts/quality/toestub_scoped.sh` delegates to `vox ci toestub-scoped`; the standalone **`toestub`** crate binary remains available for advanced tooling.

### `vox architect` (features `stub-check` or `codex`)

**Not in default builds.** Requires `cargo build -p vox-cli --features stub-check` and/or `--features codex` (same feature gates as `commands::diagnostics`). Subcommands: **`check`** (workspace layout vs `vox-schema.json`), **`fix-sprawl`** (`--apply` to move misplaced crates), **`analyze`** (optional path, default `.` — god-object scan via TOESTUB; **needs `--features stub-check`**; with `codex` only, the command is available but **`analyze` exits with a hint to add `stub-check`**). Implementation: `crates/vox-cli/src/commands/diagnostics/tools/architect.rs`.

### `vox openclaw` (feature `ars`)

**Not in default builds.** Build with `cargo build -p vox-cli --features ars`, then run `vox openclaw` (alias `oc`). Talks to an OpenClaw- or ClawHub-compatible HTTP gateway (`VOX_OPENCLAW_URL`, optional `VOX_OPENCLAW_TOKEN`). Subcommands include `import`, `list-remote`, `config`, MCP-backed `approvals` / `approve` / `deny`, and gateway helpers (`serve` expects a `vox-gateway` binary on `PATH`).

### `vox lsp`

Spawns the **`vox-lsp`** binary (from the `vox-lsp` crate) with stdio inherited. Ensure `vox-lsp` is on `PATH` (e.g. `cargo build -p vox-lsp` and use `target/debug`).

## Populi / DeI (feature-gated)

**Doc parity (`vox ci command-compliance`):** **`vox populi corpus`**, **`vox populi pipeline`**, **`vox populi status`**, **`vox populi plan`**, **`vox populi eval-gate`**, **`vox populi bench-completion`**, **`vox populi system-prompt-template`**, **`vox populi train`**, **`vox populi oratio`**, **`vox populi serve`**, **`vox populi probe`**, **`vox populi merge-weights`**, **`vox populi merge-qlora`**, **`vox populi eval-local`**.

With default features (**`populi-base` only** — corpus + `vox-runtime`, **no** Oratio / `vox-oratio` and **no** native training deps), **`vox populi`** covers corpus / pipeline / status / plan / eval-gate / bench-completion / system templates / etc. **`vox populi oratio`** and **`vox ai oratio`** require **`--features populi-oratio`** (STT stack). **Native train** / **probe** / **merge-weights** / **eval-local** (Burn + Candle) require **`cargo build -p vox-cli --features gpu`** (alias **`populi-qlora`**). For **Candle QLoRA on NVIDIA** with linked CUDA kernels, use **`cargo vox-cuda-release`** (workspace alias → `gpu,populi-candle-cuda`; see `.cargo/config.toml`). Optional: **`vox-populi`** binary (same as `vox populi …`, inserts the subcommand) — `cargo build -p vox-cli --features populi-base`; add **`populi-oratio`** on the same build for Oratio. See [vox-cli build feature inventory](../architecture/vox-cli-build-feature-inventory.md). **`vox populi pipeline`** runs the dogfood corpus → eval → optional native train stages (replaces heavy orchestration in `scripts/run_populi_pipeline.ps1`). **`vox populi serve`** (HTTP completions) is **not** in the default feature set — build with **`cargo build -p vox-cli --features execution-api`** (see `crates/vox-cli/Cargo.toml`). **`serve`** loads **Burn** LoRA `*.bin` or merged **`model_merged.bin`** (`merge-weights`); it does **not** load Candle **`merge-qlora`** f32 safetensor outputs. Corpus lives under **`vox populi corpus`** (e.g. `extract`, `validate`, `pairs`, **`mix`**, `eval`).

- **`vox populi train`** — native Populi training (contract/planner inside `vox-populi`). **`--backend lora`** (default): Burn + wgpu LoRA; **`--tokenizer vox`** (default) or **`--tokenizer hf`** with **GPT-2-shaped** HF `config.json` + optional **HF embed warm-start** from safetensors. **`--backend qlora`**: Candle + **qlora-rs** — **NF4 frozen base** linear(s) + trainable LoRA; **mmap `f32`** for context embeddings (`wte` / `model.embed_tokens`). When all per-layer **output-projection** weights exist in shards, trains a **sequential stack** + LM head; else **LM-head-only**. **`--qlora-no-double-quant`** turns off qlora-rs **double quant** of scales (default: on). **`--qlora-require-full-proxy-stack`** fails preflight if expected middle projection keys are missing from shards (strict prod gate). **`--qlora-lm-head-only`** skips the middle `o_proj` stack even when shards are complete (stable CE on some CUDA dogfood paths; conflicts with **`--qlora-require-full-proxy-stack`**). **`--qlora-proxy-max-layers N`** caps stacked middle projections for ablation (`0` = LM-head-only; conflicts with **`--qlora-lm-head-only`** when `N > 0`). **`--qlora-ce-last-k K`** (default **1**) applies next-token CE on the last **K** positions per JSONL row (bounded by **`seq_len`** and **64**). In-tree **qlora-rs** `training_step_lm`: pre-norm residual middles with **`1/√depth`** per block and again before the LM head. **`--qlora-max-skip-rate <0..=1>`** aborts training when skipped JSONL rows exceed the fraction per epoch. **`--log-dir DIR`** re-spawns **`vox populi train`** in the background with a timestamped log (parent returns immediately — avoids IDE/agent wall-clock timeouts; tail the log). **`--background`** lowers process priority and caps VRAM fraction for long runs. Same **`--device`** story; **CUDA** / **Metal** with **`populi-candle-cuda`** / **`populi-candle-metal`**. QLoRA needs **`--tokenizer hf`**, **`--model`**, HF safetensors + **`tokenizer.json`**. **`--deployment-target mobile_edge`** or **`--preset mobile_edge`**: planner gates for edge export + **`--device cpu`** required. See [`architecture/populi-training-ssot.md`](architecture/populi-training-ssot.md), [`architecture/mobile-edge-ai-ssot.md`](architecture/mobile-edge-ai-ssot.md), [`hf-finetune-capability-matrix.md`](architecture/hf-finetune-capability-matrix.md). Python QLoRA: **`vox train`** / `train_qlora.vox` with **`--features populi-dei`**.
- **`vox populi merge-weights`** — merges a **Burn** LoRA checkpoint (`*.bin`) into **`model_merged.bin`** (`gpu` only). Does **not** apply Candle qlora adapter tensors.
- **`vox populi merge-qlora`** (alias **`merge-adapter`**) — merges **`candle_qlora_adapter.safetensors`** + sidecar meta (**v2** `candle_qlora_adapter_meta.json` or **v3** `populi_adapter_manifest_v3.json`) into **f32** base shards (subset); **`*.bin`** Burn checkpoints are **rejected** (use **`merge-weights`**). See SSOT merge table.
- **`vox populi oratio`** — transcribe via **`vox-oratio`** (**Candle Whisper**, Rust + HF weights; not whisper.cpp). Build CLI with **`--features populi-oratio`**. Env: `VOX_ORATIO_MODEL`, `VOX_ORATIO_REVISION`, `VOX_ORATIO_LANGUAGE`, etc. HTTP: run **`cargo run -p vox-codex-api --bin vox-codex-dashboard`** for the small Codex + Oratio API (**`GET /api/audio/status`**, **`POST /api/audio/transcribe`** with JSON `{"path":"…"}`; relative paths use `VOX_ORATIO_WORKSPACE` or CWD). Bind with **`VOX_DASH_HOST`** / **`VOX_DASH_PORT`** (default `127.0.0.1:3847`).
- **Vox source (`Speech.transcribe`)** — builtin module **`Speech`**: **`Speech.transcribe(path: str) → Result[str]`** uses Oratio and returns **refined** text (`display_text()`). Generated Rust crates depend on **`vox-oratio`** via codegen `Cargo.toml`.
- **Corpus mix `asr_refine`** — in mix YAML, set `record_format: asr_refine` on a source whose JSONL lines match **`populi/schemas/asr_refine_pairs.schema.json`** (`noisy_text` / `corrected_text`); output lines are **`prompt`/`response`** JSON for `train.jsonl`.
- **Corpus mix `tool_trace`** — set `record_format: tool_trace` for JSONL lines shaped like **`ToolTraceRecord`** in `vox-corpus` (`task_prompt`, `tool_name`, `arguments_json`, `result_json`, `success`, optional `followup_text`); schema **`populi/schemas/tool_trace_record.schema.json`**, example lines **`populi/data/tool_traces.example.jsonl`**. Emitted rows use **`category`: `tool_trace`** for **`--context-filter tool_trace`** during training.

- **`--features populi-dei`**: enables **`vox train`** (local provider **bails** with the canonical **`vox populi train --backend qlora …`** command; Together API; **`--native`** Burn scratch) and `vox populi` surfaces that call **`vox-dei-d`** (generate, review, workflow, check, fix). RPC **method names** are centralized in [`crates/vox-cli/src/dei_daemon.rs`](../../crates/vox-cli/src/dei_daemon.rs) (`crate::dei_daemon::method::*`) so CLI and daemon stay aligned. **`vox populi review`** uses `ai.review`; it does **not** embed the old TOESTUB/Fabrica/CodeRabbit tree.
- **`--features coderabbit`**: enables **`vox review coderabbit`** — GitHub/CodeRabbit batch flows in Rust (`crates/vox-cli/src/commands/review/coderabbit/`). Build: `cargo build -p vox-cli --features coderabbit` (often pair with `populi-base` if you omit default features: `--no-default-features --features coderabbit,populi-base`). Set **`GITHUB_TOKEN`** or **`GH_TOKEN`**.

### `vox review coderabbit` (feature `coderabbit`)

Splits local changes into concern-based PRs with a **real baseline** (`origin/<default>` → `cr-baseline-*`) and **git worktrees** under **`.coderabbit/worktrees/`** so the main working tree is not checked out per chunk. **Plan-only** (default): writes **`.coderabbit-semantic-manifest.json`**. **Execute**: add **`--execute`** (pushes baseline, opens PRs into baseline, writes **`.coderabbit/run-state.json`** for resume). Before opening worktree PRs, **`semantic-submit --execute`** re-scans the dirty tree and **aborts with `[drift]`** if the changed-file set no longer matches the plan (replan without `--resume`). The drift check **ignores** paths the command itself creates as untracked files (**`.coderabbit-semantic-manifest.json`**, **`.coderabbit/run-state.json`**) so they do not false-trigger drift.

| Step | Command |
|------|---------|
| Dry-run / plan | `vox review coderabbit semantic-submit` |
| Apply | `vox review coderabbit semantic-submit --execute` |
| Resume after failure | **`--resume`** reuses baseline from **`.coderabbit/run-state.json`** if you omit **`--baseline-branch`**; or pass **`--baseline-branch`** that matches the saved baseline. **`--force-chunks`** redo all chunks. |
| Legacy “commit everything to default branch” | **`--commit-main`** (broad `git add -u` — use only if intentional) |
| Size batches from `git diff` | Plan: `vox review coderabbit batch-submit`. Write manifest: **`batch-submit --execute`**. Caps are **clamped to the selected tier** (`--tier` or `Vox.toml`, default Pro). |
| Full-repo stacked planner (orphan baseline, mutates checkout) | Plan + manifest: `vox review coderabbit stack-submit`. Live: **`stack-submit --execute`**. **`max_files_per_pr`** is tier-clamped; on failure the tool **restores your original branch** when possible. Prefer **`semantic-submit`**. |
| Single PR from current branch | `vox review coderabbit submit` (still does checkout/`git add -A` in-repo — avoid on dirty trees) |
| Ingest / tasks | `vox review coderabbit ingest <pr>` [`-o file`] / `vox review coderabbit tasks <pr> --format markdown` |
| Wait for bot review | `vox review coderabbit wait <pr> [--timeout-secs N]` |

**Manifest files (when written)**

| Subcommand | Plan-only | With `--execute` |
|------------|-----------|------------------|
| `semantic-submit` | `.coderabbit-semantic-manifest.json` | same + git/PR actions |
| `batch-submit` | console only | `.coderabbit-batch-manifest.json` |
| `stack-submit` | `.coderabbit-stack-manifest.json` (always) | same + git/PR actions |

**`Vox.toml`** — optional **`[review.coderabbit]`**: `tier`, `delay_between_prs_secs`, `max_files_per_pr`, **`exclude_prefixes`** (path prefixes, forward slashes) to drop noise paths from semantic/batch/stack planning.

**Git hygiene**: `.gitignore` includes **`.coderabbit/worktrees/`**. You may commit **`.coderabbit/run-state.json`** if you want a shared run map (or keep it local). **Ignored in drift/planning (normalized repo-relative paths, including leading `./`)**: anything under **`.coderabbit/`** (local tooling, worktrees). Chunk worktree overlays **do not recurse into `.coderabbit/`** when copying from the main tree, so nested tool dirs are not duplicated.
- **`--features dashboard`**: reserved **no-op** in `vox-cli`. The old **`vox populi` chat / agent / dei / learn** commands are removed from the CLI surface (they depended on workspace-excluded `vox-dei`). Use **`vox-codex-dashboard`** / the VS Code extension for dashboard-style surfaces.
- **`VOX_BENCHMARK=1`**: after training paths that invoke it, runs **`vox populi eval-local`** (requires `gpu`) using `VOX_BENCHMARK_MODEL` / `VOX_BENCHMARK_DIR` when set.

## Related docs

- **Rustdoc / layout**: [`docs/src/api/vox-cli.md`](api/vox-cli.md)
- **Ecosystem narrative** (may include commands beyond this binary): [`how-to-cli-ecosystem.md`](how-to-cli-ecosystem.md)
- **Compiler pipeline** (HIR path): [`reference/compiler-internals.md`](reference/compiler-internals.md)
