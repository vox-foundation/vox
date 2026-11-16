//! Continuous batched autoregressive decode session (`batch` / `batch_next` / `batch_cancel`).
//!
//! Shares a single NF4 weight read per linear projection across up to `MAX_FUSED_BATCH` (`16`)
//! concurrent sequences via `Qwen35Model::forward_decode_rows`, while isolating per-sequence
//! RoPE positions and `FullKvCache` buffers so greedy outputs stay token-identical to `batch=1`.

use std::collections::VecDeque;

use anyhow::Result;
use candle_core::Tensor;
use rand::rngs::StdRng;

use crate::inference::{
    InferenceEngine, InferenceModel, check_output_mode, generation_rng, resolve_lane,
    sample_next_token,
};
use crate::model::Qwen35LayerCache;

/// Upper bound matching `nf4_gemv_batch` (`MAX_M = 16`) under `DecodeProtocol::F32`.
const MAX_FUSED_GEMV_BATCH: usize = 16;

/// Upper bound under `DecodeProtocol::Bf16 { gemm_rows }` (further bounded by `gemm_rows`).
const MAX_PADDED_GEMM_BATCH: usize = 128;

/// Cap auto-selected KV cache budget at 12 GB so unified memory keeps generous headroom.
const MAX_AUTO_KV_BUDGET_BYTES: u64 = 12 * 1024 * 1024 * 1024;

#[derive(serde::Deserialize)]
struct BatchItemRequest {
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
    top_p: Option<f64>,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    output_mode: Option<String>,
    #[serde(default)]
    lane: Option<String>,
}

fn default_max_tokens() -> usize {
    256
}

fn default_temperature() -> f64 {
    1.0
}

#[derive(serde::Deserialize)]
struct BatchStartRequest {
    batch: Vec<BatchItemRequest>,
    #[serde(default)]
    batch_size: Option<usize>,
    #[serde(default)]
    max_batch_size: Option<usize>,
    #[serde(default)]
    free_bytes: Option<u64>,
}

struct PendingSeq {
    index: usize,
    prompt_tokens: Vec<u32>,
    prompt_token_count: usize,
    max_tokens: usize,
    temperature: f64,
    top_k: usize,
    top_p: f64,
    rng: StdRng,
}

struct ActiveMeta {
    index: usize,
    prompt_token_count: usize,
    max_tokens: usize,
    tokens_emitted: usize,
    pos: usize,
    next_token: u32,
    generated: String,
    temperature: f64,
    top_k: usize,
    top_p: f64,
    rng: StdRng,
}

/// Accumulates per-step batched decode wall-clock timings for throughput diagnostics.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct StepTimes {
    pub total: std::time::Duration,
    pub steps: u32,
}

impl StepTimes {
    pub(crate) fn record(&mut self, d: std::time::Duration) {
        self.total = self.total.saturating_add(d);
        self.steps = self.steps.saturating_add(1);
    }

    #[must_use]
    pub(crate) fn mean_ms(&self) -> f64 {
        if self.steps == 0 {
            0.0
        } else {
            self.total.as_secs_f64() * 1000.0 / f64::from(self.steps)
        }
    }
}

pub(crate) struct BatchSession {
    width: usize,
    pending: VecDeque<PendingSeq>,
    active: Vec<ActiveMeta>,
    caches: Vec<Vec<Qwen35LayerCache>>,
    finished: VecDeque<(usize, String)>,
    step_times: StepTimes,
}

fn format_response(generated: &str, prompt_tokens: usize) -> String {
    serde_json::json!({
        "generated_text": generated,
        "prompt_tokens": prompt_tokens,
    })
    .to_string()
}

impl BatchSession {
    fn unfinished_count(&self) -> usize {
        self.pending.len() + self.active.len() + self.finished.len()
    }

    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn step_times(&self) -> &StepTimes {
        &self.step_times
    }
}

