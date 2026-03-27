//! Stub for the retired Python/UV Dockerfile generator.

use crate::env::PythonEnv;

/// Legacy hook — returns a **comment-only** Dockerfile stub (Python lanes retired).
pub fn generate_python_dockerfile(
    project_name: &str,
    _env: &PythonEnv,
    _py_imports: &[String],
) -> String {
    format!(
        "# Retired: Vox no longer emits Python/uv Dockerfiles for `{project_name}`.\n\
         # Build a Rust-first image (see repository `Dockerfile`) and use `vox sync` for PM artifacts.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::PythonEnv;

    #[test]
    fn dockerfile_is_retired_stub() {
        let env = PythonEnv {
            uv_available: false,
            uv_version: None,
            python_version: None,
            cuda_version: None,
            has_gpu: false,
        };
        let df = generate_python_dockerfile("x", &env, &[]);
        assert!(df.contains("Retired"));
        assert!(!df.contains("astral.sh/uv"));
    }
}
