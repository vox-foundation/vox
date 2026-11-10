# vox-identity

Cryptographic identity management, signing keys, and trust ledger primitives from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-identity` manages actor credentials, key derivation via Argon2, ed25519 signing identities, and verifiable trust ledger entries. It depends internally only on `vox-crypto`.

## Usage

Add `vox-identity` to your `Cargo.toml`:

```toml
[dependencies]
vox-identity = "0.6"
```

Manage signing keys and actor identities:

```rust
use vox_identity::Identity;
```

## License

Apache-2.0
