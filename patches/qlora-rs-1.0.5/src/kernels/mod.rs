//! GPU kernels for QLoRA operations using CubeCL.
//!
//! This module provides CUDA-accelerated implementations of:
//! - NF4 quantization and dequantization
//! - FP4 quantization and dequantization
//! - Fused dequantization + matrix multiplication
//! - Fused NF4 + LoRA forward pass
//!
//! These kernels are critical for achieving high performance in QLoRA
//! inference and training workloads.
//!
//! # Usage
//!
//! The kernels are designed to be launched via CubeCL's kernel launch system.
//! Parameters marked with `#[comptime]` are compile-time constants that must
//! be known when the kernel is compiled.
//!
//! # Example
//!
//! ```ignore
//! use qlora_rs::kernels::nf4::nf4_quantize_kernel;
//! use cubecl::prelude::*;
//!
//! // Launch the quantization kernel
//! unsafe {
//!     nf4_quantize_kernel::launch_unchecked::<f32, CudaRuntime>(
//!         &client,
//!         cube_count,
//!         cube_dim,
//!         input_arg,
//!         scales_arg,
//!         output_arg,
//!         block_size,   // comptime
//!         num_elements, // comptime
//!     );
//! }
//! ```

#[cfg(feature = "cuda")]
pub mod fused;

#[cfg(feature = "cuda")]
pub mod fp4;

#[cfg(feature = "cuda")]
pub mod nf4;

// Re-export key items for convenience
#[cfg(feature = "cuda")]
pub use fused::{
    fused_nf4_add_kernel, fused_nf4_batched_matmul_kernel, fused_nf4_lora_forward_kernel,
    fused_nf4_matmul_bias_act_kernel, fused_nf4_matmul_kernel, simple_nf4_matmul_kernel,
};

#[cfg(feature = "cuda")]
pub use fp4::{
    fp4_compute_scale_zeropoint_kernel, fp4_compute_scales_kernel,
    fp4_dequantize_asymmetric_kernel, fp4_dequantize_kernel, fp4_quantize_asymmetric_kernel,
    fp4_quantize_kernel,
};

#[cfg(feature = "cuda")]
pub use nf4::{
    compute_scales_kernel, double_dequantize_scales_kernel, double_quantize_scales_kernel,
    nf4_dequantize_double_quant_kernel, nf4_dequantize_kernel, nf4_quantize_kernel,
};

/// Block size used for GPU quantization operations (default).
pub const DEFAULT_GPU_BLOCK_SIZE: u32 = 64;

/// Number of values packed into a single u32 (8 x 4-bit values).
pub const VALUES_PER_U32: u32 = 8;

/// Superblock size for double quantization (number of blocks per superblock).
pub const SUPERBLOCK_SIZE: u32 = 256;

/// Tile size for matrix multiplication kernels.
pub const MATMUL_TILE_SIZE: u32 = 32;
