//! `vox-mens` — same as `vox mens …` without typing the `mens` subcommand.
//!
//! Build: `cargo build -p vox-cli --features mens-base` (default features include `mens-base`).
//! Oratio STT (`oratio` subcommand): add **`mens-oratio`** on the same build.
//! Native training (`train`, QLoRA, etc.) still requires `--features gpu` on the same build.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_cli::run_vox_cli_populi_prefixed().await
}
