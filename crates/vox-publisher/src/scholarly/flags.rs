//! Kill-switches and coarse feature gates for scholarly adapters (`VOX_SCHOLARLY_*`).

#[must_use]
fn env_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on")
        })
        .unwrap_or(false)
}

/// When set, no live scholarly adapter runs (local/echo still allowed — they do not perform I/O).
#[must_use]
pub fn scholarly_live_globally_disabled() -> bool {
    env_truthy("VOX_SCHOLARLY_DISABLE_LIVE")
}

#[must_use]
pub fn scholarly_globally_disabled() -> bool {
    env_truthy("VOX_SCHOLARLY_DISABLE")
}

#[must_use]
pub fn adapter_live_disabled(adapter: &str) -> bool {
    let a = adapter.trim().to_ascii_lowercase();
    match a.as_str() {
        "zenodo" => env_truthy("VOX_SCHOLARLY_DISABLE_ZENODO"),
        "openreview" => env_truthy("VOX_SCHOLARLY_DISABLE_OPENREVIEW"),
        _ => false,
    }
}

#[must_use]
pub fn zenodo_use_sandbox() -> bool {
    env_truthy("VOX_ZENODO_SANDBOX")
}

/// Upload `manifest.body_markdown` as `body.md` to the deposition file bucket after draft creation.
#[must_use]
pub fn zenodo_attach_manifest_body() -> bool {
    env_truthy("VOX_ZENODO_ATTACH_MANIFEST_BODY")
}

/// Call Zenodo `publish` on the deposition after optional file attach (requires
/// [`zenodo_attach_manifest_body`] because Zenodo rejects publish with zero files).
#[must_use]
pub fn zenodo_publish_deposition() -> bool {
    env_truthy("VOX_ZENODO_PUBLISH_DEPOSITION")
}
