//! `vox chat <message>` — send one message through the harness's chat pipeline
//! from the terminal and print the model's reply to stdout.
//!
//! Useful as a live, hermetic-ish-adjacent eval test surface (it makes a real
//! network call by nature, so it is not itself hermetic) and as a
//! quick-and-dirty way to talk to a model without the GUI. Reuses the exact
//! same [`vox_actor_runtime::llm::llm_chat`] durable-activity chat facade the
//! rest of the harness (e.g. `vox model eval`, see
//! `crates/vox-cli/src/commands/model/eval.rs::eval_one_model`) calls — no
//! separate chat implementation.

use anyhow::{Context, Result, bail};
use clap::Parser;
use vox_actor_runtime::ActivityOptions;
use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig, llm_chat};

/// `vox chat` arguments.
#[derive(Parser, Debug)]
pub struct ChatArgs {
    /// The message to send.
    pub message: String,
    /// OpenRouter model id to use (default: `openrouter/auto`, provider-side routing).
    #[arg(long, default_value = vox_config::bootstrap_inference::OPENROUTER_AUTO)]
    pub model: String,
    /// Optional system prompt prepended before the user message.
    #[arg(long)]
    pub system: Option<String>,
}

/// Run `vox chat`: send `args.message` (with optional `args.system` prompt) to
/// `args.model` via the shared `llm_chat` facade and print the reply.
pub async fn run(args: ChatArgs) -> Result<()> {
    if args.message.trim().is_empty() {
        bail!("vox chat: message must not be empty");
    }

    let mut messages = Vec::with_capacity(2);
    if let Some(system) = &args.system {
        messages.push(LlmChatMessage {
            role: "system".to_string(),
            content: system.clone(),
            ..Default::default()
        });
    }
    messages.push(LlmChatMessage {
        role: "user".to_string(),
        content: args.message.clone(),
        ..Default::default()
    });

    let mut config = resolve_chat_config(&args.model);
    config.telemetry_task_category = Some("cli-chat".to_string());

    let is_local = config.provider == "voxlocal";
    let base_url = std::env::var("VOX_LOCAL_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

    let opts = ActivityOptions::new();
    // `llm_chat` returns a durable-activity `ActivityResult<Result<LlmResponse, String>>`;
    // flatten the activity-level outcome (Ok/Failed/Cancelled) and the chat-level
    // `Result` into a single `Result<LlmResponse, String>` before mapping to `anyhow`.
    // Same flattening `model/eval.rs::eval_one_model` does — no shared combinator exists
    // on `ActivityResult` yet to call instead.
    let chat_result: Result<vox_actor_runtime::llm::LlmResponse, String> =
        match llm_chat(&opts, messages, config).await {
            vox_actor_runtime::ActivityResult::Ok(inner) => inner,
            vox_actor_runtime::ActivityResult::Failed(e) => Err(e.to_string()),
            vox_actor_runtime::ActivityResult::Cancelled => Err("activity cancelled".to_string()),
        };

    let response = match chat_result {
        Ok(resp) => resp,
        Err(e) => {
            if is_local && is_connection_error(&e) {
                bail!(
                    "Error: Local inference server is not running on {}. Start it with 'vox mens serve' or specify a cloud model.",
                    base_url.trim_end_matches('/')
                );
            }
            return Err(anyhow::anyhow!(e)).context("vox chat: llm_chat failed");
        }
    };
    println!("{}", response.content);
    Ok(())
}

fn is_connection_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    // HTTP status errors indicate the server IS running and replied; do not treat as connection failures.
    if lower.contains("status code")
        || lower.contains("status client error")
        || lower.contains("status server error")
        || lower.contains("404 not found")
        || lower.contains("500 internal server error")
        || lower.contains("400 bad request")
    {
        return false;
    }

    lower.contains("connection refused")
        || lower.contains("failed to connect")
        || lower.contains("error sending request")
        || lower.contains("tcp connect error")
        || lower.contains("connection error")
        || lower.contains("connect error")
        || lower.contains("network unreachable")
        || lower.contains("host unreachable")
        || lower.contains("os error 61")
        || lower.contains("os error 111")
        || lower.contains("os error 10061")
}

