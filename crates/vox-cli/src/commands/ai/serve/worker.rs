//! Inference worker thread. Owns the model; communicates via mpsc + oneshot.

#[cfg(feature = "execution-api")]
use super::config::ServeConfig;
#[cfg(feature = "execution-api")]
use anyhow::Result;
#[cfg(feature = "execution-api")]
use std::path::Path;
#[cfg(feature = "execution-api")]
use std::sync::mpsc::SyncSender;

/// Internal message sent from Axum handlers to the inference worker thread.
#[cfg(feature = "execution-api")]
pub struct InferenceRequest {
    pub prompt: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_k: usize,
    pub output_mode: Option<String>,
    pub reply: tokio::sync::oneshot::Sender<Result<String, String>>,
    pub stream_tx: Option<tokio::sync::mpsc::Sender<Result<String, String>>>,
}

/// Spawn the inference worker thread and return the channel sender.
#[cfg(feature = "execution-api")]
pub fn spawn_inference_worker(
    config: &ServeConfig,
    model_name: &str,
    system_prompt: &str,
) -> SyncSender<InferenceRequest> {
    use owo_colors::OwoColorize;
    use vox_populi::tensor::data::VoxTokenizer;
    #[cfg(feature = "gpu")]
    type B = vox_populi::burn::backend::wgpu::Wgpu;
    #[cfg(not(feature = "gpu"))]
    type B = vox_populi::burn::backend::ndarray::NdArray<f32>;

    let run_dir = config.model_path.parent().unwrap_or(Path::new("."));
    let arch = match vox_populi::tensor::manifest::ArchParams::from_manifest(run_dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("  [inference-thread] Failed to resolve arch from manifest: {e}");
            return std::sync::mpsc::sync_channel::<InferenceRequest>(0).0;
        }
    };
    let mut rank = 16usize;
    let mut alpha = 32.0f32;
    let mut seq_len = 512usize;
    if let Ok(Some(m)) = vox_populi::tensor::manifest::load_manifest(run_dir) {
        rank = m.rank;
        alpha = m.alpha;
        seq_len = m.seq_len;
    }

    let model_path = config.model_path.clone();
    let max_tokens_cap = config.max_tokens;
    let model_name_clone = model_name.to_string();
    let system_prompt = system_prompt.to_string();
    let vocab_size = arch.vocab_size;
    let (d_model, n_heads, n_layers) = (arch.d_model, arch.n_heads, arch.n_layers);

    let (tx, rx) = std::sync::mpsc::sync_channel::<InferenceRequest>(8);

    std::thread::spawn(move || {
        eprintln!(
            "  Loading model ({}L/{}H/{}D, r={rank}, alpha={alpha})...",
            n_layers, n_heads, d_model
        );
        let device = <B as vox_populi::burn::tensor::backend::Backend>::Device::default();
        let spec = vox_populi::tensor::BurnInferenceLoadSpec {
            vocab_size,
            d_model,
            n_heads,
            n_layers,
            rank,
            alpha,
        };
        let model = match vox_populi::tensor::load_burn_inference_model::<B>(
            &device,
            model_path.as_path(),
            spec,
        ) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("  [inference-thread] Failed to load checkpoint: {e}");
                return;
            }
        };
        let kind = match &model {
            vox_populi::tensor::BurnInferenceModel::Lora(_) => "LoRA adapter",
            vox_populi::tensor::BurnInferenceModel::Merged(_) => "merged base (merge-weights)",
        };
        eprintln!(
            "  {} Model ready on inference thread ({}) — {}",
            "✓".green(),
            model_name_clone,
            kind
        );

        use super::prompt::mask_logits_for_json;
        use vox_populi::burn::prelude::Int;
        use vox_populi::burn::tensor::{Tensor as BurnTensor, TensorData};

        while let Ok(req) = rx.recv() {
            let cap = req.max_tokens.min(max_tokens_cap);
            let temp = req.temperature.max(0.0);
            let top_k = req.top_k.clamp(1, vocab_size);
            // Use ChatML format to match training (Wave 2)
            let mut tokens: Vec<i32> =
                VoxTokenizer::encode_chatml_inference_prefix(&system_prompt, &req.prompt)
                    .into_iter()
                    .map(|id| id as i32)
                    .collect();
            if tokens.is_empty() {
                let _ = req.reply.send(Ok(String::new()));
                continue;
            }
            let mut generated: Vec<u32> = Vec::new();
            let mut last_text = String::new();

            for _ in 0..cap {
                if tokens.len() >= seq_len {
                    break;
                }
                let len = tokens.len();
                let input = BurnTensor::<B, 2, Int>::from_data(
                    TensorData::new(tokens.clone(), [1_usize, len]),
                    &device,
                );
                let logits = model.forward(input);
                let last = logits
                    .slice([0..1, (len - 1)..len, 0..vocab_size])
                    .reshape([vocab_size]);

                let mut logits_data: Vec<f32> =
                    last.into_data().to_vec::<f32>().unwrap_or_default();

                if req.output_mode.as_deref().is_some_and(|m| {
                    m == "strict_json" || m == "jsonl_records" || m == "tool_args_json"
                }) {
                    let current_text = VoxTokenizer::decode(&generated);
                    mask_logits_for_json(&mut logits_data, &current_text);
                }

                let next = if temp < 1e-4 {
                    logits_data
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(i, _)| i as u32)
                        .unwrap_or(2)
                } else {
                    let scaled: Vec<f32> = logits_data.iter().map(|&v| v / temp).collect();
                    let mut indexed: Vec<(usize, f32)> =
                        scaled.iter().copied().enumerate().collect();
                    indexed
                        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
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
                tokens.push(next as i32);
                generated.push(next);

                if let Some(ref tx) = req.stream_tx {
                    let text = VoxTokenizer::decode(&generated);
                    // Stream only when decode strictly extends the last prefix (avoids duplicate sends).
                    if text.len() > last_text.len() && text.starts_with(&last_text) {
                        let chunk = text[last_text.len()..].to_string();
                        if tx.blocking_send(Ok(chunk)).is_err() {
                            break; // client disconnected
                        }
                        last_text = text;
                    }
                }
            }

            let text = VoxTokenizer::decode(&generated);
            // Flush any remaining partials if we broke out of the loop with a straggler
            if let Some(ref tx) = req.stream_tx {
                if text.len() > last_text.len() && text.starts_with(&last_text) {
                    let chunk = text[last_text.len()..].to_string();
                    let _ = tx.blocking_send(Ok(chunk));
                }
            }

            let _ = req.reply.send(Ok(text));
        }
    });

    tx
}
