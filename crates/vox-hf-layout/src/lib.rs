//! Minimal Hugging Face `config.json` parsing for architecture diagnostics and layout validation.
//! SSOT for `vox-populi` MENS tensor code and `vox-plugin-mens-candle-cuda`.

use std::path::Path;

use anyhow::Context;

use serde::Deserialize;
use serde_json::Value;
use vox_bounded_fs::read_utf8_path_capped;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HfArchitecture {
    Gpt2,
    Qwen35,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDims {
    pub n_embd: usize,
    pub n_head: usize,
    pub n_layer: usize,
    pub vocab_size: usize,
}

/// Structured transformer layout from HF `config.json` (GPT-2, Qwen2-style, qwen3_5-style).
#[derive(Debug, Clone, PartialEq)]
pub struct HfTransformerLayout {
    /// GPT-2 vs stacked causal LM family.
    pub architecture: HfArchitecture,
    pub model_type: String,
    pub architectures: Vec<String>,
    /// Layer namespace prefix used for weight key construction.
    pub namespace_prefix: String,
    pub hidden_size: usize,
    pub num_attention_heads: usize,
    /// Key/value head count for GQA. Falls back to `num_attention_heads` when absent.
    pub num_key_value_heads: usize,
    pub num_hidden_layers: usize,
    pub vocab_size: usize,
    pub intermediate_size: Option<usize>,
    pub max_position_embeddings: Option<usize>,
    pub rope_theta: Option<f64>,
    pub rope_partial_rotary_factor: Option<f64>,
    /// qwen3_5 hybrid stack metadata (`linear_attention` / `full_attention`).
    pub layer_types: Vec<String>,
    pub linear_attention_heads: Option<usize>,
    pub full_attention_heads: Option<usize>,
    /// Optional explicit per-head dim for attention projections.
    pub head_dim: Option<usize>,
    /// qwen3_5 linear-attention key/value and value projection geometry.
    pub linear_num_key_heads: Option<usize>,
    pub linear_num_value_heads: Option<usize>,
    pub linear_key_head_dim: Option<usize>,
    pub linear_value_head_dim: Option<usize>,
    pub linear_conv_kernel_dim: Option<usize>,
    /// Whether the checkpoint ties `lm_head.weight` to the input embedding matrix.
    /// Read from root-level `tie_word_embeddings`, falling back to the same key
    /// nested under `text_config` for VLM-shaped checkpoints. Defaults to `true`
    /// (the legacy tied-embeddings behavior) when the key is absent entirely, since
    /// small Qwen3 dense checkpoints rely on that default.
    pub tie_word_embeddings: bool,
    /// Same numbers as the HF fields above, in [`ConfigDims`] shape (legacy / graph code).
    pub dims: ConfigDims,
}

/// True for tensor keys belonging to Qwen3.8-27B's vision tower
/// (`model.visual.*`) or its multi-token-prediction head (`mtp.*`) — neither
/// is part of the text tower this crate extracts a layout for, and neither
/// should be requested, downloaded, or validated against by a text-only
/// QLoRA loader.
pub fn is_vision_or_mtp_key(key: &str) -> bool {
    key.starts_with("model.visual.") || key.starts_with("mtp.")
}

impl HfTransformerLayout {
    /// Parse `config.json` at `path` into a structured layout.
    pub fn from_config_path(path: &Path) -> anyhow::Result<Self> {
        let raw =
            read_utf8_path_capped(path).with_context(|| format!("read {}", path.display()))?;
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
        let model_l = model_type.to_lowercase();

        if (model_l.contains("qwen2") || model_l.contains("qwen2.5")) && !model_l.contains("qwen3")
        {
            tracing::warn!(
                "Qwen 2.5 / Qwen2 detected: This architecture is DEPRECATED in Vox. \
                 Qwen 3.5 is the new production default. Please migrate your base weights."
            );
        }
        let architectures: Vec<String> = v
            .get("architectures")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        // Vision-language / multimodal checkpoints (e.g. Qwen3.5-2B/4B and
        // Qwen3.8-27B ship as `Qwen3_5ForConditionalGeneration` with a
        // `vision_config` + image/video token ids) are loadable TEXT-ONLY when
        // they carry a separate `text_config` block: `qwen35_text_config` below
        // reads dims exclusively from that block, never from `vision_config`, so
        // there is nothing vision-specific for the text QLoRA trainer to choke
        // on. The weight loader never *requests* `model.visual.*` / `mtp.*`
        // tensors in the first place, because every lookup here is by exact key
        // name built from `namespace_prefix` (`model.language_model.layers.*`
        // for this text_config-wrapped shape) — nothing enumerates shard keys,
        // so vision/MTP tensors are skipped as an emergent side effect, not an
        // explicit filter. [`is_vision_or_mtp_key`] is that explicit check,
        // for callers (validation passes, diagnostics, shard selection)
        // that do enumerate keys and need to exclude these towers on purpose.
        // What genuinely cannot be trained is a VLM with NO text_config at all
        // — there would be nothing to extract dims from.
        let is_conditional_generation = architectures
            .iter()
            .any(|a| a.contains("ForConditionalGeneration"));
        let has_vision = v.get("vision_config").is_some()
            || v.get("image_token_id").is_some()
            || v.get("video_token_id").is_some();
        let has_text_config = qwen35_text_config(v, architecture).is_some();
        if (is_conditional_generation || has_vision) && !has_text_config {
            anyhow::bail!(
                "This checkpoint is a vision-language / multimodal model (architectures={architectures:?}\
                {}) with no text_config block to load a text tower from. Use a text-only causal LM \
                 (e.g. a Qwen2.5-Coder-*-Instruct or a text-only dense Qwen checkpoint), or a VLM \
                 checkpoint that ships a text_config (e.g. Qwen3.8-27B).",
                if has_vision {
                    ", has vision_config/image_token"
                } else {
                    ""
                }
            );
        }

        let cfg_source = qwen35_text_config(v, architecture).unwrap_or(v);
        let tie_word_embeddings = json_bool(v, "tie_word_embeddings")
            .or_else(|| json_bool(cfg_source, "tie_word_embeddings"))
            .unwrap_or(true);

        // Llama / Mistral / Qwen2 / Qwen3.5 and many causal LMs.
        if let (Some(h), Some(nh), Some(nl), Some(vs)) = (
            json_usize(cfg_source, "hidden_size"),
            json_usize(cfg_source, "num_attention_heads"),
            json_usize(cfg_source, "num_hidden_layers"),
            json_usize(cfg_source, "vocab_size"),
        ) {
            let dims = ConfigDims {
                n_embd: h,
                n_head: nh,
                n_layer: nl,
                vocab_size: vs,
            };
            let n_kv = json_usize(cfg_source, "num_key_value_heads").unwrap_or(nh);
            let mut layer_types = json_string_vec(cfg_source, "layer_types").unwrap_or_default();
            if layer_types.is_empty() {
                layer_types = vec!["full_attention".to_string(); nl];
            }
            // The `language_model.` weight-key infix is a property of the
            // checkpoint's actual on-disk shape (text_config-wrapped, as
            // Qwen3.5's hybrid stack ships), not of the model family name —
            // "qwen3" and "qwen3_5" both `.contains("qwen3")`, but only
            // text_config-wrapped checkpoints use the wrapped prefix. A dense
            // Qwen3 checkpoint (e.g. Qwen/Qwen3-0.6B) has no `text_config`
            // block and uses the flat `model.layers` prefix, same as Qwen2/
            // Llama/Mistral.
            let namespace_prefix = if qwen35_text_config(v, architecture).is_some() {
                "model.language_model.layers".to_string()
            } else {
                "model.layers".to_string()
            };
            return Ok(Self {
                architecture,
                model_type,
                architectures,
                namespace_prefix,
                hidden_size: h,
                num_attention_heads: nh,
                num_key_value_heads: n_kv,
                num_hidden_layers: nl,
                vocab_size: vs,
                intermediate_size: json_usize(cfg_source, "intermediate_size"),
                max_position_embeddings: json_usize(cfg_source, "max_position_embeddings"),
                rope_theta: qwen35_rope_theta(cfg_source)
                    .or_else(|| json_f64(cfg_source, "rope_theta")),
                rope_partial_rotary_factor: qwen35_partial_rotary_factor(cfg_source),
                layer_types,
                linear_attention_heads: json_usize(cfg_source, "num_linear_heads"),
                full_attention_heads: json_usize(cfg_source, "num_full_heads"),
                head_dim: json_usize(cfg_source, "head_dim"),
                linear_num_key_heads: json_usize(cfg_source, "linear_num_key_heads"),
                linear_num_value_heads: json_usize(cfg_source, "linear_num_value_heads"),
                linear_key_head_dim: json_usize(cfg_source, "linear_key_head_dim"),
                linear_value_head_dim: json_usize(cfg_source, "linear_value_head_dim"),
                linear_conv_kernel_dim: json_usize(cfg_source, "linear_conv_kernel_dim"),
                tie_word_embeddings,
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
                namespace_prefix: "h".to_string(),
                hidden_size: n_embd,
                num_attention_heads: n_head,
                num_key_value_heads: n_head, // GPT-2 is full MHA
                num_hidden_layers: n_layer,
                vocab_size: vs,
                intermediate_size: None, // GPT-2 uses linear 4x expansion in MLP
                max_position_embeddings: json_usize(v, "n_positions"),
                rope_theta: None,
                rope_partial_rotary_factor: None,
                layer_types: vec!["full_attention".to_string(); n_layer],
                linear_attention_heads: None,
                full_attention_heads: Some(n_head),
                head_dim: None,
                linear_num_key_heads: None,
                linear_num_value_heads: None,
                linear_key_head_dim: None,
                linear_value_head_dim: None,
                linear_conv_kernel_dim: None,
                tie_word_embeddings,
                dims,
            });
        }

        anyhow::bail!(
            "config.json: need either (hidden_size, num_attention_heads, num_hidden_layers, vocab_size) \
             or (n_embd, n_head, n_layer, vocab_size)"
        )
    }
}

fn qwen35_text_config(v: &Value, architecture: HfArchitecture) -> Option<&Value> {
    if architecture == HfArchitecture::Qwen35 {
        return v.get("text_config");
    }
    None
}

fn qwen35_rope_parameters(v: &Value) -> Option<&Value> {
    v.get("rope_parameters")
}

fn qwen35_rope_theta(v: &Value) -> Option<f64> {
    qwen35_rope_parameters(v)
        .and_then(|rp| rp.get("rope_theta"))
        .and_then(|x| x.as_f64())
}

fn qwen35_partial_rotary_factor(v: &Value) -> Option<f64> {
    qwen35_rope_parameters(v)
        .and_then(|rp| rp.get("partial_rotary_factor"))
        .and_then(|x| x.as_f64())
}

fn json_usize(v: &Value, key: &str) -> Option<usize> {
    v.get(key).and_then(|x| x.as_u64()).map(|u| u as usize)
}

fn json_f64(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn json_bool(v: &Value, key: &str) -> Option<bool> {
    v.get(key).and_then(|x| x.as_bool())
}

fn json_string_vec(v: &Value, key: &str) -> Option<Vec<String>> {
    v.get(key).and_then(|a| a.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect()
    })
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
    let model_l = model_type.to_lowercase();
    if model_l == "qwen3_5" || model_l == "qwen35" || model_l.contains("qwen3") {
        return HfArchitecture::Qwen35;
    }
    if arch_l.contains("qwen3") {
        return HfArchitecture::Qwen35;
    }
    if arch_l.contains("qwen") || model_l.contains("qwen") || model_l == "qwen2" {
        return HfArchitecture::Qwen35;
    }
    if arch_l.contains("llama") || model_l.contains("llama") || model_l == "llama" {
        return HfArchitecture::Qwen35;
    }
    if arch_l.contains("mistral") || model_l.contains("mistral") {
        return HfArchitecture::Qwen35;
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
struct StackedCausalCfg {
    hidden_size: usize,
    num_attention_heads: usize,
    num_hidden_layers: usize,
    vocab_size: usize,
}

pub fn detect_hf_architecture(config_path: &Path) -> anyhow::Result<HfArchitecture> {
    let raw = read_utf8_path_capped(config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let v: Value = serde_json::from_str(&raw)?;
    Ok(classify_hf_architecture(&v))
}

fn gpt2_config_from_path(config_path: &Path) -> anyhow::Result<Gpt2Cfg> {
    let raw = read_utf8_path_capped(config_path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn stacked_causal_config_from_path(config_path: &Path) -> anyhow::Result<StackedCausalCfg> {
    let raw = read_utf8_path_capped(config_path)?;
    if let Ok(parsed) = serde_json::from_str::<StackedCausalCfg>(&raw) {
        return Ok(parsed);
    }
    let v: Value = serde_json::from_str(&raw)?;
    if let Some(text) = v.get("text_config") {
        return Ok(serde_json::from_value(text.clone())?);
    }
    anyhow::bail!(
        "stacked-causal config requires hidden_size/num_attention_heads/num_hidden_layers/vocab_size"
    )
}

/// Load [`ConfigDims`] from a Hugging Face `config.json` for `arch`.
///
/// Prefer [`HfTransformerLayout::from_config_path`] for new code; this keeps older call sites stable.
pub fn config_dims_for_architecture(
    config_path: &Path,
    arch: HfArchitecture,
) -> anyhow::Result<ConfigDims> {
    match arch {
        HfArchitecture::Gpt2 => Ok(gpt2_config_from_path(config_path)?.into()),
        HfArchitecture::Qwen35 => Ok(stacked_causal_config_from_path(config_path)?.into()),
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

impl From<StackedCausalCfg> for ConfigDims {
    fn from(c: StackedCausalCfg) -> Self {
        Self {
            n_embd: c.hidden_size,
            n_head: c.num_attention_heads,
            n_layer: c.num_hidden_layers,
            vocab_size: c.vocab_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HfArchitecture, HfTransformerLayout, is_vision_or_mtp_key};

    #[test]
    fn vision_and_mtp_keys_are_excluded_from_required_key_set() {
        // Real key shapes from the Qwen3.8-27B checkpoint (verified this session).
        assert!(is_vision_or_mtp_key(
            "model.visual.blocks.0.attn.qkv.weight"
        ));
        assert!(is_vision_or_mtp_key("mtp.fc.weight"));
        assert!(!is_vision_or_mtp_key(
            "model.language_model.layers.0.self_attn.q_proj.weight"
        ));
    }

    #[test]
    fn vlm_checkpoint_with_text_config_loads_the_text_tower_only() {
        // Real shape (fetched from Qwen/Qwen3.8-27B's config.json this session):
        // model_type "qwen3_5", architectures ["Qwen3_5ForConditionalGeneration"],
        // a text_config block with the same hybrid-attention shape Qwen3.5 already
        // parses, PLUS a vision_config block and image/video token ids. The old
        // blanket bail on `ForConditionalGeneration` / vision_config rejected this
        // outright — but qwen35_text_config already extracts only the text_config
        // block, ignoring vision_config entirely, so there is no reason a VLM
        // checkpoint that HAS a text_config can't be loaded text-only.
        let raw = r#"{
            "model_type":"qwen3_5",
            "architectures":["Qwen3_5ForConditionalGeneration"],
            "text_config":{
                "hidden_size":5120,
                "num_attention_heads":24,
                "num_key_value_heads":24,
                "num_hidden_layers":8,
                "vocab_size":248320,
                "intermediate_size":13824,
                "max_position_embeddings":262144,
                "linear_num_key_heads":16,
                "linear_num_value_heads":48,
                "layer_types":["linear_attention","linear_attention","linear_attention","full_attention",
                               "linear_attention","linear_attention","linear_attention","full_attention"]
            },
            "vision_config":{
                "depth":27,
                "hidden_size":1152
            },
            "image_token_id":248056,
            "video_token_id":248057
        }"#;
        let layout =
            HfTransformerLayout::from_config_json_str(raw).expect("VLM-with-text-config must load");
        assert_eq!(layout.architecture, HfArchitecture::Qwen35);
        assert_eq!(
            layout.hidden_size, 5120,
            "must read dims from text_config, not top-level (which has none)"
        );
        assert_eq!(layout.num_hidden_layers, 8);
        assert_eq!(layout.vocab_size, 248320);
        assert_eq!(layout.namespace_prefix, "model.language_model.layers");
    }

    #[test]
    fn vlm_checkpoint_without_text_config_still_rejected() {
        // A vision-language checkpoint with genuinely no separable text
        // path (no text_config at all) must still be refused — the fix is
        // "load the text tower when one exists", not "accept any VLM".
        let raw = r#"{
            "model_type":"qwen3_5",
            "architectures":["Qwen3_5ForConditionalGeneration"],
            "vision_config":{"depth":27,"hidden_size":1152},
            "image_token_id":248056
        }"#;
        let err = HfTransformerLayout::from_config_json_str(raw)
            .expect_err("a VLM with no text_config must still be rejected");
        assert!(
            err.to_string().contains("vision"),
            "error must explain why, got: {err}"
        );
    }

    #[test]
    fn dense_qwen3_without_text_config_uses_flat_namespace_prefix() {
        // A real dense Qwen3 checkpoint (e.g. Qwen/Qwen3-0.6B) has model_type
        // "qwen3" (which `.contains("qwen3")`) but is NOT wrapped in a
        // `text_config` block the way Qwen3.5's hybrid stack is — its weight
        // tensors are named `model.layers.N....`, not
        // `model.language_model.layers.N....`. Deciding the namespace prefix
        // from a substring match on the model name (rather than from whether
        // `text_config` is actually present) misnames every weight key and
        // makes the checkpoint fail to load with "missing weight".
        let raw = r#"{
            "model_type":"qwen3",
            "architectures":["Qwen3ForCausalLM"],
            "hidden_size":1024,
            "num_attention_heads":16,
            "num_key_value_heads":8,
            "num_hidden_layers":28,
            "vocab_size":151936,
            "intermediate_size":3072
        }"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("dense qwen3 parse");
        assert_eq!(
            layout.namespace_prefix, "model.layers",
            "a checkpoint with no text_config block must use the flat weight-key prefix"
        );
    }

    #[test]
    fn parses_qwen35_nested_text_config_layout() {
        let raw = r#"{
            "model_type":"qwen3_5",
            "architectures":["Qwen3_5ForCausalLM"],
            "text_config":{
                "hidden_size":1024,
                "num_attention_heads":16,
                "num_key_value_heads":8,
                "num_hidden_layers":4,
                "vocab_size":151936,
                "intermediate_size":2816,
                "max_position_embeddings":262144,
                "layer_types":["linear_attention","full_attention","linear_attention","full_attention"]
            }
        }"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("qwen3_5 parse");
        assert_eq!(layout.architecture, HfArchitecture::Qwen35);
        assert_eq!(layout.namespace_prefix, "model.language_model.layers");
        assert_eq!(layout.num_hidden_layers, 4);
        assert_eq!(layout.layer_types.len(), 4);
        assert_eq!(layout.layer_types[0], "linear_attention");
    }

    #[test]
    fn parses_tie_word_embeddings_false_at_root() {
        // Qwen3.8-27B ships an untied lm_head.weight: tie_word_embeddings=false
        // at root level, alongside a nested text_config block.
        let raw = r#"{
            "model_type":"qwen3_5",
            "architectures":["Qwen3_5ForCausalLM"],
            "tie_word_embeddings":false,
            "text_config":{
                "hidden_size":1024,
                "num_attention_heads":16,
                "num_hidden_layers":4,
                "vocab_size":151936
            }
        }"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("qwen3_5 parse");
        assert!(
            !layout.tie_word_embeddings,
            "untied checkpoint must not report tie_word_embeddings=true"
        );
    }

    #[test]
    fn parses_tie_word_embeddings_false_nested_under_text_config() {
        // This model's config shape nests text fields under text_config; the
        // key must also be honored from there, not just the root.
        let raw = r#"{
            "model_type":"qwen3_5",
            "architectures":["Qwen3_5ForCausalLM"],
            "text_config":{
                "hidden_size":1024,
                "num_attention_heads":16,
                "num_hidden_layers":4,
                "vocab_size":151936,
                "tie_word_embeddings":false
            }
        }"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("qwen3_5 parse");
        assert!(
            !layout.tie_word_embeddings,
            "nested tie_word_embeddings=false under text_config must be honored"
        );
    }

    #[test]
    fn tie_word_embeddings_true_is_parsed() {
        let raw = r#"{
            "model_type":"qwen3",
            "architectures":["Qwen3ForCausalLM"],
            "tie_word_embeddings":true,
            "hidden_size":1024,
            "num_attention_heads":16,
            "num_hidden_layers":28,
            "vocab_size":151936
        }"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("dense qwen3 parse");
        assert!(layout.tie_word_embeddings);
    }

    #[test]
    fn tie_word_embeddings_defaults_to_true_when_absent() {
        // Small Qwen3 dense checkpoints below tying scale rely on the
        // derive-from-wte fallback continuing to work when the key is missing.
        let raw = r#"{
            "model_type":"qwen3",
            "architectures":["Qwen3ForCausalLM"],
            "hidden_size":1024,
            "num_attention_heads":16,
            "num_hidden_layers":28,
            "vocab_size":151936
        }"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("dense qwen3 parse");
        assert!(
            layout.tie_word_embeddings,
            "missing tie_word_embeddings must default to true (tied) for legacy checkpoints"
        );
    }

    #[test]
    fn qwen35_defaults_layer_types_to_full_attention() {
        let raw = r#"{
            "model_type":"qwen3_5",
            "text_config":{
                "hidden_size":512,
                "num_attention_heads":8,
                "num_hidden_layers":2,
                "vocab_size":32000
            }
        }"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("qwen3_5 parse");
        assert_eq!(layout.layer_types, vec!["full_attention", "full_attention"]);
    }
}

