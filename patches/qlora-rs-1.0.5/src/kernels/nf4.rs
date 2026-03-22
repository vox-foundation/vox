//! NF4 (4-bit NormalFloat) GPU kernels using CubeCL.
//!
//! NF4 quantization uses a lookup table optimized for normally-distributed
//! weights, providing better accuracy than uniform 4-bit quantization.
//!
//! Reference: <https://arxiv.org/abs/2305.14314> (QLoRA paper)

use cubecl::prelude::*;

/// NF4 lookup table values (for reference - not used directly in kernels).
/// These are the 16 quantization levels optimized for N(0,1) distribution.
#[allow(dead_code)]
pub const NF4_TABLE: [f32; 16] = [
    -1.0, -0.6962, -0.5251, -0.3949, -0.2844, -0.1848, -0.0911, 0.0, 0.0796, 0.1609, 0.2461,
    0.3379, 0.4407, 0.5626, 0.7230, 1.0,
];

/// NF4 quantization boundaries (midpoints between consecutive NF4 values).
#[allow(dead_code)]
pub const NF4_BOUNDARIES: [f32; 15] = [
    -0.8481, -0.6107, -0.4600, -0.3397, -0.2346, -0.1380, -0.0456, 0.0398, 0.1203, 0.2035, 0.2920,
    0.3893, 0.5017, 0.6428, 0.8615,
];

/// Find the nearest NF4 index for a normalized value [-1, 1].
///
/// Uses a series of comparisons (effectively binary search) to find
/// the closest NF4 quantization level.
#[cube]
pub fn find_nearest_nf4<F: Float + CubeElement>(val: F) -> u32 {
    // Compare against boundaries to find the correct bin
    // Boundaries are midpoints between consecutive NF4 values
    // Using subtraction instead of negative literals for CubeCL compatibility

    // Boundary: -0.8481
    if val < F::new(0.0) - F::new(0.8481) {
        0u32.into()
    } else if val < F::new(0.0) - F::new(0.6107) {
        1u32.into()
    } else if val < F::new(0.0) - F::new(0.4600) {
        2u32.into()
    } else if val < F::new(0.0) - F::new(0.3397) {
        3u32.into()
    } else if val < F::new(0.0) - F::new(0.2346) {
        4u32.into()
    } else if val < F::new(0.0) - F::new(0.1380) {
        5u32.into()
    } else if val < F::new(0.0) - F::new(0.0456) {
        6u32.into()
    } else if val < F::new(0.0398) {
        7u32.into()
    } else if val < F::new(0.1203) {
        8u32.into()
    } else if val < F::new(0.2035) {
        9u32.into()
    } else if val < F::new(0.2920) {
        10u32.into()
    } else if val < F::new(0.3893) {
        11u32.into()
    } else if val < F::new(0.5017) {
        12u32.into()
    } else if val < F::new(0.6428) {
        13u32.into()
    } else if val < F::new(0.8615) {
        14u32.into()
    } else {
        15u32.into()
    }
}

/// NF4 lookup table function.
/// Returns the dequantized value for a given 4-bit index.
#[cube]
pub fn nf4_lookup<F: Float + CubeElement>(idx: u32) -> F {
    // Using subtraction for negative values
    if idx == 0u32 {
        F::new(0.0) - F::new(1.0) // -1.0
    } else if idx == 1u32 {
        F::new(0.0) - F::new(0.6962) // -0.6962
    } else if idx == 2u32 {
        F::new(0.0) - F::new(0.5251) // -0.5251
    } else if idx == 3u32 {
        F::new(0.0) - F::new(0.3949) // -0.3949
    } else if idx == 4u32 {
        F::new(0.0) - F::new(0.2844) // -0.2844
    } else if idx == 5u32 {
        F::new(0.0) - F::new(0.1848) // -0.1848
    } else if idx == 6u32 {
        F::new(0.0) - F::new(0.0911) // -0.0911
    } else if idx == 7u32 {
        F::new(0.0)
    } else if idx == 8u32 {
        F::new(0.0796)
    } else if idx == 9u32 {
        F::new(0.1609)
    } else if idx == 10u32 {
        F::new(0.2461)
    } else if idx == 11u32 {
        F::new(0.3379)
    } else if idx == 12u32 {
        F::new(0.4407)
    } else if idx == 13u32 {
        F::new(0.5626)
    } else if idx == 14u32 {
        F::new(0.7230)
    } else {
        F::new(1.0)
    }
}

