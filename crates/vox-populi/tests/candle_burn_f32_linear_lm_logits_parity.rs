//! **LM-head-shaped** f32 logits: `hidden @ W + bias` on Burn NdArray vs Candle CPU.
//!
//! This is an additional numeric touchpoint beyond raw matmul (`candle_burn_f32_matmul_parity`)
//! and CE (`candle_burn_cross_entropy_parity`). It does **not** compare NF4 QLoRA vs Burn full-graph
//! forwards — only the shared **biased linear** primitive used at the LM head.

#![cfg(feature = "mens-train")]

use burn::backend::NdArray;
use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use candle_core::{Device, Tensor as CandleTensor};

#[test]
fn candle_burn_cpu_f32_linear_lm_logits_match() {
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
    let mut w_data = Vec::with_capacity(hidden * vocab);
    for _ in 0..(hidden * vocab) {
        w_data.push(next());
    }
    let mut b_data = Vec::with_capacity(vocab);
    for _ in 0..vocab {
        b_data.push(next());
    }

    type B = NdArray<f32>;
    let dev_b = <B as Backend>::Device::default();
    let x_b = Tensor::<B, 2>::from_data(TensorData::new(x_data.clone(), [batch, hidden]), &dev_b);
    let w_b = Tensor::<B, 2>::from_data(TensorData::new(w_data.clone(), [hidden, vocab]), &dev_b);
    let b_b = Tensor::<B, 1>::from_data(TensorData::new(b_data.clone(), [vocab]), &dev_b);
    let b_2d = b_b.reshape([1, vocab]);
    let logits_b = x_b.matmul(w_b) + b_2d;
    let burn_flat = logits_b
        .into_data()
        .as_slice::<f32>()
        .expect("f32 logits")
        .to_vec();

    let dev_c = Device::Cpu;
    let x_c = CandleTensor::from_vec(x_data, (batch, hidden), &dev_c).expect("candle x");
    let w_c = CandleTensor::from_vec(w_data, (hidden, vocab), &dev_c).expect("candle w");
    let b_c = CandleTensor::from_vec(b_data, vocab, &dev_c).expect("candle bias");
    let logits_c = x_c
        .matmul(&w_c)
        .and_then(|y| y.broadcast_add(&b_c))
        .expect("candle lm logits");
    let candle_flat = logits_c
        .flatten_all()
        .expect("flat")
        .to_vec1::<f32>()
        .expect("vec1");

    assert_eq!(burn_flat.len(), candle_flat.len(), "element count");
    for (i, (&u, &v)) in burn_flat.iter().zip(candle_flat.iter()).enumerate() {
        assert!(
            (u - v).abs() < 1e-5,
            "lm logits mismatch at {i}: burn={u} candle={v}"
        );
    }
}
