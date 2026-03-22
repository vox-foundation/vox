//! Sync inference for eval-local (no HTTP server).
//!
//! When GPU is available, loads the model and generates completions for
//! held-out benchmark evaluation.

use anyhow::Result;
use std::path::Path;

type EvalBackend = vox_populi::burn::backend::wgpu::Wgpu;

/// Holds a loaded Burn checkpoint for repeated `generate` calls (eval-local hot path).
///
/// **LoRA checkpoints** (`model_final.bin`, `checkpoint_*.bin`): uses KV-cache decode (fast).
///
/// **Merged checkpoints** (`model_merged.bin` from `merge-weights`): full-sequence forward per
/// token (slower; same math as `vox populi serve` for merged weights).
#[cfg(feature = "gpu")]
pub struct LoadedPopuliEvalModel {
    model: vox_populi::tensor::BurnInferenceModel<EvalBackend>,
    device: <EvalBackend as vox_populi::burn::tensor::backend::Backend>::Device,
    arch: vox_populi::tensor::manifest::ArchParams,
    seq_len: usize,
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
            rank = m.rank;
            alpha = m.alpha;
            seq_len = m.seq_len;
        }

        let device = <EvalBackend as vox_populi::burn::tensor::backend::Backend>::Device::default();
        let spec = vox_populi::tensor::BurnInferenceLoadSpec {
            vocab_size: arch.vocab_size,
            d_model: arch.d_model,
            n_heads: arch.n_heads,
            n_layers: arch.n_layers,
            rank,
            alpha,
        };
        let model =
            vox_populi::tensor::load_burn_inference_model::<EvalBackend>(&device, model_path, spec)
                .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(Self {
            model,
            device,
            arch,
            seq_len,
        })
    }

    /// Generate one completion reusing the loaded weights.
    pub fn generate(
        &mut self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String> {
        use vox_populi::tensor::data::VoxTokenizer;

        let tokens: Vec<i32> =
            VoxTokenizer::encode_chatml_inference_prefix(system_prompt, user_prompt)
                .into_iter()
                .map(|id| id as i32)
                .collect();
        if tokens.is_empty() {
            return Ok(String::new());
        }

        match &mut self.model {
            vox_populi::tensor::BurnInferenceModel::Lora(model) => Self::generate_lora_kv(
                model,
                &self.device,
                self.arch.vocab_size,
                self.seq_len,
                max_tokens,
                temperature,
                tokens,
            ),
            vox_populi::tensor::BurnInferenceModel::Merged(model) => generate_merged_full_forward(
                model,
                &self.device,
                self.arch.vocab_size,
                self.seq_len,
                max_tokens,
                temperature,
                tokens,
            ),
        }
    }

    fn generate_lora_kv(
        model: &mut vox_populi::tensor::lora::LoraVoxTransformer<EvalBackend>,
        device: &<EvalBackend as vox_populi::burn::tensor::backend::Backend>::Device,
        vocab_size: usize,
        seq_len: usize,
        max_tokens: usize,
        temperature: f32,
        tokens: Vec<i32>,
    ) -> Result<String> {
        use vox_populi::burn::prelude::Int;
        use vox_populi::burn::tensor::{Tensor as BurnTensor, TensorData};
        use vox_populi::tensor::data::VoxTokenizer;

        let prompt_len = tokens.len();
        let max_gen = max_tokens.min(seq_len.saturating_sub(prompt_len));
        if max_gen == 0 {
            return Ok(String::new());
        }

        let prefill_input = BurnTensor::<EvalBackend, 2, Int>::from_data(
            TensorData::new(tokens, [1_usize, prompt_len]),
            device,
        );
        let (mut logits_2d, mut kv_cache) = model.forward_prefill_last_logits_and_kv(prefill_input);

        let top_k = 40usize;
        let mut generated: Vec<u32> = Vec::new();

        for _ in 0..max_gen {
            let logits_data: Vec<f32> = logits_2d
                .clone()
                .reshape([vocab_size])
                .into_data()
                .to_vec::<f32>()
                .unwrap_or_default();

            let next = sample_next_token(&logits_data, temperature, top_k, generated.len());

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
                device,
            );
            logits_2d = model.forward_decode_logits(step_tok, abs_pos, &mut kv_cache);
        }

        Ok(VoxTokenizer::decode(&generated))
    }
}

#[cfg(feature = "gpu")]
fn generate_merged_full_forward(
    model: &vox_populi::tensor::burn_stack::VoxTransformer<EvalBackend>,
    device: &<EvalBackend as vox_populi::burn::tensor::backend::Backend>::Device,
    vocab_size: usize,
    seq_len: usize,
    max_tokens: usize,
    temperature: f32,
    mut tokens: Vec<i32>,
) -> Result<String> {
    use vox_populi::burn::prelude::Int;
    use vox_populi::burn::tensor::{Tensor as BurnTensor, TensorData};
    use vox_populi::tensor::data::VoxTokenizer;

    let prompt_len = tokens.len();
    let max_gen = max_tokens.min(seq_len.saturating_sub(prompt_len));
    if max_gen == 0 {
        return Ok(String::new());
    }

    let top_k = 40usize;
    let mut generated: Vec<u32> = Vec::new();

    for _ in 0..max_gen {
        if tokens.len() >= seq_len {
            break;
        }
        let len = tokens.len();
        let input = BurnTensor::<EvalBackend, 2, Int>::from_data(
            TensorData::new(tokens.clone(), [1_usize, len]),
            device,
        );
        let logits = model.forward(input);
        let last = logits
            .slice([0..1, (len - 1)..len, 0..vocab_size])
            .reshape([vocab_size]);
        let logits_data: Vec<f32> = last.into_data().to_vec::<f32>().unwrap_or_default();

        let next = sample_next_token(&logits_data, temperature, top_k, generated.len());

        if next == 2 {
            break;
        }
        tokens.push(next as i32);
        generated.push(next);
    }

    Ok(VoxTokenizer::decode(&generated))
}

#[cfg(feature = "gpu")]
fn sample_next_token(logits_data: &[f32], temperature: f32, top_k: usize, gen_len: usize) -> u32 {
    if temperature < 1e-4 {
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
            .get(top_k.saturating_sub(1))
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
            ^ (gen_len as u64)
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
