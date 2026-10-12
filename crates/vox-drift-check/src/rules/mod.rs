use crate::features::ExtractedFeatures;
use crate::layers_manifest::LayersManifest;
use std::path::PathBuf;
use vox_code_audit::rules::{Finding, Language, Severity};

pub mod bearer_header;
pub mod reqwest_bypass;
pub mod serde_default_dup;
pub mod timeout_literal;
pub mod version_string;
pub mod vox_dir_unanchored;
pub mod vox_path_literal;

pub struct WorkspaceContext {
    pub workspace_version: String,
    pub workspace_root: PathBuf,
    /// Parsed `docs/src/architecture/layers.toml` — single source of truth for
    /// per-crate attributes like `sibling_of`. Empty when the file is missing
    /// (rules degrade gracefully to "no siblings declared").
    pub layers: LayersManifest,
}

/// Integration-test sources (`tests/` dirs, `*_test.rs`). Rules about
/// operator-facing constants skip these: a test's own wait/timeout literal is
/// the clearest form there and is not a config surface. In-file
/// `#[cfg(test)]` modules are not detected — they use `// drift-allow(...)`.
pub(crate) fn is_test_file(p: &std::path::Path) -> bool {
    let s = p.to_string_lossy();
    s.contains("/tests/") || s.contains("\\tests\\") || s.ends_with("_test.rs")
}

pub trait DriftRule: Send + Sync {
    fn id(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn languages(&self) -> &[Language];
    fn check(&self, features: &ExtractedFeatures, ctx: &WorkspaceContext) -> Vec<Finding>;
}

pub fn all_drift_rules() -> Vec<Box<dyn DriftRule>> {
    vec![
        Box::new(reqwest_bypass::ReqwestBypassRule),
        Box::new(vox_path_literal::VoxPathLiteralRule),
        Box::new(timeout_literal::TimeoutLiteralRule),
        Box::new(serde_default_dup::SerdeDefaultDupRule),
        Box::new(version_string::VersionStringRule),
        Box::new(bearer_header::BearerHeaderRule),
        Box::new(vox_dir_unanchored::VoxDirUnanchoredRule),
    ]
}

#[cfg(test)]
mod tests {
    use super::is_test_file;
    use std::path::Path;

    #[test]
    fn is_test_file_matches_integration_tests_only() {
        assert!(is_test_file(Path::new("./crates/vox-db/tests/lock.rs")));
        assert!(is_test_file(Path::new(r"crates\vox-db\tests\lock.rs")));
        assert!(is_test_file(Path::new("crates/vox-db/src/lock_test.rs")));
        assert!(!is_test_file(Path::new("crates/vox-db/src/lock.rs")));
        assert!(!is_test_file(Path::new("crates/vox-db/src/tests_util.rs")));
    }
}
