# vox-llm-egress

Low-layer LLM provider egress, streaming transport, and adaptive throttle mechanisms from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-llm-egress` is the sanctioned egress path for OpenAI-compatible inference providers. It implements single-flight request execution (`chat_once`, `stream_once`, `embed_once`) with per-endpoint AIMD rate throttling. It operates purely at the wire level with caller-provided credentials, depending only on `vox-http-client` and `vox-openai`.

## Usage

Add `vox-llm-egress` to your `Cargo.toml`:

```toml
[dependencies]
vox-llm-egress = "0.6"
```

Dispatch an inference call through the egress facade:

```rust
use vox_llm_egress::EgressClient;
```

## License

Apache-2.0
