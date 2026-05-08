//! Kill-switches and coarse feature gates for scholarly adapters (`VOX_SCHOLARLY_*`).

#[must_use]
fn env_truthy(id: vox_secrets::SecretId) -> bool {
    vox_secrets::resolve_secret(id)
        .expose()
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on")
        })
        .unwrap_or(false)
}

/// When set, no live scholarly adapter runs (local/echo still allowed — they do not perform I/O).
#[must_use]
pub fn scholarly_live_globally_disabled() -> bool {
    env_truthy(vox_secrets::SecretId::VoxScholarlyDisableLive)
}

#[must_use]
pub fn scholarly_globally_disabled() -> bool {
    env_truthy(vox_secrets::SecretId::VoxScholarlyDisable)
}

#[must_use]
pub fn adapter_live_disabled(adapter: &str) -> bool {
    let a = adapter.trim().to_ascii_lowercase();
    match a.as_str() {
        "zenodo" => env_truthy(vox_secrets::SecretId::VoxScholarlyDisableZenodo),
        "openreview" => env_truthy(vox_secrets::SecretId::VoxScholarlyDisableOpenReview),
        _ => false,
    }
}

#[must_use]
pub fn zenodo_use_sandbox() -> bool {
    env_truthy(vox_secrets::SecretId::VoxZenodoSandbox)
}

/// Upload `manifest.body_markdown` as `body.md` to the deposition file bucket after draft creation.
#[must_use]
pub fn zenodo_attach_manifest_body() -> bool {
    env_truthy(vox_secrets::SecretId::VoxZenodoAttachManifestBody)
}

/// Call Zenodo `publish` on the deposition after optional file attach (requires
/// [`zenodo_attach_manifest_body`] because Zenodo rejects publish with zero files).
#[must_use]
pub fn zenodo_publish_deposition() -> bool {
    env_truthy(vox_secrets::SecretId::VoxZenodoPublishDeposition)
}

/// Force draft-only behavior: never call publish (overrides [`zenodo_publish_deposition`] and
/// [`zenodo_publish_now_profile`]).
#[must_use]
pub fn zenodo_draft_only() -> bool {
    env_truthy(vox_secrets::SecretId::VoxZenodoDraftOnly)
}

/// Convenience profile: attach `body.md` and publish when the deposition is otherwise valid.
/// Still respects [`zenodo_draft_only`] when set.
#[must_use]
pub fn zenodo_publish_now_profile() -> bool {
    env_truthy(vox_secrets::SecretId::VoxZenodoPublishNow)
}

/// Directory written by `publication-scholarly-staging-export` (Zenodo layout). When set, Zenodo
/// submit uploads existing files from this tree (see [`zenodo_upload_allowlist`]).
#[must_use]
pub fn zenodo_staging_dir() -> Option<std::path::PathBuf> {
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxZenodoStagingDir)
        .expose()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
}

/// Comma-separated relative names to upload from [`zenodo_staging_dir`] (e.g. `body.md,zenodo.json`).
/// When empty, uploads every file that exists from the Zenodo [`crate::submission::staging_artifacts`] plan
/// except `arxiv_bundle.tar.gz` / `arxiv_handoff.json`.
#[must_use]
pub fn zenodo_upload_allowlist() -> Vec<String> {
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxZenodoUploadAllowlist)
        .expose()
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// When uploading from staging, require `staging_checksums.json` and match SHA-256 per file.
#[must_use]
pub fn zenodo_verify_staging_checksums() -> bool {
    env_truthy(vox_secrets::SecretId::VoxZenodoVerifyStagingChecksums)
}

/// Before deposit create, require `zenodo.json` in [`zenodo_staging_dir`] to list a metadata title
/// matching the manifest title (normalization: trim / collapse ASCII space).
#[must_use]
pub fn zenodo_require_metadata_title_parity() -> bool {
    env_truthy(vox_secrets::SecretId::VoxZenodoRequireMetadataParity)
}
