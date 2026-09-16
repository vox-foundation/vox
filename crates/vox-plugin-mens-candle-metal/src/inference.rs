//! Candle inference server — ported from `vox-populi/src/mens/tensor/candle_inference_serve.rs`.
//!
//! Grammar-constrained generation (vox_constrained_gen / vox_grammar_export) is not pulled
//! into this plugin to avoid dependency bloat; the `grammar_mode` parameter accepts a JSON
//! string selecting the mode and falls back to unconstrained when the mode is unknown.
//!
//! Hub download (crate::mens::hub::download_model) is stubbed: if the base_model field in
//! the adapter metadata is not a local directory path the call returns an error directing the
//! caller to pre-download the model. This matches the deferred-rewire pattern from SP3.

use std::path::Path;

use crate::model::QuantizedLinear;
use anyhow::Result;
use candle_core::quantized::QMatMul;
use candle_core::{DType, Device, Tensor};
use safetensors::Dtype;
use safetensors::SafeTensors;
use tokenizers::Tokenizer;

use crate::adapter_schema_v3::PopuliAdapterManifestV3;
use crate::hf_layout::HfArchitecture;
use crate::model::{
    LinearStateCache, Qwen2Attention, Qwen2MLP, Qwen35AttentionBlock, Qwen35Layer,
    Qwen35LayerCache, Qwen35LinearAttention, Qwen35Model,
};

pub struct InferenceEngine {
    pub model: InferenceModel,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

pub enum InferenceModel {
    Qwen35(Qwen35Model),
}

impl InferenceModel {
    pub fn forward_with_cache(
        &self,
        input_ids: &Tensor,
        pos: usize,
        caches: Option<&mut [Qwen35LayerCache]>,
    ) -> Result<Tensor> {
        match self {
            InferenceModel::Qwen35(m) => Ok(m.forward_with_cache(input_ids, pos, caches)?),
        }
    }

