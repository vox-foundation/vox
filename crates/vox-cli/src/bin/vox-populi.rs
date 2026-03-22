//! `vox-populi` — same as `vox populi …` without typing the `populi` subcommand.
//!
//! Build: `cargo build -p vox-cli --features populi-base` (default features include `populi-base`).
//! Oratio STT (`oratio` subcommand): add **`populi-oratio`** on the same build.
//! Native training (`train`, QLoRA, etc.) still requires `--features gpu` on the same build.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_cli::run_vox_cli_populi_prefixed().await
}
