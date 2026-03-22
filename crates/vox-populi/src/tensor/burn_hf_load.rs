//! Best-effort **HF → Burn** weight warm-start for GPT-2-shaped checkpoints.
//!
//! Loads token embeddings plus, for GPT-2 safetensors, decoder blocks (Q/K/V split from `c_attn`),
//! output projection, MLP, layer norms, `wpe`, `ln_f`, and untied `lm_head` when present and
//! shapes match the built [`super::lora::LoraVoxTransformer`].

#[cfg(feature = "gpu")]
use std::path::PathBuf;

#[cfg(feature = "gpu")]
use burn::module::Param;
#[cfg(feature = "gpu")]
use burn::nn;
#[cfg(feature = "gpu")]
use burn::tensor::backend::Backend;
#[cfg(feature = "gpu")]
use burn::tensor::{Tensor, TensorData};
#[cfg(feature = "gpu")]
use safetensors::SafeTensors;

#[cfg(feature = "gpu")]
use super::hf_load::{HfArchitecture, HfTransformerLayout};
#[cfg(feature = "gpu")]
use super::lora::{LoraVoxTransformer, MAX_SEQ_LEN};

const EMBED_KEYS: &[&str] = &["wte.weight", "model.embed_tokens.weight"];

/// Summary of which global tensors were applied during GPT-2 decoder warm-start.
#[cfg(feature = "gpu")]
#[derive(Debug, Clone, Default)]
pub struct Gpt2DecoderWarmStart {
    /// Number of transformer blocks where attention + MLP + both layer norms loaded.
    pub layers_complete: usize,
    pub pos_embedding: bool,
    pub final_norm: bool,
    pub lm_head: bool,
}

#[cfg(feature = "gpu")]
fn f32_vec_from_tensor(t: safetensors::tensor::TensorView<'_>) -> anyhow::Result<Vec<f32>> {
    if t.dtype() != safetensors::tensor::Dtype::F32 {
        anyhow::bail!("expected F32 tensor, got {:?}", t.dtype());
    }
    let raw = t.data();
    let n = raw.len() / 4;
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let o = i * 4;
        v.push(f32::from_le_bytes([
            raw[o],
            raw[o + 1],
            raw[o + 2],
            raw[o + 3],
        ]));
    }
    Ok(v)
}

#[cfg(feature = "gpu")]
fn find_tensor_f32(
    weight_paths: &[PathBuf],
    key: &str,
) -> anyhow::Result<Option<(Vec<f32>, Vec<usize>)>> {
    for wp in weight_paths {
        let bytes = std::fs::read(wp)?;
        let Ok(st) = SafeTensors::deserialize(&bytes) else {
            continue;
        };
        let Ok(t) = st.tensor(key) else {
            continue;
        };
        let shape: Vec<usize> = t.shape().to_vec();
        let v = f32_vec_from_tensor(t)?;
        return Ok(Some((v, shape)));
    }
    Ok(None)
}

#[cfg(feature = "gpu")]
fn split_c_attn_qkv(weight: &[f32], d: usize) -> anyhow::Result<(Vec<f32>, Vec<f32>, Vec<f32>)> {
    let expected = d * 3 * d;
    if weight.len() != expected {
        anyhow::bail!("c_attn.weight length {} != d*3*d={expected}", weight.len());
    }
    let mut q = vec![0f32; d * d];
    let mut k = vec![0f32; d * d];
    let mut v = vec![0f32; d * d];
    for row in 0..d {
        let base = row * 3 * d;
        q[row * d..(row + 1) * d].copy_from_slice(&weight[base..base + d]);
        k[row * d..(row + 1) * d].copy_from_slice(&weight[base + d..base + 2 * d]);
        v[row * d..(row + 1) * d].copy_from_slice(&weight[base + 2 * d..base + 3 * d]);
    }
    Ok((q, k, v))
}

#[cfg(feature = "gpu")]
fn split_c_attn_bias(bias: &[f32], d: usize) -> anyhow::Result<(Vec<f32>, Vec<f32>, Vec<f32>)> {
    if bias.len() != 3 * d {
        anyhow::bail!("c_attn.bias len {} != 3*d={}", bias.len(), 3 * d);
    }
    Ok((
        bias[0..d].to_vec(),
        bias[d..2 * d].to_vec(),
        bias[2 * d..3 * d].to_vec(),
    ))
}

