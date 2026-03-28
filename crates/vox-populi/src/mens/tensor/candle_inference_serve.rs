//! Native Candle inference server bridging to `vox mens serve` payload
//!
//! Loads the base model from local cache (via HF hub or specified path) + LoRA adapter,
//! and runs autoregressive generation with KV caching.

use std::path::Path;

use anyhow::Result;
use candle_core::{Device, Tensor};
use qlora_rs::QuantizedLinear;
use safetensors::Dtype;
use safetensors::SafeTensors;
use tokenizers::Tokenizer;

use crate::mens::tensor::candle_model_qwen::{
    ForwardCache, Qwen2Attention, Qwen2Layer, Qwen2MLP, Qwen2Model, Qwen35AttentionBlock,
    Qwen35ForwardCache, Qwen35Layer, Qwen35LinearAttention, Qwen35Model,
};
use crate::mens::tensor::hf_load::HfArchitecture;

pub struct InferenceEngine {
    pub model: InferenceModel,
    pub tokenizer: Tokenizer,
    pub device: Device,
    pub kv_cache: InferenceCache,
}

pub enum InferenceModel {
    Qwen2(Qwen2Model),
    Qwen35(Qwen35Model),
}

pub enum InferenceCache {
    Qwen2(ForwardCache),
    Qwen35(Qwen35ForwardCache),
}

fn resolve_adapter_meta_path(model_dir: &Path) -> Option<std::path::PathBuf> {
    let meta_v2 = model_dir.join("adapter_meta_v2.json");
    if meta_v2.is_file() {
        return Some(meta_v2);
    }
    let meta_legacy = model_dir.join("meta.json");
    if meta_legacy.is_file() {
        return Some(meta_legacy);
    }
    None
}

impl InferenceEngine {
    pub fn load(model_dir: &Path, device_kind: &crate::mens::DeviceKind) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let _tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;

        let _device = match device_kind {
            crate::mens::DeviceKind::Cpu => Device::Cpu,
            _ => {
                // Simplified fallback if it's GPU but CUDA/Metal is missing
                #[cfg(feature = "mens-candle-qlora-cuda")]
                let dev = Device::new_cuda(0).unwrap_or(Device::Cpu);
                #[cfg(not(feature = "mens-candle-qlora-cuda"))]
                let dev = Device::Cpu;
                dev
            }
        };

        let adapter_path = model_dir.join("candle_qlora_adapter.safetensors");
        let meta_path = resolve_adapter_meta_path(model_dir)
            .unwrap_or_else(|| model_dir.join("adapter_meta_v2.json"));

        if !adapter_path.is_file() || !meta_path.is_file() {
            anyhow::bail!(
                "LoRA adapter or adapter_meta_v2.json/meta.json not found in {}",
                model_dir.display()
            );
        }

        let meta_raw = crate::mens::bounded_fs::read_utf8_path_capped(&meta_path)?;
        let meta: crate::mens::tensor::candle_qlora_merge::QloraAdapterMetaV2 =
            serde_json::from_str(&meta_raw)?;

        // Resolve base shards
        let base_shards = if let Some(ref base) = meta.base_model {
            if Path::new(base).is_dir() {
                let mut shards = Vec::new();
                for entry in std::fs::read_dir(base)? {
                    let p = entry?.path();
                    if p.extension().map(|e| e == "safetensors").unwrap_or(false)
                        && p.file_name().unwrap().to_string_lossy().contains("model")
                    {
                        shards.push(p);
                    }
                }
                shards
            } else {
                // Try to download/lookup via hub
                let files = tokio::runtime::Runtime::new()?
                    .block_on(crate::mens::hub::download_model(base))?;
                files.weights
            }
        } else {
            anyhow::bail!(
                "adapter metadata missing `base_model` reference (adapter_meta_v2.json/meta.json). Cannot load frozen weights."
            );
        };

        let merged_file = model_dir.join("merged.safetensors");
        let mut all_buffers = Vec::new();
        let mut weight_maps = Vec::new();

