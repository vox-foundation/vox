//! Fused GPU kernels for QLoRA operations using CubeCL.
//!
//! These kernels combine multiple operations to reduce memory bandwidth:
//! - Fused NF4 dequantization + matrix multiplication
//! - Fused NF4 dequantization + LoRA forward pass
//!
//! Fusing operations is CRITICAL for performance because:
//! 1. Avoids materializing the full dequantized weight matrix in memory
//! 2. Reduces global memory bandwidth (the main bottleneck on GPUs)
//! 3. Keeps intermediate values in fast shared memory and registers

use cubecl::prelude::*;

use super::nf4::nf4_lookup;

/// Tile size for matrix multiplication.
const TILE_SIZE: u32 = 32;

/// Fused NF4 dequantize + matrix multiplication kernel.
///
/// Computes: Y = X @ dequant(W_nf4)
///
/// Instead of first dequantizing W to full precision and then doing matmul,
/// we dequantize on-the-fly during the tiled matrix multiplication.
/// This saves ~4x memory bandwidth for the weight matrix.
///
/// # Arguments
/// * `x` - Input matrix [M, K] in row-major order
/// * `w_packed` - NF4 weights [(K+7)/8, N] packed (8 values per u32)
/// * `scales` - Scale factors for weight blocks
/// * `out` - Output matrix [M, N] in row-major order
/// * `m` - Number of rows in X
/// * `n` - Number of columns in W (and output)
/// * `k` - Shared dimension (columns of X, rows of W)
/// * `block_size` - Quantization block size
#[cube(launch)]
pub fn fused_nf4_matmul_kernel<F: Float + CubeElement>(
    x: &Array<F>,
    w_packed: &Array<u32>,
    scales: &Array<F>,
    out: &mut Array<F>,
    #[comptime] m: u32,
    #[comptime] n: u32,
    #[comptime] k: u32,
    #[comptime] block_size: u32,
) {
    // Calculate output position
    let row = CUBE_POS_Y * TILE_SIZE + UNIT_POS_Y;
    let col = CUBE_POS_X * TILE_SIZE + UNIT_POS_X;

    // Shared memory for tiles
    let mut x_tile = SharedMemory::<F>::new((TILE_SIZE * TILE_SIZE) as usize);
    let mut w_tile = SharedMemory::<F>::new((TILE_SIZE * TILE_SIZE) as usize);

    // Accumulator for dot product
    let mut acc = F::new(0.0);

    // Number of tiles along K dimension
    let num_k_tiles = (k + TILE_SIZE - 1u32) / TILE_SIZE;

    // Iterate over tiles in K dimension
    for k_tile in 0u32..num_k_tiles {
        let k_base = k_tile * TILE_SIZE;

        // Collaborative load of X tile
        let x_row = CUBE_POS_Y * TILE_SIZE + UNIT_POS_Y;
        let x_col = k_base + UNIT_POS_X;

        let x_val = if x_row < m as u32 && x_col < k as u32 {
            x[(x_row * k + x_col) as usize]
        } else {
            F::new(0.0)
        };
        x_tile[(UNIT_POS_Y * TILE_SIZE + UNIT_POS_X) as usize] = x_val;

        // Collaborative load and dequantize W tile
        let w_row = k_base + UNIT_POS_Y;
        let w_col = CUBE_POS_X * TILE_SIZE + UNIT_POS_X;

        let w_val = if w_row < k as u32 && w_col < n as u32 {
            // Calculate packed index
            let packed_k_idx = w_row / 8u32;
            let sub_idx = w_row % 8u32;

            // Packed array is stored as [K/8, N]
            let packed_idx = packed_k_idx * n + w_col;
            let packed = w_packed[packed_idx as usize];

            // Extract 4-bit NF4 index
            let nf4_idx = (packed >> (sub_idx * 4u32)) & 0xFu32;
            let nf4_val: F = nf4_lookup::<F>(nf4_idx);

            // Get scale for this weight
            let weight_linear_idx = w_row * n + w_col;
            let scale_idx = weight_linear_idx / block_size;
            let scale = scales[scale_idx as usize];

            nf4_val * scale
        } else {
            F::new(0.0)
        };
        w_tile[(UNIT_POS_Y * TILE_SIZE + UNIT_POS_X) as usize] = w_val;

        // Synchronize to ensure tiles are fully loaded
        sync_cube();

        // Compute partial dot product for this tile
        if row < m as u32 && col < n as u32 {
            #[unroll]
            for i in 0u32..TILE_SIZE {
                acc = acc
                    + x_tile[(UNIT_POS_Y * TILE_SIZE + i) as usize]
                        * w_tile[(i * TILE_SIZE + UNIT_POS_X) as usize];
            }
        }

        // Synchronize before loading next tile
        sync_cube();
    }

    // Write output
    if row < m as u32 && col < n as u32 {
        out[(row * n + col) as usize] = acc;
    }
}

