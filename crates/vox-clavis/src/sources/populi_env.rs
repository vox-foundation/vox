use secrecy::SecretString;

use crate::bounded_fs::read_utf8_path_capped;
use crate::types::SecretSource;

fn candidate_mesh_env_paths() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for root_var in ["VOX_WORKSPACE_ROOT", "VOX_REPO_ROOT"] {
        if let Ok(root) = std::env::var(root_var)
            && !root.trim().is_empty()
        {
            out.push(
                std::path::PathBuf::from(root.trim())
                    .join(".vox")
                    .join("populi")
                    .join("mesh.env"),
            );
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join(".vox").join("populi").join("mesh.env"));
    }
    out.push(
        crate::sources::auth_json::vox_dir()
            .join("populi")
            .join("mesh.env"),
    );
    out
}

/// Read `KEY=value` from the first `.vox/populi/mesh.env` candidate that contains `canonical_key`.
#[must_use]
pub fn read_populi_env_key(canonical_key: &str) -> Option<(SecretString, SecretSource)> {
    let needle = canonical_key.trim();
    if needle.is_empty() {
        return None;
    }
    for path in candidate_mesh_env_paths() {
        let raw = match read_utf8_path_capped(&path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        for line in raw.lines() {
            let t = line.trim();
            if t.starts_with('#') || t.is_empty() {
                continue;
            }
            let Some((key, value)) = t.split_once('=') else {
                continue;
            };
            if key.trim() != needle {
                continue;
            }
            let v = value.trim().to_string();
            if v.is_empty() {
                return None;
            }
            return Some((
                SecretString::new(v.into_boxed_str()),
                SecretSource::PopuliEnv,
            ));
        }
    }
    None
}

#[must_use]
pub fn read_mesh_token_from_populi_env() -> Option<(SecretString, SecretSource)> {
    read_populi_env_key("VOX_MESH_TOKEN")
}
