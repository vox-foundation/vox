//! Sync inference for eval-local (no HTTP server).
//!
//! When GPU is available, loads the model and generates completions for
//! held-out benchmark evaluation.

use anyhow::Result;
use std::path::{Path, PathBuf};

type EvalBackend = vox_populi::burn::backend::wgpu::Wgpu;

/// Holds a loaded LoRA checkpoint for repeated `generate` calls (eval-local hot path).
///
/// **KV cache:** `generate` runs one **prefill** forward over the prompt, captures per-layer K/V,
/// then **decodes** one token at a time while appending to that cache (no full-sequence recompute).
/// Eval-local still sorts prompts lexicographically so unrelated benchmark rows can share a common
/// text prefix when processed back-to-back (helps any host-side prompt buffers).
#[cfg(feature = "gpu")]
pub struct LoadedPopuliEvalModel {
    model: vox_populi::tensor::lora::LoraVoxTransformer<EvalBackend>,
    device: <EvalBackend as vox_populi::burn::tensor::backend::Backend>::Device,
    arch: vox_populi::tensor::manifest::ArchParams,
    seq_len: usize,
    model_path: PathBuf,
}

#[cfg(feature = "gpu")]
impl LoadedPopuliEvalModel {
    /// Load checkpoint from disk once (wgpu).
    pub fn load(model_path: &Path) -> Result<Self> {
        let run_dir = model_path.parent().unwrap_or(Path::new("."));
        let arch = vox_populi::tensor::manifest::ArchParams::from_manifest(run_dir)?;
        let mut rank = 16usize;
        let mut alpha = 32.0f32;
        let mut seq_len = 512usize;
        if let Ok(Some(m)) = vox_populi::tensor::manifest::load_manifest(run_dir) {
            rank = m.config.rank;
            alpha = m.config.alpha;
            seq_len = m.config.seq_len;
        }

        let device = <EvalBackend as vox_populi::burn::tensor::backend::Backend>::Device::default();
        let mut model: vox_populi::tensor::lora::LoraVoxTransformer<EvalBackend> =
            vox_populi::tensor::lora::LoraVoxTransformer::new(
                &device,
                arch.vocab_size,
                arch.d_model,
                arch.n_heads,
                arch.n_layers,
                rank,
                alpha,
            );
        model = vox_populi::tensor::train::Checkpoint::load(model, model_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(Self {
            model,
            device,
            arch,
            seq_len,
            model_path: model_path.to_path_buf(),
        })
    }

    /// Model path used at load time (for reports).
    #[must_use]
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Generate one completion reusing the loaded weights.
    pub fn generate(
        &mut self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String> {
        use vox_populi::burn::prelude::Int;
        use vox_populi::burn::tensor::{Tensor as BurnTensor, TensorData};
        use vox_populi::tensor::data::VoxTokenizer;

        let vocab_size = self.arch.vocab_size;
        let mut tokens: Vec<i32> =
            VoxTokenizer::encode_chatml_inference_prefix(system_prompt, user_prompt)
                .into_iter()
                .map(|id| id as i32)
                .collect();
        if tokens.is_empty() {
            return Ok(String::new());
        }

        let prompt_len = tokens.len();
        let max_gen = max_tokens.min(self.seq_len.saturating_sub(prompt_len));
        if max_gen == 0 {
            return Ok(String::new());
        }

        let prefill_input = BurnTensor::<EvalBackend, 2, Int>::from_data(
            TensorData::new(tokens, [1_usize, prompt_len]),
            &self.device,
        );
        let (mut logits_2d, mut kv_cache) = self
            .model
            .forward_prefill_last_logits_and_kv(prefill_input);

        let top_k = 40usize;
        let mut generated: Vec<u32> = Vec::new();

        for _ in 0..max_gen {
            let logits_data: Vec<f32> = logits_2d
                .clone()
                .reshape([vocab_size])
                .into_data()
                .to_vec::<f32>()
                .unwrap_or_default();

            let next = if temperature < 1e-4 {
                logits_data
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i as u32)
                    .unwrap_or(2)
            } else {
                let scaled: Vec<f32> = logits_data.iter().map(|&v| v / temperature).collect();
                let mut indexed: Vec<(usize, f32)> = scaled.iter().copied().enumerate().collect();
                indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let threshold = indexed
                    .get(top_k - 1)
                    .map(|x| x.1)
                    .unwrap_or(f32::NEG_INFINITY);
                let filtered: Vec<f32> = scaled
                    .iter()
                    .map(|&v| if v >= threshold { v } else { f32::NEG_INFINITY })
                    .collect();
                let max_l = filtered.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let exp: Vec<f32> = filtered.iter().map(|&v| (v - max_l).exp()).collect();
                let sum: f32 = exp.iter().sum();
                let probs: Vec<f32> = exp.iter().map(|&e| e / sum.max(1e-9)).collect();
                let time_ns = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(42) as u64;
                let mut rng: u64 = time_ns
                    ^ (generated.len() as u64)
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                let mut best_idx = 0usize;
                let mut best_gumbel = f32::NEG_INFINITY;
                for (i, &p) in probs.iter().enumerate() {
                    if p <= 0.0 {
                        continue;
                    }
                    rng = rng
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let u = (rng >> 33) as f32 / (u32::MAX as f32) + 1e-10;
                    let gumbel = p.ln() - (-u.ln()).ln();
                    if gumbel > best_gumbel {
                        best_gumbel = gumbel;
                        best_idx = i;
                    }
                }
                best_idx as u32
            };

            if next == 2 {
                break;
            }
            generated.push(next);

            if generated.len() >= max_gen {
                break;
            }

            let abs_pos = prompt_len + generated.len() - 1;
            let step_tok = BurnTensor::<EvalBackend, 2, Int>::from_data(
                TensorData::new(vec![next as i32], [1_usize, 1_usize]),
                &self.device,
            );
            logits_2d = self
                .model
                .forward_decode_logits(step_tok, abs_pos, &mut kv_cache);
        }

        Ok(VoxTokenizer::decode(&generated))
    }
}

/// Generate one completion from a loaded model (GPU only).
///
/// Loads weights from disk on every call; prefer [`LoadedPopuliEvalModel`] for benchmarks.
#[cfg(feature = "gpu")]
pub fn generate_one(
    model_path: &Path,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: usize,
    temperature: f32,
) -> Result<String> {
    let mut session = LoadedPopuliEvalModel::load(model_path)?;
    session.generate(system_prompt, user_prompt, max_tokens, temperature)
}
