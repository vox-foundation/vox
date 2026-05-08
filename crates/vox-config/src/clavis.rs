//! Clavis-first resolution for routing and inference strings.
//!
//! Use [`clavis_str`] instead of `std::env::var` so values resolve from the same plane as
//! `vox clavis login` / cloud vault when configured.

/// Resolve a non-empty string secret (env, vault, auth.json per Clavis policy).
#[must_use]
pub fn clavis_str(id: vox_secrets::SecretId) -> Option<String> {
    vox_secrets::resolve_secret(id)
        .expose()
        .filter(|s| !s.trim().is_empty())
        .map(std::string::ToString::to_string)
}