    pub fn forward_with_cache_opt(
        &self,
        input_ids: &Tensor,
        pos: usize,
        caches: Option<&mut [Qwen35LayerCache]>,
        only_last_token: bool,
    ) -> Result<Tensor> {
        match self {
            InferenceModel::Qwen35(m) => {
                Ok(m.forward_with_cache_opt(input_ids, pos, caches, only_last_token)?)
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct QuantTensorMeta {
    ggml_dtype: String,
    orig_shape: Vec<usize>,
    quantized: bool,
}

#[derive(serde::Deserialize)]
struct QuantMetaFile {
    tensors: std::collections::HashMap<String, QuantTensorMeta>,
}

fn parse_ggml_dtype(s: &str) -> Option<candle_core::quantized::GgmlDType> {
    match s {
        "Q4K" => Some(candle_core::quantized::GgmlDType::Q4K),
        "Q5K" => Some(candle_core::quantized::GgmlDType::Q5K),
        "Q6K" => Some(candle_core::quantized::GgmlDType::Q6K),
        "Q8_0" => Some(candle_core::quantized::GgmlDType::Q8_0),
        "F32" => Some(candle_core::quantized::GgmlDType::F32),
        _ => None,
    }
}

fn resolve_adapter_manifest_path(model_dir: &Path) -> Option<std::path::PathBuf> {
    let manifest = model_dir.join("adapter_manifest.json");
    if manifest.is_file() {
        return Some(manifest);
    }
    None
}

fn synthesize_rope_inv_freq(
    head_dim: usize,
    partial_rotary_factor: Option<f64>,
    rope_theta: Option<f64>,
    device: &Device,
) -> Result<Tensor> {
    let rotary_dim = if let Some(factor) = partial_rotary_factor {
        let d = ((head_dim as f64) * factor).round() as usize;
        d.max(2).min(head_dim)
    } else {
        head_dim
    };
    let half = rotary_dim / 2;
    if half == 0 {
        anyhow::bail!("invalid rotary_dim={rotary_dim} (head_dim={head_dim}) for RoPE synthesis");
    }
    let theta = rope_theta.unwrap_or(10_000_000.0) as f32;
    let hd = rotary_dim as f32;
    let mut vals = Vec::with_capacity(half);
    for i in 0..half {
        let exponent = (2.0_f32 * i as f32) / hd;
        vals.push(1.0_f32 / theta.powf(exponent));
    }
    Ok(Tensor::from_vec(vals, (half,), device)?)
}

fn resolve_base_model_dir(base: &str) -> Option<std::path::PathBuf> {
    let p = Path::new(base);
    if p.is_dir() {
        return Some(p.to_path_buf());
    }
    // Check local Hugging Face hub cache if base is a repo id like "Qwen/Qwen3-0.6B"
    if let Some(home) = dirs::home_dir() {
        let hub_dir = home
            .join(".cache/huggingface/hub")
            .join(format!("models--{}", base.replace('/', "--")))
            .join("snapshots");
        if hub_dir.is_dir() {
            let read_res = std::fs::read_dir(&hub_dir);
            if let Ok(entries) = read_res {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    if ep.is_dir() && ep.join("config.json").is_file() {
                        return Some(ep);
                    }
                }
            }
        }
    }
    None
}

impl InferenceEngine {
    pub fn load(model_dir: &Path, device_kind: &crate::device::DeviceKind) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let _tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;

        let _device = match device_kind {
            crate::device::DeviceKind::Cpu => Device::Cpu,
            _ => {
                #[cfg(feature = "metal")]
                let dev = Device::new_metal(0).unwrap_or(Device::Cpu);
                #[cfg(not(feature = "metal"))]
                let dev = Device::Cpu;
                dev
            }
        };

        let quant_meta_path = model_dir.join("quant-metadata.json");
        let is_quantized = quant_meta_path.is_file();
        let quant_meta: Option<QuantMetaFile> = if is_quantized {
            let raw = std::fs::read_to_string(&quant_meta_path)
                .map_err(|e| anyhow::anyhow!("read quant-metadata.json: {e}"))?;
            Some(serde_json::from_str(&raw)?)
        } else {
            None
        };

        let index_path = model_dir.join("model.safetensors.index.json");
        let is_sharded = index_path.is_file();
        let single_model_safetensors = model_dir.join("model.safetensors");

        let mut mmaps = Vec::new();
        if is_quantized {
            let model_file = model_dir.join("model.safetensors");
            if !model_file.is_file() {
                anyhow::bail!("model.safetensors not found in {}", model_dir.display());
            }
            let f = std::fs::File::open(&model_file)?;
            #[allow(unsafe_code)]
            unsafe {
                mmaps.push(memmap2::MmapOptions::new().map(&f)?);
            }
        } else if is_sharded {
            let raw_index = std::fs::read_to_string(&index_path)
                .map_err(|e| anyhow::anyhow!("read model.safetensors.index.json: {e}"))?;
            let index_val: serde_json::Value = serde_json::from_str(&raw_index)?;
            let mut shard_set = std::collections::BTreeSet::new();
            if let Some(weight_map) = index_val.get("weight_map").and_then(|w| w.as_object()) {
                for (_, shard) in weight_map {
                    if let Some(s) = shard.as_str() {
                        shard_set.insert(s.to_string());
                    }
                }
            }
            if shard_set.is_empty() {
                for entry in std::fs::read_dir(model_dir)? {
                    let p = entry?.path();
                    if p.extension().map(|e| e == "safetensors").unwrap_or(false) {
                        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                            shard_set.insert(name.to_string());
                        }
                    }
                }
            }
            for shard_name in shard_set {
                let shard_path = model_dir.join(shard_name);
                let f = std::fs::File::open(&shard_path)?;
                #[allow(unsafe_code)]
                unsafe {
                    mmaps.push(memmap2::MmapOptions::new().map(&f)?);
                }
            }
        } else if single_model_safetensors.is_file() {
            let f = std::fs::File::open(&single_model_safetensors)?;
            #[allow(unsafe_code)]
            unsafe {
                mmaps.push(memmap2::MmapOptions::new().map(&f)?);
            }
        } else {
            let adapter_path = model_dir.join("candle_qlora_adapter.safetensors");
            let meta_path = resolve_adapter_manifest_path(model_dir)
                .unwrap_or_else(|| model_dir.join("adapter_manifest.json"));

            if !adapter_path.is_file() || !meta_path.is_file() {
                anyhow::bail!(
                    "Neither quantized artifact nor LoRA adapter found in {}",
                    model_dir.display()
                );
            }

            let meta_raw = std::fs::read_to_string(&meta_path)
                .map_err(|e| anyhow::anyhow!("read manifest {}: {e}", meta_path.display()))?;
            let meta: PopuliAdapterManifestV3 = serde_json::from_str(&meta_raw)?;

            // Resolve base shards — local directory or local HuggingFace snapshot cache.
            let base_dir = if let Some(ref base) = meta.base_model {
                resolve_base_model_dir(base).ok_or_else(|| {
                    anyhow::anyhow!(
                        "base_model '{}' not found as a local directory or in HuggingFace cache (~/.cache/huggingface/hub/).",
                        base
                    )
                })?
            } else {
                anyhow::bail!(
                    "adapter manifest missing `base_model` reference. Cannot load frozen weights."
                );
            };

            let mut base_shards = Vec::new();
            for entry in std::fs::read_dir(&base_dir)? {
                let p = entry?.path();
                if p.extension().map(|e| e == "safetensors").unwrap_or(false)
                    && p.file_name().unwrap().to_string_lossy().contains("model")
                {
                    base_shards.push(p);
                }
            }
            base_shards.sort();
            if base_shards.is_empty() {
                anyhow::bail!(
                    "no model*.safetensors shards found in {}",
                    base_dir.display()
                );
            }

            let merged_file = model_dir.join("merged.safetensors");
            if !merged_file.is_file() {
                tracing::info!(
                    "merged.safetensors missing in {}; merging QLoRA adapter on the fly...",
                    model_dir.display()
                );
                crate::merge::merge_qlora_into_base_subset(
                    &base_shards,
                    &adapter_path,
                    &meta,
                    &merged_file,
                )?;
            }

            if merged_file.is_file() {
                let f = std::fs::File::open(&merged_file)?;
                #[allow(unsafe_code)]
                unsafe {
                    mmaps.push(memmap2::MmapOptions::new().map(&f)?);
                }
            }
            for p in &base_shards {
                let f = std::fs::File::open(p)?;
                #[allow(unsafe_code)]
                unsafe {
                    mmaps.push(memmap2::MmapOptions::new().map(&f)?);
                }
            }
        }

        let mut weight_maps = Vec::new();
        for m in &mmaps {
            weight_maps.push(SafeTensors::deserialize(m)?);
        }
        // Helper to find a linear layer as native QMatMul or unquantized Tensor
        let get_linear = |key: &str| -> Result<QuantizedLinear> {
            if is_quantized {
                let qmeta = quant_meta.as_ref().unwrap();
                let tm = qmeta
                    .tensors
                    .get(key)
                    .ok_or_else(|| anyhow::anyhow!("Tensor `{key}` not in quant-metadata.json"))?;
                for st in &weight_maps {
                    if let Ok(view) = st.tensor(key) {
                        if !tm.quantized {
                            let t = Tensor::from_raw_buffer(
                                view.data(),
                                candle_core::DType::F32,
                                &tm.orig_shape,
                                &_device,
                            )?;
                            return Ok(QuantizedLinear::from_tensor(t));
                        }
                        let dtype = parse_ggml_dtype(&tm.ggml_dtype).ok_or_else(|| {
                            anyhow::anyhow!("unsupported GGML dtype `{}`", tm.ggml_dtype)
                        })?;
                        let qt = candle_core::quantized::ggml_file::qtensor_from_ggml(
                            dtype,
                            view.data(),
                            tm.orig_shape.clone(),
                            &_device,
                        )?;
                        let qmm = QMatMul::from_qtensor(qt)?;
                        return Ok(QuantizedLinear::from_qmatmul(qmm));
                    }
                }
                anyhow::bail!("Quantized tensor `{key}` not found in model.safetensors");
            } else {
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
                        return Ok(QuantizedLinear::from_tensor(t));
                    }
                }
                anyhow::bail!("Weight not found: {key}");
            }
        };