/// Transpose HF `lm_head.weight` stored as `[vocab, d_model]` into Burn Row `[d_model, vocab]`.
#[cfg(feature = "gpu")]
fn transpose_lm_head(hf_row_major: &[f32], vocab: usize, d_model: usize) -> Vec<f32> {
    let mut out = vec![0f32; d_model * vocab];
    for v in 0..vocab {
        for d in 0..d_model {
            out[d * vocab + v] = hf_row_major[v * d_model + d];
        }
    }
    out
}

/// Load token embedding table from HF safetensors shards into `model` when `[vocab, d_model]` matches.
///
/// Returns `true` if weights were applied.
#[cfg(feature = "gpu")]
pub fn try_load_token_embeddings<B: Backend>(
    model: &mut LoraVoxTransformer<B>,
    weight_paths: &[std::path::PathBuf],
    device: &B::Device,
) -> anyhow::Result<bool> {
    let want_vocab = model.embedding.weight.dims()[0];
    let want_d = model.embedding.weight.dims()[1];

    for wp in weight_paths {
        let bytes = std::fs::read(wp)?;
        let Ok(st) = SafeTensors::deserialize(&bytes) else {
            continue;
        };
        for &key in EMBED_KEYS {
            let Ok(t) = st.tensor(key) else {
                continue;
            };
            let shape = t.shape();
            if shape.len() != 2 {
                continue;
            }
            if shape[0] != want_vocab || shape[1] != want_d {
                tracing::debug!(
                    key,
                    got = ?shape,
                    want_vocab,
                    want_d,
                    "skip HF embed load (shape mismatch)"
                );
                continue;
            }
            if t.dtype() != safetensors::tensor::Dtype::F32 {
                anyhow::bail!("HF embed {key} must be F32 for Burn warm-start");
            }
            let raw = t.data();
            let n = raw.len() / 4;
            let mut v = Vec::with_capacity(n);
            for i in 0..n {
                let o = i * 4;
                v.push(f32::from_le_bytes([
                    raw[o],
                    raw[o + 1],
                    raw[o + 2],
                    raw[o + 3],
                ]));
            }
            let ten = Tensor::from_data(TensorData::new(v, [want_vocab, want_d]), device);
            model.embedding.weight = Param::from_tensor(ten);
            tracing::info!(key, path = %wp.display(), "loaded HF token embeddings into Burn LoRA model");
            return Ok(true);
        }
    }
    Ok(false)
}

