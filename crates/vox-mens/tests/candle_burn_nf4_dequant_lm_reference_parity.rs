//! **Tier B** cross-stack touchpoint: qlora-rs **NF4 quantize → dequantize** yields an f32 weight
//! matrix; Burn NdArray and Candle CPU apply the **same** `QuantizedLinear`-shaped matmul
//! (`x @ Wᵀ` with `W` stored as `[out_features, in_features]` per qlora-rs).
//!
//! This is **not** production NF4 forward parity vs Burn full LM (different graphs). It locks the
//! **reference f32 path** after NF4 round-trip plus shared matmul/bias numerics.

#![cfg(feature = "train")]

use burn::backend::NdArray;
use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use candle_core::{Device, Tensor as CandleTensor};
use qlora_rs::{dequantize_nf4, quantize_nf4};

/// `numel = 13 * 8 = 104` must divide `block_size` (qlora-rs `quantize_nf4` requirement).
const NF4_BLOCK: usize = 8;

#[test]
fn candle_burn_nf4_dequant_lm_reference_linear_matches() {
    let batch = 5usize;
    let hidden = 8usize;
    let vocab = 13usize;

    let mut t = 0.11f32;
    let mut next = || {
        let v = t;
        t += 0.037;
        v
    };

    let mut x_data = Vec::with_capacity(batch * hidden);
    for _ in 0..(batch * hidden) {
        x_data.push(next());
    }

    let mut w_out_in_data = Vec::with_capacity(vocab * hidden);
    for _ in 0..(vocab * hidden) {
        w_out_in_data.push(next());
    }

    let mut b_data = Vec::with_capacity(vocab);
    for _ in 0..vocab {
        b_data.push(next());
    }

    let dev = Device::Cpu;
    let w_orig =
        CandleTensor::from_vec(w_out_in_data.clone(), (vocab, hidden), &dev).expect("w orig");
    let quantized = quantize_nf4(&w_orig, NF4_BLOCK).expect("quantize_nf4");
    let w_dq = dequantize_nf4(&quantized, &dev).expect("dequantize_nf4");
    let w_t = w_dq.t().expect("transpose");
    let wt_flat = w_t
        .flatten_all()
        .expect("flat")
        .to_vec1::<f32>()
        .expect("wt vec");

    type B = NdArray<f32>;
    let dev_b = <B as Backend>::Device::default();
    let x_b = Tensor::<B, 2>::from_data(TensorData::new(x_data.clone(), [batch, hidden]), &dev_b);
    let w_b = Tensor::<B, 2>::from_data(TensorData::new(wt_flat.clone(), [hidden, vocab]), &dev_b);
    let b_b = Tensor::<B, 1>::from_data(TensorData::new(b_data.clone(), [vocab]), &dev_b);
    let b_2d = b_b.reshape([1, vocab]);
    let logits_b = x_b.matmul(w_b) + b_2d;
    let burn_flat = logits_b
        .into_data()
        .as_slice::<f32>()
        .expect("burn logits")
        .to_vec();

    let x_c = CandleTensor::from_vec(x_data, (batch, hidden), &dev).expect("candle x");
    let b_c = CandleTensor::from_vec(b_data, vocab, &dev).expect("candle bias");
    let logits_c = x_c
        .matmul(&w_t)
        .and_then(|y| y.broadcast_add(&b_c))
        .expect("candle logits");
    let candle_flat = logits_c
        .flatten_all()
        .expect("flat")
        .to_vec1::<f32>()
        .expect("candle vec");

    assert_eq!(burn_flat.len(), candle_flat.len(), "element count");
    for (i, (&u, &v)) in burn_flat.iter().zip(candle_flat.iter()).enumerate() {
        assert!(
            (u - v).abs() < 1e-5,
            "nf4-ref lm logits mismatch at {i}: burn={u} candle={v}"
        );
    }
}