/// NF4 Quantization kernel.
///
/// Converts f32/f16/bf16 values to 4-bit NF4 indices.
/// Eight 4-bit values are packed into each u32.
///
/// # Arguments
/// * `input` - Input tensor values (flattened)
/// * `scales` - Scale factors per block
/// * `output` - Output packed u32 values (8 NF4 indices per u32)
/// * `block_size` - Number of elements per quantization block
/// * `num_elements` - Total number of input elements
#[cube(launch)]
pub fn nf4_quantize_kernel<F: Float + CubeElement>(
    input: &Array<F>,
    scales: &Array<F>,
    output: &mut Array<u32>,
    #[comptime] block_size: u32,
    #[comptime] num_elements: u32,
) {
    // Each thread processes 8 input values and produces one packed u32
    let out_idx = ABSOLUTE_POS;
    let in_base = out_idx * 8usize;

    // Early exit if out of bounds
    if in_base >= (num_elements as usize) {
        terminate!();
    }

    // Determine which scale block this thread's values belong to
    let scale_idx = in_base / (block_size as usize);
    let scale = scales[scale_idx];

    // Avoid division by zero
    let inv_scale = if scale > F::new(1e-10) {
        F::new(1.0) / scale
    } else {
        F::new(1.0)
    };

    let mut packed: u32 = 0u32;
    let neg_one = F::new(0.0) - F::new(1.0);

    // Process 8 values and pack into one u32
    #[unroll]
    for i in 0usize..8usize {
        let val_idx = in_base + i;
        if val_idx < (num_elements as usize) {
            // Normalize value by scale
            let normalized = input[val_idx] * inv_scale;

            // Clamp to [-1, 1] range
            let clamped = if normalized < neg_one {
                neg_one
            } else if normalized > F::new(1.0) {
                F::new(1.0)
            } else {
                normalized
            };

            // Find nearest NF4 index
            let nf4_idx = find_nearest_nf4::<F>(clamped);

            // Pack 4-bit index into u32 (LSB first)
            packed = packed | (nf4_idx << ((i * 4usize) as u32));
        }
    }

    output[out_idx] = packed;
}

/// NF4 Dequantization kernel.
///
/// Converts packed 4-bit NF4 indices back to f32/f16/bf16 values.
///
/// # Arguments
/// * `input` - Packed u32 values containing 8 NF4 indices each
/// * `scales` - Scale factors per block
/// * `output` - Output dequantized values
/// * `block_size` - Number of elements per quantization block
/// * `num_elements` - Total number of output elements
#[cube(launch)]
pub fn nf4_dequantize_kernel<F: Float + CubeElement>(
    input: &Array<u32>,
    scales: &Array<F>,
    output: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] num_elements: u32,
) {
    let idx = ABSOLUTE_POS;

    if idx >= (num_elements as usize) {
        terminate!();
    }

    // Calculate which packed u32 contains this value
    let packed_idx = idx / 8usize;
    let sub_idx = idx % 8usize;

    // Extract the 4-bit NF4 index
    let packed = input[packed_idx];
    let nf4_idx = (packed >> ((sub_idx * 4usize) as u32)) & 0xFu32;

    // Lookup NF4 value
    let nf4_val: F = nf4_lookup::<F>(nf4_idx);

    // Apply scale factor
    let scale_idx = idx / (block_size as usize);
    let scale = scales[scale_idx];

    output[idx] = nf4_val * scale;
}

