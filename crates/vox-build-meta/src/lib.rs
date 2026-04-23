pub const FEATURES_JSON: &str = env!("VOX_BUILD_FEATURES");

pub fn active_features() -> Vec<&'static str> {
    serde_json::from_str(FEATURES_JSON).unwrap_or_default()
}

pub fn has(feature: &str) -> bool {
    active_features().contains(&feature)
}

#[derive(Debug, thiserror::Error)]
#[error(
    "This vox binary was not built with the '{feature}' feature.\n\nTo enable it, rebuild with:\n\n  {rebuild_cmd}\n\nSee: docs/src/reference/feature-builds.md"
)]
pub struct FeatureMissingError {
    pub feature: &'static str,
    pub rebuild_cmd: &'static str,
}

pub fn require(
    feature: &'static str,
    rebuild_cmd: &'static str,
) -> Result<(), FeatureMissingError> {
    if has(feature) {
        Ok(())
    } else {
        Err(FeatureMissingError {
            feature,
            rebuild_cmd,
        })
    }
}
