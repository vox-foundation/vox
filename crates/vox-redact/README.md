# vox-redact

Regex-based redaction of secrets, credentials, and PII for the [Vox](https://github.com/vox-foundation/vox) platform.

## Overview

`vox-redact` provides robust, high-performance redaction rules for sanitizing log streams, error messages, and arbitrary JSON payloads:

- **Secret Patterns**: detects API keys, bearer tokens, private keys, passwords, and authorization headers.
- **PII Scrubbing**: masks common personally identifiable information in telemetry and logs.
- **JSON Sanitization**: recursively traverses and masks sensitive values in `serde_json::Value` structures.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-redact = "0.6"
```

Basic example:

```rust,ignore
use vox_redact::redact_secrets;

let raw = "Authorization: Bearer sk-ant-api03-abcdef123456";
let safe = redact_secrets(raw);
```

## License

Apache-2.0
