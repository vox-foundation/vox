//! `vox generate` — generate validated Vox code using the MENS fine-tuned model.
//!
//! By default this routes through the orchestrator's VoxLocal path, which gives:
//!   - TTL-cached health probes (no redundant /health calls per invocation)
//!   - Consistent endpoint resolution from `VOX_LOCAL_ENDPOINT`
//!   - Aligned telemetry with MCP codegen calls
//!
//! `--legacy-direct` (deprecated): bypasses the orchestrator and calls the inference server
//! directly. Pre-Task 1.9 behavior. Prefer orchestrator mode; this flag is an escape hatch.
//!
//! ## T2.3: accepted stateless exception (default orchestrator mode)
//!
//! Despite the name, `run_via_orchestrator`'s default path does **not** go
//! through the shared `vox-orchestrator-d` daemon's preflight/permission/
//! approval chain — it calls `vox_orchestrator_mcp::llm_bridge::vox_local_generate`
//! in-process, which talks HTTP directly to the MENS inference server
//! (`VOX_LOCAL_ENDPOINT`) with its own health-probe cache, validate/retry
//! loop, and response shape (`{code, valid, errors, warnings, attempts}`).
//!
//! This is deliberately NOT rerouted onto the daemon's `orch_daemon_method`/
//! `ai.generate` RPC (`crate::dei_daemon::method::AI_GENERATE`): that RPC's
//! server-side handler (`vox_orchestrator::orch_daemon::dei_dispatch::handle_ai_generate`)
//! calls a generic `vox_gamify::ai::FreeAiClient::auto_discover()` text
//! completion — no MENS-specific validate/retry loop, no `VOX_LOCAL_ENDPOINT`
//! resolution, and a completely different response shape. Routing through it
//! would silently change `vox generate`'s behavior rather than just its
//! transport, which the rest of T2.3's migrations were careful to avoid.
//! Making `ai.generate`'s daemon handler MENS-aware (so it's a true drop-in)
//! is a larger backend change than fits T2.3 — scoped down to an explicit
//! follow-up rather than silently left as an undocumented bypass. Unlike
//! `dei.rs`'s task/agent/approval RPCs, `vox generate` touches no shared
//! mutable orchestrator state (no task queue, no approvals) — it is a
//! stateless codegen call — so the split-brain risk this task's other
//! migrations close does not apply here in the same way.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;

/// Default base URL of the local MENS inference server (`--legacy-direct` path).
const DEFAULT_INFERENCE_URL: &str = "http://127.0.0.1:11434";

/// Resolve the inference server base URL: explicit `--server-url` wins, then the
/// `VOX_INFERENCE_URL` environment override, then the built-in default.
fn resolve_inference_url(explicit: Option<&str>) -> String {
    resolve_inference_url_from(explicit, std::env::var("VOX_INFERENCE_URL").ok().as_deref())
}

/// Pure resolution core (precedence: explicit → env override → default), split
/// out so the precedence logic is unit-testable without touching the process
/// environment.
fn resolve_inference_url_from(explicit: Option<&str>, env_override: Option<&str>) -> String {
    explicit
        .or(env_override)
        .unwrap_or(DEFAULT_INFERENCE_URL)
        .to_string()
}

/// Run the generate command.
pub async fn run(
    prompt: &str,
    output: Option<PathBuf>,
    no_validate: bool,
    server_url: Option<&str>,
    max_retries: Option<u32>,
    legacy_direct: bool,
) -> Result<()> {
    let retries = max_retries.unwrap_or(3);
    let validate = !no_validate;

    let client = vox_http_client::client_builder()
        .timeout(vox_config::timeouts::D_120S)
        .build()
        .context("Failed to build HTTP client")?;

    if server_url.is_some() && !legacy_direct {
        anyhow::bail!(
            "--server-url only applies with --legacy-direct; use VOX_LOCAL_ENDPOINT for orchestrator mode"
        );
    }

    let (code, valid, errors, warnings, attempts) = if legacy_direct {
        run_legacy_direct(&client, prompt, server_url, validate, retries).await?
    } else {
        #[cfg(feature = "mcp-server")]
        {
            run_via_orchestrator(&client, prompt, validate, retries).await?
        }
        #[cfg(not(feature = "mcp-server"))]
        {
            eprintln!(
                "ℹ️  Orchestrator mode requires the mcp-server feature. Falling back to --legacy-direct."
            );
            run_legacy_direct(&client, prompt, server_url, validate, retries).await?
        }
    };

    // Print status line
    eprintln!();
    match valid {
        Some(true) => {
            eprintln!("✅ Valid Vox code generated (attempts: {})", attempts);
        }
        Some(false) => {
            eprintln!(
                "⚠️  Generated code may have issues (attempts: {})",
                attempts
            );
            for e in &errors {
                eprintln!("   ❌ {}", e);
            }
        }
        None => {
            eprintln!("ℹ️  Validation skipped");
        }
    }
    for w in &warnings {
        eprintln!("   ⚠ {}", w);
    }
    eprintln!();

    if let Some(output_path) = output {
        std::fs::write(&output_path, &code)
            .with_context(|| format!("Failed to write to {}", output_path.display()))?;
        eprintln!("📄 Wrote {} bytes to {}", code.len(), output_path.display());
    }

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(code.as_bytes())?;
    handle.write_all(b"\n")?;

    if valid == Some(false) {
        std::process::exit(1);
    }

    Ok(())
}

