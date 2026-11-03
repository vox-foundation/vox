---
phase: 71-clavis-next-manifests-uri-acquisition
plan: 01
subsystem: secrets-clavis
tags: [clavis, secrets, provider-manifest, clavis-uri, external-import]
dependency_graph:
  requires: [05]
  provides: [CLAVIS-01, CLAVIS-02, CLAVIS-03, CLAVIS-04]
  affects: [vox-secrets, vox-cli, docs/src/reference/secrets-ssot.md]
tech_stack:
  added: []
  patterns: [provider-manifest-registry, clavis-uri-resolution, external-manager-import]
key_files:
  created: []
  modified:
    - crates/vox-secrets/src/lib.rs
    - crates/vox-secrets/src/spec/mod.rs
    - crates/vox-secrets/src/resolver.rs
    - crates/vox-secrets/src/sources/mod.rs
    - crates/vox-cli/src/commands/secrets.rs
    - docs/src/reference/secrets-ssot.md
decisions:
  - Keep compile-time `SecretId` safety while adding `ProviderManifestEntry` (`provider_manifest_for`, `all_provider_manifests`) with acquisition URLs, key prefixes, free-tier availability, and validation probe metadata.
  - Support portable `clavis://<provider_or_canonical_env>` URIs (`parse_clavis_uri`, `resolve_clavis_uri`) in `vox-secrets` and `vox secrets get`.
  - Provide structured external secret-manager import parsing (`ExternalSecretManager::{Dotenv, OnePassword, Doppler, Bitwarden, Infisical}`) and pre-flight missing-key recipe generation (`compute_missing_key_recipes`).
metrics:
  duration_minutes: 20
  completed_date: "2026-10-14"
---

# Phase 71 Plan 01: Clavis Next — Provider Manifests, `clavis://` URIs & Need-Driven Acquisition Summary

Implemented Clavis Next Part 1 (`CLAVIS-01..04`): structured provider secret manifests, portable `clavis://` URI resolution, external secret-manager import adapters, and pre-flight missing-key recipe computation.

## What Was Done

1. **Task 1 (`CLAVIS-01..02` — Data-Driven Provider Manifests & `clavis://` Reference Syntax):**
   - Added `ProviderManifestEntry`, `provider_manifest_for`, and `all_provider_manifests` in `crates/vox-secrets/src/spec/mod.rs` and re-exported them at `vox_secrets`.
   - Added `parse_clavis_uri` and `resolve_clavis_uri` in `crates/vox-secrets/src/resolver.rs` and wired `clavis://` resolution into `vox secrets get` in `crates/vox-cli/src/commands/secrets.rs`.
2. **Task 2 (`CLAVIS-03..04` — External Manager Import & Pre-Flight Acquisition Recipes):**
   - Added `ExternalSecretManager`, `ExternalImportRecord`, `parse_external_manager_export`, and `compute_missing_key_recipes` in `crates/vox-secrets/src/sources/mod.rs`.
   - Updated `docs/src/reference/secrets-ssot.md` and verified zero errors via `vox-doc-pipeline --lint-only --paths reference/secrets-ssot.md`.

## Verification

- `cargo --config 'build.rustc-wrapper=""' test -p vox-secrets --lib` -> `124 passed; 0 failed`
- `./target/debug/vox-doc-pipeline --lint-only --paths reference/secrets-ssot.md` -> `vox-doc-pipeline lint complete — no hard errors.`
