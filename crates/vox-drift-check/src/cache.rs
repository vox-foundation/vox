use std::path::PathBuf;
use anyhow::Result;
use sha2::{Sha256, Digest};
use crate::features::ExtractedFeatures;

pub struct FeatureCache {
    dir: PathBuf,
}

impl FeatureCache {
    pub fn new(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).ok();
        Self { dir }
    }

    pub fn from_workspace(root: &std::path::Path) -> Self {
        Self::new(root.join(".vox/cache/drift"))
    }

    pub fn hash_file(content: &str) -> String {
        let mut h = Sha256::new();
        h.update(content.as_bytes());
        format!("{:x}", h.finalize())
    }

    pub fn store(&self, key: &str, features: &ExtractedFeatures) -> Result<()> {
        let path = self.dir.join(format!("{}.bin", &key[..16.min(key.len())]));
        let bytes = bincode::serde::encode_to_vec(features, bincode::config::standard())?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn load(&self, key: &str) -> Option<ExtractedFeatures> {
        let path = self.dir.join(format!("{}.bin", &key[..16.min(key.len())]));
        let bytes = std::fs::read(path).ok()?;
        bincode::serde::decode_from_slice::<ExtractedFeatures, _>(&bytes, bincode::config::standard())
            .ok()
            .map(|(f, _)| f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_code_audit::rules::Language;
    use crate::features::{LiteralLoc, Loc, LiteralContext};

    #[test]
    fn cache_round_trips_features() {
        let dir = tempfile::TempDir::new().unwrap();
        let cache = FeatureCache::new(dir.path().to_path_buf());

        let mut f = ExtractedFeatures::new(std::path::PathBuf::from("test.rs"), Language::Rust);
        f.string_literals.push(LiteralLoc { value: "hi".into(), loc: Loc::default(), ctx: LiteralContext::Code });

        let key = "abc123deadbeef0000000000";
        cache.store(key, &f).unwrap();
        let loaded = cache.load(key).unwrap();
        assert_eq!(loaded.string_literals[0].value, "hi");
    }

    #[test]
    fn hash_file_is_deterministic() {
        let h1 = FeatureCache::hash_file("hello world");
        let h2 = FeatureCache::hash_file("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn load_returns_none_for_missing_key() {
        let dir = tempfile::TempDir::new().unwrap();
        let cache = FeatureCache::new(dir.path().to_path_buf());
        assert!(cache.load("nonexistentkey00").is_none());
    }
}
