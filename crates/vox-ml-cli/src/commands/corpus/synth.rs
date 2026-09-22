//! Shared plumbing for the compiler-verified synthesis stages
//! (`back-translate`, `rft`): the chat seam, the facade-backed implementation,
//! spend-gate flags, and the compile oracle.
//!
//! Every LLM call goes through `vox_actor_runtime::llm::infer_with_retry`; the
//! [`ChatBackend`] trait exists so tests can substitute a scripted backend and
//! assert how many calls were made.

use anyhow::{Context, Result, bail};
use std::path::Path;

use crate::commands::mens::metrics::{CompletionVerification, extract_code, verify_completion};

/// One chat completion.
pub(crate) struct ChatReply {
    pub text: String,
    /// Model id reported by the provider (recorded per emitted row).
    pub model: String,
    pub cost_usd: f64,
}

/// Minimal chat seam over the LLM facade.
pub(crate) trait ChatBackend {
    /// Configured model id; part of the cache key.
    fn model_id(&self) -> String;
    async fn chat(&self, system: &str, user: &str, temperature: f32) -> Result<ChatReply>;
}

/// Spend-gated LLM flags shared by the synthesis commands.
#[derive(clap::Args, Debug, Clone)]
pub struct SynthLlmArgs {
    /// Provider: `openrouter`, `openai`, or `ollama` (local Populi / `vox mens serve`).
    #[arg(long, default_value = "openrouter")]
    pub provider: String,
    /// Model id. Default: the configured chat-model preference for the provider
    /// (`VOX_OPENROUTER_CHAT_MODEL` / `VOX_POPULI_MODEL`).
    #[arg(long)]
    pub model: Option<String>,
    /// Override the chat-completions URL (e.g. a local `vox mens serve` endpoint).
    #[arg(long)]
    pub base_url: Option<String>,
    /// Hard stop once estimated spend reaches this many USD.
    #[arg(long, default_value_t = 1.0)]
    pub max_spend_usd: f64,
    /// Blended price used for the dry-run estimate and when the provider reports no cost.
    #[arg(long, default_value_t = 0.002)]
    pub usd_per_1k_tokens: f64,
    /// Actually call the model. Without this the command only prints the plan (zero calls).
    #[arg(long)]
    pub apply: bool,
}

/// [`ChatBackend`] backed by the model-agnostic facade.
pub(crate) struct FacadeBackend {
    cfg: vox_actor_runtime::llm::LlmConfig,
}

impl FacadeBackend {
    pub(crate) fn from_args(args: &SynthLlmArgs, max_tokens: u64) -> Result<Self> {
        use vox_actor_runtime::llm::LlmConfig;
        let prefs = vox_actor_runtime::model_resolution::RouteResolutionInput::default();
        let mut cfg = match args.provider.as_str() {
            "openrouter" => {
                LlmConfig::openrouter(args.model.clone().unwrap_or(prefs.openrouter_model))
            }
            "openai" => LlmConfig::openai(
                args.model
                    .clone()
                    .context("--model is required for --provider openai")?,
            ),
            "ollama" => LlmConfig::ollama(args.model.clone().unwrap_or(prefs.mens_chat_model)),
            other => bail!("unsupported --provider {other} (openrouter | openai | ollama)"),
        };
        if let Some(url) = &args.base_url {
            cfg.base_url = Some(url.clone());
        }
        cfg.cost_per_1k = Some(args.usd_per_1k_tokens);
        cfg.max_tokens = Some(max_tokens);
        Ok(Self { cfg })
    }
}

impl ChatBackend for FacadeBackend {
    fn model_id(&self) -> String {
        format!("{}/{}", self.cfg.provider, self.cfg.model)
    }

    async fn chat(&self, system: &str, user: &str, temperature: f32) -> Result<ChatReply> {
        use vox_actor_runtime::ActivityResult;
        use vox_actor_runtime::llm::{LlmChatMessage, infer_with_retry};
        let mut cfg = self.cfg.clone();
        cfg.temperature = Some(temperature);
        let msg = |role: &str, content: &str| LlmChatMessage {
            role: role.into(),
            content: content.into(),
            ..Default::default()
        };
        let messages = vec![msg("system", system), msg("user", user)];
        let opts = vox_actor_runtime::ActivityOptions::default();
        match infer_with_retry(&opts, messages, vec![cfg]).await {
            ActivityResult::Ok(Ok((res, _))) => Ok(ChatReply {
                cost_usd: res.cost_usd.unwrap_or(0.0),
                text: res.content,
                model: res.model,
            }),
            ActivityResult::Ok(Err(e)) => bail!("llm call failed: {e}"),
            ActivityResult::Failed(e) => bail!("llm activity failed: {e:?}"),
            ActivityResult::Cancelled => bail!("llm call cancelled"),
        }
    }
}

