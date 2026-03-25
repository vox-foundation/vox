use burn::tensor::backend::Backend;

use super::Tensor;

impl<B: Backend> Tensor<B> {
    /// Perform a matrix multiplication. Only valid for 2D tensors.
    pub fn matmul(&self, other: &Tensor<B>) -> Self {
        match (self, other) {
            (Tensor::D2(a), Tensor::D2(b)) => Tensor::D2(a.clone().matmul(b.clone())),
            _ => panic!("matmul currently only supports 2D Float tensors in vox-tensor"),
        }
    }

    /// Element-wise addition.
    pub fn add(&self, other: &Tensor<B>) -> Self {
        match (self, other) {
            (Tensor::D1(a), Tensor::D1(b)) => Tensor::D1(a.clone() + b.clone()),
            (Tensor::D2(a), Tensor::D2(b)) => Tensor::D2(a.clone() + b.clone()),
            (Tensor::D1Int(a), Tensor::D1Int(b)) => Tensor::D1Int(a.clone() + b.clone()),
            (Tensor::D2Int(a), Tensor::D2Int(b)) => Tensor::D2Int(a.clone() + b.clone()),
            _ => panic!("add: dimension or type mismatch"),
        }
    }

    /// Element-wise subtraction.
    pub fn sub(&self, other: &Tensor<B>) -> Self {
        match (self, other) {
            (Tensor::D1(a), Tensor::D1(b)) => Tensor::D1(a.clone() - b.clone()),
            (Tensor::D2(a), Tensor::D2(b)) => Tensor::D2(a.clone() - b.clone()),
            (Tensor::D1Int(a), Tensor::D1Int(b)) => Tensor::D1Int(a.clone() - b.clone()),
            (Tensor::D2Int(a), Tensor::D2Int(b)) => Tensor::D2Int(a.clone() - b.clone()),
            _ => panic!("sub: dimension or type mismatch"),
        }
    }

    /// Element-wise multiplication.
    pub fn mul(&self, other: &Tensor<B>) -> Self {
        match (self, other) {
            (Tensor::D1(a), Tensor::D1(b)) => Tensor::D1(a.clone() * b.clone()),
            (Tensor::D2(a), Tensor::D2(b)) => Tensor::D2(a.clone() * b.clone()),
            (Tensor::D1Int(a), Tensor::D1Int(b)) => Tensor::D1Int(a.clone() * b.clone()),
            (Tensor::D2Int(a), Tensor::D2Int(b)) => Tensor::D2Int(a.clone() * b.clone()),
            _ => panic!("mul: dimension or type mismatch"),
        }
    }

    /// Transpose a 2D tensor.
    pub fn transpose(&self) -> Self {
        match self {
            Tensor::D2(t) => Tensor::D2(t.clone().transpose()),
            Tensor::D2Int(t) => Tensor::D2Int(t.clone().transpose()),
            _ => panic!("transpose is only valid for 2D tensors"),
        }
    }
}
