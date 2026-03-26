---
title: "cli"
description: "Documentation for cli.md"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---



<!-- Merged from ref-cli.md -->

---
title: "Reference: `vox` CLI (minimal compiler binary)"
description: "Official documentation for Reference: `vox` CLI (minimal compiler binary) for the Vox language. Detailed technical reference, architectur"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Reference: `vox` CLI (minimal compiler binary)

The **`vox`** executable is built from `crates/vox-cli` (repository root). This page documents the **commands that exist in that crate today**. Other markdown pages may describe a **broader future or workspace-wide toolchain** (Mens, review, MCP, etc.) — those are not necessarily linked into this binary yet.

## Global flags, completions, Latin groupings

- **Global (before subcommand):** **`--color auto|always|never`** (see `NO_COLOR`), **`--json`** (sets `VOX_CLI_GLOBAL_JSON` for subcommands that support machine JSON), **`--verbose` / `-v`** (if `RUST_LOG` is unset, tracing uses `debug`), **`--quiet` / `-q`** (`VOX_CLI_QUIET`).
- **Completions:** **`vox completions bash`** | **`zsh`** | **`fish`** | **`powershell`** | **`elvish`** — print to stdout and install per your shell (e.g. bash: `vox completions bash > /path/to/bash_completion.d/vox`).
- **Dynamic command catalog:** **`vox commands`** — clap-derived list from the actual compiled binary; add `--recommended` for first-time essentials or `--format json --include-nested` for tooling.
- **Secrets namespace:** **`vox clavis`** (alias **`vox secrets`**) centralizes token health checks and credential compatibility storage.
- **Latin aliases (same behavior as flat commands):** **`vox fabrica`** (`fab`) — build/check/test/run/dev/bundle/fmt/script; **`vox diag`** — doctor, architect, stub-check; **`vox ars`** — snippet, share, skill, openclaw, ludus; **`vox recensio`** (`rec`, feature **`coderabbit`**) — same as **`vox review`**.

Design rules and registry parity: [`cli-design-rules-ssot.md`](../architecture/cli-design-rules-ssot.md), [`command-compliance.md`](command-compliance.md). Generated command table: [`cli-command-surface.generated.md`](cli-command-surface.generated.md) (`vox ci command-sync --write`).

**Environment variables:** canonical names and precedence — [`reference/env-vars.md`](env-vars.md) (alias: [`ref/env-vars.md`](../ref/env-vars.md)).

## Build & run

### `vox build <file>`

Compile a `.vox` source file.

| Flag | Default | Description |
|------|---------|-------------|
| `-o`, `--out-dir` | `dist` | Directory for generated **TypeScript** (and related frontend files) |
| _(positional)_ | — | Path to the `.vox` file |

**Also writes** generated **Rust** under `target/generated/` (backend crate). If the module declares `@v0` UI components and output files are missing, the CLI may call **v0.dev** when `V0_API_KEY` is set.

### `vox island …` (feature `island`)

**Not in default builds.** `cargo build -p vox-cli --features island` (often add default stack: e.g. `--features island,mens-base` if you used `--no-default-features`).

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

**pnpm workspace (repo root):** when the scaffold wrote **`pnpm-workspace.yaml`** at the repository root (for example **`islands/`** plus **`dist/.../app`**), run **`pnpm install`** once from that root so workspace packages link correctly, then use per-package **`pnpm run build`** / **`pnpm run dev`** as needed. See [tanstack-web-backlog.md](../architecture/tanstack-web-backlog.md) Phase 3.

### `vox script <file> [-- <args>…]` (feature `script-execution`)

**Not in default builds.** Same script runner as `vox run --mode script`, with explicit flags: `--sandbox`, `--no-cache`, `--isolation`, `--trust-class`. Build: `cargo build -p vox-cli --features script-execution`.

When **`VOX_MESH_ENABLED=1`** and the binary is built with **`--features populi`** (pulls in `vox-populi`; optionally combine with **`script-execution`**), `vox script` / script-mode `vox run` **best-effort** publishes a node record to the local registry file (see [mens SSOT](populi.md)).

### `vox populi …` (feature `populi`)

**Not in default builds.** One-command private mesh lifecycle helpers backed by the same Populi control plane. Build: `cargo build -p vox-cli --features populi`.

| Subcommand | Role |
|------------|------|
| `vox populi up` | Bootstraps a private populi config (`.vox/populi/mesh.env`), generates `VOX_MESH_TOKEN` + `VOX_MESH_SCOPE_ID` by default, and starts `vox populi serve` in the background. Supports `--mode lan|overlay`, `--bind`, and `--insecure-local` for dev-only LAN use. |
| `vox populi down` | Stops the background control-plane process recorded in `.vox/populi/mesh-state.json`. |
| `vox populi status` | Shows control-plane health (`/health`), token/scope posture, and overlay diagnostics (tailscale/wireguard/tunnel availability/connection hints). |
| `vox populi registry-snapshot` | Print local env and on-disk registry path + nodes (`--registry` override; `--json`; alias: `local-status`). |
| `vox populi serve` | Bind HTTP (`--bind 127.0.0.1:9847`); optional `--registry` seeds in-memory state from a JSON file. |

Interpreted **`vox mens workflow run`** (journal + `mesh_*` activity hooks; there is no top-level `vox workflow`) requires **`--features workflow-runtime`** (implies `mens-dei` + `vox-workflow-runtime`). The runtime emits **`ActivityStarted` / `ActivityCompleted`** rows with **`activity_id`** (from `with { activity_id: … }` or a generated id). Mens steps use **env-derived** `VOX_MESH_CONTROL_ADDR` / `Vox.toml` `[mens]` only — use `with { mens: "noop" | "join" | "snapshot" | "heartbeat" }` on `mesh_*` calls; see **`examples/mens/workflow_mesh_demo.vox`**. Codex append is opt-in via **`VOX_WORKFLOW_JOURNAL_CODEX`** ([orchestration SSOT](orchestration-unified.md)).

