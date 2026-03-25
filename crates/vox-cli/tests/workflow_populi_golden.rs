//! Golden interpreted workflow: `examples/mens/workflow_mesh_demo.vox` (M4.5).

use std::path::Path;

#[tokio::test]
async fn golden_mesh_workflow_journal_shape() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let vox_file = manifest_dir
        .join("..")
        .join("..")
        .join("examples")
        .join("mens")
        .join("workflow_mesh_demo.vox");
    let result = vox_cli::pipeline::run_frontend(&vox_file, false)
        .await
        .unwrap_or_else(|e| panic!("frontend failed for {}: {e}", vox_file.display()));
    let journal = vox_workflow_runtime::interpret_workflow(&result.hir, "wf_mesh_demo")
        .await
        .expect("interpret");
    let events: Vec<_> = journal
        .iter()
        .filter_map(|v| v.get("event").and_then(|e| e.as_str()))
        .collect();
    assert!(events.contains(&"WorkflowStarted"));
    assert!(events.contains(&"ActivityStarted"));
    assert!(events.contains(&"ActivityCompleted"));
    assert!(events.contains(&"LocalActivity"));
    assert!(events.contains(&"MeshActivity"));
    assert!(events.contains(&"WorkflowCompleted"));
    let mens = journal
        .iter()
        .find(|v| v.get("event").and_then(|e| e.as_str()) == Some("MeshActivity"))
        .expect("mens row");
    assert_eq!(
        mens.get("mesh_op").and_then(|v| v.as_str()),
        Some("snapshot")
    );
    assert_eq!(
        mens.get("activity_id").and_then(|v| v.as_str()),
        Some("golden-snap")
    );
}
