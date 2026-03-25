//! Optional `[mens]` section in workspace `Vox.toml` (operator SSOT; env overrides apply in consumers).

use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

/// Parsed `[mens]` table from `Vox.toml`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct VoxMeshToml {
    /// HTTP control plane base URL (`GET /v1/populi/nodes`); maps to `OrchestratorConfig::populi_control_url`.
    #[serde(default)]
    pub control_url: Option<String>,
    /// Opaque cluster / tenancy id for join/heartbeat when the server enforces scope.
    #[serde(default)]
    pub scope_id: Option<String>,
    /// When true, merge `gpu_cuda` into default agent capabilities (same intent as `VOX_MESH_ADVERTISE_GPU`).
    #[serde(default)]
    pub advertise_gpu: Option<bool>,
    /// Extra capability labels (merged with env `VOX_MESH_LABELS`).
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

/// Error reading or parsing `Vox.toml` for the mens section only.
#[derive(Debug, Error)]
pub enum VoxMeshTomlError {
    /// Underlying I/O error.
    #[error("I/O reading Vox.toml: {0}")]
    Io(#[from] std::io::Error),
    /// TOML parse error.
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    /// TOML serialize error (re-serializing a sub-table).
    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Read `[mens]` from `path` (typically `.../Vox.toml`). Returns `None` if the file or section is missing.
pub fn read_vox_populi_toml(path: &Path) -> Result<Option<VoxMeshToml>, VoxMeshTomlError> {
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&content)?;
    let Some(mesh_val) = table.get("mens") else {
        return Ok(None);
    };
    let section_str = toml::to_string(mesh_val)?;
    let mens: VoxMeshToml = toml::from_str(&section_str)?;
    let labels_empty = match mens.labels.as_ref() {
        None => true,
        Some(labels) => labels.is_empty(),
    };
    if mens.control_url.is_none()
        && mens.scope_id.is_none()
        && mens.advertise_gpu.is_none()
        && labels_empty
    {
        return Ok(None);
    }
    Ok(Some(mens))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn reads_mesh_section() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("Vox.toml");
        fs::write(
            &p,
            r#"
[mens]
control_url = "http://127.0.0.1:9999"
scope_id = "ci-scope"
advertise_gpu = true
labels = ["pool=a", "pool=b"]
"#,
        )
        .unwrap();
        let m = read_vox_populi_toml(&p).unwrap().expect("some mens");
        assert_eq!(m.control_url.as_deref(), Some("http://127.0.0.1:9999"));
        assert_eq!(m.scope_id.as_deref(), Some("ci-scope"));
        assert_eq!(m.advertise_gpu, Some(true));
        assert_eq!(m.labels.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn missing_file_returns_none() {
        let p = Path::new("/nonexistent/Vox.toml");
        assert!(read_vox_populi_toml(p).unwrap().is_none());
    }
}
