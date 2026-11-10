use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogTier {
    Recommended,
    Advanced,
    FeatureGated,
}

pub fn tier_for_path(path: &[String], feature_gated: bool) -> CatalogTier {
    if feature_gated {
        return CatalogTier::FeatureGated;
    }
    let top = path.first().map(String::as_str).unwrap_or_default();
    if path.len() == 1
        && matches!(
            top,
            "build" | "check" | "run" | "test" | "bundle" | "dev" | "doctor" | "completions"
        )
    {
        return CatalogTier::Recommended;
    }
    CatalogTier::Advanced
}
