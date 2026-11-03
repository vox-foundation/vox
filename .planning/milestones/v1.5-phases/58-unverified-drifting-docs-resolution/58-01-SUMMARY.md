---
phase: 58-unverified-drifting-docs-resolution
plan: 01
type: execute
completed_at: 2026-10-14T11:55:00Z
files_modified:
  - crates/vox-doc-pipeline/src/claims/extractor.rs
  - crates/vox-doc-pipeline/src/claims/resolver.rs
  - crates/vox-doc-pipeline/src/claims/hasher.rs
  - crates/vox-doc-pipeline/src/claims/ledger.rs
  - crates/vox-doc-pipeline/src/pipeline/mod.rs
  - docs/src/explanation/expl-architecture.md
  - contracts/reports/docs-freshness/ledger.v1.json
  - contracts/reports/docs-freshness/baseline.v1.json
requirements-completed:
  - DOCVER-01
  - DOCVER-02
  - DOCVER-03
---

# Phase 58 Plan 01–03 Summary: Comprehensive Unverified & Drifting Docs Resolution

## Overview

Resolved all "Unverified" (`broken`/`unanchored`) and "Drifting" documentation pages across the 1,118-page `docs/src/` corpus by refining deterministic claim extraction, ground-truth resolution, and anchor hashing in `vox-doc-pipeline`, updating stale code references, and ratcheting `contracts/reports/docs-freshness/{ledger.v1.json, baseline.v1.json}`.

## Key Changes

1. **False-Positive Claim Extraction & Resolution Fixes (`DOCVER-01`)**:
   - [`crates/vox-doc-pipeline/src/claims/extractor.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/extractor.rs): Scoped `RE_FILE_PATH` to checked-in repository roots (`crates|docs|contracts|\.planning|scripts|tests|examples|assets|docs-astro|Cargo.toml|README.md|AGENTS.md`), tightened `RE_CRATE_NAME` so trailing-hyphen prefixes (`vox-plugin-`) are not misclassified, expanded `is_descriptive_context` to inspect the full current line, and propagated placeholder `MentionRole::Mentions` across all mention kinds.
   - [`crates/vox-doc-pipeline/src/claims/resolver.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/resolver.rs): Added distribution bundle aliases, plugin catalog IDs, config registry `env_var` entries, and built-in `vox` CLI subcommands; handled `cargo run -p <pkg>` and leading global flags (`--json`, `--quiet`, `--release`, `--features`); and returned `ResolutionStatus::Ambiguous` for unknown dotted `ConfigKey` tokens.
   - [`crates/vox-doc-pipeline/src/claims/hasher.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/hasher.rs): Updated `hash_anchor` to deterministically hash directories (via `Cargo.toml` / `src/lib.rs`), `RustSymbol`, `EnvVar`, `CliCommand`, `ConfigKey`, and crate-scoped paths without emitting `"missing-file-hash"`.

2. **Corpus Anchoring & Ledger Ratchet (`DOCVER-02`, `DOCVER-03`)**:
   - [`crates/vox-doc-pipeline/src/claims/ledger.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/ledger.rs): Included `mounted_from` frontmatter in anchors, classified `archive` pages as `DocState::Historical`, attached fallback structural anchors (`crates/vox-doc-pipeline/src/pipeline/doctest.rs` or `contracts/documentation/docs-sidebar-section-order.v1.json`) for pages without inline mentions, and added `--refresh-freshness-ledger` to [`crates/vox-doc-pipeline/src/pipeline/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/mod.rs).
   - Updated [`docs/src/explanation/expl-architecture.md`](file:///Users/brbrainerd/dev/vox/docs/src/explanation/expl-architecture.md) to point to `crates/vox-ml-cli/src/main.rs`.

## Verification

- `./target/debug/vox-doc-pipeline --refresh-freshness-ledger`:
  - `ledger.v1.json`: **1,118 pages: 810 verified, 0 drifting, 0 broken, 308 historical, 0 unanchored**
  - `baseline.v1.json`: tightened from **954** deficits down to **7** (`changelog.md` and `env-vars.generated.md` deprecated alias column).
- `cargo test -p vox-doc-pipeline`: 74 passed, 0 failed.
