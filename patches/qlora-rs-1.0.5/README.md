# qlora-rs

4-bit quantized LoRA (QLoRA) implementation for Rust with dual GGUF and Candle native export.

[![Crates.io](https://img.shields.io/crates/v/qlora-rs.svg)](https://crates.io/crates/qlora-rs)
[![Documentation](https://docs.rs/qlora-rs/badge.svg)](https://docs.rs/qlora-rs)
[![License](https://img.shields.io/crates/l/qlora-rs.svg)](LICENSE-MIT)

## Overview

`qlora-rs` provides efficient 4-bit quantization and QLoRA inference capabilities for Rust:

- **NF4 Quantization** - 4-bit NormalFloat format optimized for neural network weights
- **Double Quantization** - Further compress scale factors for additional memory efficiency
- **Advanced Quantization** - Per-channel and zero-point asymmetric quantization strategies
- **QLoRA Inference Layer** - Forward pass with frozen quantized weights + LoRA adapters
- **Dual Export Formats** - GGUF (llama.cpp compatible) and Candle native (QNAT) formats

**Status**: 1.0.0 Release - Core quantization, QLoRA inference, training support, and dual export formats are fully functional.

## Features

- 🦀 Pure Rust implementation
- 📉 ~4x expected memory reduction for base model weights
- ⚡ Fast quantization and dequantization
- 📦 Dual export: GGUF format (llama.cpp) and Candle native (QNAT)
- 🔗 Integrates with [peft-rs](https://crates.io/crates/peft-rs) for LoRA adapter management
- ✅ 59/59 tests passing

## Installation

```toml
[dependencies]
qlora-rs = "1.0"
```

## Quick Start

### Quantize Weights

```rust
use qlora_rs::{quantize_nf4, dequantize_nf4};
use candle_core::{Device, Tensor};

fn main() -> anyhow::Result<()> {
    let device = Device::Cpu;
    
    // Create some weights
    let weights = Tensor::randn(0.0, 1.0, (4096, 4096), &device)?;
    
    // Quantize to 4-bit NF4
    let quantized = quantize_nf4(&weights, 64)?;  // block_size = 64
    
    println!("Original size: {} bytes", 4096 * 4096 * 4);
    println!("Quantized size: {} bytes", quantized.size_bytes());
    
    // Dequantize for computation
    let restored = dequantize_nf4(&quantized, &device)?;
    
    Ok(())
}
```

### QLoRA Layer

```rust
use qlora_rs::{QLoraConfig, QuantizedLinear};
use candle_core::{Device, Tensor, DType};

fn main() -> anyhow::Result<()> {
    let device = Device::Cpu;
    let config = QLoraConfig::default();
    
    // Create layer from existing weights
    let weights = Tensor::randn(0.0, 1.0, (768, 768), &device)?;
    let layer = QuantizedLinear::from_weight(&weights, None, config, &device)?;
    
    // Forward pass
    let input = Tensor::zeros(&[1, 10, 768], DType::F32, &device)?;
    let output = layer.forward(&input)?;
    
    println!("Trainable parameters: {}", layer.num_trainable_parameters());
    
    Ok(())
}
```

### Export to GGUF

```rust
use qlora_rs::{quantize_nf4, export_gguf};
use candle_core::{Device, Tensor};

fn main() -> anyhow::Result<()> {
    let device = Device::Cpu;
    
    // Quantize model weights
    let q_proj = Tensor::randn(0.0, 1.0, (4096, 4096), &device)?;
    let q_proj_quantized = quantize_nf4(&q_proj, 64)?;
    
    // Export to GGUF
    export_gguf(
        &[("model.layers.0.self_attn.q_proj.weight", &q_proj_quantized)],
        "model.gguf",
    )?;
    
    Ok(())
}
```

## NF4 Quantization

NF4 (4-bit NormalFloat) uses 16 quantization levels optimized for normally-distributed data:

```
-1.0, -0.696, -0.525, -0.395, -0.284, -0.185, -0.091, 0.0,
 0.080, 0.161, 0.246, 0.338, 0.441, 0.563, 0.723, 1.0
```

This provides better accuracy than uniform quantization for neural network weights.

## Expected Memory Reduction

Theoretical memory usage based on NF4 quantization (actual results may vary):

| Model Size | FP16 | NF4 (Expected) | Reduction |
|------------|------|----------------|-----------|
| 7B params  | 14GB | ~4GB          | 3.5x      |
| 13B params | 26GB | ~7GB          | 3.7x      |
| 70B params | 140GB| ~35GB         | 4.0x      |

## Known Issues

- **Unmaintained `paste` dependency**: The `paste` crate (used by `gemm` → `candle-core`) is unmaintained (RUSTSEC-2024-0436). This is a transitive dependency and does not affect functionality.

  **Solution**: A maintained fork `qlora-paste` (v1.0.17) has been created and published to crates.io. To resolve this issue:
  
  1. The `gemm-fork/` directory contains a patched version of the `gemm` crates that use `qlora-paste` instead of `paste`.
  2. To use this in your project, add the following to your workspace root `Cargo.toml`:
  
     ```toml
     [patch.crates-io]
     gemm = { path = "qlora-rs/gemm-fork/gemm" }
     gemm-common = { path = "qlora-rs/gemm-fork/gemm-common" }
     gemm-f16 = { path = "qlora-rs/gemm-fork/gemm-f16" }
     gemm-f32 = { path = "qlora-rs/gemm-fork/gemm-f32" }
     gemm-f64 = { path = "qlora-rs/gemm-fork/gemm-f64" }
     gemm-c32 = { path = "qlora-rs/gemm-fork/gemm-c32" }
     gemm-c64 = { path = "qlora-rs/gemm-fork/gemm-c64" }
     ```
  
  The security audit warning is currently ignored in CI as the crate remains functional. Future updates to Candle may resolve this.

## Contributing

See workspace [AGENTS.md](../AGENTS.md) for coding conventions.

## License

Dual licensed under MIT OR Apache-2.0 at your option.

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)
