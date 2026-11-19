---
phase: 6
phase_name: "model-routing-transparency-ml-dependency-health"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 12
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 6 Learnings: model-routing-transparency-ml-dependency-health

## Decisions

### Added ModelScore-to-frontier coverage for reliability, cost, and latency...
Added ModelScore-to-frontier coverage for reliability, cost, and latency tradeoffs.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-01-SUMMARY.md

---

### Pinned scoreboard and explain behavior for dominated, low-sample, and un...
Pinned scoreboard and explain behavior for dominated, low-sample, and unobserved rows while preserving router order.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-01-SUMMARY.md

---

### Added a source-boundary regression guard against Pareto reporting helper...
Added a source-boundary regression guard against Pareto reporting helpers entering `models::select` production code.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-01-SUMMARY.md

---

### Corrected the model-autonomic SSOT to identify models::select::decide an...
Corrected the model-autonomic SSOT to identify `models::select::decide` and registry routing as the production path, with ADR-046 documenting Pareto behavior as reporting-only.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-01-SUMMARY.md

---

### Added the ml-cuda-health job to .github/workflows/nightly.yml
Added the `ml-cuda-health` job to `.github/workflows/nightly.yml`: `ubuntu-latest`, 180-minute cap, `Jimver/cuda-toolkit@v0.2.36` with the CUDA 12.6.3 toolkit sub-package (12.4 has no Ubuntu 24.04 apt package; first hosted run failed on `cuda-12-4`), `CUDA_COMPUTE_CAP=80`, `nvcc --version` gate, then `cargo test --no-run -p vox-plugin-mens-candle-cuda --features cuda`, `vox ci cuda-features`, and `vox ci cuda-release-build`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-03-SUMMARY.md

---

### Added ml_cuda_health_is_hosted_compile_only_and_fail_closed to crates/vo...
Added `ml_cuda_health_is_hosted_compile_only_and_fail_closed` to `crates/vox-cli/tests/ci_workflow_contract.rs`, rejecting `continue-on-error`, `VOX_CI_ALLOW_CUDA_SKIP`, `--features cpu`, `if: false`, and `self-hosted` inside the job.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-03-SUMMARY.md

---

### The job is documented in-line as compile-only
The job is documented in-line as compile-only: it does not exercise a CUDA device or runtime kernels.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-03-SUMMARY.md

---

### ADR-034 rewritten as an accepted closure record
ADR-034 rewritten as an accepted closure record: Candle 0.10.2, peft-rs 1.0.3, qlora-rs 1.0.5 were already unified, so no speculative bump; cites the `ml_dependency_health` test, the repaired `vox-plugin-speech` / `vox-ml-cli` targets, and the successful hosted run.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-04-SUMMARY.md

---

### docs/src/ci/runner-contract.md
`docs/src/ci/runner-contract.md`: canonical CUDA package names corrected; `ml-cuda-health` (CUDA 12.6.3 toolkit, `CUDA_COMPUTE_CAP=80`) documented as fail-closed, compile-only evidence distinct from local skip behavior.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-04-SUMMARY.md

---

### workspace-dependency-audit-2026.md Candle/peft/qlora entry updated to re...
`workspace-dependency-audit-2026.md` Candle/peft/qlora entry updated to resolved reality.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-04-SUMMARY.md

---

### Intel: ADR-034 classification and decisions entry locked with acceptance...
Intel: ADR-034 classification and decisions entry locked with acceptance evidence. ADR-046 left unchanged.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-04-SUMMARY.md

---

### REQUIREMENTS/ROADMAP
REQUIREMENTS/ROADMAP: MODEL-01 and ML-01 complete; Phase 5 progress row corrected to 7/7 (its execution commits exist; the row was stale).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 06-04-SUMMARY.md

## Lessons

## Patterns

## Surprises

