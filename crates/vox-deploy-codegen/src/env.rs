//! Python environment **detection** (read-only probes).
//!
//! **Package management:** Vox does not run `uv`/`pip` from supported PM paths — use
//! `Vox.toml` / `vox lock` / `vox sync`. Mutation helpers (`uv_sync`, `uv_add_packages`)
//! hard-error with migration text.

use std::process::Command;

/// Information about the detected Python environment.
#[derive(Debug, Clone)]
pub struct PythonEnv {
    /// Whether `uv` is installed and on PATH.
    pub uv_available: bool,
    /// `uv` version string, if available.
    pub uv_version: Option<String>,
    /// Python version string (from `uv python --version` or `python --version`).
    pub python_version: Option<String>,
    /// Detected CUDA version (e.g. `"12.4"`), if available.
    pub cuda_version: Option<String>,
    /// Whether NVIDIA GPU is present (nvcc or nvidia-smi detected).
    pub has_gpu: bool,
}

impl PythonEnv {
    /// Detect the Python environment on the current machine.
    pub fn detect() -> Self {
        let uv_out = run_cmd("uv", &["--version"]);
        let uv_available = uv_out.is_some();
        let uv_version = uv_out;

        let python_version = run_cmd("uv", &["python", "--version"])
            .or_else(|| run_cmd("python3", &["--version"]))
            .or_else(|| run_cmd("python", &["--version"]));

        // Detect CUDA via nvcc (compile-time) or nvidia-smi (runtime).
        let cuda_version = detect_cuda();
        let has_gpu = cuda_version.is_some() || run_cmd("nvidia-smi", &["-L"]).is_some();

        PythonEnv {
            uv_available,
            uv_version,
            python_version,
            cuda_version,
            has_gpu,
        }
    }

    /// Returns the PyTorch extra-index-url for the detected CUDA version.
    ///
    /// Falls back to CPU-only wheels when no GPU is detected, or when the
    /// CUDA version does not have a specific wheel source.
    ///
    /// Reference: <https://pytorch.org/get-started/locally/>
    pub fn pytorch_index_url(&self) -> &'static str {
        match self.cuda_version.as_deref() {
            Some(v) if v.starts_with("13.") => "https://download.pytorch.org/whl/cu130",
            Some(v) if v.starts_with("12.4") || v.starts_with("12.5") || v.starts_with("12.6") => {
                "https://download.pytorch.org/whl/cu124"
            }
            Some(v) if v.starts_with("12.1") || v.starts_with("12.2") || v.starts_with("12.3") => {
                "https://download.pytorch.org/whl/cu121"
            }
            Some(v) if v.starts_with("11.8") => "https://download.pytorch.org/whl/cu118",
            _ => "https://download.pytorch.org/whl/cpu",
        }
    }

    /// Retired: Vox does not invoke `uv sync` from supported tooling.
    pub fn uv_sync(&self, _project_dir: &std::path::Path) -> anyhow::Result<()> {
        let _ = self;
        anyhow::bail!(
            "Python/uv `sync` is not a supported Vox PM operation.\n\
             Use **`vox lock`** then **`vox sync`** for Vox packages (`Vox.toml` / `vox.lock`).\n\
             For Python-only projects, run `uv sync` yourself outside `vox …`."
        )
    }

    /// Retired: Vox does not invoke `uv add` from supported tooling.
    pub fn uv_add_packages(
        &self,
        _project_dir: &std::path::Path,
        _packages: &[&str],
    ) -> anyhow::Result<()> {
        let _ = self;
        anyhow::bail!(
            "Python/uv `add` is not a supported Vox PM operation.\n\
             Use **`vox add <dep>`** for `Vox.toml`, then **`vox lock`** / **`vox sync`**.\n\
             For PyPI packages in a Python project, use `uv add` directly outside `vox …`."
        )
    }

    /// Return the path of the uv-managed virtual environment, if one can be
    /// located. Checks `UV_PROJECT_ENVIRONMENT`, `VIRTUAL_ENV`, then `.venv`
    /// in the current working directory, then falls back to querying uv.
    ///
    /// The returned path is the *venv root*, not site-packages. Use
    /// [`Self::site_packages_path`] to get the importable directory.
    pub fn venv_path(&self) -> Option<std::path::PathBuf> {
        use std::env;
        use std::path::PathBuf;

        // 1. UV_PROJECT_ENVIRONMENT
        if let Ok(p) = env::var("UV_PROJECT_ENVIRONMENT") {
            let base = PathBuf::from(p);
            if base.exists() {
                return Some(base);
            }
        }

        // 2. VIRTUAL_ENV
        if let Ok(p) = env::var("VIRTUAL_ENV") {
            let base = PathBuf::from(p);
            if base.exists() {
                return Some(base);
            }
        }

        // 3. .venv in cwd
        if let Ok(cwd) = env::current_dir() {
            let venv = cwd.join(".venv");
            if venv.exists() {
                return Some(venv);
            }
        }

        None
    }

    /// Return the `site-packages` directory inside the uv venv, if found.
    ///
    /// On Windows: `<venv>/Lib/site-packages`
    /// On POSIX: `<venv>/lib/python<version>/site-packages`
    pub fn site_packages_path(&self) -> Option<std::path::PathBuf> {
        let venv = self.venv_path()?;

        // Windows fast path
        let win = venv.join("Lib").join("site-packages");
        if win.exists() {
            return Some(win);
        }

        // POSIX: scan lib/ for python3.x/site-packages
        let lib = venv.join("lib");
        if lib.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&lib) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    if name.to_string_lossy().starts_with("python") {
                        let sp = entry.path().join("site-packages");
                        if sp.exists() {
                            return Some(sp);
                        }
                    }
                }
            }
        }

        None
    }

    /// Retired probe: does not spawn `uv`/Python. Returns `false` (use native Mens/CUDA paths).
    pub fn validate_pytorch_cuda(&self, _project_dir: &std::path::Path) -> anyhow::Result<bool> {
        let _ = self;
        Ok(false)
    }
}

