//! # vox-plugin-runtime-container
//!
//! Skill-runtime plugin providing Docker and Podman backends.
//!
//! Implements [`vox_skill_runtime::SkillRuntime`] for both Docker and Podman,
//! and also implements [`vox_container::ContainerRuntime`] (OCI build/push/tag/login)
//! for use by `vox deploy`.
//!
//! Registered as a Vox plugin; loaded by the plugin host on demand.
//! Install with: `vox plugin install runtime-container`

use abi_stable::{
    export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, std_types::*,
};
use vox_plugin_api::abi::{
    VoxPlugin, VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef,
};
use vox_plugin_api::host::VoxHost_TO;
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;
use vox_skill_runtime::{BuildOpts as SkillBuildOpts, RunOpts as SkillRunOpts, RunOutcome, SkillRuntime};
use vox_container::ContainerRuntime;

pub mod docker;
pub mod podman;

// ─── SkillRuntime impls ──────────────────────────────────────────────────────

impl SkillRuntime for docker::DockerRuntime {
    fn name(&self) -> &str {
        "docker"
    }

    fn available(&self) -> bool {
        vox_container::ContainerRuntime::available(self)
    }

    fn build(&self, _opts: &SkillBuildOpts) -> anyhow::Result<()> {
        // For skill builds via Docker: build the OCI image from context_dir.
        // Full wiring is done via SandboxedSkillRunner in vox-skills.
        // This stub satisfies the SkillRuntime interface; the actual image build
        // goes through vox_container::ContainerRuntime::build().
        tracing::info!(target: "container-runtime", "DockerRuntime::SkillRuntime::build (delegated via ContainerRuntime)");
        Ok(())
    }

    fn run(&self, opts: &SkillRunOpts) -> anyhow::Result<RunOutcome> {
        // Skill execution via Docker: run the skill inside the vox-skill-sandbox OCI image.
        // The SandboxedSkillRunner in vox-skills handles the full docker run args.
        // This stub returns a not-yet-implemented error; the real path is through
        // SandboxedSkillRunner.run() which calls Command::new("docker") directly.
        tracing::warn!(
            target: "container-runtime",
            "DockerRuntime::SkillRuntime::run not yet fully wired via SkillRuntime trait; \
             use SandboxedSkillRunner directly for container skill execution"
        );
        anyhow::bail!(
            "DockerRuntime::SkillRuntime::run: not yet wired. Use SandboxedSkillRunner. \
             artifact={:?}", opts.artifact_path
        )
    }
}

impl SkillRuntime for podman::PodmanRuntime {
    fn name(&self) -> &str {
        "podman"
    }

    fn available(&self) -> bool {
        vox_container::ContainerRuntime::available(self)
    }

    fn build(&self, _opts: &SkillBuildOpts) -> anyhow::Result<()> {
        tracing::info!(target: "container-runtime", "PodmanRuntime::SkillRuntime::build (delegated via ContainerRuntime)");
        Ok(())
    }

    fn run(&self, opts: &SkillRunOpts) -> anyhow::Result<RunOutcome> {
        tracing::warn!(
            target: "container-runtime",
            "PodmanRuntime::SkillRuntime::run not yet fully wired via SkillRuntime trait; \
             use SandboxedSkillRunner directly for container skill execution"
        );
        anyhow::bail!(
            "PodmanRuntime::SkillRuntime::run: not yet wired. Use SandboxedSkillRunner. \
             artifact={:?}", opts.artifact_path
        )
    }
}

// ─── Plugin scaffold ─────────────────────────────────────────────────────────

#[export_root_module]
fn root_module() -> VoxPluginRootRef {
    VoxPluginRoot {
        abi_version: VOX_PLUGIN_ABI_VERSION,
        manifest_json,
        init,
    }
    .leak_into_prefix()
}

#[sabi_extern_fn]
fn manifest_json() -> RString {
    RString::from(r#"{"id":"runtime-container","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = RuntimeContainerPlugin;
    let to = VoxPlugin_TO::from_value(plugin, abi_stable::erased_types::TD_Opaque);
    RResult::ROk(to)
}

struct RuntimeContainerPlugin;

impl VoxPlugin for RuntimeContainerPlugin {
    fn id(&self) -> RString {
        RString::from("runtime-container")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
}

// ─── Runtime detection (previously in vox-container::detect) ─────────────────

/// Re-export of `vox_container::detect::RuntimePreference` for callers that
/// use this plugin directly for runtime detection.
pub use vox_container::detect::RuntimePreference;

/// Detect and return the best available container runtime.
///
/// Prefers Podman (rootless, daemonless) over Docker when both are available.
/// Returns an error if neither is installed.
pub fn detect_runtime(
    preference: vox_container::detect::RuntimePreference,
) -> anyhow::Result<Box<dyn ContainerRuntime>> {
    use vox_container::detect::RuntimePreference;

    let try_docker = || {
        let rt = docker::DockerRuntime::new();
        if ContainerRuntime::available(&rt) {
            Some(Box::new(rt) as Box<dyn ContainerRuntime>)
        } else {
            None
        }
    };

    let try_podman = || {
        let rt = podman::PodmanRuntime::new();
        if ContainerRuntime::available(&rt) {
            Some(Box::new(rt) as Box<dyn ContainerRuntime>)
        } else {
            None
        }
    };

    match preference {
        RuntimePreference::Docker => try_docker().ok_or_else(|| {
            anyhow::anyhow!(
                "Docker was requested but is not installed or not running.\n\
                 Install from https://docs.docker.com/get-docker/"
            )
        }),
        RuntimePreference::Podman => try_podman().ok_or_else(|| {
            anyhow::anyhow!(
                "Podman was requested but is not installed.\n\
                 Install from https://podman.io/getting-started/installation"
            )
        }),
        RuntimePreference::Auto => {
            if let Some(rt) = try_podman() {
                tracing::info!("Auto-detected Podman (rootless)");
                return Ok(rt);
            }
            if let Some(rt) = try_docker() {
                tracing::info!("Auto-detected Docker");
                return Ok(rt);
            }
            anyhow::bail!(
                "No container runtime found.\n\
                 Install Podman (recommended): https://podman.io/getting-started/installation\n\
                 Or install Docker: https://docs.docker.com/get-docker/"
            )
        }
    }
}
