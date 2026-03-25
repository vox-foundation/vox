use secrecy::SecretString;

use crate::errors::SecretError;
use crate::spec::SecretId;
use crate::spec::SecretSpec;

pub trait SecretBackend: Send + Sync {
    fn resolve(&self, id: SecretId, spec: SecretSpec) -> Result<Option<SecretString>, SecretError>;
}

pub struct NoopBackend;

impl SecretBackend for NoopBackend {
    fn resolve(
        &self,
        _id: SecretId,
        _spec: SecretSpec,
    ) -> Result<Option<SecretString>, SecretError> {
        Ok(None)
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
    ) -> Result<Option<SecretString>, SecretError> {
        Err(SecretError::BackendUnavailable(self.reason.clone()))
    }
}

#[cfg(feature = "clavis-infisical")]
pub mod infisical;

#[cfg(feature = "clavis-vault")]
pub mod vault;
