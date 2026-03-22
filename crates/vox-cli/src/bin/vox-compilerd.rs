//! `vox-compilerd` — stdio JSON dispatcher for compiler RPC (`build`, `check`, `dev`, …).
//!
//! Build with: `cargo build -p vox-cli --bin vox-compilerd`
//!
//! The `vox` binary resolves this executable as a sibling next to `vox` or on `PATH`.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_cli::init_tracing_for_cli();
    vox_cli::compilerd::run().await
}
