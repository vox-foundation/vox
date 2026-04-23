use burn::tensor::backend::Backend;

use super::Tensor;

impl<B: Backend> Tensor<B> {
    /// Concatenate multiple tensors along a dimension.
    pub fn cat(tensors: Vec<Self>, dim: usize) -> Self {
        if tensors.is_empty() {
            panic!("cat: empty tensor list");
        }
        match &tensors[0] {
            Tensor::D1(_) => {
                let inner: Vec<burn::tensor::Tensor<B, 1>> = tensors
                    .into_iter()
                    .map(|t| match t {
                        Tensor::D1(it) => it,
                        _ => panic!("cat: rank mismatch"),
                    })
                    .collect();
                Tensor::D1(burn::tensor::Tensor::cat(inner, dim))
            }
            Tensor::D2(_) => {
                let inner: Vec<burn::tensor::Tensor<B, 2>> = tensors
                    .into_iter()
                    .map(|t| match t {
                        Tensor::D2(it) => it,
                        _ => panic!("cat: rank mismatch"),
                    })
                    .collect();
                Tensor::D2(burn::tensor::Tensor::cat(inner, dim))
            }
            Tensor::D3(_) => {
                let inner: Vec<burn::tensor::Tensor<B, 3>> = tensors
                    .into_iter()
                    .map(|t| match t {
                        Tensor::D3(it) => it,
                        _ => panic!("cat: rank mismatch"),
                    })
                    .collect();
                Tensor::D3(burn::tensor::Tensor::cat(inner, dim))
            }
            Tensor::D4(_) => {
                let inner: Vec<burn::tensor::Tensor<B, 4>> = tensors
                    .into_iter()
                    .map(|t| match t {
                        Tensor::D4(it) => it,
                        _ => panic!("cat: rank mismatch"),
                    })
                    .collect();
                Tensor::D4(burn::tensor::Tensor::cat(inner, dim))
            }
            Tensor::D1Int(_) => {
                let inner: Vec<burn::tensor::Tensor<B, 1, burn::tensor::Int>> = tensors
                    .into_iter()
                    .map(|t| match t {
                        Tensor::D1Int(it) => it,
                        _ => panic!("cat: rank mismatch"),
                    })
                    .collect();
                Tensor::D1Int(burn::tensor::Tensor::cat(inner, dim))
            }
            Tensor::D2Int(_) => {
                let inner: Vec<burn::tensor::Tensor<B, 2, burn::tensor::Int>> = tensors
                    .into_iter()
                    .map(|t| match t {
                        Tensor::D2Int(it) => it,
                        _ => panic!("cat: rank mismatch"),
                    })
                    .collect();
                Tensor::D2Int(burn::tensor::Tensor::cat(inner, dim))
            }
            Tensor::Tuple2(_, _) => panic!("cat: unsupported for Tuple2"),
        }
    }

    /// Reshape the tensor to a new shape.
    pub fn reshape(&self, shape: Vec<usize>) -> Self {
        let n_dims = shape.len();
        match self {
            Tensor::D1(t) => match n_dims {
                1 => Tensor::D1(t.clone().reshape([shape[0]])),
                2 => Tensor::D2(t.clone().reshape([shape[0], shape[1]])),
                3 => Tensor::D3(t.clone().reshape([shape[0], shape[1], shape[2]])),
                4 => Tensor::D4(t.clone().reshape([shape[0], shape[1], shape[2], shape[3]])),
                _ => panic!("reshape: unsupported rank"),
            },
            Tensor::D2(t) => match n_dims {
                1 => Tensor::D1(t.clone().reshape([shape[0]])),
                2 => Tensor::D2(t.clone().reshape([shape[0], shape[1]])),
                3 => Tensor::D3(t.clone().reshape([shape[0], shape[1], shape[2]])),
                4 => Tensor::D4(t.clone().reshape([shape[0], shape[1], shape[2], shape[3]])),
                _ => panic!("reshape: unsupported rank"),
            },
            Tensor::D3(t) => match n_dims {
                1 => Tensor::D1(t.clone().reshape([shape[0]])),
                2 => Tensor::D2(t.clone().reshape([shape[0], shape[1]])),
                3 => Tensor::D3(t.clone().reshape([shape[0], shape[1], shape[2]])),
                4 => Tensor::D4(t.clone().reshape([shape[0], shape[1], shape[2], shape[3]])),
                _ => panic!("reshape: unsupported rank"),
            },
            Tensor::D4(t) => match n_dims {
                1 => Tensor::D1(t.clone().reshape([shape[0]])),
                2 => Tensor::D2(t.clone().reshape([shape[0], shape[1]])),
                3 => Tensor::D3(t.clone().reshape([shape[0], shape[1], shape[2]])),
                4 => Tensor::D4(t.clone().reshape([shape[0], shape[1], shape[2], shape[3]])),
                _ => panic!("reshape: unsupported rank"),
            },
            Tensor::D1Int(t) => match n_dims {
                1 => Tensor::D1Int(t.clone().reshape([shape[0]])),
                2 => Tensor::D2Int(t.clone().reshape([shape[0], shape[1]])),
                _ => panic!("reshape: unsupported rank for Int"),
            },
            Tensor::D2Int(t) => match n_dims {
                1 => Tensor::D1Int(t.clone().reshape([shape[0]])),
                2 => Tensor::D2Int(t.clone().reshape([shape[0], shape[1]])),
                _ => panic!("reshape: unsupported rank for Int"),
            },
            Tensor::Tuple2(_, _) => panic!("reshape: unsupported for Tuple2"),
        }
    }