/// Compute absmax scale for each block.
///
/// This kernel computes the absolute maximum value in each block,
/// which is used as the scale factor for quantization.
#[cube(launch)]
pub fn compute_scales_kernel<F: Float + CubeElement>(
    input: &Array<F>,
    scales: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] num_blocks: u32,
) {
    let block_idx = ABSOLUTE_POS;

    if block_idx >= (num_blocks as usize) {
        terminate!();
    }

    let start = block_idx * (block_size as usize);
    let end = start + (block_size as usize);

    // Find absmax in this block
    let mut absmax = F::new(0.0);

    for i in start..end {
        let val = input[i];
        // Manual abs: if val < 0 then -val else val
        let abs_val = if val < F::new(0.0) {
            F::new(0.0) - val
        } else {
            val
        };
        if abs_val > absmax {
            absmax = abs_val;
        }
    }

    // Ensure non-zero scale
    scales[block_idx] = if absmax > F::new(1e-10) {
        absmax
    } else {
        F::new(1.0)
    };
}

/// Double quantization kernel for scales.
///
/// Quantizes the block scales themselves to 8-bit integers,
/// organized in superblocks for additional compression.
///
/// # Arguments
/// * `scales` - Input scale factors
/// * `quantized_scales` - Output quantized scales (4 packed per u32)
/// * `scale_of_scales` - Scale factor for each superblock
/// * `num_blocks` - Total number of blocks
/// * `blocks_per_superblock` - Number of blocks in each superblock
#[cube(launch)]
pub fn double_quantize_scales_kernel<F: Float + CubeElement>(
    scales: &Array<F>,
    quantized_scales: &mut Array<u32>,
    scale_of_scales: &mut Array<F>,
    #[comptime] num_blocks: u32,
    #[comptime] blocks_per_superblock: u32,
) {
    let superblock_idx = ABSOLUTE_POS;
    let num_superblocks_u32 = (num_blocks + blocks_per_superblock - 1u32) / blocks_per_superblock;

    if superblock_idx >= (num_superblocks_u32 as usize) {
        terminate!();
    }

    let start = superblock_idx * (blocks_per_superblock as usize);

    // Find absmax of scales in this superblock using comptime bound
    let mut absmax = F::new(0.0);
    for i in 0u32..blocks_per_superblock {
        let idx = start + (i as usize);
        if idx < (num_blocks as usize) {
            let val = scales[idx];
            let abs_val = if val < F::new(0.0) {
                F::new(0.0) - val
            } else {
                val
            };
            if abs_val > absmax {
                absmax = abs_val;
            }
        }
    }

    // Store scale of scales (ensure non-zero)
    let sos = if absmax > F::new(1e-10) {
        absmax
    } else {
        F::new(1.0)
    };
    scale_of_scales[superblock_idx] = sos;

    // Quantize scales to 8-bit (symmetric around 0, range [-127, 127])
    let inv_sos = F::new(127.0) / sos;

    // Calculate output_base using u32 arithmetic then cast
    let packed_blocks_u32 = (blocks_per_superblock + 3u32) / 4u32;
    let output_base = superblock_idx * (packed_blocks_u32 as usize);

    // Pack 4 quantized scales per u32 using comptime bound
    for p in 0u32..packed_blocks_u32 {
        let mut packed: u32 = 0u32;

        #[unroll]
        for j in 0u32..4u32 {
            let idx = start + ((p * 4u32 + j) as usize);
            if idx < (num_blocks as usize) {
                // Quantize to [-127, 127]
                let q_float = scales[idx] * inv_sos;

                // Clamp to valid range
                let neg_127 = F::new(0.0) - F::new(127.0);
                let clamped = if q_float < neg_127 {
                    neg_127
                } else if q_float > F::new(127.0) {
                    F::new(127.0)
                } else {
                    q_float
                };

                // Round and convert to unsigned [0, 254]
                // Add 0.5 for positive, subtract 0.5 for negative, then truncate
                let rounded = if clamped >= F::new(0.0) {
                    clamped + F::new(0.5)
                } else {
                    clamped - F::new(0.5)
                };
                // Convert to i32 first (truncates), then shift to unsigned
                let q_i32 = i32::cast_from(rounded);
                let q_u8 = (q_i32 + 127) as u32;

                packed = packed | ((q_u8 & 0xFFu32) << (j * 8u32));
            }
        }

        quantized_scales[(output_base + (p as usize))] = packed;
    }
}

