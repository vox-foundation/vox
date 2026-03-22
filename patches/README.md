# Cargo `[patch.crates-io]` overlays

## `aegis-0.9.8`

Upstream `aegis` runs a `cc` build (preferring `clang-cl` on `x86_64-pc-windows-msvc`) for native AES paths. This fails in minimal Windows dev/CI environments without LLVM.

The fork only changes **default features** to include `pure-rust` (Rust `softaes` backend), matching the approach `turso_core` already uses for Android.

When upgrading the workspace lockfile’s `aegis` version, refresh this tree from crates.io and re-apply the `default` feature line in `Cargo.toml`.
