//! # `vox` — minimal Vox compiler CLI
//!
//! Parses arguments with **clap** and dispatches to the `commands` module. The happy path for codegen is:
//! **lex → parse → typecheck → HIR →** TypeScript (`vox-codegen-ts`) **+** Rust (`vox-codegen-rust`).
//!
//! ## Globals and discoverability
//!
//! Root flags (before subcommand): **`--color`**, **`--json`**, **`--verbose` / `-v`**, **`--quiet` / `-q`** — see [`VoxCliRoot`](vox_cli::VoxCliRoot). **`vox completions <shell>`** emits shell completions (bash/zsh/fish/powershell/elvish).
//!
//! Latin groupings (same dispatch as flat verbs): **`vox fabrica`**, **`vox mens`**, **`vox ars`**, **`vox recensio`** (feature **`coderabbit`**).
//!
//! ## Subcommands
//!
//! | CLI | Rust module |
//! |-----|-------------|
//! | `vox build <file> [-o DIR]` | `commands::build` |
//! | `vox check <file>` | `commands::check` |
//! | `vox test <file> [--watch]` | `commands::test` |
//! | `vox snapshot orphans [--clean]` | `commands::snapshot` |
//! | `vox run <file> [--port N] [--mode auto\|app\|script] [-- …]` | `commands::run` |
//! | `vox script <file> …` | `commands::runtime::run::script` (needs `script-execution`) |
//! | `vox ci …` | `commands::ci` |
//! | `vox bundle …` (plugin bundles) | `commands::plugin_bundle` |
//! | `vox bundle-app …` / `vox fabrica bundle …` (web app) | `commands::bundle` |
//! | `vox fmt <file>` | `commands::fmt` |
//! | `vox add` / `remove` / `update` / `lock` / `sync` / `upgrade` / `pm` | `commands::add`, `remove`, `update`, `lock`, `sync`, `upgrade`, `pm` |
//! | `vox dev <file>` | `commands::dev` (via `vox-compilerd`) |
//! | `vox emit client <file>` | `commands::emit` (Library TS SDK / openapi only) |
//! | `vox live` | `commands::live` (needs `--features live`) |
//! | `vox db …` | `commands::db_cli` |
//! | `vox memory search …` | `commands::memory_cli` |
//! | `vox scientia …` | `commands::scientia` (research / capability-map facade over `db_cli`) |
//! | `vox telemetry …` | `commands::telemetry` (optional upload queue; ADR 023) |
//! | `vox codex verify \| export-legacy \| import-legacy \| cutover \| import-orchestrator-memory \| import-skill-bundle \| socrates-metrics \| socrates-eval-snapshot` | `commands::codex` |
//! | `vox openclaw …` | `commands::openclaw` (needs `--features ars`) |
//! | `vox snippet …` / `vox share …` | `commands::extras` |
//! | `vox skill …` | `commands::extras::skill_cmd` (needs `--features ars`) |
//! | `vox ludus …` | `commands::extras::ludus_cli` (needs `--features extras-ludus`) |
//! | `vox stub-check …` | `commands::stub_check` (needs `--features stub-check`) |
//! | `vox architect …` | `commands::diagnostics::tools::architect` (needs `--features stub-check` and/or `codex`) |
//! | `vox lsp` | `commands::lsp` |
//! | `vox doctor` (extended: `--build-perf` / `--scope` / `--json` need `--features codex`) | `commands::diagnostics::doctor` |
//! | `vox secrets …` / `vox clavis …` | `commands::secrets` |
//! | `vox train …` (feature `mens-dei` + `gpu`) | `commands::ai::train` |
//! | `vox review coderabbit …` | `commands::review` (needs `--features coderabbit`) |
//!
//! End-user docs: repository file `docs/src/reference/cli.md`. `@v0` integration during `build`: module `v0`.

use std::process::Command;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Render Unicode/emoji correctly on the Windows console for both `--interp`
    // (this process) and the spawned native binary (shared console code page).
    ::vox_actor_runtime::builtins::vox_console_init_utf8();

    // Intercept ML commands and delegate to vox-ml-cli, preserving leading global flags
    let args: Vec<String> = std::env::args().collect();
    if let Some((primary_cmd, forward_args)) =
        vox_cli::argv_normalize::build_ml_delegation_args(&args)
    {
        let ml_cli_binary =
            vox_cli_core::daemon_ipc::process_supervision::resolve_managed_binary_path(
                "vox-ml-cli",
            );
        let mut command = Command::new(ml_cli_binary);
        // Build-id handshake: vox-ml-cli warns when it was built from a
        // different commit than this `vox` (see `ml_cli_handshake`).
        command.env(
            vox_cli_core::ml_cli_handshake::PARENT_BUILD_ID_ENV,
            vox_cli::freshness::EMBEDDED_GIT_HASH,
        );
        command.args(&forward_args);

        // Wait for completion and exit with same status
        match command.status() {
            Ok(status) => {
                std::process::exit(status.code().unwrap_or(1));
            }
            Err(e) => {
                eprintln!("Error: vox-ml-cli is not installed or not in PATH.");
                eprintln!(
                    "The '{}' subsystem has been extracted to a separate crate.",
                    primary_cmd
                );
                if vox_cli::contributor_mode::is_contributor_mode() {
                    if primary_cmd != "quantize" {
                        eprintln!(
                            "Please run: cargo install --path crates/vox-ml-cli --features populi"
                        );
                    } else {
                        eprintln!(
                            "Please run: cargo install --path crates/vox-ml-cli --features quantize"
                        );
                    }
                } else {
                    eprintln!(
                        "Install the Vox 'full' tier to get the vox-ml-cli binary, then retry this command."
                    );
                }
                eprintln!("Underlying error: {}", e);
                std::process::exit(1);
            }
        }
    }

    let root = vox_cli::argv_normalize::parse_cli_with_tolerance();
    vox_cli::run_vox_cli_from_parsed(root).await
}