type GenerateOutput = (String, Option<bool>, Vec<String>, Vec<String>, u64);

/// T2.3: accepted stateless exception — does NOT route through the shared
/// `vox-orchestrator-d` daemon; see this module's top doc comment for why.
#[cfg(feature = "mcp-server")]
async fn run_via_orchestrator(
    client: &reqwest::Client,
    prompt: &str,
    validate: bool,
    max_retries: u32,
) -> Result<GenerateOutput> {
    eprintln!("🔮 Generating Vox code via orchestrator...");
    eprintln!("   Prompt: {}", prompt);

    let result = vox_orchestrator_mcp::llm_bridge::vox_local_generate(
        client,
        prompt,
        validate,
        max_retries,
        None,
    )
    .await
    .map_err(|e| {
        eprintln!("⚠️  VoxLocal inference unavailable: {e}");
        eprintln!("   Start it with: vox mens serve");
        anyhow::anyhow!(e)
    })?;

    Ok((
        result.code,
        result.valid,
        result.errors,
        result.warnings,
        result.attempts,
    ))
}

/// Probe whether an inference server is reachable at `url`, trying each
/// backend's real health route in turn: `/health` (MENS/VoxLocal server) then
/// `/api/tags` (Ollama, which does not serve `/health`). Both backends can
/// share the same default port (`127.0.0.1:11434`), so a single fixed path
/// misreports one of them as down.
async fn probe_reachable(client: &reqwest::Client, url: &str) -> bool {
    let base = url.trim_end_matches('/');
    for path in ["/health", "/api/tags"] {
        if let Ok(resp) = client.get(format!("{base}{path}")).send().await {
            if resp.status().is_success() {
                return true;
            }
        }
    }
    false
}

async fn run_legacy_direct(
    client: &reqwest::Client,
    prompt: &str,
    server_url: Option<&str>,
    validate: bool,
    max_retries: u32,
) -> Result<GenerateOutput> {
    let url = resolve_inference_url(server_url);
    let endpoint = format!("{}/generate", url);

    if probe_reachable(client, &url).await {
        eprintln!("📡 Connected to inference server at {}", url);
    } else {
        eprintln!("⚠️  Inference server not running at {}", url);
        eprintln!("   Start it with: vox mens serve");
        anyhow::bail!("Inference server not available");
    }

    eprintln!("🔮 Generating Vox code...");
    eprintln!("   Prompt: {}", prompt);

    let body = serde_json::json!({
        "prompt": prompt,
        "validate": validate,
        "max_retries": max_retries,
    });

    let resp = client
        .post(&endpoint)
        .json(&body)
        .send()
        .await
        .context("Failed to connect to inference server")?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response")?;

    if !status.is_success() {
        anyhow::bail!("Server error ({}): {}", status, text);
    }

    let result: serde_json::Value =
        serde_json::from_str(&text).context("Invalid JSON from server")?;

    let code = result["code"].as_str().unwrap_or("").to_string();
    let valid = result["valid"].as_bool();
    let attempts = result["attempts"].as_u64().unwrap_or(1);
    let errors: Vec<String> = result["errors"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let warnings: Vec<String> = result["warnings"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok((code, valid, errors, warnings, attempts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_inference_url_prefers_explicit() {
        // Explicit value wins regardless of any environment override.
        assert_eq!(
            resolve_inference_url(Some("http://example.invalid:9000")),
            "http://example.invalid:9000"
        );
    }

    #[test]
    fn resolve_inference_url_env_then_default() {
        // No env override → built-in default.
        assert_eq!(
            resolve_inference_url_from(None, None),
            DEFAULT_INFERENCE_URL
        );
        // Env override applies when no explicit value is given.
        assert_eq!(
            resolve_inference_url_from(None, Some("http://10.0.0.5:7000")),
            "http://10.0.0.5:7000"
        );
        // Explicit value beats the env override.
        assert_eq!(
            resolve_inference_url_from(Some("http://explicit:1"), Some("http://env:2")),
            "http://explicit:1"
        );
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// An Ollama-shaped server (no `/health`, only `/api/tags`) must be
    /// detected as reachable — this is the reported bug: the old code only
    /// ever checked `/health`, so a real Ollama instance was reported as
    /// "not available" despite running and listening on the same default port.
    #[tokio::test]
    async fn probe_reachable_detects_ollama_via_api_tags() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"models":[]}"#))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        assert!(probe_reachable(&client, &server.uri()).await);
    }

    /// A MENS/VoxLocal-shaped server (`/health` only) must still be detected.
    #[tokio::test]
    async fn probe_reachable_detects_mens_via_health() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        assert!(probe_reachable(&client, &server.uri()).await);
    }

    /// Neither route responding ⇒ genuinely unreachable.
    #[tokio::test]
    async fn probe_reachable_false_when_nothing_serves_either_route() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        assert!(!probe_reachable(&client, &server.uri()).await);
    }
}
