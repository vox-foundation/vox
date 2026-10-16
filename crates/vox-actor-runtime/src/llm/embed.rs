//! Durable embedding HTTP client.

use std::future::Future;
use std::pin::Pin;

use crate::{ActivityOptions, ActivityResult, execute_activity};

use super::types::LlmConfig;

type LlmEmbedActivityFuture =
    Pin<Box<dyn Future<Output = Result<Result<Vec<f32>, String>, String>> + Send>>;

/// Default embedding dimension for the built-in `provider = "local"` feature embedder.
pub const LOCAL_EMBED_DEFAULT_DIM: usize = 256;

fn fnv1a64(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = seed ^ 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Deterministic, offline, L2-normalized local embedding over word unigrams, word bigrams,
/// and character trigrams (keyed by `model` so distinct model IDs do not share projection slots).
#[must_use]
pub fn local_embed_text(text: &str, model: &str) -> Vec<f32> {
    let dim = LOCAL_EMBED_DEFAULT_DIM;
    let mut vec = vec![0.0_f32; dim];
    let model_seed = fnv1a64(0, model.as_bytes());
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect();

    for (idx, tok) in tokens.iter().enumerate() {
        let h = fnv1a64(model_seed, tok.as_bytes());
        let bucket = (h as usize) % dim;
        let sign = if (h >> 63) == 0 { 1.0_f32 } else { -1.0_f32 };
        vec[bucket] += sign * 1.5;

        if let Some(next) = tokens.get(idx + 1) {
            let bigram = format!("{tok} {next}");
            let hb = fnv1a64(model_seed ^ 0x9e37_79b9_7f4a_7c15, bigram.as_bytes());
            let b_bucket = (hb as usize) % dim;
            let b_sign = if (hb >> 63) == 0 { 1.0_f32 } else { -1.0_f32 };
            vec[b_bucket] += b_sign * 1.0;
        }

        let chars: Vec<char> = tok.chars().collect();
        for win in chars.windows(3) {
            let mut buf = [0u8; 12];
            let mut len = 0;
            for &ch in win {
                len += ch.encode_utf8(&mut buf[len..]).len();
            }
            let hc = fnv1a64(model_seed ^ 0x517c_c1b7_2722_0a95, &buf[..len]);
            let c_bucket = (hc as usize) % dim;
            let c_sign = if (hc >> 63) == 0 { 1.0_f32 } else { -1.0_f32 };
            vec[c_bucket] += c_sign * 0.35;
        }
    }

    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for v in &mut vec {
            *v /= norm;
        }
    } else if !vec.is_empty() {
        vec[0] = 1.0;
    }
    vec
}

/// Core durable wrapper for LLM embedding generation.
pub async fn llm_embed(
    options: &ActivityOptions,
    text: &str,
    config: LlmConfig,
) -> ActivityResult<Result<Vec<f32>, String>> {
    let activity_name = format!("llm_embed_{}_{}", config.provider, config.model);

    execute_activity(&activity_name, options, || {
        let text = text.to_string();
        let config = config.clone();

        let fut = async move {
            if config.provider == "local" && config.base_url.is_none() {
                return Ok(Ok(local_embed_text(&text, &config.model)));
            }

            // Resolve the embeddings endpoint and pass it as the base-url override so the
            // egress core posts to /embeddings (resolve_egress's default is chat/completions).
            let embed_base =
                config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| match config.provider.as_str() {
                        "openrouter" => vox_config::openrouter_embeddings_url(),
                        "openai" => vox_config::openai_embeddings_url(),
                        "hf_router" | "huggingface" => {
                            "https://router.huggingface.co/v1/embeddings".to_string()
                        }
                        _ => vox_config::openrouter_embeddings_url(),
                    });
            if matches!(config.provider.as_str(), "hf_endpoint")
                && (embed_base.trim().is_empty() || !embed_base.contains("embeddings"))
            {
                return Ok(Err(
                    "hf_endpoint embeddings require base_url pointing to …/v1/embeddings"
                        .to_string(),
                ));
            }

            let input = vox_config::resolve_egress::EgressResolveInput {
                provider: config.provider.clone(),
                model: config.model.clone(),
                base_url_override: Some(embed_base),
                timeout_ms: config.timeout_ms,
                api_key_override: config.api_key.clone(),
            };
            let ereq = match vox_config::resolve_egress::resolve_egress(&input) {
                Ok(r) => r,
                Err(e) => return Ok(Err(e)),
            };
            let vector = match vox_llm_egress::embed_once(&ereq, &text).await {
                Ok(v) => v,
                Err(e) => return Ok(Err(e.to_string())),
            };

            if vector.is_empty() {
                return Ok(Err("LLM API returned empty embedding vector".to_string()));
            }

            Ok(Ok(vector))
        };
        let fut_typed: LlmEmbedActivityFuture = Box::pin(fut);
        fut_typed
    })
    .await
}
