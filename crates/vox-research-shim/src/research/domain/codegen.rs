//! First-party Code Generation domain engine for API and syntax research.
//!
//! Generates targeted subqueries for docs.rs signatures, usage examples,
//! and migration notes, and provides sandboxed compiler verification.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Result of sandboxed compiler verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSandboxResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

static SANDBOX_COUNTER: AtomicU64 = AtomicU64::new(1);

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Generate targeted subqueries for Rust docs.rs signatures, usage, and migration guides.
pub fn generate_codegen_subqueries(crate_or_topic: &str) -> Vec<String> {
    vec![
        format!("{crate_or_topic} rust api signatures struct impl function site:docs.rs"),
        format!("{crate_or_topic} rust examples usage tutorial"),
        format!("{crate_or_topic} rust breaking changes migration guide github"),
    ]
}

/// Synthesis instructions for generating verified code blocks.
pub fn codegen_synthesis_instructions() -> &'static str {
    "DOMAIN MODE: CODE GENERATION & API RESEARCH\nIn addition to standard research synthesis:\n1. Ensure that all generated Rust code snippets strictly match verified API signatures from docs.rs.\n2. Output complete, idiomatic, and compilable code blocks with explicit types and full error handling.\n3. Note any breaking changes, feature flags, or minimum supported Rust versions (MSRV) needed to compile the code."
}

/// Verify Rust code in a lightweight sandbox.
///
/// If `dependencies` is empty, pipes the code to `rustc --crate-type lib -` via stdin
/// for sub-100ms syntax and type checking. When dependencies are supplied, sets up
/// a temporary Cargo project and executes `cargo check`.
pub async fn verify_rust_code_in_sandbox(
    code: &str,
    dependencies: &[&str],
) -> anyhow::Result<CodeSandboxResult> {
    let id = SANDBOX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_dir =
        std::env::temp_dir().join(format!("vox-code-sandbox-{}-{}", std::process::id(), id));
    std::fs::create_dir_all(&temp_dir)?;
    let _guard = TempDirGuard(temp_dir.clone());

    if dependencies.is_empty() {
        let mut cmd = tokio::process::Command::new("rustc");
        cmd.args(["--crate-type", "lib", "--emit=metadata", "--out-dir"])
            .arg(&temp_dir)
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(code.as_bytes()).await?;
            stdin.flush().await?;
            drop(stdin);
        }
        let output = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            child.wait_with_output(),
        )
        .await
        {
            Ok(res) => res?,
            Err(_) => {
                return Ok(CodeSandboxResult {
                    passed: false,
                    stdout: String::new(),
                    stderr: "Compilation timed out after 10 seconds".to_string(),
                });
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let passed = output.status.success();
        Ok(CodeSandboxResult {
            passed,
            stdout,
            stderr,
        })
    } else {
        let src_dir = temp_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;
        std::fs::write(src_dir.join("lib.rs"), code)?;

        let mut cargo_toml = String::from(
            "[package]\nname = \"sandbox_check\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        );
        for dep in dependencies {
            let trimmed = dep.trim();
            if trimmed.contains('\n')
                || trimmed.contains('\r')
                || trimmed.contains('[')
                || trimmed.contains(']')
            {
                anyhow::bail!("Invalid dependency specifier in sandbox: {dep}");
            }
            if trimmed.contains('=') {
                cargo_toml.push_str(trimmed);
                cargo_toml.push('\n');
            } else {
                if !trimmed
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    anyhow::bail!("Invalid crate name in sandbox: {trimmed}");
                }
                cargo_toml.push_str(&format!("{trimmed} = \"*\"\n"));
            }
        }
        std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

        let mut cmd = tokio::process::Command::new("cargo");
        cmd.arg("check")
            .arg("--manifest-path")
            .arg(temp_dir.join("Cargo.toml"))
            .arg("--message-format=short")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output =
            match tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output()).await {
                Ok(res) => res?,
                Err(_) => {
                    return Ok(CodeSandboxResult {
                        passed: false,
                        stdout: String::new(),
                        stderr: "Compilation timed out after 10 seconds".to_string(),
                    });
                }
            };
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let passed = output.status.success();
        Ok(CodeSandboxResult {
            passed,
            stdout,
            stderr,
        })
    }
}
