---
phase: 78-text-in-text-out-multi-spoke-verification-round-trip-evals
plan: 01
subsystem: corpus
tags: [mens, eval, roundtrip, voxlang, rust, shell]
requires: []
provides:
  - SpokeRoundtripReceipt, verify_voxlang_roundtrip, verify_rust_roundtrip, verify_shell_roundtrip
affects: [crates/vox-corpus]
key-files:
  created:
    - crates/vox-corpus/src/training/spoke_roundtrip.rs
  modified:
    - crates/vox-corpus/src/training/mod.rs
    - crates/vox-corpus/src/lib.rs
key-decisions:
  - "Implemented extract_fenced_or_raw to cleanly evaluate raw or fenced text-in/text-out model completions across all spokes."
requirements-completed: [SPOKE-01, SPOKE-02, SPOKE-03]
completed: 2026-10-16
---

# Phase 78 Plan 01: VoxLang, Rust Workspace, and Structured Shell Round-Trip Verifiers Summary

## Accomplishments
- Implemented `verify_voxlang_roundtrip` (`SPOKE-01`) combining `vox_compiler` frontend + `Interpreter` execution with AST construct coverage scoring.
- Implemented `verify_rust_roundtrip` (`SPOKE-02`) combining `syn::parse_file` + stub rejection with workspace source-policy detectors (banned crypto crate imports, hardcoded LLM vendor hostnames, raw secret env reads).
- Implemented `verify_shell_roundtrip` (`SPOKE-03`) combining `classify_shell_safety_floor`, secret-leak detection, delimiter balance, and JSON output schema validation.