impl InferenceEngine {
    pub(crate) fn batch_start(&mut self, prompt_json: &str) -> Result<String> {
        let uses_linear = match &self.model {
            InferenceModel::Qwen35(m) => m.uses_linear_attention(),
        };
        if uses_linear {
            anyhow::bail!("batched decode is not supported for linear-attention hybrid models");
        }

        let req: BatchStartRequest = serde_json::from_str(prompt_json)
            .map_err(|e| anyhow::anyhow!("parse batch request: {e}"))?;
        if req.batch.is_empty() {
            anyhow::bail!("batch request must contain at least one prompt");
        }
        if matches!(req.batch_size, Some(0)) || matches!(req.max_batch_size, Some(0)) {
            anyhow::bail!("batch_size and max_batch_size must be >= 1");
        }

        let mut pending = VecDeque::with_capacity(req.batch.len());
        let mut finished = VecDeque::new();
        let mut max_seq_cap = 256usize;

        for (index, item) in req.batch.into_iter().enumerate() {
            check_output_mode(item.output_mode.as_deref())?;
            let lane = resolve_lane(item.lane.as_deref())?;
            let assembled = crate::inference::assemble_prompt_with_switch(
                item.system.as_deref(),
                &item.prompt,
                lane,
                self.user_no_think_switch,
            );
            let prompt_tokens = self
                .tokenizer
                .encode(assembled.as_str(), true)
                .map_err(|e| anyhow::anyhow!("tokenizer error: {e}"))?
                .get_ids()
                .to_vec();
            let prompt_token_count = prompt_tokens.len();

            if prompt_tokens.is_empty() || item.max_tokens == 0 {
                finished.push_back((index, format_response("", prompt_token_count)));
                continue;
            }

            let seq_cap = (prompt_token_count + item.max_tokens).next_multiple_of(256);
            max_seq_cap = max_seq_cap.max(seq_cap);

            pending.push_back(PendingSeq {
                index,
                prompt_tokens,
                prompt_token_count,
                max_tokens: item.max_tokens,
                temperature: item.temperature,
                top_k: item.top_k,
                top_p: item.top_p.unwrap_or(1.0),
                rng: generation_rng(item.seed),
            });
        }

        let protocol_cap = match self.decode_protocol {
            vox_eval::decode_protocol::DecodeProtocol::F32 => MAX_FUSED_GEMV_BATCH,
            vox_eval::decode_protocol::DecodeProtocol::Bf16 { gemm_rows } => {
                gemm_rows.min(MAX_PADDED_GEMM_BATCH).max(1)
            }
        };
        let hard_cap = req
            .max_batch_size
            .map_or(protocol_cap, |mw| mw.min(protocol_cap))
            .max(1);

        let width = if let Some(explicit) = req.batch_size {
            explicit.min(hard_cap)
        } else {
            let kv_bpt = match &self.model {
                InferenceModel::Qwen35(m) => m.kv_bytes_per_token().max(1),
            };
            let bytes_per_slot = (kv_bpt as u64).saturating_mul(max_seq_cap as u64).max(1);
            let budget = req.free_bytes.map_or(MAX_AUTO_KV_BUDGET_BYTES, |fb| {
                (fb / 2).min(MAX_AUTO_KV_BUDGET_BYTES)
            });
            let fit = usize::try_from(budget / bytes_per_slot).unwrap_or(hard_cap);
            fit.clamp(1, hard_cap)
        }
        .min(pending.len().max(1));

        self.batch = Some(BatchSession {
            width,
            pending,
            active: Vec::with_capacity(width),
            caches: Vec::with_capacity(width),
            finished,
            step_times: StepTimes::default(),
        });

        Ok(serde_json::json!({ "batch_size": width }).to_string())
    }

    pub(crate) fn batch_cancel(&mut self) -> Result<String> {
        let cancelled = self.batch.take().map_or(0, |s| s.unfinished_count());
        Ok(serde_json::json!({ "cancelled": cancelled }).to_string())
    }