/// Load GPT-2 decoder weights (blocks, norms, pos embed, optional lm_head) when architecture and shapes match.
///
/// Skips missing tensors with `tracing::debug!`. Requires F32 shards.
#[cfg(feature = "gpu")]
pub fn try_load_gpt2_decoder_weights<B: Backend>(
    model: &mut LoraVoxTransformer<B>,
    weight_paths: &[PathBuf],
    layout: &HfTransformerLayout,
    device: &B::Device,
) -> anyhow::Result<Gpt2DecoderWarmStart> {
    if layout.architecture != HfArchitecture::Gpt2 {
        return Ok(Gpt2DecoderWarmStart::default());
    }

    let d = layout.dims.n_embd;
    let n_layer = layout.dims.n_layer;
    let vocab = layout.dims.vocab_size;

    if model.blocks.len() != n_layer {
        tracing::debug!(
            model_layers = model.blocks.len(),
            hf_layers = n_layer,
            "skip GPT-2 decoder load (layer count mismatch)"
        );
        return Ok(Gpt2DecoderWarmStart::default());
    }

    let mut report = Gpt2DecoderWarmStart::default();

    // Positional embeddings: HF `wpe` [P, d]; model table is [MAX_SEQ_LEN, d].
    if let Some((wpe, shape)) = find_tensor_f32(weight_paths, "wpe.weight")? {
        if shape.len() == 2 && shape[1] == d {
            let p = shape[0];
            let take = p.min(MAX_SEQ_LEN);
            if take == 0 {
                tracing::debug!("wpe.weight empty rows");
            } else if model.pos_embedding.weight.dims() != [MAX_SEQ_LEN, d] {
                tracing::debug!(?shape, "pos embedding shape unexpected");
            } else {
                let mut table = vec![0f32; MAX_SEQ_LEN * d];
                for row in 0..take {
                    let src_base = row * d;
                    let dst_base = row * d;
                    table[dst_base..dst_base + d].copy_from_slice(&wpe[src_base..src_base + d]);
                }
                let ten = Tensor::from_data(TensorData::new(table, [MAX_SEQ_LEN, d]), device);
                model.pos_embedding.weight = Param::from_tensor(ten);
                report.pos_embedding = true;
                if p < MAX_SEQ_LEN {
                    tracing::debug!(
                        hf_positions = p,
                        model_max = MAX_SEQ_LEN,
                        "wpe: copied {take} rows; remaining positions stay at init"
                    );
                }
                if p > MAX_SEQ_LEN {
                    tracing::debug!(
                        hf_positions = p,
                        model_max = MAX_SEQ_LEN,
                        "wpe: truncated HF positions to MAX_SEQ_LEN"
                    );
                }
            }
        } else {
            tracing::debug!(?shape, d, "wpe.weight shape mismatch");
        }
    }

    // Final layer norm
    if let (Some((gw, gs)), Some((bw, bs))) = (
        find_tensor_f32(weight_paths, "ln_f.weight")?,
        find_tensor_f32(weight_paths, "ln_f.bias")?,
    ) && gs == [d]
        && bs == [d]
        && gw.len() == d
        && bw.len() == d
    {
        let g = Tensor::from_data(TensorData::new(gw, [d]), device);
        let b = Tensor::from_data(TensorData::new(bw, [d]), device);
        model.norm.gamma = Param::from_tensor(g);
        model.norm.beta = Param::from_tensor(b);
        report.final_norm = true;
    }

    // Untied LM head (HF: [vocab, d] → Burn [d, vocab])
    if let Some((hw, hshape)) = find_tensor_f32(weight_paths, "lm_head.weight")? {
        if hshape.as_slice() == [vocab, d] {
            let tw = transpose_lm_head(&hw, vocab, d);
            let ten = Tensor::from_data(TensorData::new(tw, [d, vocab]), device);
            model.output.weight = Param::from_tensor(ten);
            report.lm_head = true;
        } else {
            tracing::debug!(?hshape, vocab, d, "lm_head.weight shape mismatch");
        }
    }

    for layer in 0..n_layer {
        let prefix = format!("h.{layer}");
        let keys_ok = [
            format!("{prefix}.attn.c_attn.weight"),
            format!("{prefix}.attn.c_attn.bias"),
            format!("{prefix}.attn.c_proj.weight"),
            format!("{prefix}.attn.c_proj.bias"),
            format!("{prefix}.mlp.c_fc.weight"),
            format!("{prefix}.mlp.c_fc.bias"),
            format!("{prefix}.mlp.c_proj.weight"),
            format!("{prefix}.mlp.c_proj.bias"),
            format!("{prefix}.ln_1.weight"),
            format!("{prefix}.ln_1.bias"),
            format!("{prefix}.ln_2.weight"),
            format!("{prefix}.ln_2.bias"),
        ];

        let mut tensors: Vec<Option<(Vec<f32>, Vec<usize>)>> = Vec::with_capacity(keys_ok.len());
        for k in &keys_ok {
            tensors.push(find_tensor_f32(weight_paths, k)?);
        }
        if tensors.iter().any(|t| t.is_none()) {
            tracing::debug!(layer, "GPT-2 layer warm-start skipped (missing keys)");
            continue;
        }
        let get = |i: usize| tensors[i].as_ref().unwrap();
        let (c_w, c_ws) = get(0);
        let (c_b, _) = get(1);
        let (cp_w, cp_ws) = get(2);
        let (cp_b, _) = get(3);
        let (fc_w, fc_ws) = get(4);
        let (fc_b, _) = get(5);
        let (mproj_w, mproj_ws) = get(6);
        let (mproj_b, _) = get(7);
        let (ln1_w, ln1_ws) = get(8);
        let (ln1_b, ln1_bs) = get(9);
        let (ln2_w, ln2_ws) = get(10);
        let (ln2_b, ln2_bs) = get(11);

        let lin_ok = c_ws.as_slice() == [d, 3 * d]
            && cp_ws.as_slice() == [d, d]
            && fc_ws.as_slice() == [d, 4 * d]
            && mproj_ws.as_slice() == [4 * d, d];
        if !lin_ok {
            tracing::debug!(layer, "GPT-2 layer shape mismatch for linear weights");
            continue;
        }
        let norm_ok = ln1_ws.as_slice() == [d]
            && ln1_bs.as_slice() == [d]
            && ln2_ws.as_slice() == [d]
            && ln2_bs.as_slice() == [d];
        if !norm_ok {
            tracing::debug!(layer, "GPT-2 layer norm shape mismatch");
            continue;
        }

        let (qw, kw, vw) = match split_c_attn_qkv(c_w, d) {
            Ok(x) => x,
            Err(e) => {
                tracing::debug!(layer, ?e, "c_attn split failed");
                continue;
            }
        };
        let (qb, kb, vb) = match split_c_attn_bias(c_b, d) {
            Ok(x) => x,
            Err(e) => {
                tracing::debug!(layer, ?e, "c_attn bias split failed");
                continue;
            }
        };

        let block = &mut model.blocks[layer];

        let apply_linear =
            |linear: &mut nn::Linear<B>, w: Vec<f32>, in_f: usize, out_f: usize, b: Vec<f32>| {
                let ten_w = Tensor::from_data(TensorData::new(w, [in_f, out_f]), device);
                linear.weight = Param::from_tensor(ten_w);
                let ten_b = Tensor::from_data(TensorData::new(b, [out_f]), device);
                if let Some(ref mut pb) = linear.bias {
                    *pb = Param::from_tensor(ten_b);
                }
            };

        apply_linear(&mut block.attention.q.base, qw, d, d, qb);
        apply_linear(&mut block.attention.k.base, kw, d, d, kb);
        apply_linear(&mut block.attention.v.base, vw, d, d, vb);
        apply_linear(&mut block.attention.out, cp_w.clone(), d, d, cp_b.clone());

        if let Some(ref mut ff1) = block.ff_linear1 {
            apply_linear(ff1, fc_w.clone(), d, 4 * d, fc_b.clone());
        }
        apply_linear(
            &mut block.ff_linear2,
            mproj_w.clone(),
            4 * d,
            d,
            mproj_b.clone(),
        );

        block.norm1.gamma = Param::from_tensor(Tensor::from_data(
            TensorData::new(ln1_w.clone(), [d]),
            device,
        ));
        block.norm1.beta = Param::from_tensor(Tensor::from_data(
            TensorData::new(ln1_b.clone(), [d]),
            device,
        ));
        block.norm2.gamma = Param::from_tensor(Tensor::from_data(
            TensorData::new(ln2_w.clone(), [d]),
            device,
        ));
        block.norm2.beta = Param::from_tensor(Tensor::from_data(
            TensorData::new(ln2_b.clone(), [d]),
            device,
        ));

        report.layers_complete += 1;
    }

    if report.layers_complete > 0 || report.pos_embedding || report.final_norm || report.lm_head {
        tracing::info!(
            layers = report.layers_complete,
            pos = report.pos_embedding,
            ln_f = report.final_norm,
            lm_head = report.lm_head,
            "HF GPT-2 decoder warm-start applied"
        );
    }

    Ok(report)
}

