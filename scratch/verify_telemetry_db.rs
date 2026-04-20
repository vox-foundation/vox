use vox_db::{VoxDb, DbConfig};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Force enable telemetry for this test
    std::env::set_var("VOX_MESH_CODEX_TELEMETRY", "1");
    
    // 2. Connect to local DB
    let cfg = DbConfig::resolve_canonical().unwrap_or(DbConfig::Memory);
    let db = VoxDb::connect(cfg).await?;
    
    // 3. Mock telemetry
    let telemetry = json!({
        "gpu_count": 1,
        "vram_mb": 16000,
        "vendor": "NVIDIA",
        "temp_c": 45.5,
        "utilization_pct": 12.0
    });
    
    // 4. Record
    println!("Recording mock telemetry...");
    vox_db::populi_registry_telemetry::record_hardware_telemetry_opt(
        "test_repo_id",
        Some("test_node_id"),
        &telemetry
    ).await;
    
    // 5. Verify (query back from research_metrics)
    // Metric type is METRIC_TYPE_POPULI_CONTROL_EVENT which is "populi_control_event"
    let metrics = db.list_research_metrics_by_type("populi_control_event", "mens:test_repo_id", 1).await?;
    
    if let Some((sid, val, meta)) = metrics.first() {
        println!("Success!");
        println!("Session ID: {}", sid);
        println!("Metadata: {}", meta.as_deref().unwrap_or("None"));
    } else {
        println!("Failed: Metric not found in database.");
    }
    
    Ok(())
}
