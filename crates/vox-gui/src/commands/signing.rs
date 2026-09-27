//! Tauri commands for the Settings → "Signing keys" surface.
//!
//! The real local node identity is a SINGLE Ed25519 keypair stored
//! Argon2-encrypted at `~/.vox/identity.key.enc` (see [`vox_identity::storage`]).
//! There is no multi-key list — the surface shows the live node identity plus
//! the locally-trusted peer keys. Rotation regenerates the keypair and re-seals
//! it under the same master password; because the signing key only ever exists
//! encrypted, rotation REQUIRES the master password by design (no passwordless
//! path), so the UI prompts for it.
//!
//! SECURITY: no command here ever returns raw private key material. The status
//! DTO carries only the node_id, algorithm, and a short public-key fingerprint.

use serde::Serialize;
use tauri::command;

/// Redaction-safe status of the local signing identity.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningKeyDto {
    /// Stable node id derived from the public key (hex, 16 bytes).
    pub node_id: String,
    /// Always `ed25519` for the node identity.
    pub algorithm: String,
    /// Short public-key fingerprint, e.g. `ed25519:7f:42:9b…2a:11`.
    pub fingerprint: String,
    /// Full hex public key (public material only — safe to surface).
    pub pubkey_hex: String,
    /// Whether an identity file exists on disk.
    pub present: bool,
}

/// Status of the local signing identity. When no identity exists yet the
/// `present` flag is false and the other fields are empty — the UI renders an
/// honest "no identity" state and offers to create one via rotate.
#[command]
pub fn signing_key_status() -> SigningKeyDto {
    if !vox_identity::storage::identity_exists() {
        return SigningKeyDto {
            node_id: String::new(),
            algorithm: "ed25519".to_string(),
            fingerprint: String::new(),
            pubkey_hex: String::new(),
            present: false,
        };
    }
    // Reading the public fields requires unlocking. We cannot decrypt without the
    // master password, so without it we can only report presence. The pubkey is
    // not stored in cleartext, so a logged-out GUI shows present-but-locked.
    match read_identity_from_env() {
        Some(dto) => dto,
        None => SigningKeyDto {
            node_id: String::new(),
            algorithm: "ed25519".to_string(),
            fingerprint: "(locked — provide master password to view)".to_string(),
            pubkey_hex: String::new(),
            present: true,
        },
    }
}

/// Best-effort unlock using `VOX_IDENTITY_MASTER_PWD` (CI/headless convenience).
/// Resolved through Clavis (`SecretId::VoxIdentityMasterPwd`). Returns `None` when it is unset,
/// blank, or does not unlock the file. The password is only handed to `load_identity`; it is
/// never logged, returned, or placed in an error.
fn read_identity_from_env() -> Option<SigningKeyDto> {
    let resolved = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxIdentityMasterPwd);
    let id = vox_identity::storage::load_identity(resolved.expose()?).ok()?;
    Some(dto_from_identity(&id, true))
}

fn dto_from_identity(id: &vox_identity::NodeIdentity, present: bool) -> SigningKeyDto {
    SigningKeyDto {
        node_id: id.node_id().to_string(),
        algorithm: "ed25519".to_string(),
        fingerprint: id.fingerprint(),
        pubkey_hex: id.pubkey_hex(),
        present,
    }
}

/// Rotate the local node identity. Verifies `password` unlocks the existing
/// identity (when present), generates a fresh Ed25519 keypair, and re-seals it
/// under the same password. Returns the new public status. NEVER returns the
/// private key.
#[command]
pub fn rotate_signing_key(password: String) -> Result<SigningKeyDto, String> {
    let fresh = vox_identity::storage::rotate_identity(&password).map_err(|e| e.to_string())?;
    Ok(dto_from_identity(&fresh, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dto_from_identity_carries_public_fields_and_never_the_private_key() {
        let id = vox_identity::NodeIdentity::generate();
        let dto = dto_from_identity(&id, true);
        assert_eq!(dto.algorithm, "ed25519");
        assert_eq!(dto.node_id, id.node_id());
        assert_eq!(dto.fingerprint, id.fingerprint());
        assert_eq!(dto.pubkey_hex, id.pubkey_hex());
        assert!(dto.present);
        // SigningKeyDto has no private-key field at all — the type itself is the guarantee
        // referenced in this module's SECURITY note; this assertion pins that shape.
        assert!(!dto.pubkey_hex.is_empty());
    }

    #[test]
    fn dto_from_identity_respects_the_present_flag() {
        let id = vox_identity::NodeIdentity::generate();
        assert!(!dto_from_identity(&id, false).present);
    }
}
