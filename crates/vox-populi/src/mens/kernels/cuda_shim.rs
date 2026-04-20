/// CUDA kernel shimming for Mens training.
/// This allows loading pre-compiled PTX artifacts without requiring nvcc at runtime.

pub const AFFINE_PTX: &str = include_str!("affine.ptx");
pub const BINARY_PTX: &str = include_str!("binary.ptx");
pub const CAST_PTX: &str = include_str!("cast.ptx");
pub const CONV_PTX: &str = include_str!("conv.ptx");
pub const FILL_PTX: &str = include_str!("fill.ptx");
pub const INDEXING_PTX: &str = include_str!("indexing.ptx");
pub const QUANTIZED_PTX: &str = include_str!("quantized.ptx");
pub const REDUCE_PTX: &str = include_str!("reduce.ptx");
pub const SORT_PTX: &str = include_str!("sort.ptx");
pub const TERNARY_PTX: &str = include_str!("ternary.ptx");
pub const UNARY_PTX: &str = include_str!("unary.ptx");

#[cfg(feature = "mens-candle-qlora")]
pub fn load_kernels(_device: &candle_core::CudaDevice) -> anyhow::Result<()> {
    // In candle 0.9.x, kernels are statically compiled and loaded automatically
    // via bindgen_cuda. This shim is no longer necessary.
    Ok(())
}
