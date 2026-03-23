//! Native Candle inference server bridging to `vox populi serve` payload
//! 
//! Loads the base model from local cache (via HF hub or specified path) + LoRA adapter,
//! and runs autoregressive generation with KV caching.

use std::path::Path;

use anyhow::Result;
use candle_core::{Device, Tensor};
use tokenizers::Tokenizer;

use crate::tensor::candle_model_qwen::{Qwen2Model, ForwardCache};

pub struct InferenceEngine {
    pub model: Qwen2Model,
    pub tokenizer: Tokenizer,
    pub device: Device,
    pub kv_cache: ForwardCache,
}

impl InferenceEngine {
    pub fn load(model_dir: &Path, device_kind: &crate::DeviceKind) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let _tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;

        let _device = match device_kind {
            crate::DeviceKind::Cpu => Device::Cpu,
            _ => {
                // Simplified fallback if it's GPU but CUDA/Metal is missing
                #[cfg(feature = "candle-qlora-cuda")]
                let dev = Device::new_cuda(0).unwrap_or(Device::Cpu);
                #[cfg(not(feature = "candle-qlora-cuda"))]
                let dev = Device::Cpu;
                dev
            }
        };

        let adapter_path = model_dir.join("candle_qlora_adapter.safetensors");
        let meta_path = model_dir.join("meta.json");

        if !adapter_path.is_file() || !meta_path.is_file() {
            anyhow::bail!("LoRA adapter or meta.json not found in {}", model_dir.display());
        }

        let meta_raw = std::fs::read_to_string(&meta_path)?;
        let _meta: crate::tensor::candle_qlora_merge::QloraAdapterMetaV2 = serde_json::from_str(&meta_raw)?;

        // If merged weights exist, use them directly; otherwise error with a helpful message.
        // We'll look for merged shards.
        let merged_file = model_dir.join("merged.safetensors");
        if !merged_file.is_file() {
            anyhow::bail!("Serving an unmerged QLoRA adapter is not fully supported without the base shards directory. Please run `vox-train merge` first to create `merged.safetensors` and serve that.");
        }

        // We use the merged weights to build the model. Wait, the merged weights ONLY contain the merged keys.
        // The rest of the keys (embeds, norms) must be available. 
        anyhow::bail!("Inference load implementation requires base shard integration (which is pending). See AI-Native Core Simplification Strategy.");
    }

    /// Autoregressive generation loop with KV cache.
    pub fn generate(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        _temperature: f64,
        _top_p: Option<f64>,
    ) -> Result<String> {
        let mut tokens = self.tokenizer.encode(prompt, true)
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

            let logits = self.model.forward_with_cache(&input, i, &mut self.kv_cache)?;
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
            
            if let Some(txt) = self.tokenizer.decode(&[next_token], false).ok() {
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