pub(crate) fn resolve_chat_config(model: &str) -> LlmConfig {
    if model == "auto"
        || model.starts_with("vox-mens-")
        || model.starts_with("mens/")
        || model.starts_with("voxlocal")
        || model.starts_with("populi_local")
        || model == "qwen/qwen-3-8b"
    {
        let effective_model = if model == "auto" {
            let is_apple_silicon = vox_orchestrator::models::auto_select::is_host_apple_silicon();
            vox_orchestrator::models::auto_select::select_optimal_local_model(is_apple_silicon)
                .selected_model_id
        } else {
            model.to_string()
        };

        let base_url = std::env::var("VOX_LOCAL_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        let chat_url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
        LlmConfig {
            provider: "voxlocal".to_string(),
            model: effective_model,
            cost_per_1k: None,
            base_url: Some(chat_url),
            api_key: None,
            temperature: None,
            top_p: None,
            max_tokens: Some(2048),
            response_format: None,
            tools: None,
            tool_choice: None,
            timeout_ms: None,
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: None,
            telemetry_strength_tag: None,
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: true,
        }
    } else if model.starts_with("ollama")
        || (model.contains("qwen3") && !model.contains('/'))
        // vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="Retired in favor of Qwen 3 (Qwen/Qwen3-8B)" canonical="qwen3"
        || (model.contains("qwen2.5-coder") && !model.contains('/'))
        || model.starts_with("local")
    {
        LlmConfig::ollama(model)
    } else {
        LlmConfig::openrouter(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `vox chat` arg parsing: the message is positional and required, `--model`
    /// defaults to `openrouter/auto`, and both are overridable. Mirrors the
    /// `cli-harness-eval-arg-parsing` golden-task pattern
    /// (`crates/vox-cli/src/commands/harness/eval.rs`) of exercising clap-derive
    /// dispatch directly, without a live backend.
    #[test]
    fn chat_args_parse_defaults_and_overrides() {
        let default_args = ChatArgs::try_parse_from(["chat", "hello there"])
            .expect("default parse should succeed");
        assert_eq!(default_args.message, "hello there");
        assert_eq!(default_args.model, "openrouter/auto");
        assert_eq!(default_args.system, None);

        let explicit_args = ChatArgs::try_parse_from([
            "chat",
            "hi",
            "--model",
            "anthropic/claude-3-7-sonnet",
            "--system",
            "be terse",
        ])
        .expect("explicit parse should succeed");
        assert_eq!(explicit_args.message, "hi");
        assert_eq!(explicit_args.model, "anthropic/claude-3-7-sonnet");
        assert_eq!(explicit_args.system.as_deref(), Some("be terse"));
    }

    #[test]
    fn chat_routing_ollama_vs_openrouter() {
        let local_qwen = resolve_chat_config("qwen3:8b");
        assert_eq!(local_qwen.provider, "ollama");

        let openrouter_qwen = resolve_chat_config("qwen/qwen3-32b-instruct");
        assert_eq!(openrouter_qwen.provider, "openrouter");

        let catalog_local_qwen = resolve_chat_config("qwen/qwen-3-8b");
        assert_eq!(catalog_local_qwen.provider, "voxlocal");

        let local_qwen25 = resolve_chat_config("qwen2.5-coder:7b");
        assert_eq!(local_qwen25.provider, "ollama");

        let openrouter_qwen25 = resolve_chat_config("qwen/qwen2.5-coder-32b-instruct");
        assert_eq!(openrouter_qwen25.provider, "openrouter");
    }

    #[test]
    fn chat_args_require_message() {
        assert!(ChatArgs::try_parse_from(["chat"]).is_err());
    }

    #[test]
    fn chat_routing_auto_and_mens() {
        let auto_cfg = resolve_chat_config("auto");
        assert_eq!(auto_cfg.provider, "voxlocal");
        assert!(auto_cfg.base_url.unwrap().contains("11434"));
        assert!(!auto_cfg.model.is_empty());

        let mens_cfg = resolve_chat_config("mens/runs/qwen3_27b_metal_check/quant_q8_0");
        assert_eq!(mens_cfg.provider, "voxlocal");
        assert_eq!(mens_cfg.model, "mens/runs/qwen3_27b_metal_check/quant_q8_0");

        let vox_mens_cfg = resolve_chat_config("vox-mens-8b-v0.6");
        assert_eq!(vox_mens_cfg.provider, "voxlocal");
        assert_eq!(vox_mens_cfg.model, "vox-mens-8b-v0.6");
    }

    #[test]
    fn test_is_connection_error_detection() {
        assert!(is_connection_error("tcp connect error: Connection refused"));
        assert!(is_connection_error(
            "error sending request for url (http://127.0.0.1:11434/v1/chat/completions): connection refused"
        ));
        assert!(is_connection_error("os error 61"));
        assert!(is_connection_error("failed to connect to host"));

        // HTTP status errors from running local server must NOT be classified as connection errors:
        assert!(!is_connection_error(
            "HTTP status client error (404 Not Found) for url (http://127.0.0.1:11434/v1/chat/completions)"
        ));
        assert!(!is_connection_error("status code: 400 Bad Request"));
        assert!(!is_connection_error(
            "status code: 500 Internal Server Error"
        ));
        assert!(!is_connection_error("status code: 404 Not Found"));
    }
}
