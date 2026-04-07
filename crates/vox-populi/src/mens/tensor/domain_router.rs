use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A registry mapping logical domains (e.g., 'rust-expert', 'vox-lang') to their
/// compiled adapter weights on disk, enabling multi-LoRA inference multiplexing.
#[derive(Debug, Clone, Default)]
pub struct DomainRouter {
    adapters: HashMap<String, PathBuf>,
}

impl DomainRouter {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Registers a domain with its compiled artifact path.
    pub fn register(&mut self, domain: &str, adapter_path: impl AsRef<Path>) {
        self.adapters
            .insert(domain.to_string(), adapter_path.as_ref().to_path_buf());
    }

    /// Returns the adapter path for the given domain, if registered.
    pub fn route(&self, domain: &str) -> Option<&PathBuf> {
        self.adapters.get(domain)
    }

    /// Attempts to auto-discover adapters in the given artifacts directory.
    /// Expects directories matching domain names (e.g., `artifacts/vox-lang/adapter_model.safetensors`).
    pub fn discover(artifacts_dir: impl AsRef<Path>) -> Result<Self> {
        let mut router = Self::new();
        let artifacts_dir = artifacts_dir.as_ref();

        if !artifacts_dir.exists() {
            return Ok(router);
        }

        for entry in std::fs::read_dir(artifacts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let adapter_file = path.join("adapter_model.safetensors");
                    if adapter_file.exists() {
                        router.register(name, adapter_file);
                    }
                }
            }
        }

        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_router() {
        let mut router = DomainRouter::new();
        router.register("rust-expert", "/fake/path/adapter_model.safetensors");
        assert!(router.route("rust-expert").is_some());
        assert!(router.route("rocks").is_none());
    }
}
