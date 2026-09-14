//! First-party Code Generation domain engine for API and syntax research.
//!
//! Generates targeted subqueries for docs.rs signatures, usage examples,
//! and migration notes, and provides sandboxed compiler verification.

use serde::{Deserialize, Serialize};

/// Result of sandboxed compiler verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSandboxResult {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Result of an iterative self-correction loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeSelfCorrectionResult {
    pub initial_passed: bool,
    pub final_passed: bool,
    pub iterations: usize,
    pub original_code: String,
    pub corrected_code: Option<String>,
    pub initial_error: Option<String>,
    pub final_error: Option<String>,
    pub repair_explanation: Option<String>,
}

/// Extract all fenced code snippets (e.g. ```rust ... ```) from markdown text.
pub fn extract_code_snippets_from_markdown(text: &str) -> Vec<String> {
    let mut snippets = Vec::new();
    let mut in_fence = false;
    let mut current = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_fence {
                if !current.is_empty() {
                    snippets.push(current.join("\n"));
                    current.clear();
                }
                in_fence = false;
            } else {
                in_fence = true;
            }
        } else if in_fence {
            current.push(line);
        }
    }
    snippets
}

/// Wrap statements or bare expressions in a test probe harness with `#![allow(unused)]`
/// so that snippets containing only statements (e.g. `let x = 1; assert!(...);`)
/// can compile cleanly as a standalone library crate.
pub fn wrap_code_snippet_if_needed(snippet: &str) -> String {
    let first_code_line = snippet.lines().map(str::trim).find(|line| {
        !line.is_empty()
            && !line.starts_with("///")
            && !line.starts_with("//")
            && !line.starts_with("#[")
    });

    let is_top_level_item = if let Some(first) = first_code_line {
        first.starts_with("pub fn ")
            || first.starts_with("fn ")
            || first.starts_with("pub async fn ")
            || first.starts_with("async fn ")
            || first.starts_with("pub struct ")
            || first.starts_with("struct ")
            || first.starts_with("pub enum ")
            || first.starts_with("enum ")
            || first.starts_with("pub trait ")
            || first.starts_with("trait ")
            || first.starts_with("pub type ")
            || first.starts_with("type ")
            || first.starts_with("pub const ")
            || first.starts_with("const ")
            || first.starts_with("impl ")
            || first.starts_with("use ")
            || first.starts_with("mod ")
    } else {
        false
    };

    if is_top_level_item {
        return snippet.to_string();
    }

    format!(
        "#![allow(unused_imports, unused_variables, dead_code, unused_must_use)]\n\
         pub fn __vox_sandbox_probe() {{\n\
             {}\n\
         }}",
        snippet
    )
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
    let temp_dir_guard = tempfile::Builder::new().prefix("vox-sandbox-").tempdir()?;
    let temp_dir = temp_dir_guard.path().to_path_buf();

    if dependencies.is_empty() {
        let mut cmd = tokio::process::Command::new("rustc");
        cmd.kill_on_drop(true);
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
        cmd.kill_on_drop(true);
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

/// Attempt to iteratively repair Rust code that fails sandboxed compiler verification.
///
/// If `initial_code` compiles cleanly, returns immediately with `iterations: 0` and `initial_passed: true`.
/// Otherwise, invokes `repair_fn(current_code, compiler_stderr)` up to `max_attempts` times,
/// re-verifying after each repair step until compilation succeeds or attempts are exhausted.
pub async fn attempt_code_self_correction<F, Fut>(
    initial_code: &str,
    dependencies: &[&str],
    max_attempts: usize,
    mut repair_fn: F,
) -> anyhow::Result<CodeSelfCorrectionResult>
where
    F: FnMut(String, String) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<(String, String)>>,
{
    let initial_run = verify_rust_code_in_sandbox(initial_code, dependencies).await?;
    if initial_run.passed {
        return Ok(CodeSelfCorrectionResult {
            initial_passed: true,
            final_passed: true,
            iterations: 0,
            original_code: initial_code.to_string(),
            corrected_code: Some(initial_code.to_string()),
            initial_error: None,
            final_error: None,
            repair_explanation: None,
        });
    }

    let initial_error = initial_run.stderr.clone();
    let mut current_code = initial_code.to_string();
    let mut last_error = initial_run.stderr;
    let mut last_explanation = None;
    let mut iterations = 0;

    for attempt in 1..=max_attempts {
        iterations = attempt;
        let (candidate_fix, explanation) =
            match repair_fn(current_code.clone(), last_error.clone()).await {
                Ok(pair) => pair,
                Err(e) => {
                    last_error = format!("Repair generation failed: {e}");
                    break;
                }
            };

        let check_res = verify_rust_code_in_sandbox(&candidate_fix, dependencies).await?;
        if check_res.passed {
            return Ok(CodeSelfCorrectionResult {
                initial_passed: false,
                final_passed: true,
                iterations,
                original_code: initial_code.to_string(),
                corrected_code: Some(candidate_fix),
                initial_error: Some(initial_error),
                final_error: None,
                repair_explanation: Some(explanation),
            });
        }

        current_code = candidate_fix;
        last_error = check_res.stderr;
        last_explanation = Some(explanation);
    }

    Ok(CodeSelfCorrectionResult {
        initial_passed: false,
        final_passed: false,
        iterations,
        original_code: initial_code.to_string(),
        corrected_code: Some(current_code),
        initial_error: Some(initial_error),
        final_error: Some(last_error),
        repair_explanation: last_explanation,
    })
}
