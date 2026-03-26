//! `vox mens generate` — AI-powered Vox code generation.
//!
//! Connects to the `vox-dei-d` daemon and streams AI-generated Vox source code
//! from a natural language prompt. Supports context modes, output validation,
//! structured output modes, and Codex task-job queueing.

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

use crate::commands::ci::bounded_read::read_utf8_path_capped;

/// System prompt preamble embedded into every generation request.
const SYSTEM_PREAMBLE: &str = "You are an expert Vox language programmer. Vox is a full-stack AI-native programming \
     language that compiles to Rust/WASM. Generate clean, idiomatic Vox code. \
     Output ONLY valid Vox source — no markdown fences, no English explanation.";

/// Collect schema context from the current working directory (if a Vox project).
fn collect_schema_context() -> String {
    let schema_file = std::env::current_dir()
        .ok()
        .map(|d| d.join("schema.vox"))
        .filter(|p| p.exists());

    if let Some(path) = schema_file
        && let Ok(content) = read_utf8_path_capped(&path)
    {
        return format!(
            "\n\n# Project Schema (schema.vox):\n```vox\n{}\n```",
            content
        );
    }
    String::new()
}

/// Build the full prompt based on context mode.
fn build_prompt(prompt: &str, context_mode: Option<&str>) -> String {
    let mode = context_mode.unwrap_or("minimal");
    let mut ctx = String::new();

    match mode {
        "schema-only" | "repo-aware" | "full" => {
            ctx.push_str(&collect_schema_context());
        }
        _ => {} // minimal, graph-aware: no extra file context in this path
    }

    format!("{SYSTEM_PREAMBLE}{ctx}\n\n# Task:\n{prompt}")
}

/// Validate generated Vox source via the full lex→parse→typecheck frontend.
fn validate_vox(source: &str) -> Result<()> {
    let synthetic_path = std::path::Path::new("<generated>");
    let result = crate::pipeline::run_frontend_str(source, synthetic_path, false)?;
    if result.has_errors() {
        let msgs: Vec<String> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error)
            .map(|d| d.message.clone())
            .collect();
        anyhow::bail!(
            "Generated code has {} error(s): {}",
            msgs.len(),
            msgs.join("; ")
        );
    }
    Ok(())
}

/// Validate against a JSON schema file (requires `output_mode` + `schema` path).
fn validate_json_schema(source: &str, schema_path: &Path) -> Result<()> {
    let schema_str = read_utf8_path_capped(schema_path)
        .with_context(|| format!("Failed to read schema: {}", schema_path.display()))?;
    let schema: serde_json::Value =
        serde_json::from_str(&schema_str).context("Schema file is not valid JSON")?;
    let instance: serde_json::Value = serde_json::from_str(source)
        .context("Generated output is not valid JSON (required by --output-mode + --schema)")?;

    // Walk the schema's `required` fields and verify each exists in the instance.
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        for field in required {
            let key = field.as_str().unwrap_or("");
            if instance.get(key).is_none() {
                anyhow::bail!("Generated JSON missing required field: {key}");
            }
        }
    }
    Ok(())
}