#[cfg(test)]
mod semcov_wave5_tests {
    use super::*;

    // --- qwen35_text_config ---

    #[test]
    fn qwen35_text_config_returns_none_for_gpt2_architecture() {
        let raw = serde_json::json!({
            "model_type": "gpt2",
            "architectures": ["GPT2LMHeadModel"],
            "text_config": {
                "hidden_size": 9999
            }
        });
        let arch = classify_hf_architecture(&raw);
        assert_eq!(arch, HfArchitecture::Gpt2);
        let result = qwen35_text_config(&raw, HfArchitecture::Gpt2);
        assert!(result.is_none(), "expected None for Gpt2 architecture");
    }

    #[test]
    fn qwen35_text_config_returns_inner_for_qwen35_architecture() {
        let raw = serde_json::json!({
            "model_type": "qwen3_5",
            "text_config": { "hidden_size": 1024 }
        });
        let result = qwen35_text_config(&raw, HfArchitecture::Qwen35);
        assert!(result.is_some(), "expected Some for Qwen35 architecture");
        let inner = result.unwrap();
        assert_eq!(
            inner.get("hidden_size").and_then(|v| v.as_u64()),
            Some(1024)
        );
    }

    // --- qwen35_rope_theta ---

    #[test]
    fn qwen35_rope_theta_returns_value_from_rope_parameters() {
        let v = serde_json::json!({
            "rope_parameters": { "rope_theta": 10000.0, "partial_rotary_factor": 0.5 }
        });
        let result = qwen35_rope_theta(&v);
        assert_eq!(result, Some(10000.0));
    }

