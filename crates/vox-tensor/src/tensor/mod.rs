//! Burn-backed dynamic-rank tensor wrapper.

use burn::tensor::backend::Backend;

#[cfg(feature = "gpu")]
pub type VoxBackend = burn::backend::Wgpu;
#[cfg(not(feature = "gpu"))]
pub type VoxBackend = burn::backend::NdArray;

/// A dynamic-dimensional Tensor.
/// Wraps Burn's statically-dimensioned tensor to provide the dynamic usage pattern
/// expected in an interpreted or scripting-language context like Vox.
#[derive(Clone, Debug)]
pub enum Tensor<B: Backend> {
    D1(burn::tensor::Tensor<B, 1>),
    D2(burn::tensor::Tensor<B, 2>),
    D3(burn::tensor::Tensor<B, 3>),
    D4(burn::tensor::Tensor<B, 4>),
    D1Int(burn::tensor::Tensor<B, 1, burn::tensor::Int>),
    D2Int(burn::tensor::Tensor<B, 2, burn::tensor::Int>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ElementType {
    Float,
    Int,
    Bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TensorShape(pub Vec<usize>);

mod activations;
mod cat_reshape;
mod ctor;
mod elemwise;
mod slice_reduce;
