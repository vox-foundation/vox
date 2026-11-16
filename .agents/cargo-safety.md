---
description: How to safely run Cargo tasks inside agent sessions to avoid target locking and deadlocks.
---

# Cargo Lock and Deadlock Prevention
When the AI is asked to run extensive checks, tests, or compiling on Rust workspaces, it should always follow these rules to avoid deadlocking the `cargo` build lock or overwhelming I/O storage:

1. **Let `vox-cargo-shim` queue and coalesce; batch multi-package verification.** `vox-cargo-shim` serializes per-target builds via `FairQueue` (`~/.vox/build-broker/targets/<target_hash>/`) BEFORE claiming a machine-wide adaptive slot (`VOX_BROKER_MAX_CONCURRENT` / `VOX_BROKER_RESERVED_SLOTS`) and coalesces identical in-flight `check`/`build`/`test`/`clippy` runs (`VOX_BROKER_COALESCE=0` to disable). Do not spawn overlapping background checks; instead, combine independent crate checks/tests into a single batched Cargo invocation (`cargo test -p crate-a -p crate-b --lib --tests`) so Cargo resolves the workspace graph and acquires `target/.cargo-lock` once instead of triggering N sequential lock/queue cycles.
2. **Use post-queue timeout (`VOX_BROKER_RUN_TIMEOUT_SECS`) and prefer `cargo check` for syntax validation.** Set `VOX_BROKER_RUN_TIMEOUT_SECS=300` (or pass `--broker-run-timeout 300`) so the active hang timer starts strictly AFTER per-target `FairQueue` and global slot admission (reaping the child process group and exiting `124` on active hangs), whereas an outer GNU `timeout 900s` includes queue wait time and serves only as a coarse runaway backstop. Prefer `cargo check` over `cargo build` when validating syntax/types to minimize `target/` I/O bloat.
3. **Workstream Target Isolation & `sccache` (Optional).** Instead of scattering ad-hoc `target_agent` folders, use `VOX_ISOLATED_TARGET=1` with `VOX_WORKSTREAM=<slug>` (or write an ASCII `[A-Za-z0-9_-]{1,48}` slug into `<worktree>/.vox/agent-target`), which routes `CARGO_TARGET_DIR` to `<worktree>/target/ws-<slug>`. When `sccache` is on `PATH`, `vox-cargo-shim` auto-injects `RUSTC_WRAPPER=sccache` (`VOX_BROKER_NO_SCCACHE=1` to opt out) and pairs isolated workstream targets with `CARGO_INCREMENTAL=0` so compiled dependencies share `.sccache`.
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
