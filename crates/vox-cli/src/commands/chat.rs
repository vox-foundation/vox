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
    /// Model id or SOTA routing mode to use:
    /// - `auto` (or default `openrouter/auto`): SOTA task-aware selector across all keyed providers
    /// - `free`: zero-cost routing across verified `:free` OpenRouter models and reachable local models
    /// - `efficient` (or `fast`): low-latency, cost-efficient SOTA tier (`CostPreference::Efficiency`)
    /// - `balanced`: optimal quality/cost pareto tier (`CostPreference::Balanced`)
    /// - `genius` (or `sota` / `frontier`): maximum reasoning & coding intelligence (`CostPreference::Performance`)
    /// - `auto-local`: best local MENS/Ollama model for this hardware
    /// - `<provider/model>`: explicit model slug
    #[arg(long, default_value = vox_config::bootstrap_inference::OPENROUTER_AUTO)]
    pub model: String,
    /// Optional system prompt prepended before the user message.
    #[arg(long)]
    pub system: Option<String>,
    /// Optional hard turn/session spend cap in USD (0% Vox markup; auto-reload is permanently disabled).
    /// When set to `<= 0.0`, routing automatically shifts to `--model free` / local MENS.
    #[arg(long)]
    pub budget_usd: Option<f64>,
}

/// Canonical Anti-Sycophancy & Craft Directness Preamble (`HUMANE-03`).
pub const ANTI_SYCOPHANCY_PREAMBLE: &str = "\
## Humane & Craft Directness Contract (HUMANE-03)\n\
- Never use hollow sycophantic praise (\"Great question!\", \"You're absolutely right!\", \"Brilliant idea!\").\n\
- Push back concisely with technical evidence when a requested pattern violates architecture, security, or YAGNI.\n\
- Optimize for single-shot verified closure rather than open-ended conversational follow-up bait.\n\
- Ground architectural claims in concrete file paths, symbol names, and compiler/test invariants.";

/// Build the effective system prompt for `vox chat`, weaving in [`ANTI_SYCOPHANCY_PREAMBLE`]
/// and (when `vox_mode` is active) the 2026 `.vox` Script Writer contract.
pub fn build_chat_system_prompt(user_system: Option<&str>, vox_mode: bool) -> String {
    if let Some(custom) = user_system {
        format!("{custom}\n\n{ANTI_SYCOPHANCY_PREAMBLE}")
    } else if vox_mode {
        let base_prompt = vox_corpus::training::generate_system_prompt();
        format!(
            "{base_prompt}\n\n\
             ## Output Contract for `.vox` Script Requests\n\
             - Always enclose `.vox` code in a single ```vox fenced block.\n\
             - Use `fn name(param: Type) to ReturnType {{ ... }}` (NEVER `->` for function return types).\n\
             - Use `return expr` for returns and `//` for comments (NEVER `#`).\n\
             - For scripts, define typed helpers and `fn main() to Unit {{ ... }}` (or `fn main() to Result[Unit] {{ ... }}`).\n\
             - Use `Result[T]` with `Ok(val)` and `Error(msg)` (NEVER `Err(...)`).\n\n\
             {ANTI_SYCOPHANCY_PREAMBLE}"
        )
    } else {
        ANTI_SYCOPHANCY_PREAMBLE.to_string()
    }
}

/// Canonical SOTA routing modes supported by `vox chat --model <mode>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SotaRoutingMode {
    Free,
    Auto,
    Efficient,
    Balanced,
    Genius,
}

impl SotaRoutingMode {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "free" | "auto-free" | "zero-cost" => Some(Self::Free),
            "auto" => Some(Self::Auto),
            "efficient" | "efficiency" | "fast" | "economy" => Some(Self::Efficient),
            "balanced" | "standard" => Some(Self::Balanced),
            "genius" | "sota" | "frontier" | "max" | "quality" => Some(Self::Genius),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Auto => "auto",
            Self::Efficient => "efficient",
            Self::Balanced => "balanced",
            Self::Genius => "genius",
        }
    }
}

/// Detect whether the user prompt asks to author `.vox` code or a `.vox` automation script.
pub fn is_vox_script_request(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    lower.contains(".vox")
        || lower.contains("vox script")
        || lower.contains("voxscript")
        || lower.contains("in vox")
        || lower.contains("vox code")
        || lower.contains("vox program")
        || lower.contains("vox function")
        || lower.contains("vox actor")
        || lower.contains("vox workflow")
        || lower.contains("vox component")
}