    /// Generate a 1D tensor with values from `start` to `end` (exclusive) with `step`.
    pub fn arange(start: f32, end: f32, step: f32, device: &B::Device) -> Self {
        let count = ((end - start) / step).ceil() as usize;
        let mut data = Vec::with_capacity(count);
        let mut val = start;
        for _ in 0..count {
            data.push(val);
            val += step;
        }
        Tensor::D1(burn::tensor::Tensor::<B, 1>::from_floats(
            data.as_slice(),
            device,
        ))
    }

    /// Generate a 1D tensor with `steps` values from `start` to `end` (inclusive).
    pub fn linspace(start: f32, end: f32, steps: usize, device: &B::Device) -> Self {
        if steps == 0 {
            return Tensor::D1(burn::tensor::Tensor::<B, 1>::from_floats([0.0; 0], device));
        }
        if steps == 1 {
            return Tensor::D1(burn::tensor::Tensor::<B, 1>::from_floats([start], device));
        }
        let step = (end - start) / (steps - 1) as f32;
        let mut data = Vec::with_capacity(steps);
        for i in 0..steps {
            data.push(start + i as f32 * step);
        }
        Tensor::D1(burn::tensor::Tensor::<B, 1>::from_floats(
            data.as_slice(),
            device,
        ))
    }

    /// Add a dimension of size 1 at the specified position.
    pub fn unsqueeze(&self, dim: usize) -> Self {
        match self {
            Tensor::D1(t) => match dim {
                0 => Tensor::D2(t.clone().unsqueeze_dim(0)),
                1 => Tensor::D2(t.clone().unsqueeze_dim(1)),
                _ => panic!("unsqueeze: invalid dim for D1"),
            },
            Tensor::D2(t) => match dim {
                0 => Tensor::D3(t.clone().unsqueeze_dim(0)),
                1 => Tensor::D3(t.clone().unsqueeze_dim(1)),
                2 => Tensor::D3(t.clone().unsqueeze_dim(2)),
                _ => panic!("unsqueeze: invalid dim for D2"),
            },
            Tensor::D3(t) => match dim {
                0 => Tensor::D4(t.clone().unsqueeze_dim(0)),
                1 => Tensor::D4(t.clone().unsqueeze_dim(1)),
                2 => Tensor::D4(t.clone().unsqueeze_dim(2)),
                3 => Tensor::D4(t.clone().unsqueeze_dim(3)),
                _ => panic!("unsqueeze: invalid dim for D3"),
            },
            Tensor::D4(_) => panic!("unsqueeze: unsupported rank or already at max rank"),
            Tensor::D1Int(t) => match dim {
                0 => Tensor::D2Int(t.clone().unsqueeze_dim(0)),
                1 => Tensor::D2Int(t.clone().unsqueeze_dim(1)),
                _ => panic!("unsqueeze: invalid dim for D1Int"),
            },
            Tensor::D2Int(_) => panic!("unsqueeze: unsupported rank or already at max rank"),
            Tensor::Tuple2(_, _) => panic!("unsqueeze: unsupported for Tuple2"),
        }
    }

    /// Stack tensors along a new dimension.
    pub fn stack(tensors: Vec<Self>, dim: usize) -> Self {
        let unsqueezed: Vec<Self> = tensors.into_iter().map(|t| t.unsqueeze(dim)).collect();
        Self::cat(unsqueezed, dim)
    }
}
