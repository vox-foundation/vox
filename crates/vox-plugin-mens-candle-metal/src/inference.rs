//! Candle inference server — ported from `vox-populi/src/mens/tensor/candle_inference_serve.rs`.
//!
//! Grammar-constrained generation (vox_constrained_gen / vox_grammar_export) is not pulled
//! into this plugin to avoid dependency bloat; the `grammar_mode` parameter accepts a JSON
//! string selecting the mode and falls back to unconstrained when the mode is unknown.
//!
//! Hub download (crate::mens::hub::download_model) is stubbed: if the base_model field in
//! the adapter metadata is not a local directory path the call returns an error directing the
//! caller to pre-download the model. This matches the deferred-rewire pattern from SP3.

use std::path::{Path, PathBuf};

use anyhow::Result;
use candle_core::{Device, Tensor};
use qlora_rs::QuantizedLinear;
use safetensors::Dtype;
use safetensors::SafeTensors;
use tokenizers::Tokenizer;

use crate::hf_layout::HfArchitecture;
use crate::model::{
    Qwen2Attention, Qwen2MLP, Qwen35AttentionBlock, Qwen35Layer, Qwen35LinearAttention, Qwen35Model,
};

pub struct InferenceEngine {
    pub model: InferenceModel,
    pub tokenizer: Tokenizer,
    pub device: Device,
    /// Stop tokens read from the model's own `config.json` at load. Empty when
    /// the config carries none — generation then runs to `max_tokens`.
    pub eos_token_ids: Vec<u32>,
}

pub enum InferenceModel {
    Qwen35(Qwen35Model),
}

// `compute_dtype_for_device` and `resolve_inference_device` moved to
// `vox-plugin-mens-candle-core::inference_device` — the latter used to be
// missing entirely from this crate (see that module's docs for the L-7 bug
// this fixes: an explicit `--device metal` request that failed used to fall
// back to CPU with no warning at all).
use vox_plugin_mens_candle_core::inference_device::{
    compute_dtype_for_device, resolve_inference_device,
};

// Base-shard / LoRA-delta resolution (including the base-only load path — a
// model directory with no adapter present) moved to
// `vox-plugin-mens-candle-core::weight_resolution`, alongside
// `adapter_deltas_must_be_folded` (previously forked byte-for-byte in this
// file and the CUDA plugin's `inference.rs`). See that module's docs for why
// a base-only directory used to be a hard error here.
use vox_plugin_mens_candle_core::weight_resolution::resolve_weights;

// `synthesize_rope_inv_freq` moved to `vox-plugin-mens-candle-core::rope` —
// it used to be forked four ways (this file and `candle_qlora_train::mod` in
// both plugins) with a comment here claiming the copies were "kept
// byte-for-byte in sync ... by hand". There is nothing left to keep in sync:
// all four call sites now use the one definition.
use vox_plugin_mens_candle_core::rope::synthesize_rope_inv_freq;

/// Stop-token ids declared by a model's `config.json`.
///
/// HF configs spell this two ways and Qwen3 uses both across its family: a
/// scalar (`"eos_token_id": 151645`) and a list (`"eos_token_id": [151645,
/// 151643]`). Anything else — absent, null, or a non-integer — yields no stop
/// tokens rather than a guess, because guessing is how `151643` (which is
/// Qwen3's **BOS**) got hardcoded here in the first place and generation never
/// terminated early.
fn eos_token_ids(config_json: &str) -> Vec<u32> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(config_json) else {
        return Vec::new();
    };
    // VLM-shaped checkpoints keep the text tower's ids under `text_config`,
    // matching how every other dim in this lane is read (see `vox_hf_layout`).
    let field = v
        .get("text_config")
        .and_then(|t| t.get("eos_token_id"))
        .or_else(|| v.get("eos_token_id"));
    match field {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|x| x.as_u64().and_then(|n| u32::try_from(n).ok()))
            .collect(),
        Some(other) => other
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .into_iter()
            .collect(),
        None => Vec::new(),
    }
}

fn resolve_adapter_manifest_path(model_dir: &Path) -> Option<std::path::PathBuf> {
    let manifest = model_dir.join("adapter_manifest.json");
    if manifest.is_file() {
        return Some(manifest);
    }
    None
}

/// Every safetensors file whose tensors feed the inference weight map, in
/// search order: adapter before merged before base shards.
///
/// The adapter file contributes no *base* keys (its tensors are named
/// `<logical>.lora_a.weight` / `.lora_b.weight`), so it never shadows a base
/// weight here; the trained delta is folded in separately by
/// `merge::lora_factors_by_base_key`, gated on `adapter_deltas_must_be_folded`.
pub fn weight_sources(model_dir: &Path, base_shards: &[PathBuf]) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let adapter = model_dir.join("candle_qlora_adapter.safetensors");
    if adapter.is_file() {
        sources.push(adapter);
    }
    let merged = model_dir.join("merged.safetensors");
    if merged.is_file() {
        sources.push(merged);
    }
    sources.extend(base_shards.iter().cloned());
    sources
}

impl InferenceEngine {
    pub fn load(model_dir: &Path, device_kind: &crate::device::DeviceKind) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let _tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;

        let _device = resolve_inference_device(device_kind)?;

        let adapter_path = model_dir.join("candle_qlora_adapter.safetensors");
        let meta_path = resolve_adapter_manifest_path(model_dir)
            .unwrap_or_else(|| model_dir.join("adapter_manifest.json"));

        // Base shards + any trained LoRA delta to fold on top of them. A
        // directory with neither adapter file present resolves as a
        // base-only load (no delta) instead of the old hard bail — see
        // `weight_resolution`'s module docs for the two supported shapes
        // this now serves, and why loading a fine-tune's base model on its
        // own used to be impossible.
        let resolved = resolve_weights(model_dir, &adapter_path, &meta_path, &_device)?;
        let base_shards = resolved.base_shards;
        let lora_factors = resolved.lora_factors;

        let mut all_buffers = Vec::new();
        let mut weight_maps = Vec::new();

        for p in weight_sources(model_dir, &base_shards) {
            let file = std::fs::File::open(&p)?;
            #[allow(unsafe_code)]
            let mmap = unsafe { memmap2::Mmap::map(&file)? };
            all_buffers.push(mmap);
        }
        for b in &all_buffers {
            weight_maps.push(SafeTensors::deserialize(b)?);
        }
        // QLoraConfig::default() hardcodes BF16 compute ("CRITICAL: BF16 for
        // stability" — tuned for CUDA training). This lane's Metal trainer
        // dequantizes to F32, and Candle's CPU backend cannot matmul BF16 at
        // all, so both devices on this lane override to F32.
        let mut qlora_cfg = qlora_rs::qlora::QLoraConfig::default();
        qlora_cfg.quantization.compute_dtype = compute_dtype_for_device(&_device);
        // Dequantize each frozen base weight ONCE at load instead of on every
        // forward pass. `dequantize_nf4_gpu` is gated on `device.is_cuda()`, so
        // off CUDA the on-the-fly path is a CPU scalar loop over every weight,
        // every token — which dominated serving time on Metal. The trade is
        // resident memory (one dequantized copy of the base, at the compute
        // dtype); this is the serving path, where that copy is the point.
        // `QLoraConfig::preset_*_inference` sets the same flag.
        qlora_cfg.cache_dequantized = true;