/// Extract fenced ```vox ... ``` blocks from an LLM reply.
pub fn extract_vox_code_blocks(reply: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_vox_block = false;
    let mut current = Vec::new();

    for line in reply.lines() {
        let trimmed = line.trim();
        if !in_vox_block {
            if trimmed.starts_with("```vox") {
                in_vox_block = true;
                current.clear();
            }
        } else if trimmed.starts_with("```") {
            in_vox_block = false;
            let joined = current.join("\n").trim().to_string();
            if !joined.is_empty() {
                blocks.push(joined);
            }
            current.clear();
        } else {
            current.push(line);
        }
    }
    blocks
}

/// Validate `.vox` source using the full compiler frontend (`lex -> parse -> hir -> typeck`).
/// Returns `Ok(())` if 0 errors, or `Err(formatted_diagnostics)` if errors exist.
pub fn verify_vox_snippet(source: &str) -> std::result::Result<(), String> {
    use vox_compiler::typeck::diagnostics::TypeckSeverity;
    match vox_compiler::pipeline::run_frontend_str(source, "chat_script.vox") {
        Ok(result) => {
            let errors: Vec<String> = result
                .diagnostics
                .iter()
                .filter(|d| d.severity == TypeckSeverity::Error)
                .map(|d| {
                    let enriched = d.clone().with_line_col(source);
                    let (line, col) = enriched
                        .line_col
                        .map(|lc| (lc.line_start, lc.col_start))
                        .unwrap_or((1, 1));
                    format!("Line {line}, Col {col}: {}", d.message)
                })
                .collect();
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("\n"))
            }
        }
        Err(e) => Err(format!("Compiler parse/lex error: {e}")),
    }
}

