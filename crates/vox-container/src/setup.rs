//! High-level orchestration hook for legacy **`vox container init`** Python flows.
//!
//! Python/UV packaging lanes are **retired**; `run_py_setup` is a hard error with migration text.

use std::path::Path;

/// Options for the (retired) Python environment setup step.
#[derive(Debug, Clone, Default)]
pub struct PySetupOpts {
    /// Project name used in generated files.
    pub project_name: String,
    /// Python module names from `@py.import` declarations.
    pub py_imports: Vec<String>,
    /// Whether to generate a Dockerfile (in addition to pyproject.toml).
    pub generate_dockerfile: bool,
    /// Output directory for generated files.
    pub out_dir: std::path::PathBuf,
}

/// Historical entrypoint — **always errors** (Python/UV container setup is not supported).
pub fn run_py_setup(opts: &PySetupOpts) -> anyhow::Result<()> {
    let _ = opts.generate_dockerfile;
    anyhow::bail!(
        "Python/UV `vox container init` is retired.\n\
         • Vox PM is **Rust-first**: use `Vox.toml` / `vox.lock` / `vox sync` for artifacts.\n\
         • Remove `@py.import` usage or host Python outside the Vox toolchain.\n\
         See docs/src/reference/cli.md (package management)."
    )
}

/// Returns whether a legacy setup would have been triggered (py imports present, no pyproject).
#[allow(dead_code)]
pub fn check_setup_needed(out_dir: &Path, py_imports: &[String]) -> bool {
    if py_imports.is_empty() {
        return false;
    }
    !out_dir.join("pyproject.toml").exists()
}
