//! Which MENS Candle plugin services a request on this host.
//!
//! One selector for training, serving, `merge-qlora` and `eval-local`, so
//! they cannot disagree about the backend. An explicit `--device` always
//! wins (the plugin then reports a clear error if the host can't do it).
//! `Best` prefers CUDA, then Metal, then the CPU plugin for this OS. `Cpu`
//! reuses the CUDA plugin on an NVIDIA host (it runs on CPU without touching
//! CUDA), else the CPU plugin for this OS.
//!
//! The `requires-tag` strings here mirror `vox-plugin-catalog/catalog.toml`;
//! `catalog_validation.rs` pins them on the catalog side.

use vox_plugin_host::CapabilitySet;

use crate::mens::tensor::device::DeviceKind;

/// CUDA build of the Candle plugin (`requires-tag = "nvidia-gpu"`).
const MENS_CANDLE_CUDA: &str = "mens-candle-cuda";
/// Metal build of the Candle plugin (`requires-tag = "metal"`); also runs on CPU.
const MENS_CANDLE_METAL: &str = "mens-candle-metal";
/// CPU-only build for non-Mac hosts without an NVIDIA GPU.
const MENS_CANDLE_CPU: &str = "mens-candle-cpu";

/// Whether a release asset of `plugin_id` exists for the platform this binary
/// was built for. `vox plugin install`/auto-heal can only fetch these; anything
/// else must be built from source.
#[must_use]
pub fn has_prebuilt_artifact(plugin_id: &str) -> bool {
    has_prebuilt_artifact_for(plugin_id, std::env::consts::OS, std::env::consts::ARCH)
}

// Mirrors the release jobs in .github/workflows/release-binaries.yml.
fn has_prebuilt_artifact_for(plugin_id: &str, os: &str, arch: &str) -> bool {
    match plugin_id {
        MENS_CANDLE_METAL => os == "macos",
        MENS_CANDLE_CUDA | MENS_CANDLE_CPU => os == "linux" && arch == "x86_64",
        _ => false,
    }
}

/// Pick the plugin id for `requested` on a host with capabilities `caps`.
#[must_use]
pub fn select_mens_backend(requested: DeviceKind, caps: &CapabilitySet) -> &'static str {
    select_for_os(requested, caps, cfg!(target_os = "macos"))
}

fn select_for_os(requested: DeviceKind, caps: &CapabilitySet, is_macos: bool) -> &'static str {
    let cpu_plugin = if is_macos {
        MENS_CANDLE_METAL
    } else {
        MENS_CANDLE_CPU
    };
    match requested {
        DeviceKind::Cuda => MENS_CANDLE_CUDA,
        DeviceKind::Metal => MENS_CANDLE_METAL,
        DeviceKind::Cpu if caps.satisfies(Some("nvidia-gpu")) => MENS_CANDLE_CUDA,
        DeviceKind::Cpu => cpu_plugin,
        DeviceKind::Best => {
            if caps.satisfies(Some("nvidia-gpu")) {
                MENS_CANDLE_CUDA
            } else if caps.satisfies(Some("metal")) {
                MENS_CANDLE_METAL
            } else {
                cpu_plugin
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(tags: &[&str]) -> CapabilitySet {
        CapabilitySet::from_tags(tags.iter().copied())
    }

    #[test]
    fn best_prefers_cuda_on_an_nvidia_host() {
        let c = caps(&["cpu-only", "nvidia-gpu"]);
        assert_eq!(select_for_os(DeviceKind::Best, &c, false), MENS_CANDLE_CUDA);
    }

    #[test]
    fn best_uses_metal_on_any_mac() {
        let c = caps(&["cpu-only", "metal"]);
        assert_eq!(select_for_os(DeviceKind::Best, &c, true), MENS_CANDLE_METAL);
    }

    #[test]
    fn best_falls_back_to_the_cpu_plugin_off_mac() {
        let c = caps(&["cpu-only"]);
        assert_eq!(select_for_os(DeviceKind::Best, &c, false), MENS_CANDLE_CPU);
    }

    #[test]
    fn cpu_on_a_mac_uses_the_metal_plugin() {
        let c = caps(&["cpu-only", "metal", "apple-silicon"]);
        assert_eq!(select_for_os(DeviceKind::Cpu, &c, true), MENS_CANDLE_METAL);
    }

    #[test]
    fn cpu_on_an_nvidia_host_reuses_the_cuda_plugin() {
        // The CUDA plugin maps DeviceKind::Cpu to Device::Cpu without touching
        // CUDA; don't make the user install a second plugin.
        let c = caps(&["cpu-only", "nvidia-gpu"]);
        assert_eq!(select_for_os(DeviceKind::Cpu, &c, false), MENS_CANDLE_CUDA);
    }

    #[test]
    fn cpu_off_mac_without_a_gpu_uses_the_cpu_plugin() {
        let c = caps(&["cpu-only"]);
        assert_eq!(select_for_os(DeviceKind::Cpu, &c, false), MENS_CANDLE_CPU);
    }

    #[test]
    fn prebuilt_artifacts_match_the_release_jobs() {
        assert!(has_prebuilt_artifact_for(
            MENS_CANDLE_METAL,
            "macos",
            "x86_64"
        ));
        assert!(has_prebuilt_artifact_for(
            MENS_CANDLE_CPU,
            "linux",
            "x86_64"
        ));
        assert!(has_prebuilt_artifact_for(
            MENS_CANDLE_CUDA,
            "linux",
            "x86_64"
        ));
        assert!(!has_prebuilt_artifact_for(
            MENS_CANDLE_CPU,
            "windows",
            "x86_64"
        ));
        assert!(!has_prebuilt_artifact_for(
            MENS_CANDLE_CPU,
            "linux",
            "aarch64"
        ));
        assert!(!has_prebuilt_artifact_for(
            MENS_CANDLE_CUDA,
            "macos",
            "aarch64"
        ));
        assert!(!has_prebuilt_artifact_for(
            MENS_CANDLE_METAL,
            "linux",
            "x86_64"
        ));
        assert!(!has_prebuilt_artifact_for(
            "some-other-plugin",
            "linux",
            "x86_64"
        ));
    }

    #[test]
    fn an_explicit_device_is_honoured_even_without_the_capability() {
        let c = caps(&["cpu-only"]);
        assert_eq!(select_for_os(DeviceKind::Cuda, &c, false), MENS_CANDLE_CUDA);
        assert_eq!(
            select_for_os(DeviceKind::Metal, &c, false),
            MENS_CANDLE_METAL
        );
    }

    /// The public entry point on a real Mac: probe + OS detection end to end.
    #[test]
    #[cfg(target_os = "macos")]
    fn this_mac_selects_metal_and_can_fetch_it() {
        let id = select_mens_backend(DeviceKind::Best, &vox_plugin_host::probe());
        assert_eq!(id, MENS_CANDLE_METAL);
        assert!(has_prebuilt_artifact(id));
    }
}
