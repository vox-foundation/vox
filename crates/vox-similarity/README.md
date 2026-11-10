# vox-similarity

Pure SimHash, MinHash, and Locality Sensitive Hashing (LSH) near-duplicate text similarity core from the [Vox](https://github.com/vox-foundation/vox) project.

## Features

- **SimHash:** 64-bit Hamming distance text hashing for near-duplicate document detection.
- **MinHash & LSH:** Jaccard similarity estimation with band-partitioned locality sensitive hashing tables.
- **Zero Heavy Dependencies:** Lightweight, fast, pure Rust with optional serde support.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-similarity = "0.6"
```

## License

Apache-2.0
