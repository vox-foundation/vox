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
//! | `vox test <file>` | `commands::test` |
//! | `vox run <file> [--port N] [--mode auto\|app\|script] [-- …]` | `commands::run` |
//! | `vox script <file> …` | `commands::runtime::run::script` (needs `script-execution`) |
//! | `vox ci …` | `commands::ci` |
//! | `vox bundle <file> [-o DIR] [--target TRIPLE] [--release]` | `commands::bundle` |
//! | `vox fmt <file>` | `commands::fmt` |
//! | `vox install <name>` | `commands::install` |
//! | `vox dev <file>` | `commands::dev` (via `vox-compilerd`) |
//! | `vox live` | `commands::live` (needs `--features live`) |
//! | `vox db …` | `commands::db_cli` |
//! | `vox scientia …` | `commands::scientia` (research / capability-map facade over `db_cli`) |
//! | `vox codex verify \| export-legacy \| import-legacy \| socrates-metrics \| socrates-eval-snapshot` | `commands::codex` |
//! | `vox openclaw …` | `commands::openclaw` (needs `--features ars`) |
//! | `vox snippet …` / `vox share …` | `commands::extras` |
//! | `vox skill …` | `commands::extras::skill_cmd` (needs `--features ars`) |
//! | `vox ludus …` | `commands::extras::ludus_cli` (needs `--features extras-ludus`) |
//! | `vox stub-check …` | `commands::stub_check` (needs `--features stub-check`) |
//! | `vox architect …` | `commands::diagnostics::tools::architect` (needs `--features stub-check` and/or `codex`) |
//! | `vox lsp` | `commands::lsp` |
//! | `vox doctor` (extended: `--build-perf` / `--scope` / `--json` need `--features codex`) | `commands::diagnostics::doctor` |
//! | `vox train …` (feature `mens-dei` + `gpu`) | `commands::ai::train` |
//! | `vox review coderabbit …` | `commands::review` (needs `--features coderabbit`) |
//! | `vox island …` | `commands::island` (needs `--features island`) |
//!
//! End-user docs: repository file `docs/src/reference/cli.md`. `@v0` integration during `build`: module `v0`.

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = vox_cli::VoxCliRoot::parse();
    vox_cli::run_vox_cli_from_parsed(root).await
}
