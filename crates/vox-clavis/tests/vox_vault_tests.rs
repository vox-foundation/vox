use secrecy::ExposeSecret;
use vox_clavis::backend::vox_vault::VoxCloudBackend;
use vox_clavis::backend::SecretBackend;
use vox_clavis::spec::{SecretId, SecretSpec};

#[test]
fn test_vox_vault_encryption_decryption_cycle() {
    // If the keyring cannot be acquired in pure headless CI, VoxCloudBackend::new() returns an error.
    // So we handle the Result gracefully to ensure this test passes locally when keyring is available.
    let backend = match VoxCloudBackend::new() {
        Ok(b) => b,
        Err(_) => {
            println!("Skipping VoxCloudBackend test because keyring or db is not available");
            return;
        }
    };

    let spec = SecretSpec {
        id: SecretId::CustomOpenAiApiKey,
        canonical_env: "FAKE_TARGET_TEST",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: vox_clavis::policy::SecretPolicy::required_fail(),
        remediation: "",
    };

    let plaintext = "vox_vault_test_secret_12345";
    
    // Test write
    backend.write_secret("FAKE_TARGET_TEST", plaintext).expect("failed to write secret to vault");

    // Test read
    let resolved = backend.resolve(SecretId::CustomOpenAiApiKey, spec)
        .expect("failed to resolve secret from vault")
        .expect("secret not found after write");
    
    assert_eq!(resolved.expose_secret(), plaintext);
}
