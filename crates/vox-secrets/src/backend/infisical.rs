use secrecy::SecretString;

use crate::backend::SecretBackend;
use crate::errors::SecretError;
use crate::spec::{SecretId, SecretSpec};

pub struct InfisicalBackend;

impl SecretBackend for InfisicalBackend {
    fn resolve(
        &self,
        _id: SecretId,
        spec: SecretSpec,
        _profile: Option<&str>,
        _caller_context: &str,
    ) -> Result<Option<SecretString>, SecretError> {
        if std::env::var("INFISICAL_TOKEN").is_err()
            && std::env::var("INFISICAL_SERVICE_TOKEN").is_err()
        {
            return Err(SecretError::BackendMisconfigured(
                "INFISICAL_TOKEN or INFISICAL_SERVICE_TOKEN is required".to_string(),
            ));
        }
        let key = spec.backend_key.unwrap_or(spec.canonical_env);
        let mut cmd = std::process::Command::new("infisical");
        cmd.args(["secrets", "get", key, "--plain", "--silent"]);
        if let Ok(project_id) = std::env::var("VOX_SECRETS_INFISICAL_PROJECT_ID")
            && !project_id.trim().is_empty()
        {
            cmd.args(["--projectId", project_id.trim()]);
        }
        if let Ok(env_name) = std::env::var("VOX_SECRETS_INFISICAL_ENV")
            && !env_name.trim().is_empty()
        {
            cmd.args(["--env", env_name.trim()]);
        }
        let out = cmd.output().map_err(|e| {
            SecretError::BackendUnavailable(format!("failed to run infisical CLI: {e}"))
        })?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if stderr.to_ascii_lowercase().contains("not found") {
                return Ok(None);
            }
            return Err(SecretError::BackendQueryFailed(format!(
                "infisical CLI returned non-zero status: {stderr}"
            )));
        }
        let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if value.is_empty() {
            return Ok(None);
        }
        Ok(Some(SecretString::new(value.into_boxed_str())))
    }

    fn write_audit_log(
        &self,
        _secret_id: &str,
        _status: &str,
        _source: Option<&str>,
        _profile: &str,
        _caller_context: &str,
        _detail: Option<&str>,
    ) -> Result<(), SecretError> {
        Ok(())
    }
}