### `vox ci …`

Repository guards (manifest lockfile, docs/Codex SSOT, `vox-cli` feature matrix, doc inventory, milestone eval matrix contract, workflow `scripts/` allowlist, Mens gate matrix, TOESTUB scoped scan, optional CUDA checks). **Canonical:** **`vox ci <subcommand>`** when `vox` is on `PATH`. **CI/bootstrap:** `cargo run -p vox-cli --quiet -- ci <subcommand>` from the repo root (same code path).

| Subcommand | Role |
|------------|------|
| `manifest` | `cargo metadata --locked` |
| `check-docs-ssot` / `check-codex-ssot` | Required doc / Codex files + inventory / OpenAPI checks |
| `doc-inventory generate \| verify` | Regenerate or verify `docs/agents/doc-inventory.json` (Rust; replaces retired Python scripts) |
| `eval-matrix verify` | Validates `contracts/eval/benchmark-matrix.json` against `contracts/eval/benchmark-matrix.schema.json` (M1–M5 milestones; `benchmark_classes` ids are a fixed enum in the schema) |
| `eval-matrix run [--milestone <id>]` | Runs `cargo` checks/tests mapped from each `benchmark_classes` entry (deduped); always re-runs `verify` first |
| `feature-matrix` / `no-dei-import` | `vox-cli` compile matrix + import guard (alias: `no-vox-dei-import`) |
| `workflow-scripts` | Fail if `.github/workflows/*.yml` references `scripts/…` not in `docs/agents/workflow-script-allowlist.txt` |
| `line-endings` | Forward-only: changed LF-policy files must not contain CR/CRLF (`*.ps1` exempt). Env: `GITHUB_BASE_SHA` / `GITHUB_SHA`, or `VOX_LINE_ENDINGS_BASE` (+ optional `VOX_LINE_ENDINGS_HEAD`). Flags: `--all`, `--base <ref>` |
| `mesh-gate --profile ci_full \| m1m4 \| training` | Runs `scripts/populi/gates.yaml` steps (CLI falls back to `scripts/mens/gates.yaml` if present). **`--isolated-runner`** builds `vox-cli` under `target/mens-gate-safe` (override `--gate-build-target-dir`), copies `vox` to a temp path, and re-invokes the gate (**Windows + Unix**; avoids file locks). Hidden alias: `--windows-isolated-runner`. Legacy argv alias: `mens-gate`. Optional `--gate-log-file <path>` tees child output. |
| `toestub-self-apply` | `cargo build -p vox-toestub --release` then full-repo `toestub` scan (replaces `scripts/toestub_self_apply.*`) |
| `toestub-scoped` | Default scan `crates/vox-repository` |
| `scaling-audit verify \| emit-reports` | Scaling SSOT: validate `contracts/scaling/policy.yaml`; `emit-reports` regenerates per-crate backlog markdown + rollup + TOESTUB JSON on `crates/` |
| `cuda-features` | Optional CUDA compile checks when `nvcc` exists |
| `cuda-release-build` | `cargo build -p vox-cli --bin vox --release --features gpu,mens-candle-cuda` with tee to `mens/runs/logs/cuda_build_<UTC>.log` (same intent as workspace alias **`cargo vox-cuda-release`** / `scripts/populi/cursor_background_cuda_build.ps1`; needs nvcc + MSVC toolchain on Windows) |
| `build-timings` | Wall-clock `cargo check` lanes: default `vox-cli`, GPU+stub, optional CUDA when `nvcc` is on `PATH` or under `CUDA_PATH`/`CUDA_HOME`; **`--json`** one object per line; **`--crates`** adds `vox-cli --no-default-features`, `vox-db`, `vox-oratio`, `vox-populi --features mens-train`, `vox-cli --features oratio`. Budgets: `docs/ci/build-timings/budgets.json`; env `VOX_BUILD_TIMINGS_BUDGET_WARN` / `VOX_BUILD_TIMINGS_BUDGET_FAIL`; `SKIP_CUDA_FEATURE_CHECK=1` skips CUDA lane. |
| `grammar-drift` | Compare/update grammar fingerprint; `--emit github` / `--emit gitlab` for CI |
| `repo-guards` | TypeVar / `opencode` / stray-root file guards (GitLab parity) |
| `secret-env-guard [--all]` | Fails if Rust files add direct managed-secret env reads outside allowed modules (default changed-files; `--all` scans all crates). |
| `clavis-parity` | Verifies Clavis managed secret names are synchronized with `docs/src/reference/clavis-ssot.md`. |
| `release-build --target <triple> [--version <tag>] [--out-dir dist] [--package vox\|bootstrap\|both]` | Build and package allowlisted release artifacts (`cargo build --locked --release`): `vox`, `vox-bootstrap`, or both. Unix archives are `.tar.gz`; Windows archives are `.zip`. Writes `checksums.txt` with one line per artifact (`<sha256>` + two spaces + `<basename>`). Contract: [`docs/src/ci/binary-release-contract.md`](../ci/binary-release-contract.md) |
| `command-compliance` | Validates `contracts/cli/command-registry.yaml` (and schema) against `vox-cli` top-level commands, CLI reference (`docs/src/reference/cli.md` or legacy `ref-cli.md`), reachability SSOT, compilerd/dei RPC names, MCP tool registry, and script duals — blocks orphan CLI drift |
| `command-sync [--write]` | Regenerates or verifies [`cli-command-surface.generated.md`](cli-command-surface.generated.md) from the registry (run `--write` after editing `command-registry.yaml`) |
| `contracts-index` | Validates `contracts/index.yaml` against `contracts/index.schema.json` and checks every listed contract path exists |
| `scientia-worthiness-contract` | Validates `contracts/scientia/publication-worthiness.default.yaml` against `publication-worthiness.schema.json` and publisher invariants (weights sum, threshold ordering) |
| `ssot-drift` | Runs `check-docs-ssot`, `check-codex-ssot`, `sql-surface-guard --all`, `command-compliance`, `contracts-index`, and `scientia-worthiness-contract` in one pass |

