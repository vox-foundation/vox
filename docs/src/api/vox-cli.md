# Crate: `vox-cli`

Rust package path: **`crates/vox-cli`**. Produces the **`vox`** binary (`src/main.rs`) and **`vox-compilerd`** (`src/bin/vox-compilerd.rs`, stdio JSON dispatcher for `dev` and compiler-subcommand RPC).

## Scope

This checkout’s `vox-cli` is a **minimal** compiler driver: clap dispatch, codegen orchestration, and a small set of subcommands. It does **not** yet expose the full Populi / review / MCP / `vox init` surface that appears in some older generated docs.

Authoritative **user-facing** command list: [`ref-cli.md`](../ref-cli.md).

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

**Library / dispatch modules (not always exposed as `vox` subcommands):** `src/commands/info.rs` (registry metadata), `src/commands/runtime/**` (extended run/dev/info/tree/shell). Inline script execution (`runtime/run/{script,backend,sandbox}`) builds with **`--features script-execution`**; Axum Populi inference server (`commands/ai/serve`) builds with **`--features execution-api`** (implies `script-execution` + `gpu` + Axum + `vox-corpus` validation helpers).

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
