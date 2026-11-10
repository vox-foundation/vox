# vox-mesh-policy

Worker resource donation policy parser and serializer for the [Vox](https://github.com/vox-foundation/vox) GPU mesh.

`vox-mesh-policy` parses, verifies, and pretty-prints `donations.vox` configuration files, enforcing execution constraints, GPU quotas, and scheduling windows for volunteer and federated compute nodes. It depends only on `vox-mesh-types` within the workspace.

## Usage

Add `vox-mesh-policy` to your `Cargo.toml`:

```toml
[dependencies]
vox-mesh-policy = "0.6"
```

Parse and evaluate donation policies:

```rust
use vox_mesh_policy::WorkerDonationPolicy;
```

## License

Apache-2.0
