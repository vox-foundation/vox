//! # QLoRA Inference Example
//!
//! Demonstrates how to:
//! - Create a QuantizedLinear layer with QLoRA
//! - Perform a forward pass with sample input
//! - Inspect output shapes and properties
//!
//! This example shows how QLoRA combines:
//! 1. Frozen quantized base weights (4-bit NF4)
//! 2. Trainable LoRA adapters (full precision)
//!
//! This enables memory-efficient inference and fine-tuning.

use anyhow::Result;
use candle_core::{Device, Tensor};
use qlora_rs::{QLoraConfig, QuantizedLinear};

fn main() -> Result<()> {
    println!("=== QLoRA Inference Example ===\n");

    // Set up device
    let device = Device::Cpu;

    // Create QLoRA configuration
    // Using preset for Q/V projections (memory-optimal)
    let lora_rank = 8;
    let lora_alpha = 16;
    let config = QLoraConfig::preset_qv_bf16(lora_rank, lora_alpha);

    println!("QLoRA Configuration:");
    println!("  LoRA rank: {}", lora_rank);
    println!("  LoRA alpha: {}", lora_alpha);
    println!(
        "  Quantization block size: {}",
        config.quantization.block_size
    );
    println!(
        "  Double quantization: {}",
        config.quantization.double_quant
    );
    println!("  Compute dtype: {:?}\n", config.quantization.compute_dtype);

    // Define layer dimensions (typical for attention Q/V projection)
    let in_features = 512;
    let out_features = 512;
    println!("Creating QuantizedLinear layer:");
    println!("  Input features: {}", in_features);
    println!("  Output features: {}\n", out_features);

    // Create a random weight tensor to initialize the layer
    // In practice, this would be loaded from a pre-trained model
    let weight = Tensor::randn(0.0f32, 1.0f32, (out_features, in_features), &device)?;

    // Create QuantizedLinear layer
    // This will:
    // 1. Quantize the weight to NF4 (4-bit)
    // 2. Initialize LoRA adapters (A and B matrices)
    println!("Quantizing base weights and initializing LoRA adapters...");
    let layer = QuantizedLinear::from_weight(&weight, None, &config, &device)?;
    println!("Layer created successfully!\n");

    // Calculate memory usage
    let base_weight_size = out_features * in_features * 4; // Original f32 size
    let quantized_size = (out_features * in_features) / 2; // 4-bit quantized
    let lora_a_size = in_features * lora_rank * 4; // f32
    let lora_b_size = lora_rank * out_features * 4; // f32
    let total_size = quantized_size + lora_a_size + lora_b_size;

    println!("Memory Usage:");
    println!(
        "  Original weight (f32): {:.2} MB",
        base_weight_size as f64 / 1024.0 / 1024.0
    );
    println!(
        "  Quantized base: {:.2} MB",
        quantized_size as f64 / 1024.0 / 1024.0
    );
    println!(
        "  LoRA A matrix: {:.2} MB",
        lora_a_size as f64 / 1024.0 / 1024.0
    );
    println!(
        "  LoRA B matrix: {:.2} MB",
        lora_b_size as f64 / 1024.0 / 1024.0
    );
    println!("  Total: {:.2} MB", total_size as f64 / 1024.0 / 1024.0);
    println!(
        "  Memory savings: {:.1}x\n",
        base_weight_size as f64 / total_size as f64
    );

    // Create sample input (batch of embeddings)
    let batch_size = 4;
    let seq_len = 128;
    println!("Running forward pass:");
    println!("  Batch size: {}", batch_size);
    println!("  Sequence length: {}", seq_len);

    let input = Tensor::randn(0.0f32, 1.0f32, (batch_size, seq_len, in_features), &device)?;
    println!("  Input shape: {:?}", input.shape());

    // Perform forward pass
    // This computes: output = dequant(W_base) @ x + (x @ A @ B) * scale
    println!("\nComputing: y = W_quantized @ x + (x @ A @ B) * (alpha/rank)...");
    let output = layer.forward(&input)?;

    println!("\nOutput:");
    println!("  Shape: {:?}", output.shape());
    println!("  Dtype: {:?}", output.dtype());

    // Verify output shape is correct
    assert_eq!(
        output.shape().dims(),
        &[batch_size, seq_len, out_features],
        "Output shape mismatch!"
    );
    println!("  Shape verification: PASSED\n");

    // Compute some statistics on the output
    let mean = output.mean_all()?.to_scalar::<f32>()?;
    let std = output.flatten_all()?.var(0)?.to_scalar::<f32>()?.sqrt();
    let min = output.flatten_all()?.min(0)?.to_scalar::<f32>()?;
    let max = output.flatten_all()?.max(0)?.to_scalar::<f32>()?;

    println!("Output Statistics:");
    println!("  Mean: {:.6}", mean);
    println!("  Std: {:.6}", std);
    println!("  Min: {:.6}", min);
    println!("  Max: {:.6}\n", max);

    // Inspect LoRA weights
    let (lora_a, lora_b) = layer.lora_weights();
    println!("LoRA Adapter Weights:");
    println!("  A matrix shape: {:?}", lora_a.shape());
    println!("  B matrix shape: {:?}", lora_b.shape());

    // Check that LoRA B is initialized to zeros (standard practice)
    let b_sum = lora_b.sum_all()?.to_scalar::<f32>()?;
    println!("  B matrix sum (should be ~0): {:.6}", b_sum);
    println!("  (LoRA B initialized to zeros ensures initial output = base model)\n");

    println!("=== Summary ===");
    println!("Successfully created a QLoRA layer with:");
    println!(
        "  - 4-bit quantized base weights ({:.2} MB)",
        quantized_size as f64 / 1024.0 / 1024.0
    );
    println!(
        "  - Full-precision LoRA adapters ({:.2} MB)",
        (lora_a_size + lora_b_size) as f64 / 1024.0 / 1024.0
    );
    println!(
        "  - {:.1}x memory reduction vs. full precision",
        base_weight_size as f64 / total_size as f64
    );
    println!("\nThis layer is ready for:");
    println!("  - Inference with the quantized base model");
    println!("  - Fine-tuning by training only the LoRA adapters");

    Ok(())
}