        if merged_file.is_file() {
            let b = std::fs::read(&merged_file)?;
            all_buffers.push(b);
        }
        for p in &base_shards {
            all_buffers.push(std::fs::read(p)?);
        }
        for b in &all_buffers {
            weight_maps.push(SafeTensors::deserialize(b)?);
        }
        let qlora_cfg = qlora_rs::qlora::QLoraConfig::default();

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

        let config_path = model_dir.join("config.json");
        let mut layout = if config_path.is_file() {
            let s = crate::mens::bounded_fs::read_utf8_path_capped(&config_path)?;
            crate::mens::tensor::hf_load::HfTransformerLayout::from_config_json_str(&s)?
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
                    let qkv_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.linear_attn.in_proj_qkv.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let z_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.linear_attn.in_proj_z.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let b_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.linear_attn.in_proj_b.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let a_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.linear_attn.in_proj_a.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let out_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.linear_attn.out_proj.weight"))?,
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
                        &get_tensor(&format!("{p}.self_attn.q_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let k_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.self_attn.k_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let v_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.self_attn.v_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    let o_proj = QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.self_attn.o_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?;
                    Qwen35AttentionBlock::Full(Qwen2Attention {
                        q_proj,
                        k_proj,
                        v_proj,
                        o_proj,
                        n_heads,
                        n_kv_heads,
                        head_dim,
                    })
                };

                let mlp = Qwen2MLP {
                    gate_proj: QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.mlp.gate_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                    up_proj: QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.mlp.up_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                    down_proj: QuantizedLinear::from_weight(
                        &get_tensor(&format!("{p}.mlp.down_proj.weight"))?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                };
                let inv_freq = get_tensor(&format!("{p}.self_attn.rotary_emb.inv_freq"))
                    .or_else(|_| get_tensor(&format!("{p}.linear_attn.rotary_emb.inv_freq")))
                    .ok();

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
                &get_tensor("lm_head.weight").or_else(|_| {
                    get_tensor("model.language_model.embed_tokens.weight")
                        .or_else(|_| get_tensor("model.embed_tokens.weight"))
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
            let mut layers = Vec::new();
            for i in 0..layout.num_hidden_layers {
                let ln1_key = format!("model.layers.{i}.input_layernorm.weight");
                let ln2_key = format!("model.layers.{i}.post_attention_layernorm.weight");
                let ln1 = candle_nn::RmsNorm::new(get_tensor(&ln1_key)?, 1e-6);
                let ln2 = candle_nn::RmsNorm::new(get_tensor(&ln2_key)?, 1e-6);

                let q_key = format!("model.layers.{i}.self_attn.q_proj.weight");
                let k_key = format!("model.layers.{i}.self_attn.k_proj.weight");
                let v_key = format!("model.layers.{i}.self_attn.v_proj.weight");
                let o_key = format!("model.layers.{i}.self_attn.o_proj.weight");

                let head_dim = layout.hidden_size / layout.num_attention_heads;
                let q_proj =
                    QuantizedLinear::from_weight(&get_tensor(&q_key)?, None, &qlora_cfg, &_device)?;
                let k_proj =
                    QuantizedLinear::from_weight(&get_tensor(&k_key)?, None, &qlora_cfg, &_device)?;
                let v_proj =
                    QuantizedLinear::from_weight(&get_tensor(&v_key)?, None, &qlora_cfg, &_device)?;
                let o_proj =
                    QuantizedLinear::from_weight(&get_tensor(&o_key)?, None, &qlora_cfg, &_device)?;

                let att = Qwen2Attention {
                    q_proj,
                    k_proj,
                    v_proj,
                    o_proj,
                    n_heads: layout.num_attention_heads,
                    n_kv_heads: layout.num_key_value_heads,
                    head_dim,
                };

                let gate_key = format!("model.layers.{i}.mlp.gate_proj.weight");
                let up_key = format!("model.layers.{i}.mlp.up_proj.weight");
                let down_key = format!("model.layers.{i}.mlp.down_proj.weight");

                let mlp = Qwen2MLP {
                    gate_proj: QuantizedLinear::from_weight(
                        &get_tensor(&gate_key)?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                    up_proj: QuantizedLinear::from_weight(
                        &get_tensor(&up_key)?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                    down_proj: QuantizedLinear::from_weight(
                        &get_tensor(&down_key)?,
                        None,
                        &qlora_cfg,
                        &_device,
                    )?,
                };

                let inv_freq_key = format!("model.layers.{i}.self_attn.rotary_emb.inv_freq");
                let inv_freq = get_tensor(&inv_freq_key).ok();

                layers.push(Qwen2Layer {
                    input_layernorm: ln1,
                    self_attn: att,
                    post_attention_layernorm: ln2,
                    mlp,
                    inv_freq,
                });
            }

            let norm = candle_nn::RmsNorm::new(get_tensor("model.norm.weight")?, 1e-6);
            let lm_head_key = if weight_maps
                .iter()
                .any(|st: &SafeTensors| st.tensor("lm_head.weight").is_ok())
            {
                "lm_head.weight"
            } else {
                "model.embed_tokens.weight"
            };
            let lm_head = QuantizedLinear::from_weight(
                &get_tensor(lm_head_key)?,
                None,
                &qlora_cfg,
                &_device,
            )?;
            let model = Qwen2Model {
                embed_tokens: get_tensor("model.embed_tokens.weight")?,
                layers,
                norm,
                lm_head,
            };
            InferenceModel::Qwen2(model)
        };
        let kv_cache = match &model {
            InferenceModel::Qwen2(_) => {
                InferenceCache::Qwen2(ForwardCache::new(layout.num_hidden_layers))
            }
            InferenceModel::Qwen35(_) => {
                InferenceCache::Qwen35(Qwen35ForwardCache::new(layout.num_hidden_layers))
            }
        };

        Ok(Self {
            model,
            tokenizer: _tokenizer,
            device: _device,
            kv_cache,
        })
    }

    /// Autoregressive generation loop with KV cache.
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        _temperature: f64,
        _top_p: Option<f64>,
    ) -> Result<String> {
        let mut tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow::anyhow!("tokenizer error: {e}"))?
            .get_ids()
            .to_vec();

        let mut generated = String::new();

        // Very basic inference loop
        for i in 0..max_tokens {
            let input = if i == 0 {
                Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?
            } else {
                let last = *tokens.last().unwrap();
                Tensor::new(&[last], &self.device)?.unsqueeze(0)?
            };

            let logits = match (&self.model, &mut self.kv_cache) {
                (InferenceModel::Qwen2(model), InferenceCache::Qwen2(cache)) => {
                    model.forward_with_cache(&input, i, cache)?
                }
                (InferenceModel::Qwen35(model), InferenceCache::Qwen35(cache)) => {
                    model.forward_with_cache(&input, i, cache)?
                }
                _ => anyhow::bail!("inference cache/model mismatch"),
            };
            let logits = logits.squeeze(0)?;
            let seq = logits.dim(0)?;
            let logits = logits.narrow(0, seq.saturating_sub(1), 1)?.squeeze(0)?;

            let slice = logits.to_vec1::<f32>()?;
            let mut next_token = 0;
            let mut max_val = f32::NEG_INFINITY;
            for (idx, &v) in slice.iter().enumerate() {
                if v > max_val {
                    max_val = v;
                    next_token = idx;
                }
            }
            let next_token = next_token as u32;

            tokens.push(next_token);

            if let Ok(txt) = self.tokenizer.decode(&[next_token], false) {
                generated.push_str(&txt);
            }

            // check EOS (assume 151643 for Qwen2 as EOS)
            if next_token == 151643 {
                break;
            }
        }

        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_adapter_meta_path;

    #[test]
    fn resolve_adapter_meta_prefers_v2_then_legacy() {
        let d = tempfile::tempdir().expect("tempdir");
        assert!(resolve_adapter_meta_path(d.path()).is_none());

        let legacy = d.path().join("meta.json");
        std::fs::write(&legacy, "{}").expect("legacy");
        assert_eq!(
            resolve_adapter_meta_path(d.path()).as_deref(),
            Some(legacy.as_path())
        );

        let v2 = d.path().join("adapter_meta_v2.json");
        std::fs::write(&v2, "{}").expect("v2");
        assert_eq!(
            resolve_adapter_meta_path(d.path()).as_deref(),
            Some(v2.as_path())
        );
    }
}
