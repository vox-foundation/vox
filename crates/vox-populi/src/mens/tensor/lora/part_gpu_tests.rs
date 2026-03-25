#[cfg(all(test, feature = "mens-gpu"))]
mod burn_full_graph_smoke {
    use burn::backend::wgpu::Wgpu;
    use burn::tensor::{Int, Tensor, TensorData};

    use super::LoraVoxTransformer;

    /// Deterministic shape smoke: one block, tiny vocab (full-graph forward path).
    #[test]
    fn lora_vox_transformer_forward_logits_shape() {
        type B = Wgpu;
        let device = <B as burn::tensor::backend::Backend>::Device::default();
        let vocab = 24usize;
        let d_model = 16usize;
        let n_heads = 2usize;
        let n_layers = 1usize;
        let model =
            LoraVoxTransformer::<B>::new(&device, vocab, d_model, n_heads, n_layers, 4, 8.0);
        let seq = 5usize;
        let input = Tensor::<B, 2, Int>::from_data(
            TensorData::new(vec![1i32, 2, 3, 4, 5], [1, seq]),
            &device,
        );
        let logits = model.forward(input);
        let dims = logits.dims();
        assert_eq!(
            dims,
            [1, seq, vocab],
            "expected [batch, seq, vocab], got {dims:?}"
        );
    }
}

#[cfg(all(test, feature = "mens-gpu"))]
mod lora_linear_merge_numeric {
    use burn::backend::NdArray;
    use burn::module::Param;
    use burn::nn;
    use burn::tensor::backend::Backend;
    use burn::tensor::{Tensor, TensorData};

    use super::{LoraConfig, LoraLinear};

    /// `merge()` weight must match `W_base + (alpha/rank) * (lora_a.weight @ lora_b.weight)` (Burn `[in, out]`).
    #[test]
    fn lora_linear_merge_matches_base_plus_scaled_ba() {
        type B = NdArray<f32>;
        let device = <B as Backend>::Device::default();

        let in_f = 3usize;
        let out_f = 4usize;
        let rank = 2usize;
        let alpha = 4.0f32;
        let scale = alpha / rank as f32;

        // Row-major `[d_input, d_output]` (Burn Row `Linear`): index (i, o) → i * out_f + o
        let mut base_w = vec![0f32; in_f * out_f];
        for i in 0..in_f {
            for o in 0..out_f {
                base_w[i * out_f + o] = (o * 10 + i) as f32 * 0.25;
            }
        }
        let mut a_w = vec![0f32; in_f * rank];
        for i in 0..in_f {
            for r in 0..rank {
                a_w[i * rank + r] = (r + 1) as f32 * 0.5 + i as f32 * 0.1;
            }
        }
        let mut b_w = vec![0f32; rank * out_f];
        for r in 0..rank {
            for o in 0..out_f {
                b_w[r * out_f + o] = (o + r) as f32 * 0.3;
            }
        }

        let w_base =
            Tensor::<B, 2>::from_data(TensorData::new(base_w.clone(), [in_f, out_f]), &device);
        let w_a = Tensor::<B, 2>::from_data(TensorData::new(a_w.clone(), [in_f, rank]), &device);
        let w_b = Tensor::<B, 2>::from_data(TensorData::new(b_w.clone(), [rank, out_f]), &device);

        let mut base = nn::LinearConfig::new(in_f, out_f)
            .with_bias(false)
            .init(&device);
        base.weight = Param::from_tensor(w_base);

        let config = LoraConfig {
            rank,
            alpha,
            dropout: 0.0,
        };
        let mut lora = LoraLinear::with_base(base, &config);
        lora.lora_a.weight = Param::from_tensor(w_a);
        lora.lora_b.weight = Param::from_tensor(w_b);

        let merged = lora.merge();
        let got = merged.weight.val().into_data();
        let slice = got.as_slice::<f32>().expect("f32 weights");

        for i in 0..in_f {
            for o in 0..out_f {
                let mut delta = 0f32;
                for k in 0..rank {
                    delta += a_w[i * rank + k] * b_w[k * out_f + o];
                }
                delta *= scale;
                let exp = base_w[i * out_f + o] + delta;
                let idx = i * out_f + o;
                assert!(
                    (slice[idx] - exp).abs() < 1e-4,
                    "i={i} o={o}: expected {exp} got {}",
                    slice[idx]
                );
            }
        }
    }
}

#[cfg(all(test, feature = "mens-gpu"))]
mod lora_attention_merged_mha_parity {
    use burn::backend::NdArray;
    use burn::module::Param;
    use burn::nn::attention::{MhaInput, generate_autoregressive_mask};
    use burn::tensor::backend::Backend;
    use burn::tensor::{Tensor, TensorData};

