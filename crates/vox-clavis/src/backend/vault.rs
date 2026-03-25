use secrecy::SecretString;

use crate::backend::SecretBackend;
use crate::errors::SecretError;
use crate::spec::{SecretId, SecretSpec};

pub struct VaultBackend;

impl SecretBackend for VaultBackend {
    fn resolve(
        &self,
        _id: SecretId,
        spec: SecretSpec,
    ) -> Result<Option<SecretString>, SecretError> {
        if std::env::var("VAULT_ADDR").is_err() || std::env::var("VAULT_TOKEN").is_err() {
            return Err(SecretError::BackendMisconfigured(
                "VAULT_ADDR and VAULT_TOKEN are required".to_string(),
            ));
        }
        let path =
            std::env::var("VOX_CLAVIS_VAULT_PATH").unwrap_or_else(|_| "secret/vox".to_string());
        let field = spec.backend_key.unwrap_or(spec.canonical_env);
        let out = std::process::Command::new("vault")
            .args(["kv", "get", &format!("-field={field}"), path.trim()])
            .output()
            .map_err(|e| {
                SecretError::BackendUnavailable(format!("failed to run vault CLI: {e}"))
            })?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if stderr.to_ascii_lowercase().contains("no value found") {
                return Ok(None);
            }
            return Err(SecretError::BackendQueryFailed(format!(
                "vault CLI returned non-zero status: {stderr}"
            )));
        }
        let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if value.is_empty() {
            return Ok(None);
        }
        Ok(Some(SecretString::new(value.into_boxed_str())))
    }
}
