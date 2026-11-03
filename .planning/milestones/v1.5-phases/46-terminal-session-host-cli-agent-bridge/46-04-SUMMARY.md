---
phase: 46-terminal-session-host-cli-agent-bridge
plan: 04
subsystem: terminal-core
tags: [terminal, redaction, context-bundle, rpc]
provides:
  - RedactedBlockContext and LocalHost::redacted_context_bundle
  - term.context RPC endpoint on vox-orchestrator-d
affects: [vox-terminal-core, vox-orchestrator-d]
key-files:
  modified:
    - crates/vox-terminal-core/src/host.rs
    - crates/vox-terminal-core/src/lib.rs
    - crates/vox-orchestrator-d/src/term_dispatch.rs
requirements-completed: [TERM-04]
completed: 2026-10-14
---

# Phase 46 Plan 04: Redacted Terminal Context Bundle Summary

Implemented `RedactedBlockContext` and `LocalHost::redacted_context_bundle` in `vox-terminal-core` (scrubbing command inputs and outputs through `vox_redact::redact_owned`), and exposed the `"term.context"` JSON-RPC method on `vox-orchestrator-d::term_dispatch`.

## Verification

- RED: `cargo test -p vox-terminal-core --lib host::tests::context_bundle_redacts_secrets_and_caps_blocks` failed with `E0599: no method named redacted_context_bundle`.
- GREEN:
  - `cargo test -p vox-terminal-core --lib host::tests` — 7 passed (`context_bundle_redacts_secrets_and_caps_blocks` ok).
  - `cargo test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch::tests` — 8 passed (`term_context_rpc_returns_redacted_blocks` ok).
