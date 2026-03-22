//! Minimal Hugging Face `config.json` parsing for architecture diagnostics and layout validation.

use std::path::Path;

use anyhow::Context;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HfArchitecture {
    Gpt2,
    Qwen2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDims {
    pub n_embd: usize,
    pub n_head: usize,
    pub n_layer: usize,
    pub vocab_size: usize,
}

/// Structured transformer layout from HF `config.json` (Llama/Mistral/Qwen-style vs GPT-2 keys).
///
/// Used for preflight validation and future multi-layer QLoRA graph construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HfTransformerLayout {
    /// GPT-2 vs stacked causal LM (Llama / Mistral / Qwen2 field layout).
    pub architecture: HfArchitecture,
    pub model_type: String,
    pub architectures: Vec<String>,
    pub hidden_size: usize,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub vocab_size: usize,
    /// Same numbers as the HF fields above, in [`ConfigDims`] shape (legacy / graph code).
    pub dims: ConfigDims,
}

impl HfTransformerLayout {
    /// Parse `config.json` at `path` into a structured layout.
    pub fn from_config_path(path: &Path) -> anyhow::Result<Self> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        Self::from_config_json_str(&raw).with_context(|| format!("parse {}", path.display()))
    }

    /// Parse a `config.json` string (tests + tooling).
    pub fn from_config_json_str(raw: &str) -> anyhow::Result<Self> {
        let v: Value = serde_json::from_str(raw)?;
        Self::from_value(&v)
    }

    fn from_value(v: &Value) -> anyhow::Result<Self> {
        let architecture = classify_hf_architecture(v);
        let model_type = v
            .get("model_type")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let architectures: Vec<String> = v
            .get("architectures")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        // Llama / Mistral / Qwen2 / many causal LMs
        if let (Some(h), Some(nh), Some(nl), Some(vs)) = (
            json_usize(v, "hidden_size"),
            json_usize(v, "num_attention_heads"),
            json_usize(v, "num_hidden_layers"),
            json_usize(v, "vocab_size"),
        ) {
            let dims = ConfigDims {
                n_embd: h,
                n_head: nh,
                n_layer: nl,
                vocab_size: vs,
            };
            return Ok(Self {
                architecture,
                model_type,
                architectures,
                hidden_size: h,
                num_attention_heads: nh,
                num_hidden_layers: nl,
                vocab_size: vs,
                dims,
            });
        }

        // GPT-2 style
        if let (Some(n_embd), Some(n_head), Some(n_layer), Some(vs)) = (
            json_usize(v, "n_embd"),
            json_usize(v, "n_head"),
            json_usize(v, "n_layer"),
            json_usize(v, "vocab_size"),
        ) {
            let dims = ConfigDims {
                n_embd,
                n_head,
                n_layer,
                vocab_size: vs,
            };
            return Ok(Self {
                architecture,
                model_type,
                architectures,
                hidden_size: n_embd,
                num_attention_heads: n_head,
                num_hidden_layers: n_layer,
                vocab_size: vs,
                dims,
            });
        }

        anyhow::bail!(
            "config.json: need either (hidden_size, num_attention_heads, num_hidden_layers, vocab_size) \
             or (n_embd, n_head, n_layer, vocab_size)"
        )
    }
}

fn json_usize(v: &Value, key: &str) -> Option<usize> {
    v.get(key).and_then(|x| x.as_u64()).map(|u| u as usize)
}

fn classify_hf_architecture(v: &Value) -> HfArchitecture {
    let arch = v
        .get("architectures")
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let model_type = v.get("model_type").and_then(|x| x.as_str()).unwrap_or("");
    let arch_l = arch.to_lowercase();
    if arch_l.contains("qwen") || model_type.contains("qwen") || model_type == "qwen2" {
        return HfArchitecture::Qwen2;
    }
    if arch_l.contains("llama") || model_type.contains("llama") || model_type == "llama" {
        return HfArchitecture::Qwen2;
    }
    if arch_l.contains("mistral") || model_type.contains("mistral") {
        return HfArchitecture::Qwen2;
    }
    HfArchitecture::Gpt2
}

impl From<&HfTransformerLayout> for ConfigDims {
    fn from(h: &HfTransformerLayout) -> Self {
        h.dims.clone()
    }
}

/// Layout + coarse [`HfArchitecture`] bucket (GPT-2 vs stacked causal LM field layout).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTransformerLayout {
    pub architecture: HfArchitecture,
    pub dims: ConfigDims,
    /// First entry in `architectures[]`, or `model_type` when empty.
    pub primary_architecture: String,
}

/// Parse `config.json` and classify architecture for legacy CLI / tests.
pub fn parse_transformer_layout(path: &Path) -> anyhow::Result<ParsedTransformerLayout> {
    let layout = HfTransformerLayout::from_config_path(path)?;
    let architecture = layout.architecture;
    let primary_architecture = layout
        .architectures
        .first()
        .cloned()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| layout.model_type.clone());
    let dims = layout.dims.clone();
    Ok(ParsedTransformerLayout {
        architecture,
        dims,
        primary_architecture,
    })
}

#[derive(Debug, Deserialize)]
struct Gpt2Cfg {
    n_embd: usize,
    n_head: usize,
    n_layer: usize,
    vocab_size: usize,
}

#[derive(Debug, Deserialize)]
struct Qwen2Cfg {
    hidden_size: usize,
    num_attention_heads: usize,
    num_hidden_layers: usize,
    vocab_size: usize,
}

pub fn detect_hf_architecture(config_path: &Path) -> anyhow::Result<HfArchitecture> {
    let raw = std::fs::read_to_string(config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let v: Value = serde_json::from_str(&raw)?;
    Ok(classify_hf_architecture(&v))
}

fn gpt2_config_from_path(config_path: &Path) -> anyhow::Result<Gpt2Cfg> {
    let raw = std::fs::read_to_string(config_path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn qwen2_config_from_path(config_path: &Path) -> anyhow::Result<Qwen2Cfg> {
    let raw = std::fs::read_to_string(config_path)?;
    Ok(serde_json::from_str(&raw)?)
}

/// Load [`ConfigDims`] from a Hugging Face `config.json` for `arch` (GPT-2 vs Qwen2-style layouts).
///
/// Prefer [`HfTransformerLayout::from_config_path`] for new code; this keeps older call sites stable.
pub fn config_dims_for_architecture(
    config_path: &Path,
    arch: HfArchitecture,
) -> anyhow::Result<ConfigDims> {
    match arch {
        HfArchitecture::Gpt2 => Ok(gpt2_config_from_path(config_path)?.into()),
        HfArchitecture::Qwen2 => Ok(qwen2_config_from_path(config_path)?.into()),
    }
}

impl From<Gpt2Cfg> for ConfigDims {
    fn from(c: Gpt2Cfg) -> Self {
        Self {
            n_embd: c.n_embd,
            n_head: c.n_head,
            n_layer: c.n_layer,
            vocab_size: c.vocab_size,
        }
    }
}

impl From<Qwen2Cfg> for ConfigDims {
    fn from(c: Qwen2Cfg) -> Self {
        Self {
            n_embd: c.hidden_size,
            n_head: c.num_attention_heads,
            n_layer: c.num_hidden_layers,
            vocab_size: c.vocab_size,
        }
    }
}
