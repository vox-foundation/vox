use crate::policy::SecretPolicy;
use crate::spec::{SecretId, SecretSpec};

pub const SPECS_IDENTITY: &[SecretSpec] = &[
    SecretSpec {
        id: SecretId::VoxIdentityKeyPath,
        canonical_env: "VOX_IDENTITY_KEY_PATH",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Run `vox auth init` or set VOX_IDENTITY_KEY_PATH.",
        scope_description: "Path to the encrypted identity key file (default: ~/.vox/identity.key.enc)",
    },
    SecretSpec {
        id: SecretId::VoxIdentityMasterPwd,
        canonical_env: "VOX_IDENTITY_MASTER_PWD",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set VOX_IDENTITY_MASTER_PWD in environment (for CI) or use OS keyring.",
        scope_description: "Master password to unlock the node identity.",
    },
];
