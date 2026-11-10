# vox-git

Pure-Rust Git operations and synchronization bridge from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-git` provides bidirectional Git operations using `gix` without any C dependencies (libgit2-free). It enforces bounds on repository access and depends only on `vox-bounded-fs` within the workspace.

## Usage

Add `vox-git` to your `Cargo.toml`:

```toml
[dependencies]
vox-git = "0.6"
```

Perform pure-Rust Git operations:

```rust
use vox_git::GitRepo;
```

## License

Apache-2.0
