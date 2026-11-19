---
phase: 73
phase_name: "mens-apple-silicon-quantize-hardening"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 73 Learnings: mens-apple-silicon-quantize-hardening

## Decisions

### Integrate the MENS Apple Silicon & Multi-Backend Hardening worktrees (me...
Integrate the MENS Apple Silicon & Multi-Backend Hardening worktrees (`mens/mac-lane-hardening` `5cb8f9dec`, `mens/qwen38-27b-run` `5a290248e`, `fix/cuda-mens-kv-cache-eos` `3a015a024`, `fix/metal-hybrid-fallback-routing` `8fb684d73`, and the 8 locked `.claude/worktrees/mens-*` branches: `mens-data-path`, `mens-decl-pairs`, `mens-sidecar-handshake`, `mens-step-corruption`, `mens-synth-stages`, `mens-train-progress`, `mens-training-fixes`, `mens-vox-source-pool`) into the unified stream without touching any locked worktrees on disk.

**Rationale:** Primary phase objective established in plan.
**Source:** 73-01-PLAN.md

## Lessons

### MENS-01 (Apple Silicon Unified-Memory Presets & Calibration)
**MENS-01 (Apple Silicon Unified-Memory Presets & Calibration)**: Measured Apple Silicon memory bounds integrated from `5a290248e` and `5cb8f9dec`, establishing memory ceilings and decimal GB reporting across M-series platforms.

**Context:** Verified in phase acceptance checks.
**Source:** 73-VERIFICATION.md

---

### MENS-02 (Metal/CUDA Hybrid Linear Attention & KV-Cache Fixes)
**MENS-02 (Metal/CUDA Hybrid Linear Attention & KV-Cache Fixes)**: Hybrid linear attention fallback and KV cache layer caching (`uses_linear_attention`, `Qwen35LayerCache`) ported across Metal and CUDA plugins.

**Context:** Verified in phase acceptance checks.
**Source:** 73-VERIFICATION.md

## Patterns

## Surprises

