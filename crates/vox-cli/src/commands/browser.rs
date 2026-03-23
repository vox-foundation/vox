use crate::cli_actions::BrowserAction;
use vox_browser::session::BrowserSession;
use std::sync::Arc;
struct DummyProvider;
impl vox_browser::protocol::AiProvider for DummyProvider {
    fn infer(&self, _prompt: String) -> std::pin::Pin<Box<dyn std::future::Future<Output = vox_browser::protocol::AiResult<String>> + Send>> {
        Box::pin(async { Err("AI not available in the CLI standalone session.".into()) })
    }
}

/// Dispatch browser-related CLI actions.
pub async fn run(action: BrowserAction) -> anyhow::Result<()> {
    match action {
        BrowserAction::Session { url, headful } => {
            println!("🚀 Launching interactive Vox Browser...");
            
            let bridge = Arc::new(DummyProvider);
            
            println!("   Starting Chromium (headless = {})...", !headful);
            
            // Note: In real usage, this needs an actual URL. BrowserSession::launch expects a context too.
            // But we can just use the API from vox-browser directly.
            let session = BrowserSession::launch(&url, bridge).await?;
            
            println!("✅ Browser started!");
            println!("   - Current URL: {}", url);
            println!("\nPress Ctrl+C to terminate the session.");
            
            // Keep alive
            tokio::signal::ctrl_c().await?;
            println!("\nShutting down browser...");
            let _ = session.close().await;
            
            Ok(())
        }
    }
}
