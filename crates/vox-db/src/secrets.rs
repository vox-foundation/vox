//! OS keyring helpers wrapping [`crate::StoreError`].
//!
//! Canonical location for secret storage used by Codex / CLI; the `vox-codex` crate re-exports this
//! module for the historical `vox_codex::secrets` path.

use crate::StoreError;

/// Store a secret in the OS keyring (`service` / `account` namespace).
pub fn store_secret(service: &str, account: &str, secret: &str) -> Result<(), StoreError> {
    let e = keyring::Entry::new(service, account)
        .map_err(|err| StoreError::Db(format!("keyring entry: {err}")))?;
    e.set_password(secret)
        .map_err(|err| StoreError::Db(format!("keyring set_password: {err}")))
}

/// Read a secret from the OS keyring (`service` / `account` namespace).
pub fn get_secret(service: &str, account: &str) -> Result<String, StoreError> {
    let e = keyring::Entry::new(service, account)
        .map_err(|err| StoreError::Db(format!("keyring entry: {err}")))?;
    e.get_password()
        .map_err(|err| StoreError::Db(format!("keyring get_password: {err}")))
}

/// Remove a secret from the OS keyring.
pub fn delete_secret(service: &str, account: &str) -> Result<(), StoreError> {
    let e = keyring::Entry::new(service, account)
        .map_err(|err| StoreError::Db(format!("keyring entry: {err}")))?;
    e.delete_credential()
        .map_err(|err| StoreError::Db(format!("keyring delete_credential: {err}")))
}
