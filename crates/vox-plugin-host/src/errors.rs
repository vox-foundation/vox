use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
#[error(
    "This Vox feature requires the '{plugin_id}' plugin (extension point '{extension_point}'), which is not installed.\n\nTo install it, run:\n\n  vox plugin install {plugin_id}\n\nSee: docs/src/reference/plugins.md"
)]
pub struct PluginMissingError {
    pub plugin_id: &'static str,
    pub extension_point: &'static str,
}

#[derive(Debug, Error)]
#[error(
    "Skill '{skill_id}' is not installed.\n\nTo install it, run:\n\n  vox plugin install {skill_id}"
)]
pub struct SkillNotInstalledError {
    pub skill_id: String,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("plugin manifest at {path:?} failed to parse: {source}")]
    ManifestParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("plugin dylib at {path:?} failed to dlopen: {source}")]
    DlopenFailed {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },
    #[error("plugin '{0}' has mismatched ABI: {0:?}")]
    AbiMismatch(AbiMismatchError),
    #[error("plugin init returned an error: {0}")]
    InitFailed(String),
    #[error("io error reading {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
#[error("plugin '{id}' has ABI version {plugin_abi}, host expects {host_abi}")]
pub struct AbiMismatchError {
    pub id: String,
    pub plugin_abi: u32,
    pub host_abi: u32,
}
