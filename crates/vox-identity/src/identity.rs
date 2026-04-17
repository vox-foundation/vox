use std::fmt;

use vox_crypto::{generate_signing_keypair, sign, SigningKey, VerifyingKey, secure_hash};

pub struct NodeIdentity {
    node_id: String,
    signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl fmt::Debug for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeIdentity")
            .field("node_id", &self.node_id)
            .finish_non_exhaustive()
    }
}

impl NodeIdentity {
    pub fn generate() -> Self {
        let (signing_key, verifying_key) = generate_signing_keypair();
        let pubkey_bytes = vox_crypto::verifying_key_to_bytes(&verifying_key);
        let hash = secure_hash(&pubkey_bytes);
        let node_id = hex::encode(&hash[0..16]);

        Self {
            node_id,
            signing_key,
            verifying_key,
        }
    }

    pub fn from_keys(signing_key: SigningKey, verifying_key: VerifyingKey) -> Self {
        let pubkey_bytes = vox_crypto::verifying_key_to_bytes(&verifying_key);
        let hash = secure_hash(&pubkey_bytes);
        let node_id = hex::encode(&hash[0..16]);

        Self {
            node_id,
            signing_key,
            verifying_key,
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }
    
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn sign_challenge(&self, nonce: &[u8]) -> [u8; 64] {
        sign(&self.signing_key, nonce)
    }
}
