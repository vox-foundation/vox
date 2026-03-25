use burn::tensor::backend::Backend;

use super::Tensor;

impl<B: Backend> Tensor<B> {
    /// Slice a tensor along a dimension.
    pub fn slice(&self, dim: usize, start: usize, end: usize) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(t.clone().slice([start..end])),
            Tensor::D2(t) => match dim {
                0 => Tensor::D2(t.clone().slice([start..end, 0..t.dims()[1]])),
                1 => Tensor::D2(t.clone().slice([0..t.dims()[0], start..end])),
                _ => panic!("slice: invalid dim for D2"),
            },
            Tensor::D3(t) => match dim {
                0 => Tensor::D3(
                    t.clone()
                        .slice([start..end, 0..t.dims()[1], 0..t.dims()[2]]),
                ),
                1 => Tensor::D3(
                    t.clone()
                        .slice([0..t.dims()[0], start..end, 0..t.dims()[2]]),
                ),
                2 => Tensor::D3(
                    t.clone()
                        .slice([0..t.dims()[0], 0..t.dims()[1], start..end]),
                ),
                _ => panic!("slice: invalid dim for D3"),
            },
            Tensor::D4(t) => match dim {
                0 => Tensor::D4(t.clone().slice([
                    start..end,
                    0..t.dims()[1],
                    0..t.dims()[2],
                    0..t.dims()[3],
                ])),
                1 => Tensor::D4(t.clone().slice([
                    0..t.dims()[0],
                    start..end,
                    0..t.dims()[2],
                    0..t.dims()[3],
                ])),
                2 => Tensor::D4(t.clone().slice([
                    0..t.dims()[0],
                    0..t.dims()[1],
                    start..end,
                    0..t.dims()[3],
                ])),
                3 => Tensor::D4(t.clone().slice([
                    0..t.dims()[0],
                    0..t.dims()[1],
                    0..t.dims()[2],
                    start..end,
                ])),
                _ => panic!("slice: invalid dim for D4"),
            },
            Tensor::D1Int(t) => Tensor::D1Int(t.clone().slice([start..end])),
            Tensor::D2Int(t) => match dim {
                0 => Tensor::D2Int(t.clone().slice([start..end, 0..t.dims()[1]])),
                1 => Tensor::D2Int(t.clone().slice([0..t.dims()[0], start..end])),
                _ => panic!("slice: invalid dim for D2Int"),
            },
        }
    }

    /// Sum all elements into a scalar tensor (1D, size 1).
    pub fn sum(&self) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(t.clone().sum().unsqueeze()),
            Tensor::D2(t) => Tensor::D1(t.clone().sum().unsqueeze()),
            Tensor::D3(t) => Tensor::D1(t.clone().sum().unsqueeze()),
            Tensor::D4(t) => Tensor::D1(t.clone().sum().unsqueeze()),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("sum: unsupported for int tensors"),
        }
    }

    /// Compute mean of all elements.
    pub fn mean(&self) -> Self {
        match self {
            Tensor::D1(t) => Tensor::D1(t.clone().mean().unsqueeze()),
            Tensor::D2(t) => Tensor::D1(t.clone().mean().unsqueeze()),
            Tensor::D3(t) => Tensor::D1(t.clone().mean().unsqueeze()),
            Tensor::D4(t) => Tensor::D1(t.clone().mean().unsqueeze()),
            Tensor::D1Int(_) | Tensor::D2Int(_) => panic!("mean: unsupported for int tensors"),
        }
    }

    /// Extract all values as a flat `Vec<f32>`.
    pub fn to_vec(&self) -> Vec<f32> {
        match self {
            Tensor::D1(t) => t.clone().into_data().to_vec::<f32>().unwrap_or_default(),
            Tensor::D2(t) => t.clone().into_data().to_vec::<f32>().unwrap_or_default(),
            Tensor::D3(t) => t.clone().into_data().to_vec::<f32>().unwrap_or_default(),
            Tensor::D4(t) => t.clone().into_data().to_vec::<f32>().unwrap_or_default(),
            Tensor::D1Int(t) => t
                .clone()
                .into_data()
                .to_vec::<i32>()
                .unwrap_or_default()
                .into_iter()
                .map(|v| v as f32)
                .collect(),
            Tensor::D2Int(t) => t
                .clone()
                .into_data()
                .to_vec::<i32>()
                .unwrap_or_default()
                .into_iter()
                .map(|v| v as f32)
                .collect(),
        }
    }

    /// Number of elements.
    pub fn numel(&self) -> usize {
        self.shape().0.iter().product()
    }
}