        // Helper to find a tensor in any map
        let get_tensor = |key: &str| -> Result<Tensor> {
            for st in &weight_maps {
                if let Ok(view) = st.tensor(key) {
                    let shape = view.shape().to_vec();
                    let dtype = match view.dtype() {
                        Dtype::F32 => candle_core::DType::F32,
                        Dtype::BF16 => candle_core::DType::BF16,
                        Dtype::F16 => candle_core::DType::F16,
                        _ => continue,
                    };
                    let t = Tensor::from_raw_buffer(view.data(), dtype, &shape, &_device)?;
                    return if t.dtype() == candle_core::DType::F32 {
                        Ok(t)
                    } else {
                        Ok(t.to_dtype(candle_core::DType::F32)?)
                    };
                }
            }
            anyhow::bail!("Weight not found: {key}");
        };

        // Every key actually fetched through `linear_weight`. Compared against
        // the adapter's keys and the layout's projections after the model is
        // built — a projection that slipped back to a plain `get_tensor`, or an
        // adapter key nothing asks for, is then a hard error instead of a
        // silent return to serving base weights.
        let requested: std::cell::RefCell<std::collections::HashSet<String>> =
            std::cell::RefCell::new(std::collections::HashSet::new());

        // Weight fetch for a *linear projection* — the only layers LoRA trains.
        // Everything else (norms, the embedding matrix, conv/dt/A_log) keeps
        // using `get_tensor`, so a tied lm_head/embed base key applies the delta
        // to the head only, exactly as training did.
        let linear_weight = |key: &str| -> Result<Tensor> {
            requested.borrow_mut().insert(key.to_string());
            fold_lora_delta(get_tensor(key)?, key, &lora_factors)
        };

