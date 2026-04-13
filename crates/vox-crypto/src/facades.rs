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

