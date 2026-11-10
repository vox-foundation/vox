# vox-orchestrator-types

Pure-data L0 leaf types for the [Vox](https://github.com/vox-foundation/vox) orchestrator.

`vox-orchestrator-types` defines agent identifiers, task definitions, file affinity records, orchestration switch actions, and model provider catalog definitions. It has zero internal dependencies within the workspace, making it safe to depend on across runtime layers.

## Usage

Add `vox-orchestrator-types` to your `Cargo.toml`:

```toml
[dependencies]
vox-orchestrator-types = "0.6"
```

Then use the shared types and IDs:

```rust
use vox_orchestrator_types::AgentId;

let agent = AgentId::new("worker-1");
```

## License

Apache-2.0
