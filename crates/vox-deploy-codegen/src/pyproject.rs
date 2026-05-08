//! Legacy **`pyproject.toml`** string helper (retired product path).
//!
//! Vox **does not** generate dependency manifests for Python/uv from `@py.import`.
//! [`generate_pyproject_toml`] returns a minimal placeholder pointing maintainers at `Vox.toml` / `vox sync`.

use crate::env::PythonEnv;

/// A Python dependency (package name + optional version constraint).
#[derive(Debug, Clone)]
pub struct PyDep {
    /// Package name as it appears on PyPI, e.g. `"torch"` or `"numpy"`.
    pub name: String,
    /// Optional PEP 440 version specifier, e.g. `">=2.0"`.
    pub version: Option<String>,
}

impl PyDep {
    /// Infer the correct PyPI package name from a Python import module name.
    ///
    /// Maps common Vox `@py.import` module names to their PyPI package names,
    /// e.g. `torch.nn` → `torch`, `sklearn` → `scikit-learn`.
    pub fn from_import_module(module: &str) -> Self {
        // Take the top-level module name.
        let top = module.split('.').next().unwrap_or(module);
        let pypi_name = canonical_pypi_name(top);
        PyDep {
            name: pypi_name.to_string(),
            version: None,
        }
    }

    /// Format as a PEP 621 dependency string, e.g. `"torch>=2.0"`.
    pub fn as_pep621(&self) -> String {
        match &self.version {
            Some(v) => format!("{}{}", self.name, v),
            None => self.name.clone(),
        }
    }
}

/// Map common import names to canonical PyPI names.
fn canonical_pypi_name(module: &str) -> &str {
    match module {
        "cv2" => "opencv-python",
        "sklearn" => "scikit-learn",
        "PIL" => "Pillow",
        "pil" => "Pillow",
        "yaml" => "PyYAML",
        "bs4" => "beautifulsoup4",
        "dateutil" => "python-dateutil",
        "dotenv" => "python-dotenv",
        "jwt" => "PyJWT",
        "attr" | "attrs" => "attrs",
        other => other,
    }
}

fn toml_escape_basic(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Returns a **placeholder** `pyproject.toml` — Vox does not populate Python deps from `@py.import`.
///
/// `imports` and `env` are ignored for manifest content; maintain PyPI dependencies yourself if needed.
pub fn generate_pyproject_toml(
    project_name: &str,
    _imports: &[String],
    _env: &PythonEnv,
) -> String {
    let safe = toml_escape_basic(project_name);
    format!(
        r#"# RETIRED: Vox does not generate Python/uv dependency graphs.
# Project label: {safe}
# Supported PM: Vox.toml + vox lock + vox sync
# @py.import module requests are ignored here — hand-author pyproject.toml if you use Python alongside Vox.

[project]
name = "{safe}"
version = "0.0.0"
requires-python = ">=3.12"
dependencies = []

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::PythonEnv;

    fn cpu_env() -> PythonEnv {
        PythonEnv {
            uv_available: true,
            uv_version: None,
            python_version: None,
            cuda_version: None,
            has_gpu: false,
        }
    }

    fn gpu_env() -> PythonEnv {
        PythonEnv {
            uv_available: true,
            uv_version: None,
            python_version: None,
            cuda_version: Some("12.4".to_string()),
            has_gpu: true,
        }
    }

    #[test]
    fn generates_retired_stub_project() {
        let toml = generate_pyproject_toml("my-app", &["torch".to_string()], &cpu_env());
        assert!(toml.contains("RETIRED"));
        assert!(toml.contains("[project]"));
        assert!(toml.contains("name = \"my-app\""));
        assert!(toml.contains("dependencies = []"));
        assert!(!toml.contains("[[tool.uv.index]]"));
    }

    #[test]
    fn imports_do_not_add_torch_deps() {
        let toml = generate_pyproject_toml("app", &["torch".to_string()], &gpu_env());
        assert!(!toml.contains("\"torch\""));
        assert!(!toml.contains("tool.uv"));
    }

    #[test]
    fn canonical_opencv() {
        let dep = PyDep::from_import_module("cv2");
        assert_eq!(dep.name, "opencv-python");
    }

    #[test]
    fn canonical_sklearn() {
        let dep = PyDep::from_import_module("sklearn");
        assert_eq!(dep.name, "scikit-learn");
    }

    #[test]
    fn deduplicates_torch_nn() {
        let imports = vec![
            "torch".to_string(),
            "torch.nn".to_string(),
            "torch.nn.functional".to_string(),
        ];
        let toml = generate_pyproject_toml("app", &imports, &cpu_env());
        assert!(!toml.contains("\"torch\""));
    }
}