        let config_path = model_dir.join("config.json");
        if !config_path.is_file() {
            anyhow::bail!("config.json missing in {}", model_dir.display());
        }
        let config_raw = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("read config.json: {e}"))?;
        let eos_token_ids = eos_token_ids(&config_raw);
        let mut layout = crate::hf_layout::HfTransformerLayout::from_config_json_str(&config_raw)?;

        if layout.architecture == HfArchitecture::Qwen35 {
            let p = format!("{}.0.input_layernorm.weight", layout.namespace_prefix);
            if get_tensor(&p).is_err()
                && get_tensor("model.layers.0.input_layernorm.weight").is_ok()
            {
                layout.namespace_prefix = "model.layers".to_string();
            }
        }

        let model = if layout.architecture == HfArchitecture::Qwen35 {
            let mut layers = Vec::new();
            let n_heads = layout.num_attention_heads.max(1);
            let n_kv_heads = layout.num_key_value_heads.max(1);
            let head_dim = layout.head_dim.unwrap_or(layout.hidden_size / n_heads);
            for i in 0..layout.num_hidden_layers {
                let p = format!("{}.{}", layout.namespace_prefix, i);
                let ln1 = candle_nn::RmsNorm::new(
                    get_tensor(&format!("{p}.input_layernorm.weight"))?,
                    1e-6,
                );
                let ln2 = candle_nn::RmsNorm::new(
                    get_tensor(&format!("{p}.post_attention_layernorm.weight"))?,
                    1e-6,
                );
                let layer_type = layout
                    .layer_types
                    .get(i)
                    .map(String::as_str)
                    .unwrap_or("full_attention");

                let attention = if layer_type == "linear_attention" {
                    let key_heads = layout.linear_num_key_heads.unwrap_or(n_heads);
                    let value_heads = layout.linear_num_value_heads.unwrap_or(n_heads);
                    let key_dim = layout.linear_key_head_dim.unwrap_or(head_dim);
                    let value_dim = layout.linear_value_head_dim.unwrap_or(head_dim);
                    let qkv_rows = (key_heads * key_dim * 2) + (value_heads * value_dim);
                    let qkv_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.linear_attn.in_proj_qkv.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let z_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.linear_attn.in_proj_z.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let b_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.linear_attn.in_proj_b.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let a_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.linear_attn.in_proj_a.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let out_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.linear_attn.out_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let conv = get_tensor(&format!("{p}.linear_attn.conv1d.weight"))?;
                    let conv_weight = if conv.rank() == 3 {
                        conv.squeeze(1)?
                    } else {
                        conv
                    };
                    if conv_weight.dim(0)? != qkv_rows {
                        anyhow::bail!(
                            "qwen3_5 linear conv rows mismatch at layer {}: expected {}, got {}",
                            i,
                            qkv_rows,
                            conv_weight.dim(0)?
                        );
                    }
                    Qwen35AttentionBlock::Linear(Qwen35LinearAttention {
                        qkv_proj,
                        z_proj,
                        b_proj,
                        a_proj,
                        out_proj,
                        conv_weight,
                        dt_bias: get_tensor(&format!("{p}.linear_attn.dt_bias"))?,
                        a_log: get_tensor(&format!("{p}.linear_attn.A_log"))?,
                        norm: candle_nn::RmsNorm::new(
                            get_tensor(&format!("{p}.linear_attn.norm.weight"))?,
                            1e-6,
                        ),
                        num_k_heads: key_heads,
                        num_v_heads: value_heads,
                        head_k_dim: key_dim,
                        head_v_dim: value_dim,
                    })
                } else {
                    let q_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.self_attn.q_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let k_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.self_attn.k_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let v_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.self_attn.v_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let o_proj = QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.self_attn.o_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    // Qwen2/Qwen2.5 additive qkv biases (optional — absent on pure Qwen3.5).
                    // Must match training (mod.rs loads these) or a merged/served model drifts.
                    let q_bias = get_tensor(&format!("{p}.self_attn.q_proj.bias")).ok();
                    let k_bias = get_tensor(&format!("{p}.self_attn.k_proj.bias")).ok();
                    let v_bias = get_tensor(&format!("{p}.self_attn.v_proj.bias")).ok();
                    // Dense Qwen3's per-head q_norm/k_norm (optional — absent on
                    // Qwen2/Qwen2.5). Must match training (mod.rs loads these the
                    // same way) or a merged/served model drifts, exactly like the
                    // qkv biases above.
                    let q_norm = get_tensor(&format!("{p}.self_attn.q_norm.weight"))
                        .ok()
                        .map(|w| candle_nn::RmsNorm::new(w, 1e-6));
                    let k_norm = get_tensor(&format!("{p}.self_attn.k_norm.weight"))
                        .ok()
                        .map(|w| candle_nn::RmsNorm::new(w, 1e-6));
                    Qwen35AttentionBlock::Full(Qwen2Attention {
                        q_proj,
                        k_proj,
                        v_proj,
                        o_proj,
                        q_bias,
                        k_bias,
                        v_bias,
                        n_heads,
                        n_kv_heads,
                        head_dim,
                        q_norm,
                        k_norm,
                    })
                };

                let mlp = Qwen2MLP {
                    gate_proj: QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.mlp.gate_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                    up_proj: QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.mlp.up_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                    down_proj: QuantizedLinear::from_weight(
                        &linear_weight(&format!("{p}.mlp.down_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                };
                // RoPE: HF Qwen2.5/Qwen3.5 shards usually omit per-layer `inv_freq`; the
                // trainer synthesizes it from `config.rope_theta` (see candle_qlora_train::
                // synthesize_rope_inv_freq). Inference MUST do the same or the forward runs
                // with zero positional encoding and emits token-salad. Match the trainer.
                let inv_freq = get_tensor(&format!("{p}.self_attn.rotary_emb.inv_freq"))
                    .or_else(|_| get_tensor(&format!("{p}.linear_attn.rotary_emb.inv_freq")))
                    .ok()
                    .or_else(|| {
                        synthesize_rope_inv_freq(head_dim, layout.rope_theta, &_device).ok()
                    });

                layers.push(Qwen35Layer {
                    input_layernorm: ln1,
                    attention,
                    post_attention_layernorm: ln2,
                    mlp,
                    inv_freq,
                });
            }
            let norm = candle_nn::RmsNorm::new(
                get_tensor("model.language_model.norm.weight")
                    .or_else(|_| get_tensor("model.norm.weight"))?,
                1e-6,
            );
            let lm_head = QuantizedLinear::from_weight(
                &linear_weight("lm_head.weight").or_else(|_| {
                    linear_weight("model.language_model.embed_tokens.weight")
                        .or_else(|_| linear_weight("model.embed_tokens.weight"))
                })?,
                None,
                &qlora_cfg,
                &_device,
            )?;
            let embed_tokens = get_tensor("model.language_model.embed_tokens.weight")
                .or_else(|_| get_tensor("model.embed_tokens.weight"))?;
            InferenceModel::Qwen35(Qwen35Model {
                embed_tokens,
                layers,
                norm,
                lm_head,
            })
        } else {
            anyhow::bail!(
                "Unsupported architecture for inference: {:?}",
                layout.architecture
            );
        };

        // Every trained delta must have reached a weight, and every weight LoRA
        // can train must have gone through `linear_weight`. Either half failing
        // means the adapter is silently not applied somewhere — the exact defect
        // this path was written to fix — so it is an error, not a warning.
        let requested = requested.into_inner();
        assert_adapter_fully_applied(lora_factors.keys().map(String::as_str), &requested, &layout)?;

        Ok(Self {
            model,
            tokenizer: _tokenizer,
            device: _device,
            eos_token_ids,
        })
    }

    /// Autoregressive generation loop (unconstrained).
    ///
    /// `prompt_json` is a JSON object with fields:
    ///   - `"prompt"`: string — the prompt text
    ///   - `"system"`: string — optional trained system prompt
    ///   - `"max_tokens"`: integer (default 256)
    ///   - `"temperature"`: float (default 1.0; `<= 0` forces greedy)
    ///   - `"top_k"`: integer (default 0 = full vocabulary; `1` forces greedy)
    ///   - `"output_mode"`: optional structured-output label — see
    ///     `check_output_mode` for what this plugin does and does not enforce.
    pub fn generate_from_json(&mut self, prompt_json: &str) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct PromptRequest {
            prompt: String,
            #[serde(default)]
            system: Option<String>,
            #[serde(default = "default_max_tokens")]
            max_tokens: usize,
            #[serde(default = "default_temperature")]
            temperature: f64,
            #[serde(default)]
            top_k: usize,
            #[serde(default)]
            output_mode: Option<String>,
        }
        fn default_max_tokens() -> usize {
            256
        }
        fn default_temperature() -> f64 {
            1.0
        }

        let req: PromptRequest = serde_json::from_str(prompt_json)
            .map_err(|e| anyhow::anyhow!("parse prompt_json: {e}"))?;

        check_output_mode(req.output_mode.as_deref())?;

        let prompt = assemble_prompt(req.system.as_deref(), &req.prompt);

        let generated = self.generate(&prompt, req.max_tokens, req.temperature, req.top_k)?;

        let out = serde_json::json!({
            "generated_text": generated,
            "prompt_tokens": self.tokenizer.encode(prompt.as_str(), true)
                .map(|e| e.len()).unwrap_or(0),
        });
        Ok(out.to_string())
    }

    fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        temperature: f64,
        top_k: usize,
    ) -> Result<String> {
        let prompt_tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow::anyhow!("tokenizer error: {e}"))?
            .get_ids()
            .to_vec();

        let mut generated = String::new();
        let mut rng = rand::thread_rng();

        // KV-cached decoding: the prompt is forwarded ONCE (the prefill, at
        // position 0), and each step after that forwards only the single token
        // just sampled, at the absolute position of the tokens already cached.
        // `pos` therefore advances by exactly what was submitted — it is what
        // RoPE keys off, so drifting it produces wrong logits rather than an
        // error. Without this the loop re-forwarded the whole context every
        // step, which is quadratic in the reply length.
        let mut cache = match &self.model {
            InferenceModel::Qwen35(model) => model.empty_cache(),
        };
        let mut pos = 0usize;
        let mut step_input = prompt_tokens;

        for _ in 0..max_tokens {
            let submitted = step_input.len();
            let input = Tensor::new(step_input.as_slice(), &self.device)?.unsqueeze(0)?;

            let logits = match &self.model {
                InferenceModel::Qwen35(model) => model.forward_cached(&input, pos, &mut cache)?,
            };
            pos += submitted;

            let logits = logits.squeeze(0)?;
            let seq = logits.dim(0)?;
            let logits = logits.narrow(0, seq.saturating_sub(1), 1)?.squeeze(0)?;

            let slice = logits.to_vec1::<f32>()?;
            let next_token = sample_next_token(&slice, temperature, top_k, &mut rng);

            // Stop tokens come from the model's own config.json (see
            // `eos_token_ids`). This used to break on a hardcoded 151643, which
            // is Qwen3's BOS — so nothing ever stopped early. Break BEFORE
            // appending, so the stop marker is not emitted as reply text.
            if self.eos_token_ids.contains(&next_token) {
                break;
            }

            if let Ok(char_str) = self.tokenizer.decode(&[next_token], false) {
                generated.push_str(&char_str);
            }
            step_input = vec![next_token];
        }

        Ok(generated)
    }
}

