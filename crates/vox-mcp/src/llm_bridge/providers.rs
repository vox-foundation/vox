//! HTTP clients for OpenRouter-compatible, Gemini, and Ollama chat APIs.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::Serialize;

use super::error::HttpInferError;
use super::limits::{OLLAMA_PROBE_CACHE_TTL_SECS, OLLAMA_PROBE_TIMEOUT_SECS};

/// Base URL for Ollama (`OLLAMA_HOST` or Mens local default).
pub(crate) fn ollama_base_url() -> String {
    std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| vox_config::inference::local_ollama_populi_base_url())
}

fn ollama_probe_ok_at() -> &'static Mutex<Option<Instant>> {
    static CACHE: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Cheap `GET /api/tags` probe so routing to Ollama fails fast with a clear message.
///
/// Successful probes are cached per-process for [`super::limits::OLLAMA_PROBE_CACHE_TTL_SECS`].
pub(crate) async fn probe_ollama_tags(client: &reqwest::Client) -> Result<(), HttpInferError> {
    let ttl = Duration::from_secs(OLLAMA_PROBE_CACHE_TTL_SECS);
    {
        let cache = ollama_probe_ok_at()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(t0) = *cache {
            if t0.elapsed() < ttl {
                return Ok(());
            }
        }
    }

    let base = ollama_base_url();
    let url = format!("{}/api/tags", base.trim_end_matches('/'));
    let res = client
        .get(&url)
        .timeout(Duration::from_secs(OLLAMA_PROBE_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| HttpInferError {
            status: 0,
            message: format!(
                "Ollama unreachable at {base} ({e}); set OLLAMA_HOST or start Ollama / Mens."
            ),
        })?;
    let code = res.status().as_u16();
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        let err = HttpInferError {
            status: code,
            message: format!("Ollama /api/tags error: {t}"),
        };
        let mut cache = ollama_probe_ok_at()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *cache = None;
        return Err(err);
    }
    let mut cache = ollama_probe_ok_at()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *cache = Some(Instant::now());
    Ok(())
}

/// OpenAI-compatible chat completion response (OpenRouter, HF router, etc.).
#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMeta>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMeta {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaMsg>,
    #[serde(default)]
    eval_count: u32,
    #[serde(default)]
    prompt_eval_count: u32,
}

#[derive(Debug, Deserialize)]
struct OllamaMsg {
    content: Option<String>,
}

#[derive(Serialize)]
struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMsg<'a>>,
    temperature: f32,
    max_tokens: u64,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct OpenAiMsg<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerateBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSys<'a>>,
    contents: Vec<GeminiTurn<'a>>,
    generation_config: GeminiGenCfg<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiSys<'a> {
    parts: Vec<GeminiPartOut<'a>>,
}

#[derive(Serialize)]
struct GeminiPartOut<'a> {
    text: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiTurn<'a> {
    role: &'a str,
    parts: Vec<GeminiPartOut<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenCfg<'a> {
    temperature: f32,
    max_output_tokens: u32,
    #[serde(rename = "responseMimeType", skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<&'a str>,
}

#[derive(Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaChatMsg<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<serde_json::Value>,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaChatMsg<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

pub(crate) async fn http_openai_compatible(
    client: &reqwest::Client,
    url: &str,
    bearer: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<(String, u32, u32), HttpInferError> {
    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(OpenAiMsg {
            role: "system",
            content: system,
        });
    }
    messages.push(OpenAiMsg {
        role: "user",
        content: user,
    });

    let response_format = if json_mode {
        Some(serde_json::json!({ "type": "json_object" }))
    } else {
        None
    };

    let body = OpenAiChatRequest {
        model,
        messages,
        temperature,
        max_tokens,
        stream: false,
        response_format,
    };

    let mut req = client.post(url).json(&body);
    if !bearer.is_empty() {
        req = req.bearer_auth(bearer);
    }

    let res = req.send().await.map_err(|e| HttpInferError {
        status: 0,
        message: format!("LLM HTTP: {e}"),
    })?;
    let status = res.status();
    let code = status.as_u16();

    if !status.is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(HttpInferError {
            status: code,
            message: t,
        });
    }

    let parsed: OpenAiChatResponse = res.json().await.map_err(|e| HttpInferError {
        status: code,
        message: format!("LLM JSON: {e}"),
    })?;

    let text = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .unwrap_or_default();

    let u = parsed.usage.unwrap_or(OpenAiUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
    });

    Ok((text, u.prompt_tokens, u.completion_tokens))
}

pub(crate) async fn http_gemini(
    client: &reqwest::Client,
    model_id: &str,
    api_key: &str,
    system: &str,
    user: &str,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<(String, u32, u32), HttpInferError> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model_id}:generateContent?key={api_key}"
    );

    let system_instruction = if system.is_empty() {
        None
    } else {
        Some(GeminiSys {
            parts: vec![GeminiPartOut { text: system }],
        })
    };

    let response_mime_type = if json_mode {
        Some("application/json")
    } else {
        None
    };

    let body = GeminiGenerateBody {
        system_instruction,
        contents: vec![GeminiTurn {
            role: "user",
            parts: vec![GeminiPartOut { text: user }],
        }],
        generation_config: GeminiGenCfg {
            temperature,
            max_output_tokens: max_tokens.min(u32::MAX as u64) as u32,
            response_mime_type,
        },
    };

    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| HttpInferError {
            status: 0,
            message: format!("Gemini HTTP: {e}"),
        })?;
    let status = res.status();
    let code = status.as_u16();

    if !status.is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(HttpInferError {
            status: code,
            message: t,
        });
    }

    let parsed: GeminiResponse = res.json().await.map_err(|e| HttpInferError {
        status: code,
        message: format!("Gemini JSON: {e}"),
    })?;

    let text = parsed
        .candidates
        .unwrap_or_default()
        .into_iter()
        .next()
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.text)
        .collect::<Vec<_>>()
        .join("");

    let prompt_t = parsed
        .usage_metadata
        .as_ref()
        .and_then(|u| u.prompt_token_count)
        .unwrap_or(0);
    let out_t = parsed
        .usage_metadata
        .as_ref()
        .and_then(|u| u.candidates_token_count)
        .unwrap_or(0);

    Ok((text, prompt_t, out_t))
}

pub(crate) async fn http_ollama(
    client: &reqwest::Client,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<(String, u32, u32), HttpInferError> {
    let base = ollama_base_url();
    let url = format!("{}/api/chat", base.trim_end_matches('/'));

    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(OllamaChatMsg {
            role: "system",
            content: system,
        });
    }
    messages.push(OllamaChatMsg {
        role: "user",
        content: user,
    });

    let format = if json_mode {
        Some(serde_json::json!("json"))
    } else {
        None
    };

    let body = OllamaChatRequest {
        model,
        messages,
        stream: false,
        format,
        options: OllamaOptions {
            temperature,
            num_predict: max_tokens.min(i32::MAX as u64) as i32,
        },
    };

    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| HttpInferError {
            status: 0,
            message: format!("Ollama HTTP: {e}"),
        })?;
    let status = res.status();
    let code = status.as_u16();

    if !status.is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(HttpInferError {
            status: code,
            message: t,
        });
    }

    let parsed: OllamaChatResponse = res.json().await.map_err(|e| HttpInferError {
        status: code,
        message: format!("Ollama JSON: {e}"),
    })?;

    let text = parsed.message.and_then(|m| m.content).unwrap_or_default();
    Ok((text, parsed.prompt_eval_count, parsed.eval_count))
}
