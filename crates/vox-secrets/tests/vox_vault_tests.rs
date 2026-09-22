use secrecy::ExposeSecret;
use vox_secrets::backend::SecretBackend;
use vox_secrets::backend::vox_vault::VoxCloudBackend;
use vox_secrets::spec::{SecretId, SecretSpec};

/// This binary links `vox_secrets` as a normal (non-`cfg(test)`) dependency,
/// so the library's internal `isolate_vault_tests_from_real_home` (gated on
/// `cfg(test)` *of the library crate*) never runs here. Every test in this
/// file MUST call this first, before constructing any `VoxCloudBackend`, so
/// it never reads, writes, or corrupts a developer's real `~/.vox` vault,
/// master key, or OS keychain entry. See the doc comment on the library-side
/// twin (`crates/vox-secrets/src/backend/vox_vault.rs`) for the full D13
/// backstory: this was previously unreachable code because
/// `VoxCloudBackend::new()` always failed on this binary's plain (no
/// ambient-runtime) `#[test]` functions, so these tests silently skipped and
/// this gap went unnoticed.
fn ensure_isolated_test_home() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let home = tempfile::tempdir()
            .expect("tempdir for isolated vault test HOME")
            .keep();
        // `open_cloudless_connection` doesn't create the `.vox` parent
        // directory itself (the real `~/.vox` already exists in practice) —
        // pre-create it here so the isolated tempdir behaves the same way.
        std::fs::create_dir_all(home.join(".vox")).expect("create isolated .vox dir");
        // SAFETY: gated by `Once::call_once` — runs exactly once, before any
        // vault-touching code in this binary.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("USERPROFILE", &home);
            std::env::set_var("VOX_ACCOUNT_ID", "vox-secrets-test-isolated-account");
            std::env::set_var(
                "VOX_SECRETS_VAULT_KEYRING_SERVICE",
                "vox-secrets-vault-test-integration",
            );
        }
    });
}