/// Fused NF4 dequantize + batched matrix multiplication.
///
/// Handles batched inputs: Y[b] = X[b] @ dequant(W)
/// Where W is shared across all batches.
#[cube(launch)]
pub fn fused_nf4_batched_matmul_kernel<F: Float + CubeElement>(
    x: &Array<F>,
    w_packed: &Array<u32>,
    scales: &Array<F>,
    out: &mut Array<F>,
    #[comptime] batch: u32,
    #[comptime] m: u32,
    #[comptime] n: u32,
    #[comptime] k: u32,
    #[comptime] block_size: u32,
) {
    let batch_idx = CUBE_POS_Z;
    let row = CUBE_POS_Y * TILE_SIZE + UNIT_POS_Y;
    let col = CUBE_POS_X * TILE_SIZE + UNIT_POS_X;

    if batch_idx >= batch {
        terminate!();
    }

    let mut x_tile = SharedMemory::<F>::new((TILE_SIZE * TILE_SIZE) as usize);
    let mut w_tile = SharedMemory::<F>::new((TILE_SIZE * TILE_SIZE) as usize);

    let mut acc = F::new(0.0);
    let num_k_tiles = (k + TILE_SIZE - 1u32) / TILE_SIZE;

    let x_batch_offset = batch_idx * m * k;
    let out_batch_offset = batch_idx * m * n;

    for k_tile in 0u32..num_k_tiles {
        let k_base = k_tile * TILE_SIZE;

        let x_row = CUBE_POS_Y * TILE_SIZE + UNIT_POS_Y;
        let x_col = k_base + UNIT_POS_X;

        let x_val = if x_row < m as u32 && x_col < k as u32 {
            x[(x_batch_offset + x_row * k + x_col) as usize]
        } else {
            F::new(0.0)
        };
        x_tile[(UNIT_POS_Y * TILE_SIZE + UNIT_POS_X) as usize] = x_val;

        let w_row = k_base + UNIT_POS_Y;
        let w_col = CUBE_POS_X * TILE_SIZE + UNIT_POS_X;

        let w_val = if w_row < k as u32 && w_col < n as u32 {
            let packed_k_idx = w_row / 8u32;
            let sub_idx = w_row % 8u32;
            let packed_idx = packed_k_idx * n + w_col;
            let packed = w_packed[packed_idx as usize];
            let nf4_idx = (packed >> (sub_idx * 4u32)) & 0xFu32;
            let nf4_val: F = nf4_lookup::<F>(nf4_idx);

            let weight_linear_idx = w_row * n + w_col;
            let scale_idx = weight_linear_idx / block_size;
            let scale = scales[scale_idx as usize];

            nf4_val * scale
        } else {
            F::new(0.0)
        };
        w_tile[(UNIT_POS_Y * TILE_SIZE + UNIT_POS_X) as usize] = w_val;

        sync_cube();

        if row < m as u32 && col < n as u32 {
            #[unroll]
            for i in 0u32..TILE_SIZE {
                acc = acc
                    + x_tile[(UNIT_POS_Y * TILE_SIZE + i) as usize]
                        * w_tile[(i * TILE_SIZE + UNIT_POS_X) as usize];
            }
        }

        sync_cube();
    }

    if row < m as u32 && col < n as u32 {
        out[(out_batch_offset + row * n + col) as usize] = acc;
    }
}