/// Entry point called from `backend.rs` `run_inference`.
///
/// Prepend the trained system prompt to the user prompt, if one was sent.
/// Plain `"{system}\n\n{prompt}"` — there's no existing chat-template
/// convention in this file (no BOS/EOS role tags), so this keeps the same
/// flat-text shape `generate` already expects.
/// Add this key's trained LoRA delta to a freshly read base weight, if the
/// adapter has one for it.
///
/// A key the adapter never touched — and every key at all when the map is empty
/// (no adapter, or an already-merged run) — comes back byte-identical, so a
/// base-only serve can never pick up a delta by accident.
/// The dense delta is formed here and dropped at the end of this function — it
/// is never stored, so peak extra memory is one weight, not one per layer.
fn fold_lora_delta(
    weight: Tensor,
    key: &str,
    factors: &std::collections::HashMap<String, crate::merge::LoraFactors>,
) -> Result<Tensor> {
    match factors.get(key) {
        Some(f) => Ok(weight.broadcast_add(&f.delta()?)?),
        None => Ok(weight),
    }
}

/// The LM head's base weight is looked up through a 3-way fallback, so any one
/// of these spellings satisfies "the head was fetched through `linear_weight`".
const LM_HEAD_KEYS: [&str; 3] = [
    "lm_head.weight",
    "model.language_model.embed_tokens.weight",
    "model.embed_tokens.weight",
];

/// Every transformer-block base key the load path must fetch through
/// `linear_weight` — i.e. every linear projection LoRA can train, for this
/// layout. Mirrors the per-layer branch in `load` exactly; the LM head is
/// excluded because it is a fallback chain (see `LM_HEAD_KEYS`).
fn linear_projection_keys(layout: &crate::hf_layout::HfTransformerLayout) -> Vec<String> {
    let mut keys = Vec::new();
    for i in 0..layout.num_hidden_layers {
        let p = format!("{}.{}", layout.namespace_prefix, i);
        let linear_attention = layout
            .layer_types
            .get(i)
            .map(String::as_str)
            .unwrap_or("full_attention")
            == "linear_attention";
        let attn: &[&str] = if linear_attention {
            &[
                "linear_attn.in_proj_qkv",
                "linear_attn.in_proj_z",
                "linear_attn.in_proj_b",
                "linear_attn.in_proj_a",
                "linear_attn.out_proj",
            ]
        } else {
            &[
                "self_attn.q_proj",
                "self_attn.k_proj",
                "self_attn.v_proj",
                "self_attn.o_proj",
            ]
        };
        for name in attn
            .iter()
            .chain(["mlp.gate_proj", "mlp.up_proj", "mlp.down_proj"].iter())
        {
            keys.push(format!("{p}.{name}.weight"));
        }
    }
    keys
}

/// Fail unless the adapter is wired end to end: every trained delta reached a
/// weight, and every trainable projection was fetched through `linear_weight`.
///
/// Both halves catch a silent return to serving base weights. An unconsumed
/// delta means a key mismatch (e.g. a tied-head spelling training stores that
/// inference's fallback chain never tries) — computed, uploaded, never applied.
/// A projection missing from `requested` means its call site is reading the base
/// weight directly again, which is the original defect reintroduced at one site.
fn assert_adapter_fully_applied<'a>(
    delta_keys: impl Iterator<Item = &'a str>,
    requested: &std::collections::HashSet<String>,
    layout: &crate::hf_layout::HfTransformerLayout,
) -> Result<()> {
    let mut unconsumed: Vec<&str> = delta_keys.filter(|k| !requested.contains(*k)).collect();
    if !unconsumed.is_empty() {
        unconsumed.sort_unstable();
        anyhow::bail!(
            "{} trained LoRA delta(s) were never applied to any weight: {:?}. \
             The adapter's base_key_map names tensors this model never loads as a linear \
             projection, so the trained layers would be served as base weights. \
             Next: check base_key_map in adapter_manifest.json against this model's config.json.",
            unconsumed.len(),
            &unconsumed[..unconsumed.len().min(8)]
        );
    }

    // Nothing to under-cover on a merged run — the deltas are already on disk.
    if requested.is_empty() {
        return Ok(());
    }

    let expected = linear_projection_keys(layout);
    let mut missing: Vec<String> = expected
        .into_iter()
        .filter(|k| !requested.contains(k))
        .collect();
    if !LM_HEAD_KEYS.iter().any(|k| requested.contains(*k)) {
        missing.push("lm_head (no candidate spelling requested)".to_string());
    }
    if !missing.is_empty() {
        missing.sort();
        anyhow::bail!(
            "{} linear projection(s) were loaded without going through the adapter path: {:?}. \
             Those layers serve base weights even when an adapter is present.",
            missing.len(),
            &missing[..missing.len().min(8)]
        );
    }
    Ok(())
}

/// Structured-output labels this plugin knows about.
///
/// This plugin has **no constrained decoder** — grammar-constrained generation
/// is deliberately not linked in (see the module doc). For these three labels
/// that is not a silent lie: `vox-ml-cli`'s serve handlers shape the prompt
/// (`prompt_for_output_mode`) and then validate + repair-retry the reply
/// (`validate_structured_output_with_reason`), so enforcement happens upstream
/// and the label reaches us as advisory context. Any *other* value has nobody
/// enforcing it, so it fails loudly here instead of being served as free-form
/// text that the caller believes is structured.
fn check_output_mode(output_mode: Option<&str>) -> Result<()> {
    const ADVISORY: [&str; 3] = ["strict_json", "jsonl_records", "tool_args_json"];
    match output_mode.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(()),
        Some(mode) if ADVISORY.contains(&mode) => Ok(()),
        Some(mode) => anyhow::bail!(
            "unsupported output_mode {mode:?}: this backend does not do constrained decoding, \
             and only {ADVISORY:?} are prompt-shaped and validated by the serving layer. \
             Next: drop output_mode, or use one of those labels."
        ),
    }
}

