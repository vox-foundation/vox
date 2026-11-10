# vox-test-harness

Shared test infrastructure and harness primitives for the [Vox](https://github.com/vox-foundation/vox) ecosystem.

## Overview

`vox-test-harness` provides common testing utilities across the workspace:

- **Environment Isolation (`EnvScratch`)**: safely mutate process environment variables during tests with guaranteed restoration on drop.
- **Temporary Root Management**: hermetic filesystem sandboxing for CLI and file operations.
- **Port Allocation**: reliable free-port acquisition for network and server test suites.
- **Barriers & Synchronization**: synchronization primitives for distributed/concurrent test scenarios.
- **Synthetic Workspaces**: generators for synthetic multi-crate workspace layouts.

## Usage

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
vox-test-harness = "0.6"
```

## License

Apache-2.0