### `vox clavis` (alias `vox secrets`)

Centralized secret diagnostics and compatibility credential storage.

| Subcommand | Role |
|------------|------|
| `vox clavis status --workflow chat\|mcp\|publish\|review\|db-remote\|mens-mesh --profile dev\|ci\|mobile\|prod --mode auto\|local\|cloud [--bundle minimal-local-dev\|minimal-cloud-dev\|gpu-cloud\|publish-review]` | Prints active-mode blocking vs optional secret readiness using requirement groups and optional bundle checks (alias: `vox clavis doctor …`). |
| `vox clavis set <registry> <token> [--username <name>]` | Stores a registry token in `~/.vox/auth.json` through the Clavis API. |
| `vox clavis get <registry>` | Reads and prints redacted token status from Clavis resolution sources. |
| `vox clavis backend-status` | Prints backend mode (`env_only`/`infisical`/`vault`/`auto`) and backend availability diagnostics. |
| `vox clavis migrate-auth-store` | Migrates plaintext `auth.json` tokens to secure local store and leaves compatibility sentinels in JSON. |

### Deprecated compatibility commands

- `vox login [--registry <name>] [<token>] [--username <name>]` — compatibility shim for older workflows; prefer `vox clavis set`.
- `vox logout [--registry <name>]` — compatibility shim; prefer `vox clavis` commands.

**Diagnostics:** `vox lock-report` remains separate (lock telemetry); it is **not** part of the `vox ci` surface.

### `vox commands`

Generate a dynamic command catalog from clap (`VoxCliRoot::command()`), so the list always matches what this binary actually exposes.

Why this exists: it is the discoverability source for first-timers, editor integrations, and docs/CI parity checks.

| Flag | Default | Description |
|------|---------|-------------|
| `--format text\|json` | `text` | Human table output or machine JSON |
| `--recommended` | `false` | Show only first-time starter commands |
| `--include-nested` | `false` | Include nested subcommands (`vox ci …`, `vox mens …`) |

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

End-to-end **shipping** flow: build → scaffold `dist/app` (Vite + React) → **`pnpm install`** + **`pnpm run build`** → copy static assets → `cargo build` on the backend → copy the resulting binary into `dist/<stem>` (plus `.exe` on Windows when applicable).

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

**Canonical path (English):** `vox doctor …` — this is the primary spelling in docs, scripts, and muscle memory.