#[cfg(all(test, feature = "gpu"))]
mod gpt2_warm_start_safetensors_tests {
    use burn::backend::NdArray;
    use burn::tensor::backend::Backend;
    use safetensors::serialize;
    use safetensors::tensor::{Dtype, TensorView};
    use tempfile::tempdir;

    use super::{Gpt2DecoderWarmStart, try_load_gpt2_decoder_weights, try_load_token_embeddings};
    use crate::tensor::hf_load::{ConfigDims, HfArchitecture, HfTransformerLayout};
    use crate::tensor::lora::LoraVoxTransformer;

    fn f32_bytes(vals: &[f32]) -> Vec<u8> {
        vals.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn arange_f32(n: usize, start: f32, step: f32) -> Vec<f32> {
        (0..n).map(|i| start + step * i as f32).collect()
    }

    #[test]
    fn synthetic_gpt2_shard_loads_one_decoder_layer_and_embeddings() {
        type B = NdArray<f32>;
        let device = <B as Backend>::Device::default();
        let d = 8usize;
        let n_heads = 2usize;
        let vocab = 24usize;

        let dir = tempdir().expect("tempdir");
        let shard_path = dir.path().join("one.safetensors");

        let mut tensors: Vec<(String, TensorView<'static>)> = Vec::new();

        let wte: Vec<f32> = arange_f32(vocab * d, 0.0, 0.001);
        let wte_view =
            TensorView::new(Dtype::F32, vec![vocab, d], f32_bytes(&wte).leak()).expect("wte view");
        tensors.push(("wte.weight".into(), wte_view));

        let c_attn_w: Vec<f32> = arange_f32(d * 3 * d, 1.0, 0.0001);
        tensors.push((
            "h.0.attn.c_attn.weight".into(),
            TensorView::new(Dtype::F32, vec![d, 3 * d], f32_bytes(&c_attn_w).leak()).unwrap(),
        ));
        let c_attn_b: Vec<f32> = arange_f32(3 * d, 2.0, 0.01);
        tensors.push((
            "h.0.attn.c_attn.bias".into(),
            TensorView::new(Dtype::F32, vec![3 * d], f32_bytes(&c_attn_b).leak()).unwrap(),
        ));

        let cp_w: Vec<f32> = arange_f32(d * d, 3.0, 0.0002);
        tensors.push((
            "h.0.attn.c_proj.weight".into(),
            TensorView::new(Dtype::F32, vec![d, d], f32_bytes(&cp_w).leak()).unwrap(),
        ));
        let cp_b: Vec<f32> = arange_f32(d, 4.0, 0.02);
        tensors.push((
            "h.0.attn.c_proj.bias".into(),
            TensorView::new(Dtype::F32, vec![d], f32_bytes(&cp_b).leak()).unwrap(),
        ));

        let fc_w: Vec<f32> = arange_f32(d * 4 * d, 5.0, 0.00005);
        tensors.push((
            "h.0.mlp.c_fc.weight".into(),
            TensorView::new(Dtype::F32, vec![d, 4 * d], f32_bytes(&fc_w).leak()).unwrap(),
        ));
        let fc_b: Vec<f32> = arange_f32(4 * d, 6.0, 0.03);
        tensors.push((
            "h.0.mlp.c_fc.bias".into(),
            TensorView::new(Dtype::F32, vec![4 * d], f32_bytes(&fc_b).leak()).unwrap(),
        ));

        let mp_w: Vec<f32> = arange_f32(4 * d * d, 7.0, 0.00003);
        tensors.push((
            "h.0.mlp.c_proj.weight".into(),
            TensorView::new(Dtype::F32, vec![4 * d, d], f32_bytes(&mp_w).leak()).unwrap(),
        ));
        let mp_b: Vec<f32> = arange_f32(d, 8.0, 0.04);
        tensors.push((
            "h.0.mlp.c_proj.bias".into(),
            TensorView::new(Dtype::F32, vec![d], f32_bytes(&mp_b).leak()).unwrap(),
        ));

        for (suffix, seed) in [("ln_1", 9.0f32), ("ln_2", 11.0f32)] {
            let gw: Vec<f32> = arange_f32(d, seed, 0.05);
            let gb: Vec<f32> = arange_f32(d, seed + 0.1, 0.05);
            tensors.push((
                format!("h.0.{suffix}.weight"),
                TensorView::new(Dtype::F32, vec![d], f32_bytes(&gw).leak()).unwrap(),
            ));
            tensors.push((
                format!("h.0.{suffix}.bias"),
                TensorView::new(Dtype::F32, vec![d], f32_bytes(&gb).leak()).unwrap(),
            ));
        }

        let buf = serialize(tensors, &None).expect("serialize safetensors");
        std::fs::write(&shard_path, buf).expect("write shard");

        let paths = vec![shard_path.clone()];
        let mut model = LoraVoxTransformer::<B>::new(&device, vocab, d, n_heads, 1, 2, 4.0);

        assert!(try_load_token_embeddings(&mut model, &paths, &device).unwrap());

        let layout = HfTransformerLayout {
            architecture: HfArchitecture::Gpt2,
            model_type: "gpt2".into(),
            architectures: vec!["GPT2LMHeadModel".into()],
            hidden_size: d,
            num_attention_heads: n_heads,
            num_hidden_layers: 1,
            vocab_size: vocab,
            dims: ConfigDims {
                n_embd: d,
                n_head: n_heads,
                n_layer: 1,
                vocab_size: vocab,
            },
        };

        let Gpt2DecoderWarmStart {
            layers_complete, ..
        } = try_load_gpt2_decoder_weights(&mut model, &paths, &layout, &device).unwrap();
        assert_eq!(layers_complete, 1, "expected full first layer load");

        let got = model.blocks[0]
            .attention
            .q
            .base
            .weight
            .val()
            .into_data()
            .as_slice::<f32>()
            .unwrap()[0];
        let (exp_q0, _, _) = super::split_c_attn_qkv(&c_attn_w, d).unwrap();
        assert!((got - exp_q0[0]).abs() < 1e-5);
    }
}
