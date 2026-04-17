use burn::tensor::backend::Backend;

use super::Tensor;

impl<B: Backend> Tensor<B> {
    /// Apply element-wise ReLU activation.
    pub fn relu(&self) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(burn::tensor::activation::relu(t.clone())),
            Tensor::D2(t) => Tensor::D2(burn::tensor::activation::relu(t.clone())),
            Tensor::D3(t) => Tensor::D3(burn::tensor::activation::relu(t.clone())),
            Tensor::D4(t) => Tensor::D4(burn::tensor::activation::relu(t.clone())),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("relu: unsupported for int tensors"),
            Tensor::Tuple2(_, _) => panic!("relu: unsupported for Tuple2"),
        }
    }

    /// Apply element-wise Tanh activation.
    pub fn tanh(&self) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(t.clone().tanh()),
            Tensor::D2(t) => Tensor::D2(t.clone().tanh()),
            Tensor::D3(t) => Tensor::D3(t.clone().tanh()),
            Tensor::D4(t) => Tensor::D4(t.clone().tanh()),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("tanh: unsupported for int tensors"),
            Tensor::Tuple2(_, _) => panic!("tanh: unsupported for Tuple2"),
        }
    }

    /// Apply element-wise Sigmoid activation.
    pub fn sigmoid(&self) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(burn::tensor::activation::sigmoid(t.clone())),
            Tensor::D2(t) => Tensor::D2(burn::tensor::activation::sigmoid(t.clone())),
            Tensor::D3(t) => Tensor::D3(burn::tensor::activation::sigmoid(t.clone())),
            Tensor::D4(t) => Tensor::D4(burn::tensor::activation::sigmoid(t.clone())),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("sigmoid: unsupported for int tensors"),
            Tensor::Tuple2(_, _) => panic!("sigmoid: unsupported for Tuple2"),
        }
    }

    /// Apply element-wise GELU activation.
    pub fn gelu(&self) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(burn::tensor::activation::gelu(t.clone())),
            Tensor::D2(t) => Tensor::D2(burn::tensor::activation::gelu(t.clone())),
            Tensor::D3(t) => Tensor::D3(burn::tensor::activation::gelu(t.clone())),
            Tensor::D4(t) => Tensor::D4(burn::tensor::activation::gelu(t.clone())),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("gelu: unsupported for int tensors"),
            Tensor::Tuple2(_, _) => panic!("gelu: unsupported for Tuple2"),
        }
    }

    /// Apply element-wise SiLU (Swish) activation.
    pub fn silu(&self) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(burn::tensor::activation::silu(t.clone())),
            Tensor::D2(t) => Tensor::D2(burn::tensor::activation::silu(t.clone())),
            Tensor::D3(t) => Tensor::D3(burn::tensor::activation::silu(t.clone())),
            Tensor::D4(t) => Tensor::D4(burn::tensor::activation::silu(t.clone())),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("silu: unsupported for Int tensors"),
            Tensor::Tuple2(_, _) => panic!("silu: unsupported for Tuple2"),
        }
    }

    /// Apply element-wise Leaky ReLU activation.
    pub fn leaky_relu(&self, negative_slope: f64) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(burn::tensor::activation::leaky_relu(
                t.clone(),
                negative_slope,
            )),
            Tensor::D2(t) => Tensor::D2(burn::tensor::activation::leaky_relu(
                t.clone(),
                negative_slope,
            )),
            Tensor::D3(t) => Tensor::D3(burn::tensor::activation::leaky_relu(
                t.clone(),
                negative_slope,
            )),
            Tensor::D4(t) => Tensor::D4(burn::tensor::activation::leaky_relu(
                t.clone(),
                negative_slope,
            )),
            Tensor::D1Int(_) | Tensor::D2Int(_) => {
                panic!("leaky_relu: unsupported for Int tensors")
            }
            Tensor::Tuple2(_, _) => panic!("leaky_relu: unsupported for Tuple2"),
        }
    }

    /// Apply Softmax activation along a dimension.
    pub fn softmax(&self, dim: usize) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(burn::tensor::activation::softmax(t.clone(), dim)),
            Tensor::D2(t) => Tensor::D2(burn::tensor::activation::softmax(t.clone(), dim)),
            Tensor::D3(t) => Tensor::D3(burn::tensor::activation::softmax(t.clone(), dim)),
            Tensor::D4(t) => Tensor::D4(burn::tensor::activation::softmax(t.clone(), dim)),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("softmax: unsupported for Int tensors"),
            Tensor::Tuple2(_, _) => panic!("softmax: unsupported for Tuple2"),
        }
    }
}
