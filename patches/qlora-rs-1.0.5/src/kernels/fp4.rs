//! FP4 (4-bit Floating Point) GPU kernels using CubeCL.
//!
//! FP4 uses a simpler uniform quantization scheme compared to NF4.
//! It's faster to compute but may have slightly lower accuracy for
//! normally-distributed weights.

use cubecl::prelude::*;

/// FP4 lookup table values.
#[allow(dead_code)]
pub const FP4_TABLE: [f32; 16] = [
    -1.0, -0.75, -0.5, -0.375, -0.25, -0.125, -0.0625, 0.0, 0.0625, 0.125, 0.25, 0.375, 0.5, 0.75,
    1.0, 0.0,
];

/// FP4 boundaries for quantization.
#[allow(dead_code)]
pub const FP4_BOUNDARIES: [f32; 14] = [
    -0.875, -0.625, -0.4375, -0.3125, -0.1875, -0.0938, -0.0313, 0.0313, 0.0938, 0.1875, 0.3125,
    0.4375, 0.625, 0.875,
];

/// Find the nearest FP4 index for a normalized value [-1, 1].
#[cube]
pub fn find_nearest_fp4<F: Float + CubeElement>(val: F) -> u32 {
    if val < F::new(0.0) - F::new(0.875) {
        0u32.into()
    } else if val < F::new(0.0) - F::new(0.625) {
        1u32.into()
    } else if val < F::new(0.0) - F::new(0.4375) {
        2u32.into()
    } else if val < F::new(0.0) - F::new(0.3125) {
        3u32.into()
    } else if val < F::new(0.0) - F::new(0.1875) {
        4u32.into()
    } else if val < F::new(0.0) - F::new(0.0938) {
        5u32.into()
    } else if val < F::new(0.0) - F::new(0.0313) {
        6u32.into()
    } else if val < F::new(0.0313) {
        7u32.into()
    } else if val < F::new(0.0938) {
        8u32.into()
    } else if val < F::new(0.1875) {
        9u32.into()
    } else if val < F::new(0.3125) {
        10u32.into()
    } else if val < F::new(0.4375) {
        11u32.into()
    } else if val < F::new(0.625) {
        12u32.into()
    } else if val < F::new(0.875) {
        13u32.into()
    } else {
        14u32.into()
    }
}

/// FP4 lookup table function.
#[cube]
pub fn fp4_lookup<F: Float + CubeElement>(idx: u32) -> F {
    if idx == 0u32 {
        F::new(0.0) - F::new(1.0)
    } else if idx == 1u32 {
        F::new(0.0) - F::new(0.75)
    } else if idx == 2u32 {
        F::new(0.0) - F::new(0.5)
    } else if idx == 3u32 {
        F::new(0.0) - F::new(0.375)
    } else if idx == 4u32 {
        F::new(0.0) - F::new(0.25)
    } else if idx == 5u32 {
        F::new(0.0) - F::new(0.125)
    } else if idx == 6u32 {
        F::new(0.0) - F::new(0.0625)
    } else if idx == 7u32 {
        F::new(0.0)
    } else if idx == 8u32 {
        F::new(0.0625)
    } else if idx == 9u32 {
        F::new(0.125)
    } else if idx == 10u32 {
        F::new(0.25)
    } else if idx == 11u32 {
        F::new(0.375)
    } else if idx == 12u32 {
        F::new(0.5)
    } else if idx == 13u32 {
        F::new(0.75)
    } else if idx == 14u32 {
        F::new(1.0)
    } else {
        F::new(0.0)
    }
}

/// FP4 Quantization kernel.
#[cube(launch)]
pub fn fp4_quantize_kernel<F: Float + CubeElement>(
    input: &Array<F>,
    scales: &Array<F>,
    output: &mut Array<u32>,
    #[comptime] block_size: u32,
    #[comptime] num_elements: u32,
) {
    let out_idx = ABSOLUTE_POS;
    let in_base = out_idx * 8usize;

    if in_base >= num_elements as usize {
        terminate!();
    }

    let scale_idx = in_base / (block_size as usize);
    let scale = scales[scale_idx];
    let inv_scale = if scale > F::new(1e-10) {
        F::new(1.0) / scale
    } else {
        F::new(1.0)
    };

    let mut packed: u32 = 0u32;
    let neg_one = F::new(0.0) - F::new(1.0);

    #[unroll]
    for i in 0usize..8usize {
        let val_idx = in_base + i;
        if val_idx < num_elements as usize {
            let normalized = input[val_idx] * inv_scale;
            let clamped = if normalized < neg_one {
                neg_one
            } else if normalized > F::new(1.0) {
                F::new(1.0)
            } else {
                normalized
            };

            let fp4_idx = find_nearest_fp4::<F>(clamped);
            packed = packed | (fp4_idx << (i as u32 * 4u32));
        }
    }

    output[out_idx] = packed;
}

