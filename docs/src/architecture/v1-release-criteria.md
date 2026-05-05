---
title: "v1-release-criteria"
category: "reference"
status: "current"
training_eligible: false
---
# Vox v1.0 Release Criteria (Hardened)

To reach a stable v1.0, the Vox foundation must satisfy the following machine-verified and human-audited criteria.

## 1. Production Validation
- **[CR-P1]** At least 3 "Marquee" applications must be deployed and live on OCI-compliant infrastructure with zero manual configuration.
- **[CR-P2]** 99.9% uptime for the `vox-mens` inference endpoint over a 7-day soak test.
- **[CR-P3]** Full "Zero-DX" deployment loop: `vox new web → vox deploy` must take under 120 seconds end-to-end.

## 2. Architectural Integrity
- **[CR-A1] K-Complexity Freeze**: The core compiler (`vox-compiler`) must maintain a cyclomatic complexity threshold under 15 for all primary lowering paths.
- **[CR-A2] Non-Null Boundary**: 100% of internal FFI and IPC interfaces must use non-null, machine-verified schemas (VoxProto v1).
- **[CR-A3] Crate Decoupling**: The workspace must maintain zero circular dependencies across the 10 core crates defined in `crates/_frozen.md`.
- **[CR-A4] Lifecycle Metadata Parity**: All orchestration contracts that affect model routing/providers must declare lifecycle metadata (`experimental`/`stable`/`deprecated`) and a migration window, with CI parity checks.

## 3. Performance & Efficiency
- **[CR-E1] Cold Start**: `vox run --interp` must initialize and execute a "Hello World" script in under 50ms on standard x86/ARM hardware.
- **[CR-E2] Bundle Size**: The standard "Marquee" application bundle (React + TanStack) must not exceed 800KB (gzip).
- **[CR-E3] Training Parity**: The native `vox-populi` training pipeline must achieve loss parity with reference PyTorch/LoRA implementations for the `vox-lang` corpus.

## 4. Agentic DX (Developer Experience)
- **[CR-D1] Planning Mode Fidelity**: AI agents must be able to execute a multi-step "Wave 2" plan with at least 85% success rate without human intervention.
- **[CR-D2] Self-Healing**: `vox repair` must successfully resolve 90% of syntactically valid but logically broken Vox programs identified during the v1 audit.
- **[CR-D3] Documentation Coverage**: 100% of `vox-cli` subcommands must have machine-readable help and associated `.vox` example scripts in the training corpus.

---
*Approved by Vox Foundation Council — April 2026*

