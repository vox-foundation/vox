# vox-http-client

Shared HTTP client presets, middleware configurations, and error envelopes for the [Vox](https://github.com/vox-foundation/vox) ecosystem.

## Overview

`vox-http-client` enforces consistent, resilient HTTP client configurations across all Vox services and CLIs:

- **Client Presets**: standardized connection pooling, user-agents, and timeout configurations.
- **Middleware Support**: optional retry strategies, backoff policies, and tracing instrumentation.
- **Error Envelopes**: uniform parsing and handling of structured HTTP error responses.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-http-client = "0.6"
```

With middleware support:

```toml
[dependencies]
vox-http-client = { version = "0.6", features = ["middleware"] }
```

## License

Apache-2.0