    pub(crate) fn batch_next(&mut self) -> Result<String> {
        if self.batch.is_none() {
            anyhow::bail!("no batch started");
        }

        loop {
            {
                let session = self.batch.as_mut().expect("checked above");
                if let Some((index, response)) = session.finished.pop_front() {
                    return Ok(serde_json::json!({
                        "index": index,
                        "response": response,
                    })
                    .to_string());
                }
            }

            // Admit pending prompts into open slots via batch=1 prefill.
            loop {
                let next_pending = {
                    let session = self.batch.as_mut().expect("checked above");
                    if session.active.len() >= session.width {
                        None
                    } else {
                        session.pending.pop_front()
                    }
                };
                let Some(mut seq) = next_pending else {
                    break;
                };

                let mut cache = match &self.model {
                    InferenceModel::Qwen35(m) => m.empty_cache(),
                };
                let prompt_len = seq.prompt_tokens.len();
                let logits =
                    crate::model::with_inference_fast_path(|| -> candle_core::Result<Tensor> {
                        let input = Tensor::new(seq.prompt_tokens.as_slice(), &self.device)?
                            .unsqueeze(0)?;
                        match &self.model {
                            InferenceModel::Qwen35(m) => {
                                m.forward_cached_last_token(&input, 0, &mut cache)
                            }
                        }
                    })?;
                let logits = logits.squeeze(0)?;
                let seq_dim = logits.dim(0)?;
                let logits = logits.narrow(0, seq_dim.saturating_sub(1), 1)?.squeeze(0)?;
                let slice = logits.to_vec1::<f32>()?;
                let next_token =
                    sample_next_token(&slice, seq.temperature, seq.top_k, seq.top_p, &mut seq.rng);

                let session = self.batch.as_mut().expect("checked above");
                if self.eos_token_ids.contains(&next_token) {
                    session
                        .finished
                        .push_back((seq.index, format_response("", seq.prompt_token_count)));
                    continue;
                }

                let mut generated = String::new();
                if let Ok(char_str) = self.tokenizer.decode(&[next_token], false) {
                    generated.push_str(&char_str);
                }

                if seq.max_tokens <= 1 {
                    session.finished.push_back((
                        seq.index,
                        format_response(&generated, seq.prompt_token_count),
                    ));
                    continue;
                }

                session.active.push(ActiveMeta {
                    index: seq.index,
                    prompt_token_count: seq.prompt_token_count,
                    max_tokens: seq.max_tokens,
                    tokens_emitted: 1,
                    pos: prompt_len,
                    next_token,
                    generated,
                    temperature: seq.temperature,
                    top_k: seq.top_k,
                    top_p: seq.top_p,
                    rng: seq.rng,
                });
                session.caches.push(cache);
            }

            {
                let session = self.batch.as_mut().expect("checked above");
                if let Some((index, response)) = session.finished.pop_front() {
                    return Ok(serde_json::json!({
                        "index": index,
                        "response": response,
                    })
                    .to_string());
                }
                if session.active.is_empty() {
                    self.batch = None;
                    return Ok(serde_json::json!({ "done": true }).to_string());
                }
            }

            // Step all active sequences together in one batched decode pass.
            let step_t0 = std::time::Instant::now();
            let rows_logits = {
                let session = self.batch.as_mut().expect("checked above");
                let b = session.active.len();
                let tokens: Vec<u32> = session.active.iter().map(|s| s.next_token).collect();
                let positions: Vec<usize> = session.active.iter().map(|s| s.pos).collect();
                let model_ref = &self.model;
                let dev_ref = &self.device;
                let caches_ref = &mut session.caches;

                let logits =
                    crate::model::with_inference_fast_path(|| -> candle_core::Result<Tensor> {
                        let input = Tensor::from_vec(tokens, (b, 1), dev_ref)?;
                        match model_ref {
                            InferenceModel::Qwen35(m) => {
                                if b == 1 {
                                    m.forward_cached_last_token(
                                        &input,
                                        positions[0],
                                        &mut caches_ref[0],
                                    )
                                } else {
                                    m.forward_decode_rows(&input, &positions, caches_ref)
                                }
                            }
                        }
                    })?;
                logits.squeeze(1)?.to_vec2::<f32>()?
            };

            let session = self.batch.as_mut().expect("checked above");
            session.step_times.record(step_t0.elapsed());
            let mut done_rows = Vec::new();
            for (row, slice) in rows_logits.iter().enumerate() {
                let meta = &mut session.active[row];
                meta.pos += 1;
                let next_token = sample_next_token(
                    slice,
                    meta.temperature,
                    meta.top_k,
                    meta.top_p,
                    &mut meta.rng,
                );
                if self.eos_token_ids.contains(&next_token) {
                    done_rows.push(row);
                    continue;
                }
                if let Ok(char_str) = self.tokenizer.decode(&[next_token], false) {
                    meta.generated.push_str(&char_str);
                }
                meta.tokens_emitted += 1;
                meta.next_token = next_token;
                if meta.tokens_emitted >= meta.max_tokens {
                    done_rows.push(row);
                }
            }

            // Collect finished responses in ascending row order, then remove in reverse order
            // so remaining indices in `active` and `caches` stay aligned.
            for &row in &done_rows {
                let meta = &session.active[row];
                session.finished.push_back((
                    meta.index,
                    format_response(&meta.generated, meta.prompt_token_count),
                ));
            }
            for &row in done_rows.iter().rev() {
                session.active.remove(row);
                session.caches.remove(row);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use candle_core::{DType, Device, Tensor};
    use candle_nn::RmsNorm;
    use qlora_rs::{ComputeDType, QLoraConfig, QuantizedLinear};
    use tokenizers::Tokenizer;

    use crate::inference::{InferenceEngine, InferenceModel};
    use crate::model::{Qwen2Attention, Qwen2MLP, Qwen35AttentionBlock, Qwen35Layer, Qwen35Model};
    use vox_plugin_mens_candle_core::rope::synthesize_rope_inv_freq;

    fn build_synthetic_full_attn_engine(dev: Device) -> InferenceEngine {
        let d = 64usize;
        let n_heads = 2usize;
        let n_kv = 1usize;
        let hd = 32usize;
        let vocab = 16usize;

        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = if dev.is_metal() {
            ComputeDType::BF16
        } else {
            ComputeDType::F32
        };
        cfg.cache_dequantized = true;

        let seeded = |rows: usize, cols: usize, salt: f32| -> Tensor {
            let vals: Vec<f32> = (0..(rows * cols))
                .map(|i| ((i as f32 * 0.137 + salt).sin()) * 0.2)
                .collect();
            Tensor::from_vec(vals, (rows, cols), &Device::Cpu).unwrap()
        };
        let qlin = |d_out: usize, d_in: usize, salt: f32| {
            let w = seeded(d_out, d_in, salt);
            crate::model::QuantizedLinear::from(
                QuantizedLinear::from_weight(&w, None, &cfg, &dev).unwrap(),
            )
        };
        let norm = |n: usize| RmsNorm::new(Tensor::ones(n, DType::F32, &dev).unwrap(), 1e-6);

        let inv_freq = synthesize_rope_inv_freq(hd, Some(10_000.0), &dev).unwrap();
        let attn = Qwen2Attention {
            q_proj: qlin(n_heads * hd, d, 0.1),
            k_proj: qlin(n_kv * hd, d, 0.2),
            v_proj: qlin(n_kv * hd, d, 0.3),
            o_proj: qlin(d, n_heads * hd, 0.4),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            q_norm: Some(norm(hd)),
            k_norm: Some(norm(hd)),
            n_heads,
            n_kv_heads: n_kv,
            head_dim: hd,
        };
        let mlp = Qwen2MLP {
            gate_proj: qlin(d * 2, d, 0.5),
            up_proj: qlin(d * 2, d, 0.6),
            down_proj: qlin(d, d * 2, 0.7),
        };
        let layer = Qwen35Layer {
            input_layernorm: norm(d),
            attention: Qwen35AttentionBlock::Full(attn),
            post_attention_layernorm: norm(d),
            mlp,
            inv_freq: Some(inv_freq),
        };
        let embed_dtype = if dev.is_metal() {
            DType::BF16
        } else {
            DType::F32
        };
        let embed_tokens = seeded(vocab, d, 0.9)
            .to_dtype(embed_dtype)
            .unwrap()
            .to_device(&dev)
            .unwrap();
        let model = Qwen35Model {
            embed_tokens,
            layers: vec![layer],
            norm: norm(d),
            lm_head: qlin(vocab, d, 1.1),
        };

        let tok_json = br#"{"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":{"type":"Whitespace"},"post_processor":null,"decoder":null,"model":{"type":"WordLevel","vocab":{"a":0,"b":1,"c":2,"d":3,"e":4,"f":5,"g":6,"h":7,"i":8,"j":9,"k":10,"l":11,"m":12,"n":13,"o":14,"<unk>":15},"unk_token":"<unk>"}}"#;
        let tokenizer = Tokenizer::from_bytes(tok_json).expect("valid tokenizer");

        InferenceEngine {
            model: InferenceModel::Qwen35(model),
            tokenizer,
            device: dev,
            eos_token_ids: Vec::new(),
            decode_protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            batch: None,
            user_no_think_switch: true,
        }
    }

    fn build_synthetic_bf16_padded_engine(dev: Device, gemm_rows: usize) -> InferenceEngine {
        let d = 64usize;
        let n_heads = 2usize;
        let n_kv = 1usize;
        let hd = 32usize;
        let vocab = 16usize;

        let mut cfg = QLoraConfig::preset_all_bf16(4, 8);
        cfg.quantization.compute_dtype = if dev.is_metal() {
            ComputeDType::BF16
        } else {
            ComputeDType::F32
        };
        cfg.cache_dequantized = true;

        let mut head_cfg = cfg.clone();
        head_cfg.quantization.compute_dtype = ComputeDType::F32;

        let seeded = |rows: usize, cols: usize, salt: f32| -> Tensor {
            let vals: Vec<f32> = (0..(rows * cols))
                .map(|i| ((i as f32 * 0.137 + salt).sin()) * 0.2)
                .collect();
            Tensor::from_vec(vals, (rows, cols), &Device::Cpu).unwrap()
        };
        let qlin_padded = |d_out: usize, d_in: usize, salt: f32| {
            let w = seeded(d_out, d_in, salt);
            crate::model::QuantizedLinear::from_weight_padded(&w, None, &cfg, &dev, gemm_rows)
                .unwrap()
        };
        let norm = |n: usize| RmsNorm::new(Tensor::ones(n, DType::F32, &dev).unwrap(), 1e-6);

        let inv_freq = synthesize_rope_inv_freq(hd, Some(10_000.0), &dev).unwrap();
        let attn = Qwen2Attention {
            q_proj: qlin_padded(n_heads * hd, d, 0.1),
            k_proj: qlin_padded(n_kv * hd, d, 0.2),
            v_proj: qlin_padded(n_kv * hd, d, 0.3),
            o_proj: qlin_padded(d, n_heads * hd, 0.4),
            q_bias: None,
            k_bias: None,
            v_bias: None,
            q_norm: Some(norm(hd)),
            k_norm: Some(norm(hd)),
            n_heads,
            n_kv_heads: n_kv,
            head_dim: hd,
        };
        let mlp = Qwen2MLP {
            gate_proj: qlin_padded(d * 2, d, 0.5),
            up_proj: qlin_padded(d * 2, d, 0.6),
            down_proj: qlin_padded(d, d * 2, 0.7),
        };
        let layer = Qwen35Layer {
            input_layernorm: norm(d),
            attention: Qwen35AttentionBlock::Full(attn),
            post_attention_layernorm: norm(d),
            mlp,
            inv_freq: Some(inv_freq),
        };
        let embed_dtype = if dev.is_metal() {
            DType::BF16
        } else {
            DType::F32
        };
        let embed_tokens = seeded(vocab, d, 0.9)
            .to_dtype(embed_dtype)
            .unwrap()
            .to_device(&dev)
            .unwrap();
        let head_w = seeded(vocab, d, 1.1);
        let lm_head =
            crate::model::QuantizedLinear::from_weight_f32_head(&head_w, None, &head_cfg, &dev)
                .unwrap();
        let model = Qwen35Model {
            embed_tokens,
            layers: vec![layer],
            norm: norm(d),
            lm_head,
        };

        let tok_json = br#"{"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":{"type":"Whitespace"},"post_processor":null,"decoder":null,"model":{"type":"WordLevel","vocab":{"a":0,"b":1,"c":2,"d":3,"e":4,"f":5,"g":6,"h":7,"i":8,"j":9,"k":10,"l":11,"m":12,"n":13,"o":14,"<unk>":15},"unk_token":"<unk>"}}"#;
        let tokenizer = Tokenizer::from_bytes(tok_json).expect("valid tokenizer");

        InferenceEngine {
            model: InferenceModel::Qwen35(model),
            tokenizer,
            device: dev,
            eos_token_ids: Vec::new(),
            decode_protocol: vox_eval::decode_protocol::DecodeProtocol::Bf16 { gemm_rows },
            batch: None,
            user_no_think_switch: true,
        }
    }

    fn verify_batch_parity_on(dev: Device) {
        let mut engine = build_synthetic_full_attn_engine(dev);
        let reqs = vec![
            serde_json::json!({"prompt": "<|im_start|>a b", "max_tokens": 6, "temperature": 0.0, "seed": 0}),
            serde_json::json!({"prompt": "<|im_start|>c d e f", "max_tokens": 4, "temperature": 0.0, "seed": 0}),
            serde_json::json!({"prompt": "<|im_start|>g", "max_tokens": 7, "temperature": 0.0, "seed": 0}),
            serde_json::json!({"prompt": "<|im_start|>h i j", "max_tokens": 5, "temperature": 0.0, "seed": 0}),
            serde_json::json!({"prompt": "<|im_start|>k l", "max_tokens": 6, "temperature": 0.0, "seed": 0}),
        ];

        let expected: Vec<String> = reqs
            .iter()
            .map(|r| engine.generate_from_json(&r.to_string()).unwrap())
            .collect();

        let start = engine
            .generate_from_json(
                &serde_json::json!({
                    "batch": reqs,
                    "batch_size": 3,
                    "max_batch_size": 16,
                })
                .to_string(),
            )
            .unwrap();
        let start_v: serde_json::Value = serde_json::from_str(&start).unwrap();
        assert_eq!(start_v["batch_size"], 3);

        let mut actual = vec![None; expected.len()];
        loop {
            let step = engine.generate_from_json(r#"{"batch_next":true}"#).unwrap();
            let v: serde_json::Value = serde_json::from_str(&step).unwrap();
            if v["done"] == true {
                break;
            }
            let idx = v["index"].as_u64().unwrap() as usize;
            let resp = v["response"].as_str().unwrap().to_string();
            assert!(actual[idx].replace(resp).is_none(), "duplicate index {idx}");
        }

        for (i, want) in expected.iter().enumerate() {
            assert_eq!(
                actual[i].as_deref(),
                Some(want.as_str()),
                "batch output mismatch at prompt {i}"
            );
        }
    }

    #[test]
    fn batched_greedy_decode_is_token_identical_to_batch_one_cpu() {
        verify_batch_parity_on(Device::Cpu);
    }

    #[cfg(any(feature = "metal", target_os = "macos"))]
    #[test]
    fn batched_greedy_decode_is_token_identical_to_batch_one_metal() {
        if let Ok(dev) = Device::new_metal(0) {
            verify_batch_parity_on(dev);
        }
    }

    #[test]
    fn batch_cancel_drops_the_session() {
        let mut engine = build_synthetic_full_attn_engine(Device::Cpu);
        let reqs = vec![
            serde_json::json!({"prompt": "<|im_start|>a b", "max_tokens": 5, "temperature": 0.0}),
            serde_json::json!({"prompt": "<|im_start|>c d", "max_tokens": 5, "temperature": 0.0}),
            serde_json::json!({"prompt": "<|im_start|>e f", "max_tokens": 5, "temperature": 0.0}),
        ];
        engine
            .generate_from_json(&serde_json::json!({ "batch": reqs, "batch_size": 2 }).to_string())
            .unwrap();

        let first = engine.generate_from_json(r#"{"batch_next":true}"#).unwrap();
        let first_v: serde_json::Value = serde_json::from_str(&first).unwrap();
        assert!(first_v.get("index").is_some());

        let cancel = engine
            .generate_from_json(r#"{"batch_cancel":true}"#)
            .unwrap();
        let cancel_v: serde_json::Value = serde_json::from_str(&cancel).unwrap();
        assert_eq!(cancel_v["cancelled"], 2);

        let err = engine
            .generate_from_json(r#"{"batch_next":true}"#)
            .expect_err("batch_next after cancel must fail");
        assert!(err.to_string().contains("no batch started"), "{err}");

        let cancel_empty = engine
            .generate_from_json(r#"{"batch_cancel":true}"#)
            .unwrap();
        let cancel_empty_v: serde_json::Value = serde_json::from_str(&cancel_empty).unwrap();
        assert_eq!(cancel_empty_v["cancelled"], 0);
    }

    #[test]
    fn bf16_protocol_bounds_batch_width_by_gemm_rows() {
        let mut engine = build_synthetic_bf16_padded_engine(Device::Cpu, 4);
        let reqs: Vec<serde_json::Value> = (0..10)
            .map(|_| serde_json::json!({"prompt": "<|im_start|>a b", "max_tokens": 4, "temperature": 0.0}))
            .collect();
        let start = engine
            .generate_from_json(&serde_json::json!({ "batch": reqs, "batch_size": 16 }).to_string())
            .unwrap();
        let start_v: serde_json::Value = serde_json::from_str(&start).unwrap();
        assert_eq!(start_v["batch_size"], 4);
    }

    #[cfg(any(feature = "metal", target_os = "macos"))]
    #[test]
    fn batched_bf16_padded_decode_is_token_identical_to_batch_one_metal() {
        let Ok(dev) = Device::new_metal(0) else {
            return;
        };
        let mut engine = build_synthetic_bf16_padded_engine(dev, 32);
        let vocab_tokens = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];
        let reqs: Vec<serde_json::Value> = (0..20)
            .map(|i| {
                let t1 = vocab_tokens[i % vocab_tokens.len()];
                let t2 = vocab_tokens[(i * 3 + 1) % vocab_tokens.len()];
                serde_json::json!({
                    "prompt": format!("<|im_start|>{t1} {t2}"),
                    "max_tokens": 5 + (i % 4),
                    "temperature": 0.0,
                    "seed": 0,
                })
            })
            .collect();

        let expected: Vec<String> = reqs
            .iter()
            .map(|r| engine.generate_from_json(&r.to_string()).unwrap())
            .collect();

        let start = engine
            .generate_from_json(&serde_json::json!({ "batch": reqs, "batch_size": 20 }).to_string())
            .unwrap();
        let start_v: serde_json::Value = serde_json::from_str(&start).unwrap();
        assert_eq!(start_v["batch_size"], 20);

        let mut actual = vec![None; expected.len()];
        loop {
            let step = engine.generate_from_json(r#"{"batch_next":true}"#).unwrap();
            let v: serde_json::Value = serde_json::from_str(&step).unwrap();
            if v["done"] == true {
                break;
            }
            let idx = v["index"].as_u64().unwrap() as usize;
            let resp = v["response"].as_str().unwrap().to_string();
            assert!(actual[idx].replace(resp).is_none(), "duplicate index {idx}");
        }

        for (i, want) in expected.iter().enumerate() {
            assert_eq!(
                actual[i].as_deref(),
                Some(want.as_str()),
                "bf16 padded batch=20 mismatch vs batch=1 at prompt {i}"
            );
        }
    }

    #[test]
    fn step_times_records_mean_ms() {
        let mut st = super::StepTimes::default();
        assert_eq!(st.mean_ms(), 0.0);
        st.record(std::time::Duration::from_millis(12));
        st.record(std::time::Duration::from_millis(18));
        assert_eq!(st.steps, 2);
        assert!((st.mean_ms() - 15.0).abs() < 1e-6);
    }
}