        // Helper to find a non-linear tensor (layernorm, embedding, bias) in any map
        let get_tensor = |key: &str| -> Result<Tensor> {
            if is_quantized {
                let qmeta = quant_meta.as_ref().unwrap();
                let tm = qmeta
                    .tensors
                    .get(key)
                    .ok_or_else(|| anyhow::anyhow!("Tensor `{key}` not in quant-metadata.json"))?;
                for st in &weight_maps {
                    if let Ok(view) = st.tensor(key) {
                        if !tm.quantized {
                            let t = Tensor::from_raw_buffer(
                                view.data(),
                                candle_core::DType::F32,
                                &tm.orig_shape,
                                &_device,
                            )?;
                            return Ok(t);
                        }
                        let dtype = parse_ggml_dtype(&tm.ggml_dtype).ok_or_else(|| {
                            anyhow::anyhow!("unsupported GGML dtype `{}`", tm.ggml_dtype)
                        })?;
                        let qt = candle_core::quantized::ggml_file::qtensor_from_ggml(
                            dtype,
                            view.data(),
                            tm.orig_shape.clone(),
                            &_device,
                        )?;
                        let deq = qt.dequantize(&_device)?;
                        return Ok(deq);
                    }
                }
                anyhow::bail!("Quantized tensor `{key}` not found in model.safetensors");
            } else {
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
            }
        };

