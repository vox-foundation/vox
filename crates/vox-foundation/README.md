# vox-foundation

Fundamental foundation primitives, daemon wire protocols, and tracing utilities from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-foundation` consolidates base primitives (IDs, backoff policies, AgentOS mutations), daemon wire protocol definitions, and telemetry initialization routines (`tracing_subscriber` presets with optional OpenTelemetry support) into a single L0 foundation crate with zero internal dependencies.

## Features

- **primitives**: Unique IDs, exponential backoff, and runtime mutations.
- **protocol**: Daemon wire message formats and framing.
- **tracing-init**: Opinionated logging and diagnostic presets.
- **otel**: Optional OpenTelemetry OTLP tracing integration.

## Usage

Add `vox-foundation` to your `Cargo.toml`:

```toml
[dependencies]
vox-foundation = "0.6"
```

Initialize standard tracing in an application entrypoint:

```rust
use vox_foundation::tracing_init;

fn main() {
    tracing_init::init_default();
}
```

## License

Apache-2.0