/// Run `vox chat`: send `args.message` (with optional `args.system` prompt) to
/// `args.model` via the shared `llm_chat` facade and print the reply.
pub async fn run(args: ChatArgs) -> Result<()> {
    if args.message.trim().is_empty() {
        bail!("vox chat: message must not be empty");
    }

    let vox_mode = is_vox_script_request(&args.message);
    let mut messages = Vec::with_capacity(4);
    let system_prompt = build_chat_system_prompt(args.system.as_deref(), vox_mode);
    messages.push(LlmChatMessage {
        role: "system".to_string(),
        content: system_prompt,
        ..Default::default()
    });
    messages.push(LlmChatMessage {
        role: "user".to_string(),
        content: args.message.clone(),
        ..Default::default()
    });

    let effective_model_arg = if let Some(cap) = args.budget_usd {
        if cap <= 0.0 {
            let sig = vox_orchestrator::budget::BudgetSignal::CostExceeded {
                cost_usd: 0.0,
                limit_usd: cap.max(0.0),
            };
            if let Some(note) = sig.cost_exceeded_recommendation() {
                eprintln!("budget-cap: {note}");
            }
            "free".to_string()
        } else {
            args.model.clone()
        }
    } else {
        args.model.clone()
    };

    let selected_tier_label = SotaRoutingMode::parse(&effective_model_arg)
        .map(|m| m.label().to_string())
        .unwrap_or_else(|| "explicit".to_string());

    let candidates: Vec<(LlmConfig, String, String)> =
        if let Some(mode) = SotaRoutingMode::parse(&effective_model_arg) {
            let routed = route_sota_mode(mode, &args.message).await?;
            if let Some((_, primary_id, reason)) = routed.first() {
                eprintln!("sota-route[{}]: {primary_id} ({reason})", mode.label());
            }
            routed
        } else {
            let mut cfg = resolve_chat_config(&effective_model_arg);
            cfg.telemetry_task_category = Some("cli-chat".to_string());
            vec![(
                cfg,
                effective_model_arg.clone(),
                "explicit model".to_string(),
            )]
        };

    let base_url = std::env::var("VOX_LOCAL_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

    let opts = ActivityOptions::new();
    let mut last_err = String::new();
    let mut final_response = None;
    let mut active_config = candidates[0].0.clone();

    for (idx, (mut config, model_id, _reason)) in candidates.into_iter().enumerate() {
        config.telemetry_task_category = Some(if vox_mode {
            "cli-chat-vox-script".to_string()
        } else {
            "cli-chat".to_string()
        });
        let is_local = config.provider == "voxlocal";
        let chat_result: Result<vox_actor_runtime::llm::LlmResponse, String> =
            match llm_chat(&opts, messages.clone(), config.clone()).await {
                vox_actor_runtime::ActivityResult::Ok(inner) => inner,
                vox_actor_runtime::ActivityResult::Failed(e) => Err(e.to_string()),
                vox_actor_runtime::ActivityResult::Cancelled => {
                    Err("activity cancelled".to_string())
                }
            };

        match chat_result {
            Ok(resp) if !resp.content.trim().is_empty() => {
                if idx > 0 {
                    eprintln!("sota-route[fallback#{idx}]: succeeded with {model_id}");
                }
                active_config = config;
                final_response = Some(resp);
                break;
            }
            Ok(_) => {
                last_err = format!("model {model_id} returned an empty response");
            }
            Err(e) => {
                if is_local && is_connection_error(&e) {
                    last_err = format!(
                        "Error: Local inference server is not running on {}. Start it with 'vox mens serve' or specify a cloud model.",
                        base_url.trim_end_matches('/')
                    );
                } else {
                    last_err = e;
                }
                eprintln!("sota-route[candidate#{idx} {model_id}]: {last_err}");
            }
        }
    }

    let mut response = match final_response {
        Some(r) => r,
        None => {
            if last_err.starts_with("Error: Local inference server is not running") {
                bail!("{last_err}");
            }
            return Err(anyhow::anyhow!(last_err)).context("vox chat: llm_chat failed");
        }
    };

    // Compiler-in-the-loop verification & self-repair for `.vox` script writer requests.
    if vox_mode {
        for attempt in 1..=2 {
            let blocks = extract_vox_code_blocks(&response.content);
            let target_code = blocks
                .first()
                .cloned()
                .unwrap_or_else(|| response.content.trim().to_string());
            match verify_vox_snippet(&target_code) {
                Ok(()) => {
                    eprintln!("vox-script-verify: PASS (compiler frontend 0 errors)");
                    break;
                }
                Err(diag) => {
                    eprintln!(
                        "vox-script-verify[attempt {attempt}]: repairing compiler diagnostics:\n{diag}"
                    );
                    messages.push(LlmChatMessage {
                        role: "assistant".to_string(),
                        content: response.content.clone(),
                        ..Default::default()
                    });
                    messages.push(LlmChatMessage {
                        role: "user".to_string(),
                        content: format!(
                            "The `.vox` code failed the Vox 2026 compiler check with:\n{diag}\n\n\
                             Fix all compiler errors and reply with the corrected `.vox` program inside a ```vox fenced block. \
                             Remember: use `fn name(...) to RetType {{ ... }}`, `return`, and `Error(msg)` (never `Err`)."
                        ),
                        ..Default::default()
                    });
                    if let vox_actor_runtime::ActivityResult::Ok(Ok(repaired)) =
                        llm_chat(&opts, messages.clone(), active_config.clone()).await
                    {
                        if !repaired.content.trim().is_empty() {
                            response = repaired;
                        }
                    }
                }
            }
        }
    }

    println!("{}", response.content);

    let receipt = vox_orchestrator::budget::TurnCostReceipt::new(
        if response.model.is_empty() {
            active_config.model.clone()
        } else {
            response.model.clone()
        },
        selected_tier_label,
        response.prompt_tokens as u64,
        response.cache_read_tokens as u64,
        response.completion_tokens as u64,
        response.cost_usd.unwrap_or(0.0),
    );
    eprintln!("{}", receipt.format_footer());

    Ok(())
}

#[allow(dead_code)]
fn is_local_mens_server_reachable() -> bool {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let endpoint = std::env::var("VOX_LOCAL_ENDPOINT")
        .or_else(|_| std::env::var("VOX_LOCAL_LLM_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());
    let host_port = endpoint
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or("127.0.0.1:11435");
    if let Ok(addr) = host_port.parse::<SocketAddr>() {
        TcpStream::connect_timeout(&addr, Duration::from_millis(120)).is_ok()
    } else {
        false
    }
}

/// Build the SOTA routing request for a given [`SotaRoutingMode`] and user prompt.
pub fn build_sota_selection_request(
    mode: SotaRoutingMode,
    prompt: &str,
) -> vox_orchestrator::models::ModelSelectionRequest {
    use vox_orchestrator::models::{ModelSelectionRequest, SelectionAxes, SelectionIntent};
    use vox_orchestrator::types::TaskCategory;

    let is_code = is_vox_script_request(prompt)
        || prompt.to_ascii_lowercase().contains("write a ")
        || prompt.to_ascii_lowercase().contains("implement ")
        || prompt.to_ascii_lowercase().contains("function ")
        || prompt.to_ascii_lowercase().contains("script");
    let task_category = if is_code {
        TaskCategory::CodeGen
    } else {
        TaskCategory::Chat
    };

    let intent = match mode {
        SotaRoutingMode::Free => SelectionIntent {
            task: task_category,
            axes: SelectionAxes::COST_FIRST,
            complexity: 4,
            context_size_hint: None,
            caller_hint: Some("chat-free"),
            prefer_local: false,
            max_cost_usd_per_call: Some(0.0),
            cacheable_workload: false,
            allow_free_in_performance_mode: true,
        },
        SotaRoutingMode::Efficient => SelectionIntent {
            task: task_category,
            axes: SelectionAxes::COST_FIRST,
            complexity: 3,
            context_size_hint: None,
            caller_hint: Some("chat-efficient"),
            prefer_local: false,
            max_cost_usd_per_call: None,
            cacheable_workload: false,
            allow_free_in_performance_mode: false,
        },
        SotaRoutingMode::Balanced => SelectionIntent {
            task: task_category,
            axes: SelectionAxes::BALANCED,
            complexity: 6,
            context_size_hint: None,
            caller_hint: Some("chat-balanced"),
            prefer_local: false,
            max_cost_usd_per_call: None,
            cacheable_workload: false,
            allow_free_in_performance_mode: false,
        },
        SotaRoutingMode::Genius => SelectionIntent {
            task: if is_code {
                TaskCategory::CodeGen
            } else {
                TaskCategory::Chat
            },
            axes: SelectionAxes::QUALITY_FIRST,
            complexity: 9,
            context_size_hint: None,
            caller_hint: Some("chat-genius"),
            prefer_local: false,
            max_cost_usd_per_call: None,
            cacheable_workload: false,
            allow_free_in_performance_mode: false,
        },
        SotaRoutingMode::Auto => SelectionIntent {
            task: task_category,
            axes: if is_code {
                SelectionAxes::QUALITY_FIRST
            } else {
                SelectionAxes::BALANCED
            },
            complexity: if is_code { 7 } else { 5 },
            context_size_hint: None,
            caller_hint: Some("chat-auto"),
            prefer_local: false,
            max_cost_usd_per_call: None,
            cacheable_workload: false,
            allow_free_in_performance_mode: false,
        },
    };
    ModelSelectionRequest::from_intent(intent)
}

/// Vox's canonical selection for a chat turn: the cached catalog plus every persisted
/// routing signal, through `decide()`, returning a ranked cascade of `(LlmConfig, id, reason)`.
async fn route_sota_mode(
    mode: SotaRoutingMode,
    prompt: &str,
) -> Result<Vec<(LlmConfig, String, String)>> {
    use vox_orchestrator::models::{ModelRegistry, decide, llm_config_for_spec};

    let mut registry = ModelRegistry::from_cache();
    vox_orchestrator::catalog::sync_installed_ollama(
        &mut registry,
        vox_config::local_ollama_populi_base_url(),
    )
    .await;
    if let Some(db) =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::CliWorkspace, true)
            .await
    {
        registry.inject_evidence_from_db(&db).await;
    }

    let request = build_sota_selection_request(mode, prompt);
    let task_category = request.intent.task;
    let decision = decide(&request, &registry).with_context(|| {
        format!(
            "vox chat --model {}: no routable model (no provider key resolves, or no candidate is \
             confirmed — run `vox model discover --force` or set a provider key)",
            mode.label()
        )
    })?;

    let mut cascade = Vec::new();
    let primary_reason = decision.outcome.reason.to_string();
    let primary_cfg = llm_config_for_spec(&decision.outcome.model_spec, task_category);
    cascade.push((
        primary_cfg,
        decision.selected_model.clone(),
        primary_reason.clone(),
    ));

    for cand_id in &decision.alternatives {
        if *cand_id == decision.selected_model {
            continue;
        }
        if let Some(spec) = registry.get(cand_id) {
            let cfg = llm_config_for_spec(&spec, task_category);
            cascade.push((cfg, cand_id.clone(), "alternative candidate".to_string()));
        }
        if cascade.len() >= 4 {
            break;
        }
    }

    if mode == SotaRoutingMode::Free {
        let mut or_free = LlmConfig::openrouter("qwen/qwen3-coder:free");
        or_free.cost_per_1k = Some(0.0);
        cascade.push((
            or_free,
            "qwen/qwen3-coder:free".to_string(),
            "openrouter verified :free fallback".to_string(),
        ));
    } else if cascade.len() < 2 {
        let or_auto = LlmConfig::openrouter(vox_config::bootstrap_inference::OPENROUTER_AUTO);
        cascade.push((
            or_auto,
            vox_config::bootstrap_inference::OPENROUTER_AUTO.to_string(),
            "openrouter/auto router fallback".to_string(),
        ));
    }

    Ok(cascade)
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
    if model == "auto-local"
        || model.starts_with("vox-mens-")
        || model.starts_with("mens/")
        || model.starts_with("voxlocal")
        || model.starts_with("populi_local")
        || model == "qwen/qwen-3-8b"
    {
        let effective_model = if model == "auto-local" {
            // Discrete-GPU VRAM comes from the nvml-probe plugin, if installed.
            vox_orchestrator_mcp::server_state::vram_probe::register_nvml_vram_probe();
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
            seed: None,
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
        let auto_cfg = resolve_chat_config("auto-local");
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

    #[test]
    fn sota_routing_modes_parse_and_configure_intents() {
        use vox_orchestrator::types::TaskCategory;

        assert_eq!(SotaRoutingMode::parse("free"), Some(SotaRoutingMode::Free));
        assert_eq!(SotaRoutingMode::parse("auto"), Some(SotaRoutingMode::Auto));
        assert_eq!(
            SotaRoutingMode::parse("efficient"),
            Some(SotaRoutingMode::Efficient)
        );
        assert_eq!(
            SotaRoutingMode::parse("balanced"),
            Some(SotaRoutingMode::Balanced)
        );
        assert_eq!(
            SotaRoutingMode::parse("genius"),
            Some(SotaRoutingMode::Genius)
        );
        assert_eq!(SotaRoutingMode::parse("anthropic/claude-sonnet-4"), None);

        let free_req = build_sota_selection_request(SotaRoutingMode::Free, "Write a .vox script");
        assert!(free_req.intent.allow_free_in_performance_mode);
        assert_eq!(free_req.intent.task, TaskCategory::CodeGen);

        let genius_req =
            build_sota_selection_request(SotaRoutingMode::Genius, "Analyze distributed consensus");
        assert!(!genius_req.intent.allow_free_in_performance_mode);
        assert_eq!(genius_req.intent.task, TaskCategory::Chat);
        assert_eq!(genius_req.intent.complexity, 9);
    }

    #[test]
    fn vox_script_extractor_and_compiler_verifier_work() {
        assert!(is_vox_script_request(
            "Write a .vox script that formats files"
        ));
        assert!(!is_vox_script_request("What is the capital of France?"));

        let sample_reply = "Here is your script:\n```vox\nfn add(a: int, b: int) to int {\n    return a + b\n}\n\nfn main() to Unit {\n    print(str(add(2, 3)))\n}\n```\n";
        let blocks = extract_vox_code_blocks(sample_reply);
        assert_eq!(blocks.len(), 1);
        assert!(
            verify_vox_snippet(&blocks[0]).is_ok(),
            "valid 2026 .vox script must pass compiler verification"
        );
    }

    #[test]
    fn humane_anti_sycophancy_prompt_and_budget_usd_flag_work() {
        let prompt = build_chat_system_prompt(None, false);
        assert!(prompt.contains("HUMANE-03"));
        assert!(prompt.contains("Never use hollow sycophantic praise"));
        assert!(prompt.contains("single-shot verified closure"));

        let vox_prompt = build_chat_system_prompt(None, true);
        assert!(vox_prompt.contains("Output Contract for `.vox` Script Requests"));
        assert!(vox_prompt.contains("HUMANE-03"));

        let args = ChatArgs::try_parse_from(["chat", "check budget", "--budget-usd", "0.0"])
            .expect("budget_usd flag should parse");
        assert_eq!(args.budget_usd, Some(0.0));
    }
}