    #[test]
    fn qwen35_rope_theta_returns_none_when_rope_parameters_absent() {
        let v = serde_json::json!({ "rope_theta": 5000.0 });
        let result = qwen35_rope_theta(&v);
        assert!(result.is_none());
    }

    #[test]
    fn qwen35_rope_theta_returns_none_when_rope_theta_key_absent_in_rope_parameters() {
        let v = serde_json::json!({ "rope_parameters": { "partial_rotary_factor": 0.25 } });
        let result = qwen35_rope_theta(&v);
        assert!(result.is_none());
    }

    // --- qwen35_partial_rotary_factor ---

    #[test]
    fn qwen35_partial_rotary_factor_returns_value_from_rope_parameters() {
        let v = serde_json::json!({
            "rope_parameters": { "partial_rotary_factor": 0.25, "rope_theta": 10000.0 }
        });
        let result = qwen35_partial_rotary_factor(&v);
        assert_eq!(result, Some(0.25));
    }

    #[test]
    fn qwen35_partial_rotary_factor_returns_none_when_key_absent() {
        let v = serde_json::json!({ "rope_parameters": { "rope_theta": 10000.0 } });
        let result = qwen35_partial_rotary_factor(&v);
        assert!(result.is_none());
    }

    #[test]
    fn qwen35_partial_rotary_factor_returns_none_when_rope_parameters_absent() {
        let v = serde_json::json!({ "partial_rotary_factor": 0.5 });
        let result = qwen35_partial_rotary_factor(&v);
        assert!(result.is_none());
    }

