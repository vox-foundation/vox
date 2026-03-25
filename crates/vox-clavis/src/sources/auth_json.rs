use std::path::{Path, PathBuf};

use keyring::Entry;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::bounded_fs::{read_utf8_path_capped, read_utf8_path_capped_opt};
use crate::errors::SecretError;
use crate::types::SecretSource;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CliCredentials {
    pub registries: std::collections::HashMap<String, RegistryAuth>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryAuth {
    pub token: String,
    pub username: Option<String>,
}

const SECURE_SERVICE: &str = "vox-clavis";
const SECURE_SENTINEL: &str = "__clavis_keyring__";

#[must_use]
pub fn vox_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vox")
}

fn auth_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("VOX_CLAVIS_AUTH_PATH")
        && !override_path.trim().is_empty()
    {
        return PathBuf::from(override_path.trim());
    }
    vox_dir().join("auth.json")
}

fn secure_entry(registry: &str) -> Result<Entry, SecretError> {
    Entry::new(SECURE_SERVICE, registry)
        .map_err(|e| SecretError::BackendUnavailable(format!("secure store unavailable: {e}")))
}

fn read_secure_token(registry: &str) -> Option<String> {
    let entry = secure_entry(registry).ok()?;
    let value = entry.get_password().ok()?;
    if value.trim().is_empty() {
        return None;
    }
    Some(value)
}

fn write_secure_token(registry: &str, token: &str) -> Result<(), SecretError> {
    let entry = secure_entry(registry)?;
    entry
        .set_password(token)
        .map_err(|e| SecretError::BackendUnavailable(format!("failed to write secure token: {e}")))
}

fn read_credentials_file(path: &Path) -> Result<CliCredentials, SecretError> {
    if !path.exists() {
        return Ok(CliCredentials::default());
    }
    let content = read_utf8_path_capped(path)?;
    Ok(serde_json::from_str::<CliCredentials>(&content).unwrap_or_default())
}

#[cfg_attr(not(unix), allow(unused_variables))]
fn set_file_permissions(path: &Path) -> Result<(), SecretError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(|e| {
            SecretError::Io(format!(
                "failed to set restrictive permissions on {}: {e}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

fn write_credentials_file(path: &PathBuf, creds: &CliCredentials) -> Result<(), SecretError> {
    let content = serde_json::to_string_pretty(creds)
        .map_err(|e| SecretError::Serialization(format!("encode auth json: {e}")))?;
    std::fs::write(path, content)
        .map_err(|e| SecretError::Io(format!("write {}: {e}", path.display())))?;
    set_file_permissions(path)?;
    Ok(())
}

#[must_use]
pub fn read_registry_token(registry: &str) -> Option<(SecretString, SecretSource)> {
    if let Some(token) = read_secure_token(registry) {
        return Some((
            SecretString::new(token.into_boxed_str()),
            SecretSource::SecureStore,
        ));
    }
    let path = auth_path();
    if !path.exists() {
        if registry == "voxpm" {
            let legacy = vox_dir().join("auth_token");
            let token = read_utf8_path_capped_opt(legacy.as_path())?;
            let token = token.trim().to_string();
            if token.is_empty() {
                return None;
            }
            return Some((
                SecretString::new(token.into_boxed_str()),
                SecretSource::LegacyAuthToken,
            ));
        }
        return None;
    }

    let content = read_utf8_path_capped_opt(path.as_path())?;
    let creds = serde_json::from_str::<CliCredentials>(&content).ok()?;
    let auth = creds.registries.get(registry)?;
    if auth.token == SECURE_SENTINEL {
        return None;
    }
    if auth.token.trim().is_empty() {
        return None;
    }
    Some((
        SecretString::new(auth.token.clone().into_boxed_str()),
        SecretSource::AuthJson,
    ))
}

pub fn write_registry_token(
    registry: &str,
    token: &str,
    username: Option<String>,
) -> Result<PathBuf, SecretError> {
    let config_dir = vox_dir();
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(|e| {
            SecretError::Io(format!("Failed to create {}: {e}", config_dir.display()))
        })?;
    }
    let auth_path = auth_path();
    let mut config = read_credentials_file(&auth_path)?;
    let secure_store_ok = write_secure_token(registry, token).is_ok();

    config.registries.insert(
        registry.to_string(),
        RegistryAuth {
            token: if secure_store_ok {
                SECURE_SENTINEL.to_string()
            } else {
                token.to_string()
            },
            username,
        },
    );
    write_credentials_file(&auth_path, &config)?;
    Ok(auth_path)
}

pub fn migrate_to_secure_store() -> Result<usize, SecretError> {
    let path = auth_path();
    let mut creds = read_credentials_file(&path)?;
    let mut migrated = 0usize;
    for (registry, auth) in &mut creds.registries {
        if auth.token.trim().is_empty() || auth.token == SECURE_SENTINEL {
            continue;
        }
        write_secure_token(registry, &auth.token)?;
        auth.token = SECURE_SENTINEL.to_string();
        migrated += 1;
    }
    if migrated > 0 {
        write_credentials_file(&path, &creds)?;
    }
    Ok(migrated)
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn auth_path_uses_override() {
        let _g = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::set_var("VOX_CLAVIS_AUTH_PATH", "/tmp/vox-clavis-auth.json");
        }
        let got = auth_path();
        assert!(got.to_string_lossy().contains("vox-clavis-auth.json"));
        unsafe {
            std::env::remove_var("VOX_CLAVIS_AUTH_PATH");
        }
    }
}
