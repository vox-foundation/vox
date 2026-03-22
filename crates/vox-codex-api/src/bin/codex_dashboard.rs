//! Local Codex dashboard + `/api/audio/*` (Oratio). Prefer this over the unfinished `vox dash` CLI feature gate.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    vox_codex_api::run_dashboard().await
}
