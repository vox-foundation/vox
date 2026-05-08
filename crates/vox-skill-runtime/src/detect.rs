//! Runtime preference detection for skill sandboxing.
//!
//! Determines which `SkillRuntime` implementation should be used for a given skill
//! based on the configured preference and environment. Runtime implementations are
//! shipped as plugins and dispatched through the plugin host.

use crate::runtime::SkillRuntime;

/// Preferred skill runtime selection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimePreference {
    /// WASM (wasmtime) — default for pure-compute skills with no subprocess/GPU needs.
    /// Fastest cold start (~µs), smallest footprint (~5MB), no external daemon.
    #[default]
    Wasm,
    /// Auto-detect: prefer WASM if available, fall back to Podman then Docker.
    Auto,
    /// Docker container runtime only.
    Docker,
    /// Podman container runtime only (rootless, daemonless).
    Podman,
}

impl std::str::FromStr for RuntimePreference {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wasm" => Ok(Self::Wasm),
            "auto" => Ok(Self::Auto),
            "docker" => Ok(Self::Docker),
            "podman" => Ok(Self::Podman),
            other => anyhow::bail!(
                "Unknown runtime preference: {other:?}. Use wasm, auto, docker, or podman."
            ),
        }
    }
}

/// Detect and return the best available skill runtime for the given preference.
///
/// Currently returns `None` because runtime implementations are shipped as plugins
/// and instantiated through the plugin host. This function acts as a placeholder
/// for the dispatch surface — once `vox-plugin-runtime-container` and
/// `vox-plugin-runtime-wasm` are loaded by the plugin host, the host wires the
/// runtime implementations to this interface.
///
/// # TODO
/// Replace this stub with plugin-host dispatch once the runtime plugin extension
/// point (`SkillRuntimeProvider`) is wired up in vox-plugin-host.
///
/// # Examples
///
/// ```no_run
/// use vox_skill_runtime::detect::{RuntimePreference, detect_runtime};
/// // Returns None until plugin-host dispatch is wired:
/// let _runtime = detect_runtime(RuntimePreference::Auto);
/// ```
pub fn detect_runtime(_preference: RuntimePreference) -> Option<Box<dyn SkillRuntime>> {
    // TODO: dispatch via plugin host once SkillRuntimeProvider extension point is registered.
    // The plugin host loads vox-plugin-runtime-wasm and vox-plugin-runtime-container;
    // this function should enumerate loaded providers and return the best match.
    tracing::warn!(
        "detect_runtime: SkillRuntimeProvider plugin dispatch not yet wired; \
         returning None. Install vox-plugin-runtime-wasm or vox-plugin-runtime-container."
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_preference_from_str() {
        assert_eq!(
            "wasm".parse::<RuntimePreference>().unwrap(),
            RuntimePreference::Wasm
        );
        assert_eq!(
            "auto".parse::<RuntimePreference>().unwrap(),
            RuntimePreference::Auto
        );
        assert_eq!(
            "docker".parse::<RuntimePreference>().unwrap(),
            RuntimePreference::Docker
        );
        assert_eq!(
            "podman".parse::<RuntimePreference>().unwrap(),
            RuntimePreference::Podman
        );
        assert!("invalid".parse::<RuntimePreference>().is_err());
    }
}
