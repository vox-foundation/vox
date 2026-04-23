//! Scientia social façade — delegates distribution planning to `vox-publisher` (single SSOT).

/// Compile syndication caps and derivation digest for a news item (no duplicate policy).
#[must_use]
pub fn compile_distribution_preview(
    item: &vox_publisher::types::UnifiedNewsItem,
) -> vox_publisher::DistributionCompileReport {
    vox_publisher::compile_for_publish(item)
}

/// Ensure embedded topic packs only reference known projection profiles (CI-style guard).
pub fn validate_topic_pack_projection_profiles() -> Result<(), String> {
    vox_publisher::distribution_compile::validate_topic_pack_projection_profiles()
}
