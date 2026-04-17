use blake3;
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use sha3::{Digest, Sha3_256};
use xxhash_rust::xxh3::xxh3_64;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure cryptographic hash (BLAKE3)
pub fn secure_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Fast generic hashing for caches (XXH3)
pub fn fast_hash(data: &[u8]) -> u64 {
    xxh3_64(data)
}

/// Compliance / standardized hash (SHA-3 256)
pub fn compliance_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct SymKey(pub [u8; 32]);

pub fn generate_sym_key() -> SymKey {
    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key);
    SymKey(key)
}

pub fn encrypt(key: &SymKey, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(&key.0.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits; unique per message
    
    let ciphertext = cipher.encrypt(&nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;
        
    let mut output = nonce.to_vec();
    output.extend(ciphertext);
    Ok(output)
}

pub fn decrypt(key: &SymKey, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.len() < 12 {
        return Err("Ciphertext too short".into());
    }
    let (nonce_bytes, payload) = ciphertext.split_at(12);
    decrypt_with_nonce(key, nonce_bytes, payload)
}

pub fn encrypt_with_nonce(key: &SymKey, nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(&key.0.into());
    let nonce = Nonce::from_slice(nonce);
    
    cipher.encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))
}

pub fn decrypt_with_nonce(key: &SymKey, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(&key.0.into());
    let nonce = Nonce::from_slice(nonce);
    
    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))
}

// --- Ed25519 Signing ---

pub struct SigningKey {
    pub inner: ed25519_dalek::SigningKey,
}

#[derive(Clone, PartialEq, Eq)]
pub struct VerifyingKey {
    pub inner: ed25519_dalek::VerifyingKey,
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningKey").finish_non_exhaustive()
    }
}

impl std::fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifyingKey").finish_non_exhaustive()
    }
}

pub fn generate_signing_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = rand::rngs::OsRng;
    let signing_key = ed25519_dalek::SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    (
        SigningKey { inner: signing_key },
        VerifyingKey { inner: verifying_key },
    )
}

pub fn sign(key: &SigningKey, message: &[u8]) -> [u8; 64] {
    use ed25519_dalek::Signer;
    key.inner.sign(message).to_bytes()
}

pub fn verify(key: &VerifyingKey, message: &[u8], sig: &[u8; 64]) -> bool {
    use ed25519_dalek::Verifier;
    let signature = ed25519_dalek::Signature::from_bytes(sig);
    key.inner.verify(message, &signature).is_ok()
}

pub fn signing_key_from_bytes(bytes: &[u8; 32]) -> SigningKey {
    SigningKey {
        inner: ed25519_dalek::SigningKey::from_bytes(bytes),
    }
}

pub fn to_verifying_key(signing_key: &SigningKey) -> VerifyingKey {
    VerifyingKey {
        inner: ed25519_dalek::VerifyingKey::from(&signing_key.inner),
    }
}

pub fn verifying_key_to_bytes(key: &VerifyingKey) -> [u8; 32] {
    key.inner.to_bytes()
}

pub fn verifying_key_from_bytes(bytes: &[u8; 32]) -> Result<VerifyingKey, String> {
    ed25519_dalek::VerifyingKey::from_bytes(bytes)
        .map(|k| VerifyingKey { inner: k })
        .map_err(|e| format!("Invalid verifying key: {}", e))
}

/// Verify a signature against a hex-encoded public key and signature.
pub fn verify_signature_hex(
    pubkey_hex: &str,
    message: &[u8],
    signature_hex: &str,
) -> Result<bool, String> {
    let pk_bytes = hex::decode(pubkey_hex).map_err(|e| e.to_string())?;
    let sig_bytes = hex::decode(signature_hex).map_err(|e| e.to_string())?;

    if pk_bytes.len() != 32 {
        return Err("Invalid public key length (expected 32 bytes)".into());
    }
    if sig_bytes.len() != 64 {
        return Err("Invalid signature length (expected 64 bytes)".into());
    }

    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&pk_bytes);
    let pk = verifying_key_from_bytes(&pk_arr)?;

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);

    Ok(verify(&pk, message, &sig_arr))
}