**Grouped Latin path:** `vox diag doctor …` — identical behavior; `diag` is the **registry `latin_ns`** bucket for diagnostics (see [Nomenclature migration map](../architecture/nomenclature-migration-map.md#latin_ns-command-registry-group-labels)). Prefer `vox doctor` in new prose; use `vox diag doctor` when teaching the Latin lane.

Development environment checks (Rust/Cargo, Node/pnpm, Git, optional Docker/Podman, `Vox.toml`, Codex workspace registration, API keys, etc.). With **`VOX_WEB_TS_OUT`** set to your **`vox build`** TypeScript output directory, doctor also verifies **`@v0`** components use **named** exports for TanStack **`routes:`** (see [`env-vars.md`](env-vars.md#web--vite--tanstack-codegen)).

| Build | Flags |
|-------|--------|
| **Default** | `--auto-heal`, `--test-health`, **`--probe`** (OCI healthcheck: exit non-zero if any default check fails; no banner) |
| **`--features codex`** | Also `--build-perf`, `--scope`, `--json` (extended doctor in `commands::diagnostics::doctor`) |

Build: `cargo build -p vox-cli --features codex` for the extended path.

## Tooling

### `vox install <package_name>`

**Not implemented** in the shipped binary: exits with an error. Registry install is tracked for **`vox-pm`**.

### `vox db`

Local **VoxDB** inspection and research helpers (`crates/vox-cli/src/commands/db.rs`, `db_cli.rs`). Uses the same connection resolution as Codex (`VOX_DB_*`, compatibility `VOX_TURSO_*`, legacy `TURSO_*`, or local path).

`vox db audit` prints read-only JSON to stdout: schema version, database paths, select storage `PRAGMA`s, and per-user-table row counts. Add `--timestamps` for heuristic `MIN`/`MAX` on a chosen time-like column per table (extra queries).

`vox db prune-plan` prints JSON counts for rows older than policy thresholds (`contracts/db/retention-policy.yaml`). `vox db prune-apply --i-understand` runs matching `DELETE`s.

Common subcommands: `status`, `audit`, `schema`, `sample`, `migrate`, `export` / `import`, `vacuum`, `pref-get` / `pref-set` / `pref-list`, plus research flows (`research-ingest-url`, `research-list`, `capability-list`, …). Publication operator controls: `publication-route-simulate`, `publication-publish`, and `publication-retry-failed` accept **`--json`** for structured stdout. **`publication-publish`** enforces the same live gate as other surfaces when `--dry-run` is off: VoxDb with two digest approvers and `VOX_NEWS_PUBLISH_ARMED=1` (or orchestrator publish_armed is not read by this path); successful live runs update manifest state to `published` / `publish_failed` like MCP/orchestrator. Run `vox db --help` for the full tree.

### `vox scientia`

**Typing / ergonomics:** Publication subcommands are **long on purpose**—they are stable for scripting and match [`command-registry.yaml`](../../contracts/cli/command-registry.yaml) / `vox ci command-compliance`. Mitigations: **`vox completions <shell>`** (tab-complete partial subcommand paths); repeat operators may use shell aliases or wrappers. There is no separate Latin umbrella for `scientia` today; use English **`vox scientia …`** only.

**Vox Scientia** — facade over Codex research and publication workflows.

- Research/capability helpers: `capability-list`, `research-list`, `research-map-list`, `retrieval-status`, `research-refresh`.
- Scientific publication lifecycle:
  - `vox scientia publication-prepare --publication-id <id> --author <name> --title <title> [--scholarly-metadata-json <file>] [--preflight] [--preflight-profile default|double-blind] <path.md>`
  - `vox scientia publication-prepare-validated` (same flags as prepare except preflight is always on)
  - `vox scientia publication-preflight --publication-id <id> [--profile default|double-blind] [--with-worthiness]`
  - `vox scientia publication-zenodo-metadata --publication-id <id>` (stdout JSON for Zenodo deposit metadata; no HTTP)
  - `vox scientia publication-openreview-profile --publication-id <id>` (stdout JSON: merged OpenReview invitation/signature/readers + API base; no HTTP)
  - `vox scientia publication-worthiness-evaluate [--contract-yaml <path>] --metrics-json <path>` (stdout worthiness decision JSON from repo contract + metrics file; no DB)
  - `vox scientia publication-approve --publication-id <id> --approver <identity>`
  - `vox scientia publication-submit-local --publication-id <id>`
  - `vox scientia publication-status --publication-id <id>`
  - `vox scientia publication-scholarly-remote-status --publication-id <id> [--external-submission-id <id>]` (poll remote scholarly repository / deposit state for a stored submission)
  - `vox scientia publication-scholarly-remote-status-sync-all --publication-id <id>` (poll remote status for every `scholarly_submissions` row on that publication)
  - `vox scientia publication-scholarly-remote-status-sync-batch [--limit <n>] [--iterations <n>] [--interval-secs <s>] [--max-runtime-secs <s>] [--jitter-secs <s>]` (batch sync across publications ranked by recent submission activity; optional bounded loop for supervised workers)
  - `vox scientia publication-scholarly-staging-export --publication-id <id> --output-dir <dir> --venue zenodo|open-review|arxiv-assist` (write venue-scoped scholarly staging artifacts under `output-dir` and validate layout; Zenodo adds `zenodo.json`, arXiv assist adds `arxiv_handoff.json`, **`main.tex`** stub, and `arxiv_bundle.tar.gz`; mirrors `vox db publication-scholarly-staging-export`)
  - `vox scientia publication-scholarly-pipeline-run --publication-id <id> [--preflight-profile default|double-blind|metadata-complete] [--dry-run] [--staging-output-dir <dir> --venue zenodo|open-review|arxiv-assist] [--adapter <kind>] [--json]` (preflight → dual-approval gate → optional staging export → scholarly submit unless `--dry-run`; `--json` = compact single-line JSON on stdout; mirrors `vox db publication-scholarly-pipeline-run`)
  - `vox scientia publication-arxiv-handoff-record --publication-id <id> --stage <staging-exported|…|published> [--operator <id>] [--note <text>] [--arxiv-id <id>]` (append-only operator milestone for arXiv assist; `published` requires `--arxiv-id`)
  - `vox scientia publication-external-jobs-due [--limit <n>]` (list external submission jobs due for retry/tick)
  - `vox scientia publication-external-jobs-dead-letter [--limit <n>]` (list terminal `failed` external submission jobs)
  - `vox scientia publication-external-jobs-replay --job-id <id>` (requeue one dead-letter job to `queued`)
  - `vox scientia publication-external-jobs-tick [--limit <n>] [--lock-ttl-ms <ms>] [--lock-owner <id>] [--iterations <n>] [--interval-secs <s>] [--max-runtime-secs <s>] [--jitter-secs <s>]` (advance external submission worker queue; optional repeated ticks)
  - `vox scientia publication-external-pipeline-metrics [--since-hours <h>]` (read-only JSON rollup: jobs, attempts, snapshots, scholarly rows, `publication_attempts` by channel; mirrors `vox db publication-external-pipeline-metrics`)

Connection resolution matches `vox db` (`VOX_DB_*`, …). The publication flow uses digest-bound dual approvals before scholarly submission.
For architecture/lingo and multi-platform routing internals, see `docs/src/architecture/voxgiantia-publication-architecture.md`.

### `vox codex`

**Codex** (Turso / Arca) utilities backed by `vox-db`.

`vox codex cutover` automates legacy-chain migration: exports JSONL + a JSON sidecar, creates a new local SQLite file at `--target-db`, imports, and prints the `VOX_DB_PATH` you should export next. Requires a **local** legacy file (`--source-db` or configured `VOX_DB_PATH`). Use `--force` only after backing up an existing target path.

| Subcommand | Description |
|------------|-------------|
| `verify` | Prints `schema_version` (baseline **1**), manifest-derived reactivity table check, and legacy-chain flag |
| `export-legacy -o <file>` | Writes JSONL for legacy table set (see `vox_db::codex_legacy::LEGACY_EXPORT_TABLES`) |
| `import-legacy -i <file>` | Restores rows from that JSONL (clears allowlisted tables on the target, then inserts; for fresh baselines only) |
| `cutover --target-db <new.db> [--source-db <old.db>] [--artifact-dir <dir>] [--force]` | Export + fresh target + import + `codex-cutover-*.{jsonl,sidecar.json}` artifacts |
| `import-orchestrator-memory --dir <dir> --agent-id <id> [--session-id <s>]` | One `memories` row per top-level `*.md` |
| `import-skill-bundle --file <bundle.json>` | JSON `{ id, version, manifest_json, skill_md }` → `skill_manifests` |
| `socrates-metrics [--repository-id <id>] [--limit N]` | Prints `SocratesSurfaceAggregate` JSON from recent `socrates_surface` `research_metrics` rows |
| `socrates-eval-snapshot --eval-id <id> [--repository-id <id>] [--limit N]` | Writes one `eval_runs` row via `VoxDb::record_socrates_eval_summary` (errors if no `socrates_surface` rows in window) |

Connection uses `DbConfig::resolve_standalone()` (`VOX_DB_*`, `VOX_TURSO_*`, legacy `TURSO_*`, or local path).

Always available in the minimal binary. **`vox snippet`** — `save`, `search`, and `export` use the local Codex database (`VOX_DB_URL` / `VOX_DB_TOKEN` or `.vox/store.db`). **`vox share`** — `publish`, `search`, `list`, `review` against the same index.

### `vox skill` (feature `ars`)

**Not in default builds.** `cargo build -p vox-cli --features ars`. Subcommands mirror the ARS helpers: `list`, `install`, `uninstall`, `search`, `info`, `create`, `eval-task`, `promote`, `run`, `context-assemble`, `discover` (see `commands::extras::ars`).

### `vox ludus` (feature `extras-ludus`)

**Not in default builds.** `cargo build -p vox-cli --features extras-ludus`. Companions, quests, shop, arena, collegium, etc. (`commands::extras::ludus`). Terminal HUD: **`vox ludus hud`** requires **`--features ludus-hud`** (implies `extras-ludus` + `vox-orchestrator`).

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

**`toestub` binary (crate `vox-toestub`):** besides `--mode`, `--format`, `--canary-crates`, and `--suppressions`, the rollout surface includes **`--tests-mode`** (`off` \| `include` \| `strict`, default `off` — skips noisy unresolved-ref under `.../tests/...` when `off`), **`--prelude-allowlist`** (JSON per `contracts/toestub/prelude-allowlist.v1.json`), and **`--feature-flags`** (comma-separated, e.g. `unwired-graph`, `scaling-fs-heuristic-fallback`).

### `vox architect` (features `stub-check` or `codex`)

**Not in default builds.** Requires `cargo build -p vox-cli --features stub-check` and/or `--features codex` (same feature gates as `commands::diagnostics`). Subcommands: **`check`** (workspace layout vs `vox-schema.json`), **`fix-sprawl`** (`--apply` to move misplaced crates), **`analyze`** (optional path, default `.` — god-object scan via TOESTUB; **needs `--features stub-check`**; with `codex` only, the command is available but **`analyze` exits with a hint to add `stub-check`**). Implementation: `crates/vox-cli/src/commands/diagnostics/tools/architect.rs`.

### `vox openclaw` (feature `ars`)

**Not in default builds.** Build with `cargo build -p vox-cli --features ars`, then run `vox openclaw` (alias `oc`). Talks to an OpenClaw- or ClawHub-compatible HTTP gateway (`VOX_OPENCLAW_URL`, optional `VOX_OPENCLAW_TOKEN`). Subcommands include `import`, `list-remote`, `config`, MCP-backed `approvals` / `approve` / `deny`, and gateway helpers (`serve` expects a `vox-gateway` binary on `PATH`).

### `vox lsp`

Spawns the **`vox-lsp`** binary (from the `vox-lsp` crate) with stdio inherited. Ensure `vox-lsp` is on `PATH` (e.g. `cargo build -p vox-lsp` and use `target/debug`).

## Mens / DeI (feature-gated)

**Doc parity (`vox ci command-compliance`):** **`vox mens corpus`**, **`vox mens pipeline`**, **`vox mens status`**, **`vox mens watch-telemetry`** (alias **`vox mens watch`**; tails stderr + training JSONL ~3s), **`vox mens plan`**, **`vox mens eval-gate`**, **`vox mens bench-completion`**, **`vox mens system-prompt-template`**, **`vox mens train`** (GPU / Candle QLoRA; same intent as **`vox-mens` shim** (`vox mens …`)), **`vox oratio`**, **`vox mens serve`**, **`vox mens probe`**, **`vox mens merge-weights`**, **`vox mens merge-qlora`**, **`vox mens eval-local`**.

With default features (**`mens-base` only** — corpus + `vox-runtime`, **no** Oratio / `vox-oratio` and **no** native training deps), **`vox mens`** covers corpus / pipeline / status / plan / eval-gate / bench-completion / system templates / etc. **`vox oratio`** (alias **`vox speech`**) requires **`--features oratio`** (STT stack; separate from the **`mens`** command tree). **Native train** / **serve** / **probe** / **merge-weights** / **merge-qlora** / **eval-local** (Burn + Candle) require **`cargo build -p vox-cli --features gpu`** (alias **`mens-qlora`**). For **Candle QLoRA on NVIDIA** with linked CUDA kernels, use **`cargo vox-cuda-release`** (workspace alias → `gpu,mens-candle-cuda`; see `.cargo/config.toml`). Optional: **`vox-mens`** shim binary inserts the **`mens`** subcommand for argv ergonomics — use **`vox oratio`** for speech. `cargo build -p vox-cli --features mens-base`; add **`oratio`** on the same build for Oratio. See [vox-cli build feature inventory](../architecture/vox-cli-build-feature-inventory.md). **`vox mens pipeline`** runs the dogfood corpus → eval → optional native train stages (replaces heavy orchestration in `scripts/run_mens_pipeline.ps1`). **`vox mens serve`** (HTTP/OpenAI-compatible API) requires **`gpu`** (Axum/control-plane pieces may additionally need **`execution-api`** for other REST surfaces — see `crates/vox-cli/Cargo.toml`). **`serve`** loads **Burn** LoRA `*.bin` or merged **`model_merged.bin`** (`merge-weights`); it does **not** load Candle **`merge-qlora`** f32 safetensor outputs. Corpus lives under **`vox mens corpus`** (e.g. `extract`, `validate`, `pairs`, **`mix`**, `eval`).

- **`vox mens train`** — native Mens training (contract/planner inside **`vox-populi`** (`mens::tensor`); use **`vox-mens`** argv shim when you want the binary that inserts `mens`). **`--backend lora`** (default): Burn + wgpu LoRA; **`--tokenizer vox`** (default) or **`--tokenizer hf`** with **GPT-2-shaped** HF `config.json` + optional **HF embed warm-start** from safetensors. **`--backend qlora`**: Candle + **qlora-rs** — **NF4 frozen base** linear(s) + trainable LoRA; **mmap `f32`** for context embeddings (`wte` / `model.embed_tokens`). When all per-layer **output-projection** weights exist in shards, trains a **sequential stack** + LM head; else **LM-head-only**. **`--qlora-no-double-quant`** turns off qlora-rs **double quant** of scales (default: on). **`--qlora-require-full-proxy-stack`** fails preflight if expected middle projection keys are missing from shards (strict prod gate). **`--qlora-lm-head-only`** skips the middle `o_proj` stack even when shards are complete (stable CE on some CUDA dogfood paths; conflicts with **`--qlora-require-full-proxy-stack`**). **`--qlora-proxy-max-layers N`** caps stacked middle projections for ablation (`0` = LM-head-only; conflicts with **`--qlora-lm-head-only`** when `N > 0`). **`--qlora-ce-last-k K`** (default **1**) applies next-token CE on the last **K** positions per JSONL row (bounded by **`seq_len`** and **64**). In-tree **qlora-rs** `training_step_lm`: pre-norm residual middles with **`1/√depth`** per block and again before the LM head. **`--qlora-max-skip-rate <0..=1>`** aborts training when skipped JSONL rows exceed the fraction per epoch. **`--log-dir DIR`** re-spawns training in the background with a timestamped log (parent returns immediately — avoids IDE/agent wall-clock timeouts; tail the log). **`--background`** lowers process priority and caps VRAM fraction for long runs. Same **`--device`** story; **CUDA** / **Metal** with **`mens-candle-cuda`** / **`mens-candle-metal`**. QLoRA needs **`--tokenizer hf`**, **`--model`**, HF safetensors + **`tokenizer.json`**. **`--deployment-target mobile_edge`** or **`--preset mobile_edge`**: planner gates for edge export + **`--device cpu`** required. See [`reference/mens-training.md`](mens-training.md), [`reference/mobile-edge-ai.md`](mobile-edge-ai.md), [`hf-finetune-capability-matrix.md`](../architecture/hf-finetune-capability-matrix.md). Python QLoRA: **`vox train`** / `train_qlora.vox` with **`--features mens-dei`**.
- **`vox mens merge-weights`** — merges a **Burn** LoRA checkpoint (`*.bin`) into **`model_merged.bin`** (`gpu` only). Does **not** apply Candle qlora adapter tensors.
- **`vox mens merge-qlora`** (alias **`merge-adapter`**) — merges **`candle_qlora_adapter.safetensors`** + sidecar meta (**v2** `candle_qlora_adapter_meta.json` or **v3** `populi_adapter_manifest_v3.json`) into **f32** base shards (subset); **`*.bin`** Burn checkpoints are **rejected** (use **`merge-weights`**). See SSOT merge table.
- **`vox oratio`** (alias **`vox speech`**) — transcribe via **`vox-oratio`** (**Candle Whisper**, Rust + HF weights; not whisper.cpp). Build CLI with **`--features oratio`**. Includes `transcribe`, `status`, and sessionized `listen` (Enter-or-timeout gate, correction profile, route mode). Env: `VOX_ORATIO_MODEL`, `VOX_ORATIO_REVISION`, `VOX_ORATIO_LANGUAGE`, etc. HTTP: run **`cargo run -p vox-codex-api --bin vox-codex-dashboard`** for the small Codex + Oratio API (**`GET /api/audio/status`**, **`POST /api/audio/transcribe`** with JSON `{"path":"…"}`; relative paths use `VOX_ORATIO_WORKSPACE` or CWD). Bind with **`VOX_DASH_HOST`** / **`VOX_DASH_PORT`** (default `127.0.0.1:3847`).
- **Vox source (`Speech.transcribe`)** — builtin module **`Speech`**: **`Speech.transcribe(path: str) → Result[str]`** uses Oratio and returns **refined** text (`display_text()`). Generated Rust crates depend on **`vox-oratio`** via codegen `Cargo.toml`.
- **Corpus mix `asr_refine`** — in mix YAML, set `record_format: asr_refine` on a source whose JSONL lines match **`mens/schemas/asr_refine_pairs.schema.json`** (`noisy_text` / `corrected_text`); output lines are **`prompt`/`response`** JSON for `train.jsonl`.
- **Corpus mix `tool_trace`** — set `record_format: tool_trace` for JSONL lines shaped like **`ToolTraceRecord`** in `vox-corpus` (`task_prompt`, `tool_name`, `arguments_json`, `result_json`, `success`, optional `followup_text`); schema **`mens/schemas/tool_trace_record.schema.json`**, example lines **`mens/data/tool_traces.example.jsonl`**. Emitted rows use **`category`: `tool_trace`** for **`--context-filter tool_trace`** during training.

- **`--features mens-dei`**: enables **`vox train`** (local provider **bails** with the canonical **`vox mens train --backend qlora …`** command; Together API; **`--native`** Burn scratch) and `vox mens` surfaces that call **`vox-dei-d`** (generate, review, workflow, check, fix). RPC **method names** are centralized in [`crates/vox-cli/src/dei_daemon.rs`](../../../crates/vox-cli/src/dei_daemon.rs) (`crate::dei_daemon::method::*`) so CLI and daemon stay aligned. **`vox mens review`** uses `ai.review`; it does **not** embed the old TOESTUB/Fabrica/CodeRabbit tree.
- **`--features dei`**: **`vox dei`** (alias **`vox orchestrator`**) — DEI orchestrator CLI (`commands::dei`); build with `cargo build -p vox-cli --features dei`.
- **`--features coderabbit`**: enables **`vox review coderabbit`** — GitHub/CodeRabbit batch flows in Rust (`crates/vox-cli/src/commands/review/coderabbit/`). Build: `cargo build -p vox-cli --features coderabbit` (often pair with `mens-base` if you omit default features: `--no-default-features --features coderabbit,mens-base`). Set **`GITHUB_TOKEN`** or **`GH_TOKEN`**.

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
- **`--features dashboard`**: reserved **no-op** in `vox-cli`. The old **`vox mens` chat / agent / dei / learn** commands are removed from the CLI surface (they depended on workspace-excluded `vox-dei`). Use **`vox-codex-dashboard`** / the VS Code extension for dashboard-style surfaces.
- **`VOX_BENCHMARK=1`**: after training paths that invoke it, runs **`vox mens eval-local`** (requires `gpu`) using `VOX_BENCHMARK_MODEL` / `VOX_BENCHMARK_DIR` when set.

## Related docs

- **Rustdoc / layout**: [`docs/src/api/vox-cli.md`](#)
- **Ecosystem narrative** (may include commands beyond this binary): [`how-to-cli-ecosystem.md`](../how-to/how-to-cli-ecosystem.md)
- **Compiler pipeline** (HIR path): [`reference/compiler-internals.md`](#)


<!-- Merged from vox-cli.md -->

---
title: "Crate: `vox-cli`"
description: "Official documentation for Crate: `vox-cli` for the Vox language. Detailed technical reference, architecture guides, and implementation p"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Crate: `vox-cli`

Rust package path: **`crates/vox-cli`**. Produces the **`vox`** binary (`src/main.rs`) and **`vox-compilerd`** (`src/bin/vox-compilerd.rs`, stdio JSON dispatcher for `dev` and compiler-subcommand RPC).

## Scope

This checkout’s `vox-cli` is a **minimal** compiler driver: clap dispatch, codegen orchestration, and a small set of subcommands. It does **not** yet expose the full Mens / review / MCP / `vox init` surface that appears in some older generated docs.

Authoritative **user-facing** command list: [`reference/cli.md`](cli.md).

## Subcommands → source

| CLI | Module |
|-----|--------|
| `vox build` | `src/commands/build.rs` |
| `vox check` | `src/commands/check.rs` |
| `vox test` | `src/commands/test.rs` |
| `vox run` | `src/commands/run.rs` |
| `vox bundle` | `src/commands/bundle.rs` |
| `vox fmt` | `src/commands/fmt.rs` |
| `vox install` | `src/commands/install.rs` |
| `vox lsp` | `src/commands/lsp.rs` |
| `vox architect` | `src/commands/diagnostics/tools/architect.rs` (features **`codex`** and/or **`stub-check`**) |

**Library / dispatch modules (not always exposed as `vox` subcommands):** `src/commands/info.rs` (registry metadata), `src/commands/runtime/**` (extended run/dev/info/tree/shell). Inline script execution (`runtime/run/{script,backend,sandbox}`) builds with **`--features script-execution`**; Axum Mens inference server (`commands/ai/serve`) builds with **`--features execution-api`** (implies `script-execution` + `gpu` + Axum + `vox-corpus` validation helpers).

## Shared modules

| Path | Role |
|------|------|
| `src/pipeline.rs` | Shared lex → parse → typecheck → HIR frontend (prefer for new commands) |
| `src/config.rs` | `VOX_PORT` / `default_port()`, `set_process_vox_port` (compilerd + `vox run --port`) |
| `src/templates.rs` | Embedded Vite/React scaffold strings for `bundle` / `run` |
| `src/fs_utils.rs` | Directory helpers, `resolve_vox_runtime_path`, script-cache GC |
| `src/dispatch_protocol.rs` | JSON line types shared by `dispatch.rs` and `compilerd` |
| `src/dei_daemon.rs` | Stable **`vox-dei-d`** RPC method ids + `call()` wrapper (spawn error hints) |
| `src/dispatch.rs` | Spawn `vox-compilerd` / named daemons, stream responses; `DAEMON_SPAWN_FAILED_PREFIX` for consistent spawn-failure text (`dei_daemon` enriches errors) |
| `src/compilerd.rs` | In-process stdio RPC implementation for `vox-compilerd` |
| `src/watcher.rs` | `notify` watch helper for `compilerd` `dev` rebuilds |
| `src/v0.rs` | Optional v0.dev API integration for `@v0` components (`V0_API_KEY`) |

## Library target

`src/lib.rs` owns the `Cli` parser, `run_vox_cli()`, and shared modules; `src/main.rs` only initializes tracing and calls `run_vox_cli()`.

## Build

```bash
cargo build -p vox-cli
# binaries: target/debug/vox(.exe), target/debug/vox-compilerd(.exe)
```

Install from the repo:

```bash
cargo install --path crates/vox-cli
```


<!-- Merged from cli-design-rules.md -->

---
title: "CLI design rules"
description: "Official documentation for CLI design rules for the Vox language. Detailed technical reference, architecture guides, and implementation p"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# CLI design rules

Single source for **shipped `vox` CLI** conventions (see also [`reference/cli.md`](cli.md), [`cli-scope-policy.md`](../architecture/cli-scope-policy.md), [`cli-reachability.md`](cli.md)).

## Hierarchy and naming

- **One primary tree** of nouns/verbs; avoid near-synonyms (`update` vs `upgrade`) for the same action.
- **Latin-themed group commands** (`fabrica`, `mens`, `ars`, `recensio`) mirror the flat top-level commands for discoverability; legacy top-level names remain **active** (not hidden).
- **Subcommand depth** should stay ≤ 2 for most flows; deeper trees only for dense domains (e.g. `mens corpus`).
- **Retired / deprecated** commands stay in the registry with `status` and doc’d migration (see [`command-surface-duals.md`](../ci/command-surface-duals.md)).

## Help, output, and exit codes

- Every subcommand supports **`--help`**; root supports **`--version`** (via clap on `VoxCliRoot`).
- **Machine-readable / JSON** output belongs on **stdout** where a command documents it; **diagnostics and errors** on **stderr**.
- Prefer **`--json`**, **`--quiet`**, **`--verbose`** on subcommands that emit structured or noisy output; root sets hints via env (`VOX_CLI_GLOBAL_JSON`, `VOX_CLI_QUIET`) when using global flags.
- **Non-zero exits** must mean something actionable (document in help where non-obvious).

## Description style standard

Use one canonical command description in clap for each command, then reuse it in docs/editor surfaces.

- **What**: one sentence describing the operation.
- **Why/When**: one short phrase for first-time guidance when non-obvious.
- Keep wording stable so `vox commands` output, docs tables, and editor quick-picks do not drift.

## Global flags (root)

- **`--color auto|always|never`** — forwarded to `vox_cli::diagnostics` (`NO_COLOR` still wins when set).
- **`--json`** — sets `VOX_CLI_GLOBAL_JSON=1` for subcommands that honor it.
- **`--verbose` / `-v`** — if `RUST_LOG` is unset, sets it to `debug` before tracing init.
- **`--quiet` / `-q`** — sets `VOX_CLI_QUIET=1` for supported commands.
- **`doctor --json`** is the subcommand’s own machine JSON; **`vox --json doctor`** only sets `VOX_CLI_GLOBAL_JSON` for code paths that read it — do not assume they are interchangeable.

## Completions

- **`vox completions <shell>`** — use **`clap_complete`**; shells: **bash**, **zsh**, **fish**, **powershell**, **elvish**. Install by redirecting stdout to the appropriate completion path for your shell (see [`reference/cli.md`](cli.md)).

## Adding or renaming commands

1. Implement in `crates/vox-cli` (and internal surfaces as needed).
2. Add or update rows in **`contracts/cli/command-registry.yaml`** (schema: **`contracts/cli/command-registry.schema.json`**).
3. Update **`docs/src/ref-cli.md`** and, for top-level reachability, **`cli-reachability.md`** when `reachability_required` is not `false`.
4. Run **`vox ci command-compliance`** before merge (also enforced in CI).


<!-- Merged from cli-reachability.md -->

---
title: "CLI command reachability"
description: "Official documentation for CLI command reachability for the Vox language. Detailed technical reference, architecture guides, and implemen"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# CLI command reachability

This page maps **`vox` subcommands** in [`crates/vox-cli/src/lib.rs`](../../../crates/vox-cli/src/lib.rs) to their **implementation modules** under [`crates/vox-cli/src/commands/`](../../../crates/vox-cli/src/commands).

## Reachable from default / feature matrix

| CLI variant | Feature gate | Handler module |
|-------------|--------------|----------------|
| `build` | default | `commands::build` |
| `check` | default | `commands::check` |
| `test` | default | `commands::test` |
| `run` | default | `commands::run` |
| `script` | `script-execution` | `commands::runtime::run::script` |
| `dev` | default | `commands::dev` |
| `live` | `live` | `commands::live` |
| `bundle` | default | `commands::bundle` |
| `fmt` | default | `commands::fmt` (not implemented; fails with doc pointer; see `ref-cli.md`) |
| `install` | default | `commands::install` (not implemented; fails with doc pointer; see `ref-cli.md`) |
| `login` | default | `commands::login` (deprecated compatibility shim) |
| `logout` | default | `commands::logout` (deprecated compatibility shim) |
| `lsp` | default | `commands::lsp` |
| `doctor` | default / `codex` | `commands::doctor` or `commands::diagnostics::doctor` |
| `clavis` | default | `commands::clavis` |
| `secrets` | default | alias of `clavis` |
| `architect` | `codex` or `stub-check` | `commands::diagnostics::tools::architect` |
| `snippet` | default | `commands::extras::snippet_cli` |
| `share` | default | `commands::extras::share_cli` |
| `codex` | default | `commands::codex` |
| `db` | default | `commands::db` + `commands::db_cli` dispatch |
| `scientia` | default | `commands::scientia` (facade over `db_cli` research helpers) |
| `openclaw` | `ars` | `commands::openclaw` |
| `skill` | `ars` | `commands::extras::skill_cmd` |
| `ludus` | `extras-ludus` | `commands::extras::ludus_cli` |
| `stub-check` | `stub-check` | `commands::stub_check` |
| `ci` | default | `commands::ci` |
| `commands` | default | `command_catalog` |
| `mens` | `mens-base` or `gpu` | `commands::mens` |
| `populi` | `populi` | `commands::populi_cli` |
| `oratio` | `oratio` | `commands::oratio_cmd` |
| `speech` | `oratio` | `commands::oratio_cmd` (visible alias of `oratio`) |
| `review` | `coderabbit` | `commands::review` |
| `island` | `island` | `commands::island` |
| `train` | `gpu` + `mens-dei` | `commands::ai::train` |
| `dei` | `dei` | `commands::dei` (alias `orchestrator`) |

## `vox-compilerd` RPC (not CLI variants)

Daemon dispatch lives in [`crates/vox-cli/src/compilerd.rs`](../../../crates/vox-cli/src/compilerd.rs). Methods call **`commands::build`**, **`check`**, **`bundle`**, **`fmt`**, **`doc`**, **`test`**, **`run`**, **`dev`** — not the removed `commands/compiler/` tree.

## Removed / non-compiled trees (historical)

The following directories under `commands/` were **not** referenced from `commands/mod.rs` or the CLI and have been **removed** to reduce dead surface:

- `commands/compiler/` — duplicate of canonical `build` / `check` / `doc` / `fmt` / `bundle` paths used by `compilerd` and CLI.
- `commands/pkg/` — unwired package manager experiment.
- `commands/serve_dashboard/` — superseded by `vox-codex-dashboard` / extension flows.
- `commands/infra/` — unwired deploy/execution subtree.
- `commands/learn.rs`, `commands/dashboard.rs` — orphan modules with no `mod` declaration.

## Shared subtrees

- `commands::runtime` — used by `run` (script lane), `dev` re-exports, and feature-gated script execution.
- `commands::extras` — snippet, share, skill, ludus, ARS helpers.
