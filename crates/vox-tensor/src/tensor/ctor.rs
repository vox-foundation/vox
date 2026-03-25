use burn::tensor::Distribution;
use burn::tensor::backend::Backend;

use super::{Tensor, TensorShape};

impl<B: Backend> Tensor<B> {
    /// Create a new 1D Tensor of zeros.
    pub fn zeros_1d(len: usize, device: &B::Device) -> Self {
        let t = burn::tensor::Tensor::<B, 1>::zeros([len], device);
        Tensor::D1(t)
    }

    /// Create a new 2D Tensor of zeros.
    pub fn zeros_2d(rows: usize, cols: usize, device: &B::Device) -> Self {
        let t = burn::tensor::Tensor::<B, 2>::zeros([rows, cols], device);
        Tensor::D2(t)
    }

    /// Create a new 1D Tensor of ones.
    pub fn ones_1d(len: usize, device: &B::Device) -> Self {
        let t = burn::tensor::Tensor::<B, 1>::ones([len], device);
        Tensor::D1(t)
    }

    /// Create a new 2D Tensor of ones.
    pub fn ones_2d(rows: usize, cols: usize, device: &B::Device) -> Self {
        let t = burn::tensor::Tensor::<B, 2>::ones([rows, cols], device);
        Tensor::D2(t)
    }

    /// Create a 1D tensor from a `Vec<f32>`.
    pub fn from_vec_1d(data: Vec<f32>, device: &B::Device) -> Self {
        let t = burn::tensor::Tensor::<B, 1>::from_floats(data.as_slice(), device);
        Tensor::D1(t)
    }

    /// Create a 3D tensor from a 1D array.
    pub fn from_vec_3d(
        data: Vec<f32>,
        d0: usize,
        d1: usize,
        d2: usize,
        device: &B::Device,
    ) -> Self {
        let t = burn::tensor::Tensor::<B, 1>::from_floats(data.as_slice(), device)
            .reshape([d0, d1, d2]);
        Tensor::D3(t)
    }

    /// Create a 2D tensor from a flat `Vec<f32>` with explicit shape [rows, cols].
    pub fn from_vec_2d(data: Vec<f32>, rows: usize, cols: usize, device: &B::Device) -> Self {
        let t = burn::tensor::Tensor::<B, 1>::from_floats(data.as_slice(), device)
            .reshape([rows, cols]);
        Tensor::D2(t)
    }

    /// Create a 2D tensor with random normal values (mean=0, std=1).
    pub fn randn_2d(rows: usize, cols: usize, device: &B::Device) -> Self {
        let t = burn::tensor::Tensor::<B, 2>::random(
            [rows, cols],
            Distribution::Normal(0.0, 1.0),
            device,
        );
        Tensor::D2(t)
    }

    /// Retrieve the shape of the tensor.
    pub fn shape(&self) -> TensorShape {
        match self {
            Tensor::D1(t) => TensorShape(t.shape().dims.into()),
            Tensor::D2(t) => TensorShape(t.shape().dims.into()),
            Tensor::D3(t) => TensorShape(t.shape().dims.into()),
            Tensor::D4(t) => TensorShape(t.shape().dims.into()),
            Tensor::D1Int(t) => TensorShape(t.shape().dims.into()),
            Tensor::D2Int(t) => TensorShape(t.shape().dims.into()),
        }
    }
}
