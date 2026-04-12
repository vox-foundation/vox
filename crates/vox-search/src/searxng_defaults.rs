//! Embedded SSOT for SearXNG query defaults — [`contracts/scientia/searxng-query.defaults.v1.yaml`](../../../contracts/scientia/searxng-query.defaults.v1.yaml).

use serde::Deserialize;
use std::sync::OnceLock;
use tracing::debug;

const EMBEDDED_YAML: &str =
    include_str!("../../../contracts/scientia/searxng-query.defaults.v1.yaml");

/// Repo-relative path for docs and tooling messages.
pub const SEARXNG_QUERY_DEFAULTS_YAML_REPO_PATH: &str =
    "contracts/scientia/searxng-query.defaults.v1.yaml";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SearxngQueryDefaults {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default = "default_engines")]
    pub engines: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_engines() -> String {
    "google,bing,ddg".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

static EMBEDDED: OnceLock<SearxngQueryDefaults> = OnceLock::new();

pub(crate) fn embedded_searxng_query_defaults() -> &'static SearxngQueryDefaults {
    EMBEDDED.get_or_init(|| {
        let d: SearxngQueryDefaults = serde_yaml::from_str(EMBEDDED_YAML)
            .expect("embedded contracts/scientia/searxng-query.defaults.v1.yaml must parse");
        debug!(
            schema_version = d.schema_version,
            repo_path = SEARXNG_QUERY_DEFAULTS_YAML_REPO_PATH,
            engines = %d.engines,
            language = %d.language,
            "embedded SearXNG query defaults loaded"
        );
        d
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_yaml_baseline() {
        let d = embedded_searxng_query_defaults();
        assert!(d.schema_version >= 1);
        assert_eq!(d.engines, "google,bing,ddg");
        assert_eq!(d.language, "en");
    }
}