        let config_path = model_dir.join("config.json");
        let mut layout = if config_path.is_file() {
            let s = std::fs::read_to_string(&config_path)
                .map_err(|e| anyhow::anyhow!("read config.json: {e}"))?;
            crate::hf_layout::HfTransformerLayout::from_config_json_str(&s)?
        } else {
            anyhow::bail!("config.json missing in {}", model_dir.display());
        };

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
                    let qkv_proj = get_linear(&format!("{p}.linear_attn.in_proj_qkv.weight"))?;
                    let z_proj = get_linear(&format!("{p}.linear_attn.in_proj_z.weight"))?;
                    let b_proj = get_linear(&format!("{p}.linear_attn.in_proj_b.weight"))?;
                    let a_proj = get_linear(&format!("{p}.linear_attn.in_proj_a.weight"))?;
                    let out_proj = get_linear(&format!("{p}.linear_attn.out_proj.weight"))?;
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
                    let q_proj = get_linear(&format!("{p}.self_attn.q_proj.weight"))?;
                    let k_proj = get_linear(&format!("{p}.self_attn.k_proj.weight"))?;
                    let v_proj = get_linear(&format!("{p}.self_attn.v_proj.weight"))?;
                    let o_proj = get_linear(&format!("{p}.self_attn.o_proj.weight"))?;
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
                        q_norm,
                        k_norm,
                        n_heads,
                        n_kv_heads,
                        head_dim,
                    })
                };

                let mlp = Qwen2MLP {
                    gate_proj: get_linear(&format!("{p}.mlp.gate_proj.weight"))?,
                    up_proj: get_linear(&format!("{p}.mlp.up_proj.weight"))?,
                    down_proj: get_linear(&format!("{p}.mlp.down_proj.weight"))?,
                };
                let inv_freq = get_tensor(&format!("{p}.self_attn.rotary_emb.inv_freq"))
                    .or_else(|_| get_tensor(&format!("{p}.linear_attn.rotary_emb.inv_freq")))
                    .ok()
                    .or_else(|| {
                        synthesize_rope_inv_freq(
                            head_dim,
                            layout.rope_partial_rotary_factor,
                            layout.rope_theta,
                            &_device,
                        )
                        .ok()
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
            let lm_head = get_linear("lm_head.weight").or_else(|_| {
                get_linear("model.language_model.embed_tokens.weight")
                    .or_else(|_| get_linear("model.embed_tokens.weight"))
            })?;
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
        Ok(Self {
            model,
            tokenizer: _tokenizer,
            device: _device,
        })
    }

    /// Autoregressive generation loop with KV cache (greedy decoding, unconstrained).
    ///
    /// Grammar-constrained decoding is not available in the plugin build.
    /// `prompt_json` is a JSON object with fields:
    ///   - `"prompt"`: string — the prompt text
    ///   - `"max_tokens"`: integer (default 256)
    ///   - `"temperature"`: float (default 1.0, currently unused — greedy)
    pub fn generate_from_json(&mut self, prompt_json: &str) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct PromptRequest {
            prompt: String,
            #[serde(default = "default_max_tokens")]
            max_tokens: usize,
            #[serde(default = "default_temperature")]
            temperature: f64,
        }
        fn default_max_tokens() -> usize {
            256
        }
        fn default_temperature() -> f64 {
            1.0
        }

        let req: PromptRequest = serde_json::from_str(prompt_json)
            .map_err(|e| anyhow::anyhow!("parse prompt_json: {e}"))?;

        let chatml_prompt = if req.prompt.contains("<|im_start|>") {
            req.prompt.clone()
        } else {
            format!(
                "<|im_start|>system\nYou are a helpful coding assistant that writes correct, robust Vox code.<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                req.prompt.trim()
            )
        };

        let generated = self.generate(&chatml_prompt, req.max_tokens, req.temperature)?;

        let out = serde_json::json!({
            "generated_text": generated,
            "prompt_tokens": self.tokenizer.encode(chatml_prompt.as_str(), true)
                .map(|e| e.len()).unwrap_or(0),
        });
        Ok(out.to_string())
    }

    fn generate(&mut self, prompt: &str, max_tokens: usize, _temperature: f64) -> Result<String> {
        let prompt_tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow::anyhow!("tokenizer error: {e}"))?
            .get_ids()
            .to_vec();

        let prompt_len = prompt_tokens.len();
        if prompt_len == 0 {
            return Ok(String::new());
        }

        let mut generated = String::new();

        // Initialize layer caches
        let mut caches: Vec<Qwen35LayerCache> = match &self.model {
            InferenceModel::Qwen35(model) => model
                .layers
                .iter()
                .map(|l| match &l.attention {
                    Qwen35AttentionBlock::Full(_) => Qwen35LayerCache::Full(None),
                    Qwen35AttentionBlock::Linear(a) => {
                        let state = Tensor::zeros(
                            (1, a.num_v_heads, a.head_k_dim, a.head_v_dim),
                            DType::F32,
                            &self.device,
                        )
                        .unwrap_or_else(|_| {
                            Tensor::zeros((1, 1, 1, 1), DType::F32, &self.device).unwrap()
                        });
                        Qwen35LayerCache::Linear(LinearStateCache {
                            recurrent_state: state,
                            conv_state: None,
                        })
                    }
                })
                .collect(),
        };

        // Phase 1: Prefill prompt tokens all at once
        let input = Tensor::new(prompt_tokens.as_slice(), &self.device)?.unsqueeze(0)?;
        let logits = self
            .model
            .forward_with_cache_opt(&input, 0, Some(&mut caches), true)?;
        let last_logits = logits.squeeze(0)?.squeeze(0)?;
        let slice = last_logits.to_vec1::<f32>()?;

        let mut next_token = 0u32;
        let mut max_val = f32::NEG_INFINITY;
        for (idx, &v) in slice.iter().enumerate() {
            if v > max_val {
                max_val = v;
                next_token = idx as u32;
            }
        }

        if next_token == 151645 || next_token == 151643 {
            return Ok(generated);
        }
        if let Ok(char_str) = self.tokenizer.decode(&[next_token], false) {
            generated.push_str(&char_str);
        }

        // Phase 2: Autoregressive decoding with KV cache
        for cur_pos in prompt_len..(prompt_len + max_tokens.saturating_sub(1)) {
            let input = Tensor::new(&[next_token], &self.device)?.unsqueeze(0)?;
            let logits = self
                .model
                .forward_with_cache(&input, cur_pos, Some(&mut caches))?;
            let logits = logits.squeeze(0)?.squeeze(0)?;
            let slice = logits.to_vec1::<f32>()?;

            let mut best_token = 0u32;
            let mut best_val = f32::NEG_INFINITY;
            for (idx, &v) in slice.iter().enumerate() {
                if v > best_val {
                    best_val = v;
                    best_token = idx as u32;
                }
            }

            if best_token == 151645 || best_token == 151643 {
                break;
            }
            if let Ok(char_str) = self.tokenizer.decode(&[best_token], false) {
                generated.push_str(&char_str);
            }
            next_token = best_token;
        }

        Ok(generated)
    }
}