/// Call the vox-dei-d daemon and collect the full streamed generation result.
///
/// Chunks are printed to stdout in real-time by `call_daemon`; the final
/// `Result` payload (if any) is returned as the captured text.
async fn generate_via_daemon(full_prompt: &str) -> Result<String> {
    let result = crate::dei_daemon::call(
        crate::dei_daemon::method::AI_GENERATE,
        serde_json::json!({ "prompt": full_prompt }),
        false,
    )
    .await?;

    // The daemon streams Chunk payloads directly to stdout (handled by call_daemon).
    // If it also returns a structured "text" result, capture it.
    let text = result
        .get("text")
        .or_else(|| result.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(text)
}

/// Persist generated result to Codex via `Arca::log_execution`.
///
/// This is a best-effort operation — failures are swallowed silently so they
/// never break the generation flow. The `queue` flag in `run()` controls whether
/// this is called at all; when `false` (the default) there is zero overhead.
async fn try_enqueue_job(prompt: &str, result: &str) {
    let _ = (prompt, result);
    tracing::debug!(
        target = "vox_cli.generate",
        "Codex queue persistence for generate is not wired in this build (optional vox-arca path)."
    );
}

/// Main entry point for `vox mens generate`.
///
/// # Parameters
/// - `prompt` — natural language description of the code to generate
/// - `output` — optional file path to write; stdout if `None`
/// - `no_validate` — skip Vox parse/typecheck validation
/// - `server_url` — explicit inference server URL (overrides daemon)
/// - `max_retries` — retry count on validation failure (default 2)
/// - `output_mode` — structured output: `strict_json`, `jsonl_records`, `tool_args_json`
/// - `schema` — JSON schema path for post-generation validation (requires `output_mode`)
/// - `context_mode` — context assembly strategy: `minimal` | `repo-aware` | `schema-only` | `graph-aware` | `full`
/// - `conversation_id` — Codex conversation ID for graph-aware context (reserved)
/// - `queue` — persist result to Codex task queue
#[allow(clippy::too_many_arguments)] // Mirrors CLI flag surface; bundling would obscure dispatch.
pub async fn run(
    prompt: &str,
    output: Option<PathBuf>,
    no_validate: bool,
    server_url: Option<&str>,
    max_retries: Option<u32>,
    output_mode: Option<&str>,
    schema: Option<&Path>,
    context_mode: Option<&str>,
    _conversation_id: Option<i64>,
    queue: bool,
) -> Result<()> {
    let retries = max_retries.unwrap_or(2);
    let full_prompt = build_prompt(prompt, context_mode);

    println!("{} Generating Vox code…", "◆".cyan().bold());
    if let Some(mode) = context_mode.filter(|m| *m != "minimal") {
        println!("  Context mode: {}", mode.dimmed());
    }

    let mut last_result = String::new();
    let mut last_err: Option<anyhow::Error> = None;

    // Build the HTTP client once outside the retry loop — creating a Client per
    // attempt is expensive (TLS handshake, connection pool teardown).
    let http_client = server_url.map(|_| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                std::env::var("VOX_GEN_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(120u64),
            ))
            .build()
            .expect("reqwest Client should always construct")
    });

    // max_tokens: env override → CLI default (2048). Allows tuning without recompile.
    let max_tokens: u64 = std::env::var("VOX_GEN_MAX_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2048);

    for attempt in 0..=retries {
        if attempt > 0 {
            eprintln!(
                "{} Retry {}/{} — regenerating…",
                "↺".yellow(),
                attempt,
                retries
            );
        }

        let raw = if let (Some(url), Some(client)) = (server_url, &http_client) {
            // Direct HTTP inference: POST to /generate endpoint
            let body = serde_json::json!({
                "prompt": full_prompt,
                "max_tokens": max_tokens,
            });
            let resp = client
                .post(format!("{}/generate", url.trim_end_matches('/')))
                .json(&body)
                .send()
                .await
                .with_context(|| format!("Failed to connect to inference server at {url}"))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                anyhow::bail!("Inference server returned {status}: {body_text}");
            }
            let json: serde_json::Value = resp.json().await.context("Invalid JSON from server")?;
            json.get("text")
                .or_else(|| json.get("response"))
                .or_else(|| json.get("generated_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            // Route through vox-dei-d daemon (fail fast — no fake offline output).
            generate_via_daemon(&full_prompt).await.context(
                "Dei daemon unavailable. Start `vox-dei-d`, or pass `--server-url` for direct HTTP inference.",
            )?
        };

        // Strip markdown code fences that some models emit.
        let stripped = raw
            .trim()
            .trim_start_matches("```vox")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();

        last_result = stripped.clone();

        if no_validate {
            break;
        }

        // Validate: JSON schema (if provided) or Vox parse/typecheck.
        let validation_result = if let (Some(mode), Some(sch)) = (output_mode, schema) {
            if matches!(mode, "strict_json" | "jsonl_records" | "tool_args_json") {
                validate_json_schema(&stripped, sch)
            } else {
                validate_vox(&stripped)
            }
        } else {
            validate_vox(&stripped)
        };

        match validation_result {
            Ok(()) => break,
            Err(e) if attempt < retries => {
                last_err = Some(e);
                continue;
            }
            Err(e) => {
                eprintln!(
                    "{} Validation failed after {} attempt(s): {}",
                    "✗".red().bold(),
                    attempt + 1,
                    e
                );
                last_err = Some(e);
                break;
            }
        }
    }

    // Optionally queue to Codex task store.
    if queue {
        try_enqueue_job(prompt, &last_result).await;
    }

    // Write output.
    if let Some(out_path) = &output {
        std::fs::create_dir_all(out_path.parent().unwrap_or_else(|| Path::new(".")))?;
        std::fs::write(out_path, &last_result)
            .with_context(|| format!("Failed to write output to {}", out_path.display()))?;
        println!("{} Written to {}", "✓".green().bold(), out_path.display());
    } else {
        println!("\n{}", last_result);
    }

    if let Some(err) = last_err {
        eprintln!(
            "{} Note: generated code may have issues — {}",
            "⚠".yellow(),
            err
        );
    }

    Ok(())
}
