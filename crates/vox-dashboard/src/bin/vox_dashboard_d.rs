//! Thin standalone binary: `vox-dashboard-d`
//! Spawnable by `vox dashboard` when not embedded in vox-orchestrator.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("VOX_DASHBOARD_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3921);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    let app = vox_dashboard::dashboard_router();
    println!("[VOX_DASHBOARD_READY: http://127.0.0.1:{port}/dashboard]");
    axum::serve(listener, app).await?;
    Ok(())
}
