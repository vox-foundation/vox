//! Local mens registry introspection (`vox_populi_local_status`).

use serde_json::{Value, json};

/// Return mens environment + on-disk registry as JSON text.
pub fn mesh_local_status(args: Value) -> anyhow::Result<String> {
    let path = args
        .get("registry_path")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(vox_populi::local_registry_path);
    let reg = vox_populi::LocalRegistry::new(path.clone());
    let file = reg.load()?;
    let env = vox_populi::mesh_env();
    let out = json!({
        "mesh_env": env,
        "registry_path": reg.path().display().to_string(),
        "registry": file,
    });
    Ok(out.to_string())
}
