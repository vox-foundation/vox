---
phase: 72-clavis-next-zero-trust-agent-broker-rotation
plan: 01
subsystem: secrets-clavis
tags: [clavis, credential-broker, zero-trust, rotation-lifecycle, pull-sync]
dependency_graph:
  requires: [12, 71]
  provides: [CLAVIS-05, CLAVIS-06, CLAVIS-07, CLAVIS-08]
  affects: [vox-secrets, vox-actor-runtime, vox-cli, docs/src/reference/secrets-ssot.md]
tech_stack:
  added: []
  patterns: [capability-scoped-credential-broker, sealed-header-injection, lifecycle-rotation-runbook]
key_files:
  created:
    - crates/vox-secrets/src/lifecycle.rs
    - crates/vox-actor-runtime/src/credential_broker.rs
  modified:
    - crates/vox-secrets/src/lib.rs
    - crates/vox-actor-runtime/src/lib.rs
    - crates/vox-cli/src/commands/secrets.rs
    - docs/src/reference/secrets-ssot.md
decisions:
  - Enforce capability-scoped authorization (`BrokerAgentGrant`) in `vox_actor_runtime::credential_broker::CredentialBroker` so agents and MCP tools never receive raw key material and only inject `SealedHeaderInjection` tokens at the HTTP boundary while scrubbing tool outputs via `vox_secrets::redact`.
  - Centralize rotation health evaluation (`evaluate_lifecycle_health`), read-only validation probe specs (`validation_probe_for`), revoke-rotate-redistribute runbooks (`rotation_runbook_for`), and shareable pull-sync manifest filtering (`prepare_pull_sync_manifest`) in `crates/vox-secrets/src/lifecycle.rs`.
metrics:
  duration_minutes: 25
  completed_date: "2026-10-14"
---

# Phase 72 Plan 01: Clavis Next — Zero-Trust Agent Broker, Validation Probes & Rotation Lifecycle Summary

Implemented Clavis Next Part 2 (`CLAVIS-05..08`): capability-scoped `CredentialBroker` in `vox-actor-runtime` (`Agents Never See Keys`) and `vox_secrets::lifecycle` rotation health, validation probes, rotation runbooks, and shareable pull-sync manifest preparation.

## What Was Done

1. **Task 1 (`CLAVIS-05` — Zero-Trust Credential-Injecting Agent Broker):**
   - Created `crates/vox-actor-runtime/src/credential_broker.rs` with `CredentialBroker`, `BrokerAgentGrant`, `SealedHeaderInjection`, and `CredentialBrokerError`.
   - Verified that `SealedHeaderInjection` redacts `Debug` and serialized output, marks `reqwest` headers sensitive, rejects unauthorized capability scopes, and scrubs brokered secrets from tool output and logs via `vox_secrets::redact`.
2. **Task 2 (`CLAVIS-06..08` — Live Provider Validation Probes, `LifecycleMeta` Rotation Alerts & Pull-Based Vault Sync):**
   - Created `crates/vox-secrets/src/lifecycle.rs` with `RotationHealth`, `evaluate_lifecycle_health`, `ValidationProbeSpec`, `validation_probe_for`, `RotationRunbook`, `rotation_runbook_for`, and `prepare_pull_sync_manifest`.
   - Wired `inspect_secret_lifecycle_plan` in `crates/vox-cli/src/commands/secrets.rs` and updated `docs/src/reference/secrets-ssot.md`.

## Verification

- `cargo --config 'build.rustc-wrapper=""' test -p vox-actor-runtime --lib credential_broker` -> `2 passed; 0 failed`
- `cargo --config 'build.rustc-wrapper=""' test -p vox-secrets --lib` -> `124 passed; 0 failed`
- `./target/debug/vox-doc-pipeline --lint-only --paths reference/secrets-ssot.md` -> `vox-doc-pipeline lint complete — no hard errors.`