/// Pick the next token: greedy unless both a positive temperature and a top-k
/// wider than one ask for sampling.
///
/// Greedy is the floor, not a special case — `temperature <= 0`, `top_k == 1`,
/// and a degenerate logit vector all land there, so a caller that does not opt
/// into sampling keeps the deterministic behavior this path has always had.
fn sample_next_token(
    logits: &[f32],
    temperature: f64,
    top_k: usize,
    rng: &mut impl rand::Rng,
) -> u32 {
    let argmax = |v: &[f32]| -> u32 {
        v.iter()
            .enumerate()
            .fold((0u32, f32::NEG_INFINITY), |(bi, bv), (i, &x)| {
                if x > bv { (i as u32, x) } else { (bi, bv) }
            })
            .0
    };
    if temperature <= 0.0 || top_k == 1 || logits.is_empty() {
        return argmax(logits);
    }

    let mut ranked: Vec<(usize, f32)> = logits.iter().copied().enumerate().collect();
    ranked.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
    if top_k > 0 {
        ranked.truncate(top_k);
    }

    // Softmax over the kept logits, shifted by the max for numerical stability.
    let max = ranked[0].1;
    let mut probs: Vec<f64> = ranked
        .iter()
        .map(|(_, l)| (((*l - max) as f64) / temperature).exp())
        .collect();
    let sum: f64 = probs.iter().sum();
    if !sum.is_finite() || sum <= 0.0 {
        return argmax(logits);
    }
    for p in &mut probs {
        *p /= sum;
    }

    let mut draw = rng.r#gen::<f64>();
    for (i, p) in probs.iter().enumerate() {
        draw -= p;
        if draw <= 0.0 {
            return ranked[i].0 as u32;
        }
    }
    ranked[0].0 as u32
}

fn assemble_prompt(system: Option<&str>, prompt: &str) -> String {
    match system {
        Some(system) if !system.is_empty() => format!("{system}\n\n{prompt}"),
        _ => prompt.to_string(),
    }
}

/// `model_dir_json` is a JSON string with a `"model_dir"` field pointing to the
/// local model directory, plus the prompt fields consumed by `generate_from_json`.
pub fn run(model_dir: &str, prompt_json: &str) -> Result<String> {
    let path = std::path::Path::new(model_dir);
    let mut engine = InferenceEngine::load(path, &crate::device::DeviceKind::Best)?;
    engine.generate_from_json(prompt_json)
}

/// Live-hardware gates: these need a real model on disk and a real Metal GPU, so
/// they are `#[ignore]`d and run explicitly:
///
/// ```text
/// VOX_MENS_DEMO_RUN_DIR=<run dir> \
///   cargo test -p vox-plugin-mens-candle-metal --features metal -- --ignored
/// ```
///
/// The run dir is a MENS run directory (`config.json`, `tokenizer.json`,
/// `candle_qlora_adapter.safetensors`, `adapter_manifest.json`) — exactly what
/// `vox mens serve --model` is pointed at.
#[cfg(test)]
mod live_metal_tests {
    use super::InferenceEngine;

    fn demo_run_dir() -> std::path::PathBuf {
        let raw = std::env::var("VOX_MENS_DEMO_RUN_DIR").expect(
            "set VOX_MENS_DEMO_RUN_DIR to a MENS run directory to run the live Metal gates",
        );
        std::path::PathBuf::from(raw)
    }

    fn load_demo_engine() -> InferenceEngine {
        InferenceEngine::load(&demo_run_dir(), &crate::device::DeviceKind::Best)
            .expect("load demo engine")
    }

    fn load_demo_config() -> String {
        std::fs::read_to_string(demo_run_dir().join("config.json")).expect("read demo config.json")
    }

    #[test]
    #[ignore = "needs a real model directory and Metal hardware"]
    fn generation_sustains_a_usable_token_rate() {
        // 0.6B on Metal. The pre-fix engine measured 1.76 tok/s at this length;
        // the floor below is deliberately far under what a cached engine gives,
        // so this asserts "not quadratic", not "fast".
        //
        // 256, not 128: at 128 tokens the dequant cache alone clears 15 tok/s
        // even with the KV cache reverted (measured: 14.88), so a shorter run
        // barely separates the two fixes. The quadratic term only dominates once
        // the reply is long — which is also the length the serve gate cares
        // about (`max_tokens` defaults to 256).
        const TOKENS: usize = 256;
        let mut engine = load_demo_engine();
        let t0 = std::time::Instant::now();
        let out = engine
            .generate("Write a Vox function that adds two ints.", TOKENS, 0.0, 0)
            .unwrap();
        let rate = TOKENS as f64 / t0.elapsed().as_secs_f64();
        eprintln!("measured rate: {rate:.4} tok/s over {:?}", t0.elapsed());
        assert!(
            rate >= 15.0,
            "only {rate:.2} tok/s — the KV cache or the dequant cache is not engaged"
        );
        assert!(!out.is_empty());
    }

    #[test]
    #[ignore = "needs a real model directory"]
    fn the_demo_config_eos_is_read_from_disk_not_hardcoded() {
        // 151643 is BOS. The real EOS is 151645 and must come from config.json.
        assert_eq!(
            super::eos_token_ids(&load_demo_config()),
            vec![151645],
            "EOS must be read from config.json, not hardcoded"
        );
    }