/// Detect CUDA version from `nvcc --version` or `nvidia-smi`.
fn detect_cuda() -> Option<String> {
    // Try nvcc first (most reliable — it's the compiler version)
    if let Some(out) = run_cmd("nvcc", &["--version"]) {
        // Output line: "Cuda compilation tools, release 12.4, V12.4.131"
        for line in out.lines() {
            if line.contains("release") {
                if let Some(v) = line
                    .split("release")
                    .nth(1)
                    .and_then(|s| s.split(',').next())
                    .map(|s| s.trim().to_string())
                {
                    return Some(v);
                }
            }
        }
    }

    // Fall back to nvidia-smi (runtime version, may differ from nvcc)
    if let Some(out) = run_cmd("nvidia-smi", &[]) {
        // First line usually contains "CUDA Version: 12.4"
        for line in out.lines() {
            if line.contains("CUDA Version:") {
                if let Some(v) = line.split("CUDA Version:").nth(1) {
                    let ver = v.split('|').next().unwrap_or("").trim().to_string();
                    if !ver.is_empty() {
                        return Some(ver);
                    }
                }
            }
        }
    }

    None
}

/// Run a command and return trimmed stdout, or None on failure.
fn run_cmd(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Summary of what setup actions are needed, without performing them.
#[derive(Debug, Clone)]
pub struct SetupPlan {
    /// uv needs to be installed.
    pub needs_uv: bool,
    /// Python packages that will be added/synced.
    pub packages: Vec<String>,
    /// PyTorch wheel source URL.
    pub torch_index_url: &'static str,
    /// GPU detected.
    pub has_gpu: bool,
}

impl SetupPlan {
    /// Build a setup plan from the detected environment and required modules.
    pub fn from_env(env: &PythonEnv, required_modules: &[String]) -> Self {
        SetupPlan {
            needs_uv: !env.uv_available,
            packages: required_modules.to_vec(),
            torch_index_url: env.pytorch_index_url(),
            has_gpu: env.has_gpu,
        }
    }

    /// Diagnostic summary only; does not imply a supported setup path.
    pub fn print_summary(&self) {
        println!(
            "  • Python/uv container setup is **not supported** — use `Vox.toml` / `vox sync`."
        );
        if self.needs_uv {
            println!("  • (probe) uv not on PATH");
        }
        if !self.packages.is_empty() {
            println!(
                "  • (legacy probe) requested modules ignored here: {}",
                self.packages.join(", ")
            );
        }
        println!(
            "  • (probe only) PyTorch wheel URL would be: {}",
            self.torch_index_url
        );
        if self.has_gpu {
            println!("  • (probe) GPU signals present — use Mens/Candle CUDA builds for Vox ML");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pytorch_index_cuda_130() {
        let env = PythonEnv {
            uv_available: true,
            uv_version: None,
            python_version: None,
            cuda_version: Some("13.0".to_string()),
            has_gpu: true,
        };
        assert!(env.pytorch_index_url().contains("cu130"));
    }

    #[test]
    fn pytorch_index_cuda_124() {
        let env = PythonEnv {
            uv_available: true,
            uv_version: None,
            python_version: None,
            cuda_version: Some("12.4".to_string()),
            has_gpu: true,
        };
        assert!(env.pytorch_index_url().contains("cu124"));
    }

    #[test]
    fn pytorch_index_cuda_118() {
        let env = PythonEnv {
            uv_available: true,
            uv_version: None,
            python_version: None,
            cuda_version: Some("11.8".to_string()),
            has_gpu: true,
        };
        assert!(env.pytorch_index_url().contains("cu118"));
    }

    #[test]
    fn pytorch_index_no_cuda() {
        let env = PythonEnv {
            uv_available: true,
            uv_version: None,
            python_version: None,
            cuda_version: None,
            has_gpu: false,
        };
        assert!(env.pytorch_index_url().contains("cpu"));
    }
}
