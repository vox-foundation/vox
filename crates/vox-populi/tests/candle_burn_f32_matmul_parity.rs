//! Bounded **numeric touchpoint** between Candle CPU and Burn NdArray (f32 matmul).
//!
//! Full LM logits parity is not asserted (different graphs, QLoRA NF4 vs f32 LoRA); this test
//! locks a shared f32 primitive both stacks rely on.

#![cfg(feature = "train")]

use burn::backend::NdArray;
use burn::tensor::backend::Backend;
use burn::tensor::{Tensor, TensorData};
use candle_core::{Device, Tensor as CandleTensor};

#[test]
fn candle_burn_cpu_f32_matmul_matches() {
    let m = 5usize;
    let k = 7usize;
    let n = 4usize;

    let mut a_data = Vec::with_capacity(m * k);
    let mut t = 0.1f32;
    for _ in 0..(m * k) {
        a_data.push(t);
        t += 0.03;
    }
    let mut b_data = Vec::with_capacity(k * n);
    t = -0.2f32;
    for _ in 0..(k * n) {
        b_data.push(t);
        t += 0.02;
    }

    type B = NdArray<f32>;
    let device_b = <B as Backend>::Device::default();
    let burn_a = Tensor::<B, 2>::from_data(TensorData::new(a_data.clone(), [m, k]), &device_b);
    let burn_b = Tensor::<B, 2>::from_data(TensorData::new(b_data.clone(), [k, n]), &device_b);
    let burn_c = burn_a.matmul(burn_b);
    let burn_flat = burn_c.into_data().as_slice::<f32>().expect("f32").to_vec();

    let dev = Device::Cpu;
    let c_a = CandleTensor::from_vec(a_data, (m, k), &dev).expect("candle a");
    let c_b = CandleTensor::from_vec(b_data, (k, n), &dev).expect("candle b");
    let c_c = c_a.matmul(&c_b).expect("candle matmul");
    let candle_flat = c_c
        .flatten_all()
        .expect("flatten")
        .to_vec1::<f32>()
        .expect("vec1");

    assert_eq!(burn_flat.len(), candle_flat.len(), "element count");
    for (i, (&u, &v)) in burn_flat.iter().zip(candle_flat.iter()).enumerate() {
        assert!(
            (u - v).abs() < 1e-5,
            "matmul mismatch at {i}: burn={u} candle={v}"
        );
    }
}
