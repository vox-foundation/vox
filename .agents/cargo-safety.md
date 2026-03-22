---
description: How to safely run Cargo tasks inside agent sessions to avoid target locking and deadlocks.
---

# Cargo Lock and Deadlock Prevention
When the AI is asked to run extensive checks, tests, or compiling on Rust workspaces, it should always follow these rules to avoid deadlocking the `cargo` build lock or overwhelming I/O storage:

1. **Do not run overlapping background checks.** Ensure only one `cargo test` or `cargo build` command runs at a time. If the user requests to verify multiple crates interactively, either run them synchronously, queue them, or use `cargo check --workspace` as a single job.
2. **Handle I/O bloat.** If the `target/` directory bloats, it will slow down file indexing drastically and lock `cargo` for minutes while checking `.rmeta` metadata. When running in an agent session, always use `cargo check` instead of `cargo build` when validating syntax and structures, as it generates far fewer artifacts.
3. **Isolate Agent Outputs (Optional).** If testing a newly written sub-project or integration test script within the codebase, explicitly define `$env:CARGO_TARGET_DIR = 'target_agent'` (Windows) before invoking cargo to skip colliding with the user's primary IDE rust-analyzer or their existing build target. Wait until the background command completes and then verify.
4. **Isolate Integration Test Workspaces.** If writing Rust Integration tests that execute `Command::new("cargo")` to compile dynamically generated code, append `\n[workspace]\n` to the generated `Cargo.toml` of the child invocation, AND output to a unique folder. Otherwise, the child `cargo build` will attach to the parent workspace and deadlock waiting for a lock on the `target/` directory which is currently held by the running `cargo test` itself.

## Clippy / `-D warnings` when Turso or LLVM are unavailable

Some workspace members are **skipped for local/agent `cargo clippy`** when the host lacks native toolchain pieces:

| Skip reason | Crates to `--exclude` |
| :--- | :--- |
| **Turso / libSQL** pulls `aegis`, which wants **`clang-cl.exe`** on Windows | `vox-pm`, `vox-db`, `vox-gamify`, and dependents **`vox-orchestrator`**, **`vox-skills`**, **`vox-webhook`** (`vox-runtime` no longer depends on `turso`) |
| **LLVM / Inkwell** (`vox-codegen-llvm`) | **Excluded** from `[workspace].members` (crate remains under `crates/` for archival); do not pull into builds |

**Example (PowerShell, from repo root, isolated target dir):**

```powershell
$env:CARGO_TARGET_DIR = "$PWD\.cargo-targets\clippy-agent"
& "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --workspace `
  --exclude vox-pm `
  --exclude vox-db `
  --exclude vox-gamify `
  --exclude vox-orchestrator `
  --exclude vox-skills `
  --exclude vox-webhook `
  --keep-going -- -D warnings
```

- **`vox-mcp`** is already in `[workspace].exclude` in the root `Cargo.toml`; do not add it to this list unless it is re-enabled as a member.
- For a **full** workspace graph (including Turso + LLVM), use **Linux CI** or a machine with **LLVM + `clang-cl`** (Windows) / **clang** (Linux) as required by those crates. Bootstrap: `scripts/install.sh --install-clang` or `scripts/install.ps1 -InstallClang` (see `docs/src/how-to-setup.md`).

Workspace `missing_docs` is **not** enabled globally (keeps `cargo clippy -- -D warnings` green). Prefer real `///` / `//!` over crate-level `#![allow(missing_docs)]`; tighten per-crate with `cargo rustc -p <crate> -- --deny=missing-docs` when editing a package.

## Async subprocess / filesystem (`vox-cli`, `vox-mcp`)

`vox doctor` (diagnostics), `vox clean`, `vox architect`, `vox bundle`, and MCP **`vox_*`** cargo/git tools use **`tokio::process`** / **`tokio::fs`** where they run under an async dispatcher. The **`vox stub-check`** handler runs the TOESTUB **`run_and_report`** pass inside **`tokio::task::spawn_blocking`** so the scan does not block a Tokio worker; staged fix work in the same command also uses **`spawn_blocking`** where appropriate.

The **`vox`** binary’s **`build.rs`** raises the **Windows** executable stack (MSVC **`/STACK`** / GNU **`--stack`**) because the large clap **`Cli`** enum can overflow the default **1 MiB** stack while printing **`--help`** (clap’s help generation is deeply recursive on some targets).
