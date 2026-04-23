use rand::RngCore;
use vox_crypto::{VerifyingKey, verify};

pub fn generate_challenge() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce
}

pub fn verify_challenge_response(
    verifying_key: &VerifyingKey,
    nonce: &[u8; 32],
    signature: &[u8; 64],
) -> bool {
    // Basic verification of the nonce
    // In a real protocol, the message might include a timestamp or context
    verify(verifying_key, nonce, signature)
}
