# vox-db-types

Pure-data L0 leaf types for database rows, IDs, and schema descriptors in the [Vox](https://github.com/vox-foundation/vox) system.

`vox-db-types` provides core database entity models, IDs, schema descriptors, and sanitization logic without any internal workspace dependencies or runtime baggage. It is designed to be consumed across client, server, and storage layers with minimal overhead.

## Usage

Add `vox-db-types` to your `Cargo.toml`:

```toml
[dependencies]
vox-db-types = "0.6"
```

Then use the shared types and identifiers:

```rust
use vox_db_types::ids::SessionId;

let session_id = SessionId::new();
```

## License

Apache-2.0