/// Rough token estimate (chars / 4) priced at `usd_per_1k`.
pub(crate) fn estimate_usd(prompt_chars: usize, output_tokens: usize, usd_per_1k: f64) -> f64 {
    ((prompt_chars / 4 + output_tokens) as f64 / 1000.0) * usd_per_1k
}

/// Strip fences / chat framing from a completion and normalize it the same way
/// `verify_completion` does, returning the code that would be trained on.
pub(crate) fn clean_code(completion: &str) -> String {
    let extracted = extract_code(completion);
    let normalized = vox_compiler::generated_vox::normalize_generated_vox(
        &extracted,
        vox_compiler::generated_vox::OutputSurfaceMode::RawCodeOnly,
    );
    if normalized.normalized.trim().is_empty() {
        extracted
    } else {
        normalized.normalized
    }
}

/// Compile oracle: the eval-local verifier (parse + typecheck + anti-stub, and
/// `@test` execution when the code has tests).
pub(crate) fn verify(code: &str, tag: &str, idx: usize) -> CompletionVerification {
    verify_completion(code, Path::new("."), "", tag, idx, &[])
}

/// Read non-empty JSONL lines as JSON objects (malformed lines are skipped with a warning).
pub(crate) fn read_jsonl(path: &Path) -> Result<Vec<serde_json::Value>> {
    let raw = vox_bounded_fs::read_utf8_path_capped(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    Ok(raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| match serde_json::from_str(l) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!("skipping malformed JSONL line in {}: {e}", path.display());
                None
            }
        })
        .collect())
}

/// Write rows as JSONL, creating the parent directory.
pub(crate) fn write_jsonl(path: &Path, rows: &[serde_json::Value]) -> Result<()> {
    if let Some(p) = path.parent()
        && !p.as_os_str().is_empty()
    {
        std::fs::create_dir_all(p)?;
    }
    let mut body = String::new();
    for r in rows {
        body.push_str(&serde_json::to_string(r)?);
        body.push('\n');
    }
    std::fs::write(path, body).with_context(|| format!("cannot write {}", path.display()))
}

/// Standard vox_codegen pair row.
pub(crate) fn pair_row(
    prompt: &str,
    response: &str,
    category: &str,
    source: &str,
    difficulty: u64,
    rating: u64,
) -> serde_json::Value {
    serde_json::json!({
        "prompt": prompt,
        "response": response,
        "messages": [
            {"role": "user", "content": prompt},
            {"role": "assistant", "content": response},
        ],
        "category": category,
        "lane": "vox_codegen",
        "response_mode": "code_only",
        "task_family": "vox_codegen",
        "difficulty": difficulty,
        "source": source,
        "rating": rating,
        "schema_version": "vox_dogfood_v1",
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::cell::RefCell;

    /// Scripted backend: replies with `reply(system, user)` and counts calls.
    pub(crate) struct MockBackend<F: Fn(&str, &str) -> String> {
        pub reply: F,
        pub calls: RefCell<usize>,
    }

    impl<F: Fn(&str, &str) -> String> MockBackend<F> {
        pub(crate) fn new(reply: F) -> Self {
            Self {
                reply,
                calls: RefCell::new(0),
            }
        }
        pub(crate) fn calls(&self) -> usize {
            *self.calls.borrow()
        }
    }

    impl<F: Fn(&str, &str) -> String> ChatBackend for MockBackend<F> {
        fn model_id(&self) -> String {
            "mock/model".into()
        }
        async fn chat(&self, system: &str, user: &str, _t: f32) -> Result<ChatReply> {
            *self.calls.borrow_mut() += 1;
            Ok(ChatReply {
                text: (self.reply)(system, user),
                model: "mock/model".into(),
                cost_usd: 0.01,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_accepts_valid_and_rejects_broken_vox() {
        assert!(
            verify(
                "fn add(a: int, b: int) to int {\n    return a + b\n}\n",
                "t",
                0
            )
            .pass
        );
        assert!(!verify("fn add(a: int, b: int) to int {\n    return a +\n", "t", 1).pass);
    }

    #[test]
    fn clean_code_strips_fences() {
        let c = clean_code("```vox\nfn f() to int {\n    return 1\n}\n```");
        assert!(!c.contains("```"));
        assert!(c.contains("fn f()"));
    }

    #[test]
    fn estimate_is_linear_in_price() {
        assert!((estimate_usd(4000, 0, 1.0) - 1.0).abs() < 1e-9);
        assert_eq!(estimate_usd(4000, 1000, 0.0), 0.0);
    }

    #[test]
    fn facade_backend_rejects_unknown_provider() {
        let args = SynthLlmArgs {
            provider: "acme".into(),
            model: Some("m".into()),
            base_url: None,
            max_spend_usd: 1.0,
            usd_per_1k_tokens: 0.0,
            apply: false,
        };
        assert!(FacadeBackend::from_args(&args, 10).is_err());
    }
}
