//! Optional BLAKE3 + Ed25519 attestation for `job_result` / `job_fail` A2A deliveries.

use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};

fn parse_blake3_hex(h: &str) -> Result<[u8; 32], String> {
    let bytes = data_encoding::HEXLOWER_PERMISSIVE
        .decode(h.as_bytes())
        .map_err(|_| "invalid payload_blake3_hex".to_string())?;
    if bytes.len() != 32 {
        return Err("payload_blake3_hex must represent 32 bytes (64 hex chars)".into());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Enforce attestation rules for [`super::A2ADeliverRequest`].
///
/// Workers sign the **raw 32-byte BLAKE3 digest** (binary), not the hex string.
pub(super) fn enforce_deliver_attestation(
    message_type: &str,
    payload: &str,
    digest_hex: Option<&str>,
    sig_b64: Option<&str>,
    verify_pk_bytes: Option<&[u8; 32]>,
) -> Result<(), String> {
    let mt = message_type.trim();
    let needs_type = mt.eq_ignore_ascii_case(super::A2A_MESSAGE_JOB_RESULT)
        || mt.eq_ignore_ascii_case(super::A2A_MESSAGE_JOB_FAIL);
    let d = digest_hex.map(str::trim).filter(|s| !s.is_empty());
    let s = sig_b64.map(str::trim).filter(|s| !s.is_empty());
    match (d, s) {
        (None, None) => Ok(()),
        (Some(_), None) | (None, Some(_)) => Err(
            "populi: payload_blake3_hex and worker_ed25519_sig_b64 must both be set or both omitted"
                .into(),
        ),
        (Some(dh), Some(sb)) => {
            if !needs_type {
                return Err(
                    "populi: attestation fields are only allowed for job_result and job_fail".into(),
                );
            }
            let vk_bytes = verify_pk_bytes.ok_or_else(|| {
                "populi: VOX_MESH_WORKER_RESULT_VERIFY_KEY is not configured (required when attestation is present)"
                    .to_string()
            })?;
            let vk = VerifyingKey::from_bytes(vk_bytes)
                .map_err(|_| "populi: invalid configured worker verify key".to_string())?;
            let digest = parse_blake3_hex(dh)?;
            let computed = *blake3::hash(payload.as_bytes()).as_bytes();
            if computed != digest {
                return Err("populi: payload_blake3_hex does not match payload".into());
            }
            let sig_bytes = base64::engine::general_purpose::STANDARD
                .decode(sb.as_bytes())
                .map_err(|_| "populi: invalid worker_ed25519_sig_b64".to_string())?;
            let sig_arr: [u8; 64] = sig_bytes
                .as_slice()
                .try_into()
                .map_err(|_| "populi: Ed25519 signature must be 64 bytes".to_string())?;
            let signature = Signature::from_bytes(&sig_arr);
            vk.verify_strict(&digest, &signature)
                .map_err(|_| "populi: worker_ed25519_sig_b64 verification failed".to_string())?;
            Ok(())
        }
    }
}