/// Dequantize double-quantized scales.
///
/// Reconstructs the original scales from their quantized representation.
#[cube(launch)]
pub fn double_dequantize_scales_kernel<F: Float + CubeElement>(
    quantized_scales: &Array<u32>,
    scale_of_scales: &Array<F>,
    scales: &mut Array<F>,
    #[comptime] num_blocks: u32,
    #[comptime] blocks_per_superblock: u32,
) {
    let block_idx = ABSOLUTE_POS;

    if block_idx >= (num_blocks as usize) {
        terminate!();
    }

    // Determine which superblock and position within it
    let superblock_idx = block_idx / (blocks_per_superblock as usize);
    let local_idx = block_idx % (blocks_per_superblock as usize);

    // Get scale of scales for this superblock
    let sos = scale_of_scales[superblock_idx];

    // Find packed u32 containing this scale
    let packed_per_superblock = ((blocks_per_superblock as usize) + 3usize) / 4usize;
    let packed_idx = superblock_idx * packed_per_superblock + local_idx / 4usize;
    let sub_idx = local_idx % 4usize;

    let packed = quantized_scales[packed_idx];
    let q_u8 = (packed >> ((sub_idx * 8usize) as u32)) & 0xFFu32;

    // Dequantize: convert from [0, 254] back to [-127, 127], then scale
    let q_signed = (q_u8 as i32) - 127;
    let dequantized = F::cast_from(q_signed) * sos / F::new(127.0);

    scales[block_idx] = dequantized;
}

/// NF4 dequantization with double-quantized scales.
///
/// Combines scale dequantization and NF4 dequantization in a single pass.
#[cube(launch)]
pub fn nf4_dequantize_double_quant_kernel<F: Float + CubeElement>(
    input: &Array<u32>,
    quantized_scales: &Array<u32>,
    scale_of_scales: &Array<F>,
    output: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] blocks_per_superblock: u32,
    #[comptime] num_elements: u32,
) {
    let idx = ABSOLUTE_POS;

    if idx >= (num_elements as usize) {
        terminate!();
    }

    // Get NF4 value
    let packed_idx = idx / 8usize;
    let sub_idx = idx % 8usize;
    let packed = input[packed_idx];
    let nf4_idx = (packed >> ((sub_idx * 4usize) as u32)) & 0xFu32;
    let nf4_val: F = nf4_lookup::<F>(nf4_idx);

    // Get dequantized scale
    let block_idx = idx / (block_size as usize);
    let superblock_idx = block_idx / (blocks_per_superblock as usize);
    let local_block_idx = block_idx % (blocks_per_superblock as usize);

    let sos = scale_of_scales[superblock_idx];
    let packed_per_superblock = ((blocks_per_superblock as usize) + 3usize) / 4usize;
    let scale_packed_idx_u32 = superblock_idx * packed_per_superblock + local_block_idx / 4usize;
    let scale_sub_idx_u32 = local_block_idx % 4usize;

    let scale_packed = quantized_scales[scale_packed_idx_u32];
    let q_u8 = (scale_packed >> ((scale_sub_idx_u32 * 8usize) as u32)) & 0xFFu32;
    let q_signed = (q_u8 as i32) - 127;
    let scale = F::cast_from(q_signed) * sos / F::new(127.0);

    // Apply scale to NF4 value
    output[idx] = nf4_val * scale;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nf4_boundaries() {
        // Verify that boundaries are correct midpoints
        for i in 0..15 {
            let midpoint = (NF4_TABLE[i] + NF4_TABLE[i + 1]) / 2.0;
            assert!(
                (NF4_BOUNDARIES[i] - midpoint).abs() < 0.001,
                "Boundary {} mismatch: {} vs {}",
                i,
                NF4_BOUNDARIES[i],
                midpoint
            );
        }
    }
}
