# vox-openai

OpenAI-compatible chat completions wire formats and server-sent event (SSE) streaming types from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-openai` provides type-safe deserialization and serialization for OpenAI-compatible REST APIs, including function calling, token usage statistics, and streaming delta chunks. It has zero internal workspace dependencies.

## Usage

Add `vox-openai` to your `Cargo.toml`:

```toml
[dependencies]
vox-openai = "0.6"
```

Deserialize OpenAI chat completion requests or streaming chunks:

```rust
use vox_openai::ChatCompletionChunk;
```

## License

Apache-2.0
