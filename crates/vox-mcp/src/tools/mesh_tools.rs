//! Local mesh registry introspection (`vox_mesh_local_status`).

use serde_json::{Value, json};

/// Return mesh environment + on-disk registry as JSON text.
pub fn mesh_local_status(args: Value) -> anyhow::Result<String> {
    let path = args
        .get("registry_path")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(vox_mesh::local_registry_path);
    let reg = vox_mesh::LocalRegistry::new(path.clone());
    let file = reg.load()?;
    let env = vox_mesh::mesh_env();
    let out = json!({
        "mesh_env": env,
        "registry_path": reg.path().display().to_string(),
        "registry": file,
    });
    Ok(out.to_string())
}