    /// The base-only load path's live gate (Task followup: base-only
    /// inference). `VOX_MENS_BASE_ONLY_DIR` must point at a directory with
    /// NEITHER `candle_qlora_adapter.safetensors` NOR `adapter_manifest.json`
    /// — a bare HF snapshot (`config.json`, `tokenizer.json`, its own
    /// `*.safetensors` shard) — the exact shape an adapter run's
    /// `adapter_manifest.json`'s `base_model` field points at.
    ///
    /// ```text
    /// VOX_MENS_BASE_ONLY_DIR=<base snapshot dir> \
    ///   cargo test -p vox-plugin-mens-candle-metal --features metal -- --ignored base_only
    /// ```
    #[test]
    #[ignore = "needs a real base-model directory and Metal hardware"]
    fn base_only_load_runs_real_inference_with_no_adapter_present() {
        let raw = std::env::var("VOX_MENS_BASE_ONLY_DIR")
            .expect("set VOX_MENS_BASE_ONLY_DIR to a base-model directory with no adapter present");
        let dir = std::path::PathBuf::from(raw);
        assert!(
            !dir.join("candle_qlora_adapter.safetensors").is_file()
                && !dir.join("adapter_manifest.json").is_file(),
            "VOX_MENS_BASE_ONLY_DIR must have NO adapter present — this test verifies the \
             base-only path, not the adapter path"
        );

        let mut engine = InferenceEngine::load(&dir, &crate::device::DeviceKind::Best)
            .expect("a base-only directory must load without an adapter");

        let out = engine
            .generate("Write a Vox function that adds two ints.", 64, 0.0, 0)
            .expect("base-only engine must run real inference");
        eprintln!("base-only generation: {out:?}");
        assert!(
            !out.trim().is_empty(),
            "base-only inference produced no output at all"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::assemble_prompt;
    use super::check_output_mode;
    use super::resolve_adapter_manifest_path;
    use super::sample_next_token;
    use super::weight_sources;
    use vox_plugin_mens_candle_core::weight_resolution::adapter_deltas_must_be_folded;

    #[test]
    fn compute_dtype_is_f32_off_cuda() {
        // Candle's CPU backend cannot matmul BF16 at all, and this lane's
        // Metal trainer dequantizes to F32 — serving must match, or the
        // adapter is applied against a different compute dtype than it
        // trained under. CUDA (not this crate's device) keeps BF16.
        assert!(matches!(
            super::compute_dtype_for_device(&candle_core::Device::Cpu),
            qlora_rs::ComputeDType::F32
        ));
    }

    // Ported from `vox-plugin-mens-candle-cuda::inference`'s tests of the same
    // name (see `vox_plugin_mens_candle_core::inference_device`, which now
    // backs both). Before this crate depended on
    // `vox-plugin-mens-candle-core::inference_device`, `resolve_inference_device`
    // did not exist here at all — `InferenceEngine::load` inlined a bare
    // `Device::new_metal(0).unwrap_or(Device::Cpu)` with no warning and no
    // error path, so an explicit `--device metal` request that failed to
    // initialize silently ran on CPU. These two tests, and the
    // `#[cfg(not(any(feature = "cuda", feature = "metal")))]` explicit-request
    // tests in `inference_device`, are what would have caught that.
    #[test]
    fn resolve_inference_device_cpu_is_cpu() {
        let d = super::resolve_inference_device(&crate::device::DeviceKind::Cpu).unwrap();
        assert!(d.is_cpu());
    }

    #[cfg(feature = "metal")]
    #[test]
    fn resolve_inference_device_best_prefers_metal_when_available() {
        let d = super::resolve_inference_device(&crate::device::DeviceKind::Best).unwrap();
        assert!(
            d.is_metal() || d.is_cpu(),
            "Best must land on Metal or CPU fallback, got {d:?}"
        );
    }

    #[test]
    fn inference_rope_synthesis_matches_the_trainer() {
        // Regression guard for the real serving bug: HF Qwen3 checkpoints ship
        // no `rotary_emb.inv_freq`, so inference must synthesize the SAME table
        // the trainer did. If these two copies ever drift, the forward runs with
        // different rotary frequencies than the adapter trained against.
        let dev = candle_core::Device::Cpu;
        for (head_dim, theta) in [(128usize, Some(1_000_000.0f64)), (64, None)] {
            let from_inference = super::synthesize_rope_inv_freq(head_dim, theta, &dev)
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            let from_trainer =
                crate::candle_qlora_train::synthesize_rope_inv_freq(head_dim, theta, &dev)
                    .unwrap()
                    .to_vec1::<f32>()
                    .unwrap();
            assert_eq!(from_inference, from_trainer, "head_dim={head_dim}");
            assert_eq!(from_inference.len(), head_dim / 2);
        }
        assert!(super::synthesize_rope_inv_freq(0, None, &dev).is_err());
    }

    /// A tiny `LoraFactors` whose delta is `alpha` everywhere.
    fn ones_factors(
        out: usize,
        inp: usize,
        rank: usize,
        alpha: usize,
    ) -> crate::merge::LoraFactors {
        use candle_core::{DType, Device, Tensor};
        let dev = Device::Cpu;
        crate::merge::LoraFactors {
            a: Tensor::ones(&[rank, inp], DType::F32, &dev).unwrap(),
            b: Tensor::ones(&[out, rank], DType::F32, &dev).unwrap(),
            alpha: alpha as f64,
            rank,
        }
    }

    /// The core regression: a trained adapter must actually change what the
    /// served layer computes. Before this fix every `QuantizedLinear` was built
    /// from a plain `get_tensor`, so a fine-tuned model answered exactly like
    /// the base one.
    ///
    /// This drives the **production** fold (`fold_lora_delta`, the one
    /// `linear_weight` calls) rather than hand-building the sum in the test
    /// body, then feeds both weights to the real serving primitive and asserts
    /// the output moves. Break `fold_lora_delta` and this fails.
    #[test]
    fn a_trained_delta_measurably_changes_what_the_served_layer_outputs() {
        use candle_core::{DType, Device, Tensor};
        use qlora_rs::QuantizedLinear;

        let dev = Device::Cpu;
        // NF4 quantizes in blocks of 64, so the weight's element count must be
        // a multiple of that; F32 compute because candle's CPU backend has no
        // BF16 matmul kernel.
        let (out_f, in_f, rank, alpha) = (8usize, 64usize, 2usize, 16usize);
        let mut cfg = qlora_rs::qlora::QLoraConfig::default();
        cfg.quantization.compute_dtype = qlora_rs::ComputeDType::F32;

        let key = "model.layers.0.self_attn.q_proj.weight";
        let mut factors = std::collections::HashMap::new();
        factors.insert(key.to_string(), ones_factors(out_f, in_f, rank, alpha));

        let w = Tensor::ones(&[out_f, in_f], DType::F32, &dev).unwrap();
        // Exactly what the load path does: base weight in, folded weight out.
        let folded = super::fold_lora_delta(w.clone(), key, &factors).unwrap();

        let x = Tensor::ones(&[1, in_f], DType::F32, &dev).unwrap();
        let base = QuantizedLinear::from_weight(&w, None, &cfg, &dev).unwrap();
        let adapted = QuantizedLinear::from_weight(&folded, None, &cfg, &dev).unwrap();

        let y0 = base.forward(&x).unwrap().flatten_all().unwrap();
        let y1 = adapted.forward(&x).unwrap().flatten_all().unwrap();
        let (y0, y1) = (y0.to_vec1::<f32>().unwrap(), y1.to_vec1::<f32>().unwrap());
        for (a, b) in y0.iter().zip(y1.iter()) {
            assert!(
                (a - b).abs() > 1.0,
                "the adapter delta did not reach the served layer: base {a} vs adapted {b}"
            );
        }
    }

    /// A three-layer Qwen-shaped layout: two full-attention blocks and one
    /// linear-attention block, so both projection branches are covered.
    fn test_layout() -> crate::hf_layout::HfTransformerLayout {
        let mut layout = crate::hf_layout::HfTransformerLayout::from_config_json_str(
            r#"{
                "model_type": "qwen3_5",
                "architectures": ["Qwen3_5ForCausalLM"],
                "hidden_size": 64,
                "num_hidden_layers": 3,
                "num_attention_heads": 4,
                "num_key_value_heads": 2,
                "intermediate_size": 128,
                "vocab_size": 32,
                "layer_types": ["full_attention", "linear_attention", "full_attention"]
            }"#,
        )
        .expect("layout");
        layout.namespace_prefix = "model.layers".to_string();
        layout
    }

    /// Every key the 13 real call sites fetch, as `load` would fetch them.
    fn requested_by_a_correct_load() -> std::collections::HashSet<String> {
        let mut set: std::collections::HashSet<String> =
            super::linear_projection_keys(&test_layout())
                .into_iter()
                .collect();
        set.insert("lm_head.weight".to_string());
        set
    }

    /// A CI-time early warning that a linear projection stopped going through
    /// the adapter path — NOT the safety guarantee. The real guarantee is
    /// `assert_adapter_fully_applied`, which runs at the end of every `load` in
    /// production and fails loudly there; this test just catches the same
    /// mistake earlier, at review time, without a model on disk.
    ///
    /// It asserts on source text because no unit test can execute those call
    /// sites (they need a real multi-shard model), and the text is the only
    /// thing that can see them. Two review mutations motivated its current
    /// shape: reverting a call site to `get_tensor` in place, and hoisting the
    /// fetch into a local (`let q_raw = get_tensor(..)`) before constructing
    /// the layer. The substring check catches the first; only an EXACT count
    /// catches the second, since a loose `>=` bound tolerates losing one site.
    #[test]
    fn no_linear_layer_is_built_from_an_unadapted_weight() {
        let src = include_str!("inference.rs");
        let offenders = src.matches("QuantizedLinear::from_weight(").count();
        let adapted = src.matches("&linear_weight(").count();
        assert!(
            !src.contains("from_weight(\n                        &get_tensor(")
                && !src.contains("from_weight(\n                &get_tensor("),
            "a QuantizedLinear is being built straight from get_tensor — that layer serves \
             base weights even with an adapter loaded. Use linear_weight instead."
        );
        // 13 real call sites + 1 for the matcher literal on the line above,
        // which `include_str!` also sees. Exact, not `>=`: a lower bound lets a
        // refactor delete one call site and still pass.
        assert_eq!(
            adapted, EXPECTED_ADAPTED_OCCURRENCES,
            "expected exactly {EXPECTED_ADAPTED_OCCURRENCES} occurrences (13 linear projections \
             + this test's own matcher literal), found {adapted}. A projection is being loaded \
             without the adapter, or a call site moved — {offenders} QuantizedLinear built total."
        );
    }

    /// 13 linear projections + the matcher string literal in the test above.
    const EXPECTED_ADAPTED_OCCURRENCES: usize = 14;

    /// B3: the wiring guard. Reverting ONE call site to a plain `get_tensor`
    /// drops its key from the requested set — and that must be a hard load
    /// error, because that layer would then serve base weights while the
    /// adapter claims to be applied.
    #[test]
    fn one_projection_reverted_to_a_raw_read_fails_the_load() {
        let layout = test_layout();
        let full = requested_by_a_correct_load();
        super::assert_adapter_fully_applied(std::iter::empty(), &full, &layout)
            .expect("a correct load must pass");

        for reverted in [
            "model.layers.0.self_attn.q_proj.weight",
            "model.layers.1.linear_attn.in_proj_qkv.weight",
            "model.layers.2.mlp.down_proj.weight",
            "lm_head.weight",
        ] {
            let mut partial = full.clone();
            partial.remove(reverted);
            let err = super::assert_adapter_fully_applied(std::iter::empty(), &partial, &layout)
                .expect_err(&format!("reverting {reverted} must fail the load"));
            assert!(
                err.to_string()
                    .contains("without going through the adapter path"),
                "{err}"
            );
        }
    }

    /// B3 companion: every base key training can put in `base_key_map` is a key
    /// inference actually asks for. A spelling drift on either side breaks this.
    #[test]
    fn every_key_training_can_map_to_is_one_inference_requests() {
        let requested = requested_by_a_correct_load();
        // The base keys `candle_qlora_train` builds for this layout, spelled
        // exactly as it spells them (format!("{layer_prefix}.self_attn.q_proj.weight") etc).
        let mut training_keys = vec!["lm_head.weight".to_string()];
        for (i, kind) in ["full", "linear", "full"].iter().enumerate() {
            let layer_prefix = format!("model.layers.{i}");
            let names: Vec<&str> = if *kind == "linear" {
                vec![
                    "linear_attn.in_proj_qkv",
                    "linear_attn.in_proj_z",
                    "linear_attn.in_proj_b",
                    "linear_attn.in_proj_a",
                    "linear_attn.out_proj",
                ]
            } else {
                vec![
                    "self_attn.q_proj",
                    "self_attn.k_proj",
                    "self_attn.v_proj",
                    "self_attn.o_proj",
                ]
            };
            for n in names
                .iter()
                .chain(["mlp.gate_proj", "mlp.up_proj", "mlp.down_proj"].iter())
            {
                training_keys.push(format!("{layer_prefix}.{n}.weight"));
            }
        }
        for k in &training_keys {
            assert!(
                requested.contains(k),
                "training can key an adapter to {k}, but inference never fetches it —                  that delta would be computed and silently dropped"
            );
        }
    }

    /// The tied-head / spelling-mismatch case: a delta keyed to something the
    /// load path never asks for used to be computed, uploaded, and silently
    /// dropped — the original "serves base weights" bug in a new location.
    #[test]
    fn a_delta_no_call_site_asks_for_is_a_hard_error() {
        let layout = test_layout();
        let requested = requested_by_a_correct_load();
        let err = super::assert_adapter_fully_applied(
            ["model.embed_tokens.weight"].into_iter(),
            &requested,
            &layout,
        )
        .expect_err("an unreachable delta must not be silently dropped");
        assert!(err.to_string().contains("never applied"), "{err}");
    }

    /// A merged run requests nothing through the adapter path and carries no
    /// deltas — it must not trip the under-coverage half of the guard.
    #[test]
    fn a_merged_run_passes_the_guard_with_nothing_requested() {
        super::assert_adapter_fully_applied(
            std::iter::empty(),
            &std::collections::HashSet::new(),
            &test_layout(),
        )
        .expect("a merged run folds nothing and must still load");
    }

    #[test]
    fn a_key_the_adapter_never_trained_is_returned_untouched() {
        use candle_core::{DType, Device, Tensor};

        let dev = Device::Cpu;
        let w = Tensor::ones(&[2, 2], DType::F32, &dev).unwrap();
        let mut factors = std::collections::HashMap::new();
        // delta == alpha/rank * (B@A) == 1 everywhere.
        factors.insert("trained.weight".to_string(), ones_factors(2, 2, 1, 1));

        let untouched = super::fold_lora_delta(w.clone(), "untrained.weight", &factors).unwrap();
        assert_eq!(
            untouched.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            vec![1.0; 4]
        );

        let folded = super::fold_lora_delta(w.clone(), "trained.weight", &factors).unwrap();
        assert_eq!(
            folded.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            vec![2.0; 4]
        );

        // A base-only run (empty map) must never grow a delta.
        let empty = std::collections::HashMap::new();
        let base_only = super::fold_lora_delta(w, "trained.weight", &empty).unwrap();
        assert_eq!(
            base_only.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            vec![1.0; 4]
        );
    }

    #[test]
    fn a_merged_run_does_not_fold_the_delta_a_second_time() {
        let d = tempfile::tempdir().unwrap();
        assert!(
            adapter_deltas_must_be_folded(d.path()),
            "an un-merged run must fold the trained delta at load"
        );
        std::fs::write(d.path().join("merged.safetensors"), b"").unwrap();
        assert!(
            !adapter_deltas_must_be_folded(d.path()),
            "merged.safetensors already contains W + BA·alpha/r — folding again doubles it"
        );
    }

    /// 151643 is Qwen3's **BOS**; the generation loop used to break on it, so
    /// nothing ever stopped early. The stop ids must come from config.json, and
    /// both HF spellings — scalar and list — have to work.
    #[test]
    fn eos_ids_come_from_config_json_in_both_shapes() {
        use super::eos_token_ids;
        assert_eq!(
            eos_token_ids(r#"{"bos_token_id": 151643, "eos_token_id": 151645}"#),
            vec![151645],
            "the scalar spelling must be read, and must not pick up bos_token_id"
        );
        assert_eq!(
            eos_token_ids(r#"{"eos_token_id": [151645, 151643]}"#),
            vec![151645, 151643],
            "Qwen3 configs can carry a list of stop ids"
        );
        // VLM-shaped checkpoints keep the text tower's ids in `text_config`.
        assert_eq!(
            eos_token_ids(r#"{"eos_token_id": 1, "text_config": {"eos_token_id": 151645}}"#),
            vec![151645]
        );
        // No usable value => no stop tokens, rather than a guessed one.
        for absent in [
            r#"{"bos_token_id": 151643}"#,
            r#"{"eos_token_id": null}"#,
            r#"{"eos_token_id": "im_end"}"#,
            "not json at all",
        ] {
            assert!(
                eos_token_ids(absent).is_empty(),
                "must not invent a stop token from {absent}"
            );
        }
    }

    #[test]
    fn top_k_of_one_and_zero_temperature_stay_greedy() {
        let mut rng = rand::thread_rng();
        let logits = [0.1f32, 9.0, 0.2, 0.3];
        assert_eq!(sample_next_token(&logits, 1.0, 1, &mut rng), 1);
        assert_eq!(sample_next_token(&logits, 0.0, 40, &mut rng), 1);
    }

    #[test]
    fn top_k_confines_sampling_to_the_k_best_logits() {
        let mut rng = rand::thread_rng();
        // Index 3 is the runner-up; index 0 and 2 must never be drawn at k=2.
        let logits = [0.0f32, 5.0, 0.0, 4.9];
        for _ in 0..200 {
            let t = sample_next_token(&logits, 1.0, 2, &mut rng);
            assert!(t == 1 || t == 3, "top_k=2 drew outside the top 2: {t}");
        }
    }

    #[test]
    fn an_output_mode_nobody_enforces_is_an_error_not_free_text() {
        assert!(check_output_mode(None).is_ok());
        assert!(check_output_mode(Some("")).is_ok());
        assert!(check_output_mode(Some("strict_json")).is_ok());
        let err = check_output_mode(Some("yaml")).expect_err("must not silently serve free text");
        assert!(err.to_string().contains("constrained decoding"), "{err}");
    }

    #[test]
    fn assemble_prompt_prepends_system_when_present() {
        let out = assemble_prompt(Some("YOU ARE VOX"), "hello");
        assert_eq!(out, "YOU ARE VOX\n\nhello");
    }

    #[test]
    fn assemble_prompt_is_unchanged_when_system_absent() {
        assert_eq!(assemble_prompt(None, "hello"), "hello");
    }

    #[test]
    fn the_adapter_is_a_weight_source_and_outranks_the_base() {
        let d = tempfile::tempdir().unwrap();
        let adapter = d.path().join("candle_qlora_adapter.safetensors");
        std::fs::write(&adapter, b"").unwrap();
        let base = d.path().join("model-00001-of-00001.safetensors");

        let sources = weight_sources(d.path(), std::slice::from_ref(&base));
        assert!(
            sources.contains(&adapter),
            "the trained adapter must be read at inference; without it the frozen base model answers"
        );
        assert_eq!(
            sources[0], adapter,
            "the adapter must precede the base so a merged file wins over raw shards"
        );
    }

    #[test]
    fn a_run_directory_with_no_adapter_still_serves_its_base() {
        let d = tempfile::tempdir().unwrap();
        let base = d.path().join("model-00001-of-00001.safetensors");
        assert_eq!(
            weight_sources(d.path(), std::slice::from_ref(&base)),
            vec![base]
        );
    }

    #[test]
    fn resolve_adapter_manifest_finds_v3() {
        let d = tempfile::tempdir().expect("tempdir");
        assert!(resolve_adapter_manifest_path(d.path()).is_none());

        let manifest = d.path().join("adapter_manifest.json");
        std::fs::write(&manifest, "{}").expect("manifest");
        assert_eq!(
            resolve_adapter_manifest_path(d.path()).as_deref(),
            Some(manifest.as_path())
        );
    }

    /// Functional-correctness regression test for the mmap substitution in
    /// the shard-loading block above (`all_buffers: Vec<memmap2::Mmap>`).
    /// This proves the mmap path reads the same bytes a `std::fs::read`
    /// would have — it does NOT prove RSS stays low; a broken mmap (wrong
    /// offset/length, use-after-drop) would corrupt or fail this test, but
    /// so would a correct implementation regardless of whether it mmaps or
    /// copies into memory. See task-6-report.md for the honest scope note.
    #[test]
    fn serve_load_does_not_read_shards_into_memory() {
        use candle_core::{Device, Tensor};
        use safetensors::SafeTensors;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixture.safetensors");

        let src = Tensor::new(&[1.0f32, 2.0, 3.0, 4.0], &Device::Cpu).expect("tensor");
        src.save_safetensors("w", &path).expect("write fixture");

        // Same mechanism as the shard-loading fix: open + mmap + deserialize
        // from the mmap's byte slice, never `std::fs::read`.
        let file = std::fs::File::open(&path).expect("open");
        #[allow(unsafe_code)]
        let mmap = unsafe { memmap2::Mmap::map(&file).expect("mmap") };
        let st = SafeTensors::deserialize(&mmap).expect("deserialize");

        let view = st.tensor("w").expect("tensor present");
        // Same reconstruction the `get_tensor` closure above uses in production.
        let loaded = Tensor::from_raw_buffer(
            view.data(),
            candle_core::DType::F32,
            view.shape(),
            &Device::Cpu,
        )
        .expect("from_raw_buffer")
        .to_vec1::<f32>()
        .expect("to_vec1");
        assert_eq!(loaded, vec![1.0f32, 2.0, 3.0, 4.0]);
    }
}
