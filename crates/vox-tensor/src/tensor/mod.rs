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
    Tuple2(Box<Tensor<B>>, Box<Tensor<B>>),
}

// Implement GC barrier crossing capability. Tensors are inherently thread-safe
// handles backed by WGPU/Burn, so crossing a mailbox boundary only requires
// cloning the reference-counted handle rather than deep-copying matrix bytes.
impl<B: Backend> vox_runtime::gc::DeepCloneToOwned for Tensor<B> {
    type Owned = Self;

    fn deep_clone_to_owned(&self) -> Self::Owned {
        self.clone()
    }
}

// Treat Tensors as Unmanaged Compute Resources inside the GC engine.
// When the ActorHeap drops the `Gc<Tensor<B>>` pointer, this hook ensures
// we explicitly drop the Rust handle so the WGPU backend releases the memory.
impl<B: Backend> vox_runtime::gc::GcDrop for Tensor<B> {
    fn gc_drop(&mut self) {
        // Safe to let Rust's default drop architecture handle the internal
        // `burn::tensor` backend decrements once invoked.
        unsafe { std::ptr::drop_in_place(self) };
    }
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
