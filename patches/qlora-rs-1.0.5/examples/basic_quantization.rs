//! # Basic NF4 Quantization Example
//!
//! Demonstrates how to:
//! - Create a random tensor
//! - Quantize it to 4-bit NF4 format
//! - Dequantize back to full precision
//! - Measure quantization error
//!
//! This example shows the core quantization/dequantization flow which is the foundation
//! of QLoRA's memory savings. NF4 quantization reduces model size by ~4x while maintaining
//! acceptable accuracy.

use anyhow::Result;
use candle_core::{Device, Tensor};
use qlora_rs::{dequantize_nf4, quantize_nf4};

fn main() -> Result<()> {
    println!("=== NF4 Quantization Example ===\n");

    // Set up device (CPU for portability)
    let device = Device::Cpu;

    // Create a random weight tensor (simulating a model layer)
    // Using 512x512 dimension - typical for transformer feed-forward layers
    let shape = (512, 512);
    let original = Tensor::randn(0.0f32, 1.0f32, shape, &device)?;
    println!("Original tensor shape: {:?}", original.shape());
    println!("Original tensor dtype: {:?}", original.dtype());

    // Calculate original size in bytes (f32 = 4 bytes per element)
    let numel = shape.0 * shape.1;
    let original_size_bytes = numel * 4;
    println!(
        "Original size: {} bytes ({:.2} MB)\n",
        original_size_bytes,
        original_size_bytes as f64 / 1024.0 / 1024.0
    );

    // Quantize to NF4 format
    // Block size of 64 is recommended - balances accuracy vs compression
    let block_size = 64;
    println!("Quantizing to NF4 with block_size={}...", block_size);
    let quantized = quantize_nf4(&original, block_size)?;

    // Calculate quantized size
    // Each element uses 4 bits (0.5 bytes) + overhead for scales
    let num_blocks = numel.div_ceil(block_size);
    let data_size_bytes = numel / 2; // 4 bits per element = 0.5 bytes
    let scales_size_bytes = num_blocks * 4; // f32 scale per block
    let quantized_size_bytes = data_size_bytes + scales_size_bytes;
    println!(
        "Quantized size: {} bytes ({:.2} MB)",
        quantized_size_bytes,
        quantized_size_bytes as f64 / 1024.0 / 1024.0
    );
    println!(
        "Compression ratio: {:.2}x\n",
        original_size_bytes as f64 / quantized_size_bytes as f64
    );

    // Dequantize back to full precision
    println!("Dequantizing back to f32...");
    let dequantized = dequantize_nf4(&quantized, &device)?;
    println!("Dequantized tensor shape: {:?}", dequantized.shape());
    println!("Dequantized tensor dtype: {:?}\n", dequantized.dtype());

    // Measure quantization error
    println!("Computing quantization error...");

    // Mean Absolute Error (MAE)
    let diff = ((&original - &dequantized)?.abs())?;
    let mae = diff.mean_all()?.to_scalar::<f32>()?;
    println!("Mean Absolute Error (MAE): {:.6}", mae);

    // Root Mean Square Error (RMSE)
    let squared_diff = diff.sqr()?;
    let mse = squared_diff.mean_all()?.to_scalar::<f32>()?;
    let rmse = mse.sqrt();
    println!("Root Mean Square Error (RMSE): {:.6}", rmse);

    // Maximum error
    let max_error = diff.max(1)?.max(0)?.to_scalar::<f32>()?;
    println!("Maximum Error: {:.6}", max_error);

    // Signal-to-Noise Ratio (SNR)
    let signal_power = original.sqr()?.mean_all()?.to_scalar::<f32>()?;
    let snr_db = 10.0 * (signal_power / mse).log10();
    println!("Signal-to-Noise Ratio (SNR): {:.2} dB\n", snr_db);

    println!("=== Summary ===");
    println!(
        "NF4 quantization achieved {:.2}x compression",
        original_size_bytes as f64 / quantized_size_bytes as f64
    );
    println!(
        "with RMSE of {:.6} ({:.4}% relative error)",
        rmse,
        (rmse / signal_power.sqrt()) * 100.0
    );
    println!("\nThis low error rate makes NF4 suitable for storing frozen base model weights");
    println!("in QLoRA, where we only train small LoRA adapters in full precision.");

    Ok(())
}