/// Entry point called from `backend.rs` `run_inference`.
///
/// `model_dir_json` is a JSON string with a `"model_dir"` field pointing to the
/// local model directory, plus the prompt fields consumed by `generate_from_json`.
pub fn run(model_dir: &str, prompt_json: &str) -> Result<String> {
    let path = std::path::Path::new(model_dir);
    let mut engine = InferenceEngine::load(path, &crate::device::DeviceKind::Best)?;
    engine.generate_from_json(prompt_json)
}

#[cfg(test)]
mod tests {
    use super::resolve_adapter_manifest_path;

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

    #[test]
    fn test_rope_inv_freq_synthesis_qwen_default() {
        let dev = candle_core::Device::Cpu;
        let t = super::synthesize_rope_inv_freq(128, None, None, &dev).expect("synthesize");
        assert_eq!(t.dims(), &[64]);
        let vals = t.to_vec1::<f32>().expect("vec");
        assert!((vals[0] - 1.0).abs() < 1e-6);
        let expected_last = 1.0 / 10_000_000.0_f32.powf(126.0 / 128.0);
        assert!((vals[63] - expected_last).abs() < 1e-6);
    }

    #[test]
    fn test_rope_inv_freq_synthesis_partial_rotary_factor() {
        let dev = candle_core::Device::Cpu;
        let t = super::synthesize_rope_inv_freq(128, Some(0.5), Some(10_000_000.0), &dev)
            .expect("synthesize");
        // 128 * 0.5 = 64 rotary_dim -> 32 frequencies
        assert_eq!(t.dims(), &[32]);
        let vals = t.to_vec1::<f32>().expect("vec");
        assert!((vals[0] - 1.0).abs() < 1e-6);
        let expected_last = 1.0 / 10_000_000.0_f32.powf(62.0 / 64.0);
        assert!((vals[31] - expected_last).abs() < 1e-6);
    }
}
