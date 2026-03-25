---
title: "Binary release artifact contract"
description: "SSOT for GitHub Release binary names, archive layout, checksum manifest, and supported host triples for `vox-bootstrap` and `vox ci release-build`."
category: "reference"
last_updated: 2026-03-25
training_eligible: true
---

# Binary release artifact contract

This document is the **authoritative contract** between:

- [`vox ci release-build`](../reference/cli.md) (packaging in CI / locally),
- [`.github/workflows/release-binaries.yml`](../../../.github/workflows/release-binaries.yml) (tag-triggered publish),
- [`vox-bootstrap`](../api/vox-bootstrap.md) (binary-first install).

## Supported release targets

These triples are built and published for each release **tag** `v*`:

| Target | Notes |
|--------|--------|
| `x86_64-unknown-linux-gnu` | Linux x86_64, glibc |
| `x86_64-pc-windows-msvc` | Windows x86_64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |

`vox-bootstrap` maps the **compile-time host** to one of these triples. If no matching asset exists published for that tag, binary install **fails** and the installer falls back to **`cargo install --path crates/vox-cli`** (requires repo root).

## Asset file names

For a Git tag `<tag>` (for example `v1.2.3`), each artifact **basename** is:

- **Unix** (Linux + macOS): `vox-<tag>-<target>.tar.gz`
- **Windows**: `vox-<tag>-<target>.zip`

Example: `vox-v1.2.3-x86_64-unknown-linux-gnu.tar.gz`

## Archive contents

| Platform | Single entry name |
|----------|-------------------|
| Unix archives | `vox` (executable) |
| Windows zip | `vox.exe` |

No nested directory prefix inside the archive for the executable entry.

## Checksums

- **Authoritative** `checksums.txt` for end users is produced in the **publish** job by hashing each uploaded release asset and emitting **basename-only** lines:

  ```text
  <sha256_hex><two_spaces><basename>
  ```

- Per-job `dist/checksums.txt` from `release-build` is for **local debugging** only; release downloads should use the root `checksums.txt` attached to the GitHub Release.

## Download URLs (bootstrap)

- Tagged asset: `https://github.com/vox-foundation/vox/releases/download/<tag>/<basename>`
- Latest asset: `https://github.com/vox-foundation/vox/releases/latest/download/<basename>`

The **basename** for `latest` must match the **actual** filename on the latest release (same tag in the name as `tag_name` on that release). Installers **must not** invent a fake `vox-latest-…` filename.

## Smoke checks

Before artifacts are uploaded from a matrix build, each platform job extracts the produced archive and runs **`vox` / `vox.exe --version`** on that OS. If any job fails smoke, **do not** consider the release green.

## Rollback

If a bad release is published: delete or edit the GitHub Release assets, or ship a **new patch tag** with corrected artifacts. Semver: prefer `vX.Y.(Z+1)` over reusing a tag.

## Release dry-run (operators)

Before shipping a real tag:

1. Locally: `cargo run -p vox-cli -- ci release-build --target <host-triple>` (optional `--version`), extract the archive, run `./vox --version`.
2. `cargo test -p vox-cli release_build`, `cargo test -p vox-bootstrap`, `cargo run -p vox-cli -- ci command-compliance`.
3. CI: push a disposable test tag `v0.0.0-test.<timestamp>`, confirm all matrix jobs + publish; then delete the test tag/release if it was only for verification.
