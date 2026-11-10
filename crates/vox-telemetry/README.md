# vox-telemetry

Low-overhead telemetry facade, canonical metric-type constants, and recording abstractions from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-telemetry` provides the `TelemetryRecorder` trait, structural metric constants, research trial contracts, and event recording macros. It has zero domain dependencies, enabling lightweight instrumentation across all application tiers.

## Usage

Add `vox-telemetry` to your `Cargo.toml`:

```toml
[dependencies]
vox-telemetry = "0.6"
```

Record telemetry events:

```rust
use vox_telemetry::{TelemetryRecorder, NoopRecorder};

let recorder = NoopRecorder;
```

## License

Apache-2.0