/// Fused NF4 dequant + LoRA forward kernel.
///
/// Computes: Y = X @ dequant(W_base) + alpha/r * (X @ A) @ B
///
/// This is the complete QLoRA forward pass:
/// - W_base: NF4 quantized base model weights
/// - A, B: Full-precision LoRA adapter matrices
/// - alpha: LoRA scaling factor
/// - r: LoRA rank
#[cube(launch)]
pub fn fused_nf4_lora_forward_kernel<F: Float + CubeElement>(
    x: &Array<F>,
    w_packed: &Array<u32>,
    w_scales: &Array<F>,
    lora_a: &Array<F>,
    lora_b: &Array<F>,
    out: &mut Array<F>,
    #[comptime] batch_seq: u32,
    #[comptime] in_dim: u32,
    #[comptime] out_dim: u32,
    #[comptime] rank: u32,
    #[comptime] block_size: u32,
) {
    // Use 16x16 tiles for LoRA kernel
    let row = CUBE_POS_Y * 16u32 + UNIT_POS_Y;
    let col = CUBE_POS_X * 16u32 + UNIT_POS_X;

    if row >= batch_seq as u32 || col >= out_dim as u32 {
        terminate!();
    }

    // Compute base output: X @ dequant(W)
    let mut base_acc = F::new(0.0);
    for i in 0u32..in_dim {
        // Dequantize weight on the fly
        let packed_k_idx = i / 8u32;
        let sub_idx = i % 8u32;
        let packed_idx = packed_k_idx * out_dim + col;
        let packed = w_packed[packed_idx as usize];
        let nf4_idx = (packed >> (sub_idx * 4u32)) & 0xFu32;
        let nf4_val: F = nf4_lookup::<F>(nf4_idx);

        let weight_idx = i * out_dim + col;
        let scale_idx = weight_idx / block_size;
        let w_val = nf4_val * w_scales[scale_idx as usize];

        base_acc = base_acc + x[(row * in_dim + i) as usize] * w_val;
    }

    // Compute LoRA contribution: (X @ A) @ B
    // First compute x @ A -> hidden [rank]
    let mut lora_out = F::new(0.0);

    for r in 0u32..rank {
        // Compute x[row] @ A[:, r]
        let mut hidden = F::new(0.0);
        for i in 0u32..in_dim {
            hidden = hidden + x[(row * in_dim + i) as usize] * lora_a[(i * rank + r) as usize];
        }
        // Multiply by B[r, col]
        lora_out = lora_out + hidden * lora_b[(r * out_dim + col) as usize];
    }

    // Combine: base + lora (scaling is applied externally or via alpha param)
    out[(row * out_dim + col) as usize] = base_acc + lora_out;
}

/// Fused NF4 dequant + bias + activation kernel.
///
/// Computes: Y = activation(X @ dequant(W) + bias)
/// activation: 0=none, 1=relu, 2=gelu, 3=silu
#[cube(launch)]
pub fn fused_nf4_matmul_bias_act_kernel<F: Float + CubeElement>(
    x: &Array<F>,
    w_packed: &Array<u32>,
    scales: &Array<F>,
    bias: &Array<F>,
    out: &mut Array<F>,
    #[comptime] m: u32,
    #[comptime] n: u32,
    #[comptime] k: u32,
    #[comptime] block_size: u32,
    #[comptime] activation: u32,
) {
    let row = CUBE_POS_Y * TILE_SIZE + UNIT_POS_Y;
    let col = CUBE_POS_X * TILE_SIZE + UNIT_POS_X;

    let mut x_tile = SharedMemory::<F>::new((TILE_SIZE * TILE_SIZE) as usize);
    let mut w_tile = SharedMemory::<F>::new((TILE_SIZE * TILE_SIZE) as usize);

    let mut acc = F::new(0.0);
    let num_k_tiles = (k + TILE_SIZE - 1u32) / TILE_SIZE;

    for k_tile in 0u32..num_k_tiles {
        let k_base = k_tile * TILE_SIZE;

        let x_row = CUBE_POS_Y * TILE_SIZE + UNIT_POS_Y;
        let x_col = k_base + UNIT_POS_X;
        x_tile[(UNIT_POS_Y * TILE_SIZE + UNIT_POS_X) as usize] =
            if x_row < m as u32 && x_col < k as u32 {
                x[(x_row * k + x_col) as usize]
            } else {
                F::new(0.0)
            };

        let w_row = k_base + UNIT_POS_Y;
        let w_col = CUBE_POS_X * TILE_SIZE + UNIT_POS_X;
        w_tile[(UNIT_POS_Y * TILE_SIZE + UNIT_POS_X) as usize] =
            if w_row < k as u32 && w_col < n as u32 {
                let packed_k_idx = w_row / 8u32;
                let sub_idx = w_row % 8u32;
                let packed_idx = packed_k_idx * n + w_col;
                let packed = w_packed[packed_idx as usize];
                let nf4_idx = (packed >> (sub_idx * 4u32)) & 0xFu32;
                let nf4_val: F = nf4_lookup::<F>(nf4_idx);
                let weight_idx = w_row * n + w_col;
                let scale_idx = weight_idx / block_size;
                nf4_val * scales[scale_idx as usize]
            } else {
                F::new(0.0)
            };

        sync_cube();

        if row < m as u32 && col < n as u32 {
            #[unroll]
            for i in 0u32..TILE_SIZE {
                acc = acc
                    + x_tile[(UNIT_POS_Y * TILE_SIZE + i) as usize]
                        * w_tile[(i * TILE_SIZE + UNIT_POS_X) as usize];
            }
        }

        sync_cube();
    }

    if row < m as u32 && col < n as u32 {
        // Add bias
        let biased = acc + bias[col as usize];

        // Apply activation
        let activated = if activation == 0u32 {
            biased
        } else if activation == 1u32 {
            // ReLU
            if biased > F::new(0.0) {
                biased
            } else {
                F::new(0.0)
            }
        } else if activation == 2u32 {
            // GELU approximation
            let sigmoid_input = biased * F::new(1.702);
            let neg_sig = F::new(0.0) - sigmoid_input;
            let sigmoid = F::new(1.0) / (F::new(1.0) + F::exp(neg_sig));
            biased * sigmoid
        } else {
            // SiLU (activation == 3)
            let neg_biased = F::new(0.0) - biased;
            let sigmoid = F::new(1.0) / (F::new(1.0) + F::exp(neg_biased));
            biased * sigmoid
        };

        out[(row * n + col) as usize] = activated;
    }
}

