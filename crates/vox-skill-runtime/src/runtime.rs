//! Abstract skill runtime trait for sandboxed skill execution.
//!
//! `SkillRuntime` is the single abstraction over all execution backends:
//! - WASM (wasmtime-based, default for pure-compute skills)
//! - Container (Docker/Podman, fallback for skills requiring subprocess/GPU)
//! - Bare-metal (trusted process fork, for host-level skills)
//!
//! Implementations are shipped as plugins (`vox-plugin-runtime-container`,
//! `vox-plugin-runtime-wasm`).

use std::path::PathBuf;

/// Options for the "build" phase of a skill runtime.
///
/// For container runtimes: builds an OCI image.
/// For WASM runtimes: validates or precompiles the `.wasm` artifact.
/// For bare-metal: no-op.
#[derive(Debug, Clone)]
pub struct BuildOpts {
    /// Directory containing the build context (or the `.wasm` file for WASM runtimes).
    pub context_dir: PathBuf,
    /// Path to an explicit Dockerfile or WASM artifact. If `None`, auto-detected.
    pub artifact_path: Option<PathBuf>,
    /// Image tag or artifact label (used by container runtimes).
    pub tag: String,
    /// Key-value build arguments (used by container runtimes; ignored by WASM).
    pub build_args: Vec<(String, String)>,
}

/// Options for running a skill in its sandbox.
#[derive(Debug, Clone)]
pub struct RunOpts {
    /// Image tag (container), path to `.wasm` artifact (WASM), or command (bare-metal).
    pub artifact_path: PathBuf,
    /// Port mappings as `(host, container)`. Ignored by WASM.
    pub ports: Vec<(u16, u16)>,
    /// Environment variables.
    pub env: Vec<(String, String)>,
    /// Volume mounts as `(host_path, container_path)`. Ignored by WASM (uses preopens).
    pub volumes: Vec<(String, String)>,
    /// Run in detached/background mode.
    pub detach: bool,
    /// Container/process name.
    pub name: Option<String>,
    /// Remove container after exit (container runtimes only).
    pub rm: bool,
    /// CPU fuel limit for WASM runtimes (in wasmtime fuel units).
    /// `None` means use the runtime's default.
    pub cpu_limit_fuel: Option<u64>,
}

impl Default for RunOpts {
    fn default() -> Self {
        Self {
            artifact_path: PathBuf::new(),
            ports: Vec::new(),
            env: Vec::new(),
            volumes: Vec::new(),
            detach: false,
            name: None,
            rm: true,
            cpu_limit_fuel: None,
        }
    }
}

/// The captured outcome of a skill execution.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    /// Process exit code (0 = success).
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Wall-clock execution time in milliseconds.
    pub wall_ms: u64,
}

impl RunOutcome {
    /// Returns `true` if the skill exited successfully (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Abstract sandbox runtime for skill execution.
///
/// Implementations exist for:
/// - WASM (`vox-plugin-runtime-wasm`, the default for pure-compute skills)
/// - Docker/Podman (`vox-plugin-runtime-container`, fallback for subprocess/GPU skills)
///
/// The runtime is selected by `vox-skill-runtime::detect::detect_runtime()` based
/// on the skill's declared requirements.
pub trait SkillRuntime: Send + Sync {
    /// Human-readable runtime name (e.g. `"wasm"`, `"docker"`, `"podman"`).
    fn name(&self) -> &str;

    /// Returns `true` when this runtime is installed and reachable.
    fn available(&self) -> bool;

    /// Build or prepare the skill artifact for execution.
    ///
    /// For container runtimes: builds an OCI image.
    /// For WASM runtimes: validates or precompiles; typically a no-op.
    fn build(&self, opts: &BuildOpts) -> anyhow::Result<()>;

    /// Run the skill in its sandbox and return the outcome.
    fn run(&self, opts: &RunOpts) -> anyhow::Result<RunOutcome>;
}
