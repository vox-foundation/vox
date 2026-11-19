---
phase: 320
phase_name: "train-the-sizes-size-03"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 2
  patterns: 4
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 320 Learnings: train-the-sizes-size-03

## Decisions

### D-01: Filter qwen3_code overlay rows to methods containing qlora; sort m...
D-01: Filter qwen3_code overlay rows to methods containing qlora; sort measured rows ascending by floor_mb (2000, 12000, 20000, 60000) followed by unmeasured rows (1.7B, 4B) in Qwen3 size order.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 320-01-SUMMARY.md

---

### D-03/D-04/D-05
D-03/D-04/D-05: Strict decision precedence (floor_unmeasured -> does_not_fit -> base_snapshot_absent -> trained / train_not_run) with check_out_path containment guard.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 320-01-SUMMARY.md

---

### D-02/D-06: Expose only --hub-root and --runs-root on vox mens size-train...
D-02/D-06: Expose only --hub-root and --runs-root on vox mens size-train (no --free-mb or --adapter flags) so production callers always take one live reading via read_accelerator_free_mb.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 320-02-SUMMARY.md

## Lessons

### CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --lib size_tra...
`CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --lib size_train_variant_parses -- --test-threads=8` → `1 passed; 0 failed`

**Context:** Verified in phase acceptance checks.
**Source:** 320-VERIFICATION.md

---

### CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --features gpu...
`CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --features gpu --lib size_train_ -- --test-threads=8` → `7 passed; 0 failed`

**Context:** Verified in phase acceptance checks.
**Source:** 320-VERIFICATION.md

## Patterns

### Injected free_mb and adapter map allow full unit testing of size_train.v...
Injected free_mb and adapter map allow full unit testing of size_train.v1 records without GPU hardware, network calls, or weight downloads.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 320-01-SUMMARY.md

---

### Model-free snapshot completeness and manifest pin verification
Model-free snapshot completeness and manifest pin verification

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 320-01-SUMMARY.md

---

### PopuliMensTail::SizeTrain compiles under both default and gpu feature se...
PopuliMensTail::SizeTrain compiles under both default and gpu feature sets.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 320-02-SUMMARY.md

---

### Single FnOnce free-memory reading seam for deterministic CLI testing
Single FnOnce free-memory reading seam for deterministic CLI testing

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 320-02-SUMMARY.md

## Surprises