    use super::LoraAttention;

    #[test]
    fn merged_multi_head_attention_matches_lora_attention_forward() {
        type B = NdArray<f32>;
        let device = <B as Backend>::Device::default();
        let d_model = 16usize;
        let n_heads = 4usize;
        let rank = 3usize;
        let alpha = 6.0f32;
        let seq = 5usize;
        let batch = 2usize;

        let mut attn = LoraAttention::new(&device, d_model, n_heads, rank, alpha);

        let fill_mat = |rows: usize, cols: usize, seed: f32| -> Vec<f32> {
            let mut v = Vec::with_capacity(rows * cols);
            let mut t = seed;
            for _ in 0..(rows * cols) {
                v.push(t);
                t += 0.013;
            }
            v
        };

        for (proj, seed) in [
            (&mut attn.q, 0.02f32),
            (&mut attn.k, 0.03f32),
            (&mut attn.v, 0.04f32),
        ] {
            proj.lora_a.weight = Param::from_tensor(Tensor::from_data(
                TensorData::new(fill_mat(d_model, rank, seed), [d_model, rank]),
                &device,
            ));
            proj.lora_b.weight = Param::from_tensor(Tensor::from_data(
                TensorData::new(fill_mat(rank, d_model, seed + 0.1), [rank, d_model]),
                &device,
            ));
        }

        let mut input_flat: Vec<f32> = Vec::with_capacity(batch * seq * d_model);
        let mut t = -0.5f32;
        for _ in 0..(batch * seq * d_model) {
            input_flat.push(t);
            t += 0.007;
        }
        let x =
            Tensor::<B, 3>::from_data(TensorData::new(input_flat, [batch, seq, d_model]), &device);

        let y_lora = attn.forward(x.clone());
        let mha = attn.merge();
        let mask = generate_autoregressive_mask(batch, seq, &device);
        let y_mha = mha.forward(MhaInput::self_attn(x).mask_attn(mask)).context;

        let y_lora_s = y_lora.into_data().as_slice::<f32>().unwrap().to_vec();
        let y_mha_s = y_mha.into_data().as_slice::<f32>().unwrap().to_vec();
        assert_eq!(y_lora_s.len(), y_mha_s.len());
        for (i, (&a, &b)) in y_lora_s.iter().zip(y_mha_s.iter()).enumerate() {
            assert!((a - b).abs() < 1e-4, "divergence at {i}: lora={a} mha={b}");
        }
    }
}

#[cfg(all(test, feature = "mens-gpu"))]
mod lora_transformer_block_merge_forward_parity {
    use burn::backend::NdArray;
    use burn::module::Param;
    use burn::tensor::backend::Backend;
    use burn::tensor::{Tensor, TensorData};

    use super::LoraTransformerBlock;

    #[test]
    fn merged_transformer_block_matches_lora_block_forward() {
        type B = NdArray<f32>;
        let device = <B as Backend>::Device::default();
        let d = 16usize;
        let n_heads = 4usize;
        let rank = 3usize;
        let alpha = 6.0f32;
        let seq = 5usize;
        let batch = 2usize;

        let mut block = LoraTransformerBlock::new(&device, d, n_heads, rank, alpha);

        let fill_mat = |rows: usize, cols: usize, seed: f32| -> Vec<f32> {
            let mut v = Vec::with_capacity(rows * cols);
            let mut t = seed;
            for _ in 0..(rows * cols) {
                v.push(t);
                t += 0.011;
            }
            v
        };

        for (proj, seed) in [
            (&mut block.attention.q, 0.02f32),
            (&mut block.attention.k, 0.03f32),
            (&mut block.attention.v, 0.04f32),
        ] {
            proj.lora_a.weight = Param::from_tensor(Tensor::from_data(
                TensorData::new(fill_mat(d, rank, seed), [d, rank]),
                &device,
            ));
            proj.lora_b.weight = Param::from_tensor(Tensor::from_data(
                TensorData::new(fill_mat(rank, d, seed + 0.1), [rank, d]),
                &device,
            ));
        }

        let mut input_flat: Vec<f32> = Vec::with_capacity(batch * seq * d);
        let mut t = -0.3f32;
        for _ in 0..(batch * seq * d) {
            input_flat.push(t);
            t += 0.006;
        }
        let x = Tensor::<B, 3>::from_data(TensorData::new(input_flat, [batch, seq, d]), &device);

        let y_lora = block.forward(x.clone());
        let merged = block.merge();
        let y_plain = merged.forward(x);

        let a = y_lora.into_data().as_slice::<f32>().unwrap().to_vec();
        let b = y_plain.into_data().as_slice::<f32>().unwrap().to_vec();
        assert_eq!(a.len(), b.len());
        for (i, (&u, &v)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                (u - v).abs() < 1e-4,
                "block merge forward mismatch at {i}: {u} vs {v}"
            );
        }
    }
}

