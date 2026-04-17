use std::path::PathBuf;
use std::fs;
use anyhow::Result;
use argon2::Argon2;
use vox_crypto::{encrypt_with_nonce, decrypt_with_nonce, SymKey};
use rand::RngCore;
use crate::NodeIdentity;

pub fn identity_key_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".vox");
    path.push("identity.key.enc");
    path
}

fn derive_key(password: &str, salt: &[u8]) -> Result<SymKey> {
    let mut key = [0u8; 32];
    let argon2 = Argon2::default();
    argon2.hash_password_into(password.as_bytes(), salt, &mut key).map_err(|e| anyhow::anyhow!("KDF failed: {}", e))?;
    Ok(SymKey(key))
}

pub fn save_identity(identity: &NodeIdentity, password: &str) -> Result<()> {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    
    let sym_key = derive_key(password, &salt)?;
    
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    
    let raw_signing_key = identity.signing_key().inner.to_bytes();
    let ciphertext = encrypt_with_nonce(&sym_key, &nonce, &raw_signing_key)
        .map_err(|e| anyhow::anyhow!(e))?;
        
    let mut payload = Vec::new();
    payload.extend_from_slice(&salt);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);
    
    let path = identity_key_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Explicitly setting permissions to 600 would be done here for Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true).mode(0o600);
        let mut file = options.open(&path)?;
        use std::io::Write;
        file.write_all(&payload)?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, payload)?;
    }
    
    Ok(())
}

pub fn load_identity(password: &str) -> Result<NodeIdentity> {
    let path = identity_key_path();
    if !path.exists() {
        return Err(anyhow::anyhow!("Identity file not found at {:?}", path));
    }
    
    let payload = fs::read(&path)?;
    if payload.len() < 16 + 12 + 32 {
        return Err(anyhow::anyhow!("Identity file corrupted or too short"));
    }
    
    let salt = &payload[0..16];
    let nonce = &payload[16..28];
    let ciphertext = &payload[28..];
    
    let sym_key = derive_key(password, salt)?;
    
    let raw_signing_key = decrypt_with_nonce(&sym_key, nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        
    if raw_signing_key.len() != 32 {
        return Err(anyhow::anyhow!("Invalid signing key length after decryption"));
    }
    
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&raw_signing_key);
    
    let signing_key = vox_crypto::signing_key_from_bytes(&bytes);
    let verifying_key = vox_crypto::to_verifying_key(&signing_key);
    
    Ok(NodeIdentity::from_keys(signing_key, verifying_key))
}
