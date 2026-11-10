# vox-quantize

Data-free k-quant post-training quantization (PTQ) engine from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-quantize` converts standard SafeTensors weight checkpoints into Candle GGML quantized SafeTensors with GPU acceleration (CUDA / Metal) and automatic CPU fallback. It operates without internal workspace dependencies.

## Features

- **default**: Lightweight parser and metadata inspection without heavyweight tensor runtimes.
- **engine**: Enables Candle-based quantization routines for linear and attention weights.
- **cuda**: CUDA-accelerated quantization kernels.
- **metal**: Apple Silicon Metal-accelerated quantization kernels.

## Usage

Add `vox-quantize` to your `Cargo.toml`:

```toml
[dependencies]
vox-quantize = { version = "0.6", features = ["engine"] }
```

Inspect or quantize SafeTensors models:

```rust
use vox_quantize::QuantEngine;
```

## License

Apache-2.0
