//! Clavis secret lifecycle evaluation, live validation probe descriptors,
//! rotation runbooks (`revoke -> rotate -> redistribute`), and pull-based
//! vault sync metadata (`CLAVIS-06..08`).

use crate::spec::{LifecycleMeta, Profile, SecretId, all_specs, provider_manifest_for};

/// Rotation health status computed from [`LifecycleMeta`] and secret age (`CLAVIS-06`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotationHealth {
    /// Key does not track automated rotation (`LifecycleMeta::MANUAL` / `CONFIG`).
    ManualOnly,
    /// Key is within its healthy rotation cadence window.
    Healthy {
        age_days: u32,
        cadence_days: u32,
        days_remaining: u32,
    },
    /// Key is approaching its rotation deadline (within `expiry_warning_days`).
    ExpiringSoon {
        age_days: u32,
        cadence_days: u32,
        days_remaining: u32,
    },
    /// Key has exceeded its `rotation_cadence_days` and should be rotated immediately.
    StaleRotation {
        age_days: u32,
        cadence_days: u32,
        overdue_days: u32,
    },
}

/// Evaluate [`RotationHealth`] for a secret given its [`LifecycleMeta`] and age in days (`CLAVIS-06`).
#[must_use]
pub const fn evaluate_lifecycle_health(meta: LifecycleMeta, age_days: u32) -> RotationHealth {
    if !meta.track_stale_rotation {
        return RotationHealth::ManualOnly;
    }
    let Some(cadence_days) = meta.rotation_cadence_days else {
        return RotationHealth::ManualOnly;
    };
    if age_days >= cadence_days {
        return RotationHealth::StaleRotation {
            age_days,
            cadence_days,
            overdue_days: age_days - cadence_days,
        };
    }
    let days_remaining = cadence_days - age_days;
    let warn_window = match meta.expiry_warning_days {
        Some(days) => days,
        None => 7,
    };
    if days_remaining <= warn_window {
        RotationHealth::ExpiringSoon {
            age_days,
            cadence_days,
            days_remaining,
        }
    } else {
        RotationHealth::Healthy {
            age_days,
            cadence_days,
            days_remaining,
        }
    }
}

/// Read-only HTTP validation probe descriptor for `vox secrets verify` (`CLAVIS-06`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationProbeSpec {
    pub provider_id: &'static str,
    pub secret_id: SecretId,
    pub endpoint_url: &'static str,
    pub method: &'static str,
    pub auth_header_name: &'static str,
    pub auth_scheme_prefix: &'static str,
}

/// Return the read-only validation probe descriptor for a [`SecretId`], if supported (`CLAVIS-06`).
#[must_use]
pub fn validation_probe_for(id: SecretId) -> Option<ValidationProbeSpec> {
    let manifest = provider_manifest_for(id);
    let endpoint_url = manifest.validation_probe_endpoint?;
    let (auth_header_name, auth_scheme_prefix) = match id {
        SecretId::AnthropicApiKey => ("x-api-key", ""),
        SecretId::GeminiApiKey => ("x-goog-api-key", ""),
        // drift-allow(drift/bearer-header-inline): static ValidationProbeSpec auth_scheme_prefix metadata in foundation vox-secrets crate
        _ => ("Authorization", "Bearer "),
    };
    Some(ValidationProbeSpec {
        provider_id: manifest.provider_id,
        secret_id: id,
        endpoint_url,
        method: "GET",
        auth_header_name,
        auth_scheme_prefix,
    })
}

/// Structured 3-step emergency/scheduled rotation runbook (`revoke -> rotate -> redistribute`, `CLAVIS-07`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationRunbook {
    pub secret_id: SecretId,
    pub canonical_env: &'static str,
    pub revoke_step: String,
    pub rotate_step: String,
    pub redistribute_step: String,
}

/// Generate the canonical `revoke -> rotate -> redistribute` runbook for `id` (`CLAVIS-07`).
#[must_use]
pub fn rotation_runbook_for(id: SecretId) -> RotationRunbook {
    let manifest = provider_manifest_for(id);
    let revoke_step = match manifest.acquisition_url {
        Some(url) => format!(
            "1. REVOKE: Open {url} and revoke the previous `{}` credential immediately.",
            manifest.canonical_env
        ),
        None => format!(
            "1. REVOKE: Invalidate the previous `{}` token in the upstream provider console.",
            manifest.canonical_env
        ),
    };
    let rotate_step = format!(
        "2. ROTATE: Store the replacement credential via `vox secrets set {} --stdin`.",
        manifest.canonical_env
    );
    let redistribute_step = if id.metadata().shareable {
        "3. REDISTRIBUTE: Run `vox secrets sync --mesh` to push the sealed ChaCha20-Poly1305 vault update to trusted mesh peers.".to_string()
    } else {
        "3. REDISTRIBUTE: Device-local secret (`shareable = false`); restart local `vox-orchestrator-d` so active workers pick up the rotated key.".to_string()
    };
    RotationRunbook {
        secret_id: id,
        canonical_env: manifest.canonical_env,
        revoke_step,
        rotate_step,
        redistribute_step,
    }
}

/// Pull-based vault synchronization manifest descriptor (`CLAVIS-08`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullSyncManifest {
    pub profile: Profile,
    pub shareable_keys: Vec<&'static str>,
}

/// Enumerate all shareable, persistable account secrets eligible for pull-based encrypted vault sync (`CLAVIS-08`).
#[must_use]
pub fn prepare_pull_sync_manifest(profile: Profile) -> PullSyncManifest {
    let shareable_keys = all_specs()
        .into_iter()
        .filter(|s| {
            let meta = s.id.metadata();
            meta.shareable && meta.persistable_account_secret
        })
        .map(|s| s.canonical_env)
        .collect();
    PullSyncManifest {
        profile,
        shareable_keys,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_health_distinguishes_healthy_expiring_and_stale() {
        assert_eq!(
            evaluate_lifecycle_health(LifecycleMeta::MANUAL, 500),
            RotationHealth::ManualOnly
        );
        assert_eq!(
            evaluate_lifecycle_health(LifecycleMeta::QUARTERLY, 30),
            RotationHealth::Healthy {
                age_days: 30,
                cadence_days: 90,
                days_remaining: 60,
            }
        );
        assert_eq!(
            evaluate_lifecycle_health(LifecycleMeta::QUARTERLY, 80),
            RotationHealth::ExpiringSoon {
                age_days: 80,
                cadence_days: 90,
                days_remaining: 10,
            }
        );
        assert_eq!(
            evaluate_lifecycle_health(LifecycleMeta::QUARTERLY, 95),
            RotationHealth::StaleRotation {
                age_days: 95,
                cadence_days: 90,
                overdue_days: 5,
            }
        );
    }

    #[test]
    fn validation_probe_and_rotation_runbook_cover_primary_providers() {
        let probe = validation_probe_for(SecretId::OpenRouterApiKey).expect("openrouter probe");
        assert_eq!(probe.endpoint_url, "https://openrouter.ai/api/v1/auth/key");
        assert_eq!(probe.auth_header_name, "Authorization");

        let runbook = rotation_runbook_for(SecretId::OpenRouterApiKey);
        assert!(runbook.revoke_step.contains("https://openrouter.ai/keys"));
        assert!(
            runbook
                .rotate_step
                .contains("vox secrets set OPENROUTER_API_KEY --stdin")
        );
    }

    #[test]
    fn pull_sync_manifest_includes_only_shareable_secrets() {
        let sync = prepare_pull_sync_manifest(Profile::Dev);
        assert!(sync.shareable_keys.contains(&"OPENROUTER_API_KEY"));
    }
}
