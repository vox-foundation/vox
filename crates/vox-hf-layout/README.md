# vox-hf-layout

Hugging Face transformer architecture and `config.json` inspection utilities from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-hf-layout` parses and validates Hugging Face model configuration metadata to reconstruct transformer topology (layers, attention heads, hidden dimensions, kv-heads) for inference engines and quantization pipelines. It depends only on `vox-bounded-fs` within the workspace.

## Usage

Add `vox-hf-layout` to your `Cargo.toml`:

```toml
[dependencies]
vox-hf-layout = "0.6"
```

Parse a Hugging Face configuration file:

```rust
use vox_hf_layout::HfModelLayout;
```

## License

Apache-2.0
