//! Cross-stack **cross-entropy** on identical **f32** logits (Burn `vox_tensor::vox_nn` vs `candle_nn`).
//!
//! This does not compare NF4 QLoRA vs full Burn LM logits — only the CE primitive on shared inputs.

#![cfg(feature = "train")]

use burn::backend::NdArray;
use burn::tensor::backend::Backend;
use burn::tensor::{Int, Tensor as BurnTensor, TensorData};
use candle_core::{Device, Tensor as CandleTensor};
use candle_nn::loss::cross_entropy;
use vox_tensor::tensor::Tensor as Vt;
use vox_tensor::vox_nn::cross_entropy_loss;

#[test]
fn candle_burn_cross_entropy_scalar_matches() {
    type B = NdArray<f32>;
    let device = <B as Backend>::Device::default();
    let batch = 4usize;
    let classes = 11usize;

    let mut logits_data = Vec::with_capacity(batch * classes);
    let mut t = 0.05f32;
    for _ in 0..(batch * classes) {
        logits_data.push(t);
        t += 0.071;
    }
    let labels_i32: Vec<i32> = vec![0, 5, 2, 9];

    let burn_logits = BurnTensor::<B, 2>::from_data(
        TensorData::new(logits_data.clone(), [batch, classes]),
        &device,
    );
    let burn_labels =
        BurnTensor::<B, 1, Int>::from_data(TensorData::new(labels_i32.clone(), [batch]), &device);
    let loss_wrapped = cross_entropy_loss(&Vt::D2(burn_logits), &Vt::D1Int(burn_labels));
    let burn_scalar = match loss_wrapped {
        Vt::D1(x) => x.into_data().as_slice::<f32>().expect("f32 loss")[0],
        _ => panic!("expected D1 loss"),
    };

    let dev = Device::Cpu;
    let c_logits =
        CandleTensor::from_vec(logits_data, (batch, classes), &dev).expect("candle logits");
    let c_target = CandleTensor::from_vec(
        labels_i32.iter().map(|&i| i as u32).collect::<Vec<_>>(),
        batch,
        &dev,
    )
    .expect("candle target");
    let c_loss = cross_entropy(&c_logits, &c_target).expect("candle ce");
    let candle_scalar = c_loss
        .flatten_all()
        .expect("flat")
        .to_vec1::<f32>()
        .expect("vec1")[0];

    assert!(
        (burn_scalar - candle_scalar).abs() < 5e-5,
        "CE mismatch: burn={burn_scalar} candle={candle_scalar}"
    );
}