/// FP4 Dequantization kernel.
#[cube(launch)]
pub fn fp4_dequantize_kernel<F: Float + CubeElement>(
    input: &Array<u32>,
    scales: &Array<F>,
    output: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] num_elements: u32,
) {
    let idx = ABSOLUTE_POS;

    if idx >= num_elements as usize {
        terminate!();
    }

    let packed_idx = idx / 8usize;
    let sub_idx = idx % 8usize;

    let packed = input[packed_idx];
    let fp4_idx = (packed >> (sub_idx as u32 * 4u32)) & 0xFu32;

    let fp4_val: F = fp4_lookup::<F>(fp4_idx);

    let scale_idx = idx / (block_size as usize);
    let scale = scales[scale_idx];

    output[idx] = fp4_val * scale;
}

/// Compute absmax scales for FP4 quantization.
#[cube(launch)]
pub fn fp4_compute_scales_kernel<F: Float + CubeElement>(
    input: &Array<F>,
    scales: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] num_blocks: u32,
) {
    let block_idx = ABSOLUTE_POS;

    if block_idx >= num_blocks as usize {
        terminate!();
    }

    let start = block_idx * (block_size as usize);
    let end = start + (block_size as usize);

    let mut absmax = F::new(0.0);
    for i in start..end {
        let val = input[i];
        let abs_val = if val < F::new(0.0) {
            F::new(0.0) - val
        } else {
            val
        };
        if abs_val > absmax {
            absmax = abs_val;
        }
    }

    scales[block_idx] = if absmax > F::new(1e-10) {
        absmax
    } else {
        F::new(1.0)
    };
}

/// FP4 quantization with zero-point (asymmetric quantization).
#[cube(launch)]
pub fn fp4_quantize_asymmetric_kernel<F: Float + CubeElement>(
    input: &Array<F>,
    scales: &Array<F>,
    zero_points: &Array<F>,
    output: &mut Array<u32>,
    #[comptime] block_size: u32,
    #[comptime] num_elements: u32,
) {
    let out_idx = ABSOLUTE_POS;
    let in_base = out_idx * 8usize;

    if in_base >= num_elements as usize {
        terminate!();
    }

    let scale_idx = in_base / (block_size as usize);
    let scale = scales[scale_idx];
    let zp = zero_points[scale_idx];

    let inv_scale = if scale > F::new(1e-10) {
        F::new(1.0) / scale
    } else {
        F::new(1.0)
    };

    let mut packed: u32 = 0u32;
    let neg_one = F::new(0.0) - F::new(1.0);

    #[unroll]
    for i in 0usize..8usize {
        let val_idx = in_base + i;
        if val_idx < num_elements as usize {
            let centered = input[val_idx] - zp;
            let normalized = centered * inv_scale;

            let clamped = if normalized < neg_one {
                neg_one
            } else if normalized > F::new(1.0) {
                F::new(1.0)
            } else {
                normalized
            };

            let fp4_idx = find_nearest_fp4::<F>(clamped);
            packed = packed | (fp4_idx << (i as u32 * 4u32));
        }
    }

    output[out_idx] = packed;
}

/// FP4 dequantization with zero-point (asymmetric quantization).
#[cube(launch)]
pub fn fp4_dequantize_asymmetric_kernel<F: Float + CubeElement>(
    input: &Array<u32>,
    scales: &Array<F>,
    zero_points: &Array<F>,
    output: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] num_elements: u32,
) {
    let idx = ABSOLUTE_POS;

    if idx >= num_elements as usize {
        terminate!();
    }

    let packed_idx = idx / 8usize;
    let sub_idx = idx % 8usize;

    let packed = input[packed_idx];
    let fp4_idx = (packed >> (sub_idx as u32 * 4u32)) & 0xFu32;
    let fp4_val: F = fp4_lookup::<F>(fp4_idx);

    let scale_idx = idx / (block_size as usize);
    let scale = scales[scale_idx];
    let zp = zero_points[scale_idx];

    output[idx] = fp4_val * scale + zp;
}

/// Compute both scale and zero-point for asymmetric quantization.
#[cube(launch)]
pub fn fp4_compute_scale_zeropoint_kernel<F: Float + CubeElement>(
    input: &Array<F>,
    scales: &mut Array<F>,
    zero_points: &mut Array<F>,
    #[comptime] block_size: u32,
    #[comptime] num_blocks: u32,
) {
    let block_idx = ABSOLUTE_POS;

    if block_idx >= num_blocks as usize {
        terminate!();
    }

    let start = block_idx * (block_size as usize);
    let end = start + (block_size as usize);

    // Find min and max in block
    let mut min_val = F::new(1e10);
    let mut max_val = F::new(0.0) - F::new(1e10);

    for i in start..end {
        let val = input[i];
        if val < min_val {
            min_val = val;
        }
        if val > max_val {
            max_val = val;
        }
    }

    // Compute scale and zero-point for [-1, 1] mapping
    let range = max_val - min_val;
    let scale = if range > F::new(1e-10) {
        range / F::new(2.0)
    } else {
        F::new(1.0)
    };

    let zero_point = (min_val + max_val) / F::new(2.0);

    scales[block_idx] = scale;
    zero_points[block_idx] = zero_point;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fp4_table_symmetry() {
        // FP4 should be roughly symmetric around zero
        for i in 0..7 {
            let neg = FP4_TABLE[i];
            let pos = FP4_TABLE[14 - i];
            assert!(
                (neg + pos).abs() < 0.01,
                "FP4 asymmetry at {}: {} vs {}",
                i,
                neg,
                pos
            );
        }
    }
}
