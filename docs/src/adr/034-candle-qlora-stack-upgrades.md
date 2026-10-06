---
title: "ADR 034 — Candle / QLoRA stack upgrades"
description: "Decision record: Candle, peft-rs, and qlora-rs resolve to one unified version set, verified by a fail-closed hosted CUDA-toolchain compile job."
category: "Architecture Decisions (ADRs)"
status: "current"
training_eligible: true
training_rationale: "ADR text encodes stack-upgrade policy and risk gates for MENS/Populi GPU paths; useful for model grounding on dependency discipline."

schema_type: "TechArticle"
---

# ADR 034 — Candle / QLoRA stack upgrades

## Context

- Mens / Populi training paths depend on **Candle**, **qlora-rs** (vendored patch), **peft-rs**, and transitive stacks (`zip`, CUDA kernels).
- The workspace dependency audit showed **duplicate majors** (e.g. `zip`) that cannot be collapsed without coordinated Candle + HF ecosystem bumps.
- GPU builds (`mens-candle-cuda`, NVCC) require explicit CI coverage.

## Decision

- **No ad-hoc Candle major bump** inside manifest-normalization PRs.
- Track **one upgrade initiative** with: MSRV check, CUDA release-build smoke, and lockfile diff review for `zip` / `rand` / `half` transitive shifts.
- Keep using workspace pins + patches documented in root `Cargo.toml`.

## Status

**Accepted (2026-10-01)** — closed against measured repository state and a successful hosted CUDA-toolchain compile run.

## Implementation (Phase 6, ML-01)

The dedicated upgrade train found the stack **already unified**, so it made no speculative major bump:

- **Candle 0.10.2** (`candle-core` / `candle-nn` / `candle-transformers` pinned to `0.10` in `[workspace.dependencies]`; `candle-metal-kernels` patched from `patches/candle-metal-kernels-0.10.2`).
- **peft-rs 1.0.3** (patched from `patches/peft-rs-1.0.3`).
- **qlora-rs 1.0.5** (patched from `patches/qlora-rs-1.0.5`).

Evidence:

- `crates/vox-cli/tests/ml_dependency_health.rs` reads locked `cargo metadata` and fails if more than one version of any of these crates resolves.
- `vox ci cuda-features` / `vox ci cuda-release-build` were repaired to target the current owners, `vox-plugin-speech` and `vox-ml-cli`.
- `nightly.yml`'s fail-closed `ml-cuda-health` job (`ubuntu-latest`, pinned CUDA 12.6.3 toolkit, `CUDA_COMPUTE_CAP=80`) passed: [run 36821415478](https://github.com/vox-foundation/vox/actions/runs/36821415478) at head SHA `bc3214504f981e31817da3d9b4dbeae1b6cb0892`.

**Coverage limit.** This evidence validates CUDA compilation and linkage only. It does **not** exercise a CUDA device or prove physical-GPU runtime behavior; that remains a local check (see [runner contract](../ci/runner-contract.md)).

## Consequences

- Duplicate transitive versions outside this stack may persist; they are tracked in the [workspace dependency audit](../architecture/workspace-dependency-audit-2026.md).
- A future Candle major bump is a new upgrade train under the same gates, not an incidental change.