    // --- classify_hf_architecture ---

    #[test]
    fn classify_gpt2_returns_gpt2() {
        let v = serde_json::json!({ "model_type": "gpt2", "architectures": ["GPT2LMHeadModel"] });
        assert_eq!(classify_hf_architecture(&v), HfArchitecture::Gpt2);
    }

    #[test]
    fn classify_llama_returns_qwen35() {
        let v = serde_json::json!({ "model_type": "llama", "architectures": ["LlamaForCausalLM"] });
        assert_eq!(classify_hf_architecture(&v), HfArchitecture::Qwen35);
    }

    #[test]
    fn classify_mistral_returns_qwen35() {
        let v =
            serde_json::json!({ "model_type": "mistral", "architectures": ["MistralForCausalLM"] });
        assert_eq!(classify_hf_architecture(&v), HfArchitecture::Qwen35);
    }

    #[test]
    fn classify_qwen2_via_arch_string_returns_qwen35() {
        let v = serde_json::json!({ "architectures": ["Qwen2ForCausalLM"] });
        assert_eq!(classify_hf_architecture(&v), HfArchitecture::Qwen35);
    }

    #[test]
    fn classify_empty_json_returns_gpt2_fallback() {
        let v = serde_json::json!({});
        assert_eq!(classify_hf_architecture(&v), HfArchitecture::Gpt2);
    }

    #[test]
    fn from_config_json_str_parses_gpt2_flat_config() {
        let raw = r#"{"model_type":"gpt2","architectures":["GPT2LMHeadModel"],"n_embd":768,"n_head":12,"n_layer":12,"vocab_size":50257,"n_positions":1024}"#;
        let layout = HfTransformerLayout::from_config_json_str(raw).expect("gpt2 parse");
        assert_eq!(layout.architecture, HfArchitecture::Gpt2);
        assert_eq!(layout.hidden_size, 768);
        assert_eq!(layout.num_attention_heads, 12);
        assert_eq!(layout.num_hidden_layers, 12);
        assert_eq!(layout.vocab_size, 50257);
        assert_eq!(layout.max_position_embeddings, Some(1024));
        assert_eq!(layout.namespace_prefix, "h");
        assert_eq!(layout.num_key_value_heads, 12);
        assert_eq!(layout.layer_types, vec!["full_attention"; 12]);
    }
}
