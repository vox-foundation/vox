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

use crate::mens::tensor::candle_model_qwen::{ForwardCache, Qwen2Model};

pub struct InferenceEngine {
    pub model: Qwen2Model,
    pub tokenizer: Tokenizer,
    pub device: Device,
    pub kv_cache: ForwardCache,
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
        let meta_path = model_dir.join("meta.json");

        if !adapter_path.is_file() || !meta_path.is_file() {
            anyhow::bail!(
                "LoRA adapter or meta.json not found in {}",
                model_dir.display()
            );
        }

        let meta_raw = std::fs::read_to_string(&meta_path)?;
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
            anyhow::bail!("meta.json missing base_model reference. Cannot load frozen weights.");
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

        let _n_layers = meta.layer_order.iter().filter(|k| k.contains(".q")).count(); // rough count
        // Actually, we should get this from the layout in config.json.
        // For now, we'll try to find the layout or assume a default.
        let config_path = model_dir.join("config.json");
        let layout = if config_path.is_file() {
            let s = std::fs::read_to_string(&config_path)?;
            crate::mens::tensor::hf_load::HfTransformerLayout::from_config_json_str(&s)?
        } else {
            // Fallback to a common layout if config.json is missing?
            anyhow::bail!("config.json missing in {}", model_dir.display());
        };

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
            let q_proj = QuantizedLinear::from_weight(
                &get_tensor(&q_key)?,
                None,
                &qlora_rs::qlora::QLoraConfig::default(),
                &_device,
            )?;
            let k_proj = QuantizedLinear::from_weight(
                &get_tensor(&k_key)?,
                None,
                &qlora_rs::qlora::QLoraConfig::default(),
                &_device,
            )?;
            let v_proj = QuantizedLinear::from_weight(
                &get_tensor(&v_key)?,
                None,
                &qlora_rs::qlora::QLoraConfig::default(),
                &_device,
            )?;
            let o_proj = QuantizedLinear::from_weight(
                &get_tensor(&o_key)?,
                None,
                &qlora_rs::qlora::QLoraConfig::default(),
                &_device,
            )?;

            let att = crate::mens::tensor::candle_model_qwen::Qwen2Attention {
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

            let mlp = crate::mens::tensor::candle_model_qwen::Qwen2MLP {
                gate_proj: QuantizedLinear::from_weight(
                    &get_tensor(&gate_key)?,
                    None,
                    &qlora_rs::qlora::QLoraConfig::default(),
                    &_device,
                )?,
                up_proj: QuantizedLinear::from_weight(
                    &get_tensor(&up_key)?,
                    None,
                    &qlora_rs::qlora::QLoraConfig::default(),
                    &_device,
                )?,
                down_proj: QuantizedLinear::from_weight(
                    &get_tensor(&down_key)?,
                    None,
                    &qlora_rs::qlora::QLoraConfig::default(),
                    &_device,
                )?,
            };

            let inv_freq_key = format!("model.layers.{i}.self_attn.rotary_emb.inv_freq");
            let inv_freq = get_tensor(&inv_freq_key).ok();

            layers.push(crate::mens::tensor::candle_model_qwen::Qwen2Layer {
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
            &qlora_rs::qlora::QLoraConfig::default(),
            &_device,
        )?;

        let model = crate::mens::tensor::candle_model_qwen::Qwen2Model {
            embed_tokens: get_tensor("model.embed_tokens.weight")?,
            layers,
            norm,
            lm_head,
        };

        Ok(Self {
            model,
            tokenizer: _tokenizer,
            device: _device,
            kv_cache: crate::mens::tensor::candle_model_qwen::ForwardCache::new(
                layout.num_hidden_layers,
            ),
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

            let logits = self
                .model
                .forward_with_cache(&input, i, &mut self.kv_cache)?;
            let logits = logits.squeeze(0)?.squeeze(0)?; // Assuming bs=1, seq=1

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
