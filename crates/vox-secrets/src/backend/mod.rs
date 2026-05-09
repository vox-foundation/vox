use secrecy::SecretString;

use crate::errors::SecretError;
use crate::spec::SecretId;
use crate::spec::SecretSpec;

pub trait SecretBackend: Send + Sync {
    fn resolve(
        &self,
        id: SecretId,
        spec: SecretSpec,
        profile: Option<&str>,
        caller_context: &str,
    ) -> Result<Option<SecretString>, SecretError>;
    fn write_audit_log(
        &self,
        secret_id: &str,
        status: &str,
        source: Option<&str>,
        profile: &str,
        caller_context: &str,
        detail: Option<&str>,
    ) -> Result<(), SecretError>;
}

pub struct NoopBackend;

impl SecretBackend for NoopBackend {
    fn resolve(
        &self,
        _id: SecretId,
        _spec: SecretSpec,
        _profile: Option<&str>,
        _caller_context: &str,
    ) -> Result<Option<SecretString>, SecretError> {
        Ok(None)
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

pub struct UnavailableBackend {
    pub reason: String,
}

impl SecretBackend for UnavailableBackend {
    fn resolve(
        &self,
        _id: SecretId,
        _spec: SecretSpec,
        _profile: Option<&str>,
        _caller_context: &str,
    ) -> Result<Option<SecretString>, SecretError> {
        Err(SecretError::BackendUnavailable(self.reason.clone()))
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

#[cfg(feature = "secrets-infisical")]
pub mod infisical;

#[cfg(feature = "secrets-vault")]
pub mod vault;

pub mod vox_vault;
