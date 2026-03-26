//! `vox-mens` — same as `vox mens …` without typing the `mens` subcommand.
//!
//! Build: `cargo build -p vox-cli --features mens-base` (default features include `mens-base`).
//! Speech (Oratio) is only **`vox oratio`** / **`vox speech`** — build `vox-cli` with **`--features oratio`** (not via `vox-mens`).
//! Native training (`train`, QLoRA, etc.) still requires `--features gpu` on the same build.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_cli::run_vox_cli_mens_prefixed().await
}
