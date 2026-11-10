# vox-jsonschema-util

Shared JSON Schema validation and compilation utilities for the [Vox](https://github.com/vox-foundation/vox) ecosystem.

## Overview

`vox-jsonschema-util` wraps the `jsonschema` validator with ergonomics tailored for CLI commands, contract enforcement, and configuration schemas:

- **Schema Compilation**: cached compilation of JSON and YAML schemas.
- **Detailed Diagnostics**: structured validation error reporting with JSON pointers and path context.
- **Contract Verification**: utilities for ensuring runtime values adhere to formal interface contracts.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-jsonschema-util = "0.6"
```

## License

Apache-2.0