#[cfg(all(test, feature = "mens-gpu"))]
mod lora_checkpoint_roundtrip {
    use burn::backend::NdArray;
    use burn::tensor::backend::Backend;
    use burn::tensor::{Int, Tensor, TensorData};
    use tempfile::tempdir;
    use vox_tensor::train::Checkpoint;

    use super::LoraVoxTransformer;

    #[test]
    fn lora_vox_transformer_checkpoint_roundtrip_preserves_forward() {
        type B = NdArray<f32>;
        let device = <B as Backend>::Device::default();
        let vocab_size = 32usize;
        let d_model = 16usize;
        let n_heads = 4usize;
        let n_layers = 2usize;

        let model =
            LoraVoxTransformer::<B>::new(&device, vocab_size, d_model, n_heads, n_layers, 2, 4.0);
        let input = Tensor::<B, 2, Int>::from_data(
            TensorData::new(vec![1i32, 2, 3, 4, 5i32], [1, 5]),
            &device,
        );
        let logits_before = model.forward(input.clone());

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("lora_ckpt.bin");
        Checkpoint::save(&model, &path).expect("Checkpoint::save");

        let fresh =
            LoraVoxTransformer::<B>::new(&device, vocab_size, d_model, n_heads, n_layers, 2, 4.0);
        let loaded = Checkpoint::load(fresh, &path).expect("Checkpoint::load");
        let logits_after = loaded.forward(input);

        let a = logits_before
            .into_data()
            .as_slice::<f32>()
            .expect("f32")
            .to_vec();
        let b = logits_after
            .into_data()
            .as_slice::<f32>()
            .expect("f32")
            .to_vec();
        assert_eq!(a.len(), b.len());
        for (i, (&u, &v)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                (u - v).abs() < 1e-5,
                "logits differ at {i}: before={u} after={v}"
            );
        }
    }
}

#[cfg(all(test, feature = "mens-gpu"))]
mod lora_full_model_merge_forward_parity {
    use burn::backend::NdArray;
    use burn::module::Param;
    use burn::tensor::backend::Backend;
    use burn::tensor::{Int, Tensor, TensorData};

    use super::LoraVoxTransformer;

    #[test]
    fn merged_vox_transformer_matches_lora_full_forward() {
        type B = NdArray<f32>;
        let device = <B as Backend>::Device::default();
        let vocab_size = 24usize;
        let d_model = 16usize;
        let n_heads = 4usize;
        let n_layers = 2usize;
        let rank = 3usize;
        let alpha = 6.0f32;

        let mut model = LoraVoxTransformer::<B>::new(
            &device, vocab_size, d_model, n_heads, n_layers, rank, alpha,
        );

        let fill_mat = |rows: usize, cols: usize, seed: f32| -> Vec<f32> {
            let mut v = Vec::with_capacity(rows * cols);
            let mut t = seed;
            for _ in 0..(rows * cols) {
                v.push(t);
                t += 0.019;
            }
            v
        };

        for block in &mut model.blocks {
            for (proj, seed) in [
                (&mut block.attention.q, 0.015f32),
                (&mut block.attention.k, 0.025f32),
                (&mut block.attention.v, 0.035f32),
            ] {
                proj.lora_a.weight = Param::from_tensor(Tensor::from_data(
                    TensorData::new(fill_mat(d_model, rank, seed), [d_model, rank]),
                    &device,
                ));
                proj.lora_b.weight = Param::from_tensor(Tensor::from_data(
                    TensorData::new(fill_mat(rank, d_model, seed + 0.11), [rank, d_model]),
                    &device,
                ));
            }
        }

        let input = Tensor::<B, 2, Int>::from_data(
            TensorData::new(vec![2i32, 5, 1, 3, 7i32], [1, 5]),
            &device,
        );

        let logits_lora = model.forward(input.clone());
        let merged = model.merge();
        let logits_merged = merged.forward(input);

        let a = logits_lora
            .into_data()
            .as_slice::<f32>()
            .expect("f32")
            .to_vec();
        let b = logits_merged
            .into_data()
            .as_slice::<f32>()
            .expect("f32")
            .to_vec();
        assert_eq!(a.len(), b.len());
        for (i, (&u, &v)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                (u - v).abs() < 1e-4,
                "full merge forward mismatch at {i}: {u} vs {v}"
            );
        }
    }
}