/// A secret id unique to this process+test invocation, so reruns (and
/// concurrent test binaries sharing the same isolated tempdir home within a
/// single `cargo test` process) never collide on stale rows left by a prior
/// run.
fn unique_key(label: &str) -> String {
    format!(
        "{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )
}

#[test]
fn test_vox_vault_encryption_decryption_cycle() {
    ensure_isolated_test_home();
    // If the keyring cannot be acquired in pure headless CI, VoxCloudBackend::new() returns an error.
    // So we handle the Result gracefully to ensure this test passes locally when keyring is available.
    let backend = match VoxCloudBackend::new() {
        Ok(b) => b,
        Err(_) => {
            println!("Skipping VoxCloudBackend test because keyring or db is not available");
            return;
        }
    };

    let key: &str = Box::leak(unique_key("FAKE_TARGET_TEST").into_boxed_str());
    let spec = SecretSpec {
        id: SecretId::CustomOpenaiApiKey,
        canonical_env: key,
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: vox_secrets::policy::SecretPolicy::required_fail(),
        remediation: "",
        scope_description: "",
    };

    let plaintext = "vox_vault_test_secret_12345";

    // Test write
    backend
        .write_secret(key, plaintext)
        .expect("failed to write secret to vault");

    // Test read
    let resolved = backend
        .resolve(SecretId::CustomOpenaiApiKey, spec, None, "test")
        .expect("failed to resolve secret from vault")
        .expect("secret not found after write");

    assert_eq!(resolved.expose_secret(), plaintext);
}

#[test]
fn test_vox_vault_rewrap_and_backup_corruption_detection() {
    ensure_isolated_test_home();
    let backend = match VoxCloudBackend::new() {
        Ok(b) => b,
        Err(_) => {
            println!("Skipping VoxCloudBackend test because keyring or db is not available");
            return;
        }
    };
    let key: &str = Box::leak(unique_key("FAKE_TARGET_REWRAP").into_boxed_str());
    backend
        .write_secret(key, "rewrap_plaintext")
        .expect("seed secret");
    let rewrapped = backend
        .rewrap_secret(key, "kek-rotated", 2)
        .expect("rewrap call");
    assert!(rewrapped, "rewrap should mutate existing row");

    let backup = backend
        .export_account_backup(
            &std::env::var("VOX_ACCOUNT_ID").unwrap_or_else(|_| "default-account".to_string()),
        )
        .expect("export backup");
    assert!(!backup.is_empty(), "backup should include seeded row");
    let mut corrupted = backup.clone();
    let idx = corrupted
        .iter()
        .position(|r| r.secret_id == key)
        .expect("seeded row present in backup");
    corrupted[idx].ciphertext[0] ^= 0x01;
    let err = backend
        .import_account_backup(&corrupted, true)
        .expect_err("corrupted backup must fail integrity check");
    assert!(
        err.to_string().contains("checksum mismatch"),
        "error must mention checksum mismatch"
    );
}

#[test]
fn test_rewrap_rotation_across_secret_material_kinds() {
    ensure_isolated_test_home();
    let backend = match VoxCloudBackend::new() {
        Ok(b) => b,
        Err(_) => {
            println!("Skipping VoxCloudBackend test because keyring or db is not available");
            return;
        }
    };
    let cases: [(SecretId, &str, &str); 3] = [
        (
            SecretId::CustomOpenaiApiKey,
            Box::leak(unique_key("ROTATE_API_KEY_KIND").into_boxed_str()),
            "kind-api-key-value",
        ),
        (
            SecretId::VoxOpenReviewAccessToken,
            Box::leak(unique_key("ROTATE_BEARER_TOKEN_KIND").into_boxed_str()),
            "kind-bearer-token-value",
        ),
        (
            SecretId::VoxOpenReviewPassword,
            Box::leak(unique_key("ROTATE_PASSWORD_KIND").into_boxed_str()),
            "kind-password-value",
        ),
    ];
    for (id, key, value) in cases {
        backend.write_secret(key, value).expect("seed secret");
        let rotated = backend
            .rewrap_secret(key, "kek-rotation-suite", 3)
            .expect("rewrap");
        assert!(rotated);
        let spec = SecretSpec {
            id,
            canonical_env: key,
            aliases: &[],
            deprecated_aliases: &[],
            backend_key: None,
            auth_registry: None,
            policy: vox_secrets::policy::SecretPolicy::required_fail(),
            remediation: "",
            scope_description: "",
        };
        let resolved = backend
            .resolve(id, spec, None, "test")
            .expect("resolve after rewrap")
            .expect("secret exists");
        assert_eq!(resolved.expose_secret(), value);
    }
}

/// Regression test for the `write_secret_v2` rotation_epoch bug found while
/// fixing D13: the `INSERT ... ON CONFLICT DO UPDATE` previously left
/// `rotation_epoch`/`rotated_at_ms`/`consistency_version` at whatever a prior
/// `rewrap_secret` call had set them to, while the checksum written on that
/// same UPDATE was always computed assuming a fresh row (rotation_epoch=0).
/// So: write -> rewrap (rotation_epoch becomes 1) -> write again (checksum
/// computed with 0, row still has rotation_epoch=1) -> any subsequent
/// checksum verification (rewrap or resolve) spuriously fails.
#[test]
fn write_after_rewrap_then_rewrap_again_does_not_corrupt_checksum() {
    ensure_isolated_test_home();
    let backend = match VoxCloudBackend::new() {
        Ok(b) => b,
        Err(_) => {
            println!("Skipping VoxCloudBackend test because keyring or db is not available");
            return;
        }
    };
    let key: &str = Box::leak(unique_key("ROTATION_EPOCH_RESET").into_boxed_str());

    backend.write_secret(key, "v1").expect("seed v1");
    let rewrapped_once = backend
        .rewrap_secret(key, "kek-epoch-reset", 2)
        .expect("first rewrap");
    assert!(rewrapped_once, "first rewrap should mutate the row");

    // A plain write after a rewrap is new key material under the current
    // KEK; the old rotation count no longer describes anything.
    backend.write_secret(key, "v2").expect("overwrite with v2");

    let spec = SecretSpec {
        id: SecretId::CustomOpenaiApiKey,
        canonical_env: key,
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: vox_secrets::policy::SecretPolicy::required_fail(),
        remediation: "",
        scope_description: "",
    };
    let resolved = backend
        .resolve(SecretId::CustomOpenaiApiKey, spec, None, "test")
        .expect("resolve after overwrite must not report a checksum mismatch")
        .expect("secret exists");
    assert_eq!(resolved.expose_secret(), "v2");

    let rewrapped_again = backend
        .rewrap_secret(key, "kek-epoch-reset-2", 3)
        .expect("second rewrap must not report a checksum mismatch");
    assert!(rewrapped_again, "second rewrap should mutate the row");
}