/// Fused NF4 dequant + vector addition kernel.
///
/// Computes: Y = dequant(W_nf4) + X
#[cube(launch)]
pub fn fused_nf4_add_kernel<F: Float + CubeElement>(
    w_packed: &Array<u32>,
    scales: &Array<F>,
    x: &Array<F>,
    out: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] num_elements: u32,
) {
    let idx = ABSOLUTE_POS;

    if idx >= (num_elements as usize) {
        terminate!();
    }

    let packed_idx = idx / 8usize;
    let sub_idx = idx % 8usize;
    let packed = w_packed[packed_idx];
    let nf4_idx = (packed >> ((sub_idx * 4usize) as u32)) & 0xFu32;
    let nf4_val: F = nf4_lookup::<F>(nf4_idx);
    let block_sz = block_size as usize;
    let scale_idx = idx / block_sz;
    let w_val = nf4_val * scales[scale_idx];
    out[idx] = w_val + x[idx];
}

/// Simple dequantize + matmul kernel without tiling (for small matrices).
///
/// Less efficient than tiled version but simpler and works for any size.
#[cube(launch)]
pub fn simple_nf4_matmul_kernel<F: Float + CubeElement>(
    x: &Array<F>,
    w_packed: &Array<u32>,
    scales: &Array<F>,
    out: &mut Array<F>,
    #[comptime] m: u32,
    #[comptime] n: u32,
    #[comptime] k: u32,
    #[comptime] block_size: u32,
) {
    let row = CUBE_POS_Y;
    let col = CUBE_POS_X;

    if row >= m as u32 || col >= n as u32 {
        terminate!();
    }

    let mut acc = F::new(0.0);

    for i in 0u32..k {
        // Get input value
        let x_val = x[(row * k + i) as usize];

        // Dequantize weight
        let packed_k_idx = i / 8u32;
        let sub_idx = i % 8u32;
        let packed_idx = packed_k_idx * n + col;
        let packed = w_packed[packed_idx as usize];
        let nf4_idx = (packed >> (sub_idx * 4u32)) & 0xFu32;
        let nf4_val: F = nf4_lookup::<F>(nf4_idx);

        let weight_idx = i * n + col;
        let scale_idx = weight_idx / block_size;
        let w_val = nf4_val * scales[scale_idx as usize];

        acc = acc + x_val * w_val;
    }

    out[(row * n + col) as usize] = acc;
}
