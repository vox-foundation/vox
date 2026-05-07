//! Clavis-first LLM routing readiness (model prefs + at least one provider key).

use super::super::common::{Check, redact_key};
use vox_clavis::SecretId;
use vox_config::clavis_str;
use vox_config::inference::{OPENROUTER_CHAT_COMPLETIONS_URL, openrouter_chat_model_preference};

pub fn run(checks: &mut Vec<Check>) {
    let model = openrouter_chat_model_preference();
    let routing_profile = clavis_str(SecretId::VoxRoutingProfile).unwrap_or_else(|| {
        // Default from routing contract / Clavis spec — not a hard error when unset.
        "quality_first".to_string()
    });

    let mut keys: Vec<&'static str> = Vec::new();
    if vox_clavis::resolve_secret(SecretId::OpenRouterApiKey)
        .expose()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        keys.push("OpenRouter");
    }
    if vox_clavis::resolve_secret(SecretId::OpenaiApiKey)
        .expose()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        keys.push("OpenAI");
    }
    if vox_clavis::resolve_secret(SecretId::GeminiApiKey)
        .expose()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        keys.push("Gemini");
    }
    if vox_clavis::resolve_secret(SecretId::AnthropicApiKey)
        .expose()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        keys.push("Anthropic");
    }

    let detail = format!(
        "routing_profile={routing_profile}; openrouter_model={}; chat_completions_url={}; provider_keys_present=[{}]",
        model,
        OPENROUTER_CHAT_COMPLETIONS_URL,
        if keys.is_empty() {
            "(none)".to_string()
        } else {
            keys.join(", ")
        }
    );

    if keys.is_empty() {
        checks.push(Check::fail(
            "LLM routing (Clavis)",
            format!(
                "{detail} — no LLM API key resolved via Clavis; set e.g. OpenRouter via `vox clavis doctor` / login."
            ),
        ));
    } else {
        checks.push(Check::pass("LLM routing (Clavis)", detail));
    }

    // Informational: confirm account id for vault sync (optional).
    let acct = std::env::var(vox_clavis::OPERATOR_ACCOUNT_ID).unwrap_or_default();
    if acct.trim().is_empty() {
        checks.push(Check::new(
            "LLM routing — VOX_ACCOUNT_ID",
            true,
            "not set (optional for local keys only); use `vox clavis login` for cross-machine sync"
                .to_string(),
        ));
    } else {
        checks.push(Check::pass(
            "LLM routing — VOX_ACCOUNT_ID",
            format!("set ({})", redact_key(&acct)),
        ));
    }

    checks.push(Check::new(
        "Cloud vault login (profile)",
        true,
        crate::commands::login_shared::login_status_summary(),
    ));

    let cache_path = vox_config::paths::dot_vox_user_dir()
        .join("cache")
        .join("model-catalog.v1.json");
    let cache_status = if cache_path.exists() {
        match std::fs::read_to_string(&cache_path) {
            Ok(raw) => match serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
                Ok(v) => format!(
                    "{} exists ({} cached model entries)",
                    cache_path.display(),
                    v.len()
                ),
                Err(_) => format!("{} exists (unparseable JSON)", cache_path.display()),
            },
            Err(e) => format!("{} exists (read error: {e})", cache_path.display()),
        }
    } else {
        format!(
            "{} missing — run `vox model discover`",
            cache_path.display()
        )
    };
    checks.push(Check::new(
        "LLM routing — model catalog cache",
        true,
        cache_status,
    ));
}
