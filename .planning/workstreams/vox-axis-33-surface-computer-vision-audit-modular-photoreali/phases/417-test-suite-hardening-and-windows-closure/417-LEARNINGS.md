---
phase: 417
phase_name: "test-suite-hardening-and-windows-closure"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 5
  patterns: 2
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 417 Learnings: test-suite-hardening-and-windows-closure

## Decisions

### Exposed reasoning_compare/detect.rs unit tests under default mens-base v...
Exposed `reasoning_compare/detect.rs` unit tests under default `mens-base` via `#[cfg(all(test, not(feature = "gpu"), feature = "mens-base"))]` shim in `crates/vox-ml-cli/src/commands/mens/mod.rs` so model-free snapshot/escape tests execute without GPU backends.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 417-01-SUMMARY.md

---

### Exposed spoke_base_resolver.rs unit tests under zero features in vox-pop...
Exposed `spoke_base_resolver.rs` unit tests under zero features in `vox-populi` and narrowed `training_presets_yaml_contract.rs` from `mens-train` to `mens`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 417-01-SUMMARY.md

---

### Re-wired orphaned feedback module (#[cfg(feature = "database")] pub mod ...
Re-wired orphaned `feedback` module (`#[cfg(feature = "database")] pub mod feedback;`) in `vox-actor-runtime`, fixed 8-argument `log_interaction` bit-rot, and un-ignored `test_feedback_log_and_thumbs_up_round_trip`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 417-01-SUMMARY.md

## Lessons

### Critical / High
**Critical / High:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 417-REVIEW.md

---

### Medium: 0 (Caught and resolved an unreferenced orphan module crates/vox-...
**Medium:** `0` (Caught and resolved an unreferenced orphan module `crates/vox-actor-runtime/src/feedback.rs` during `HYGIENE-TEST-03` test verification, fixing its bit-rotted 7-arg `VoxDb::log_interaction` callsite to match the current 8-arg `(input_tokens, output_tokens)` signature.)

**Context:** Surfaced in structured phase code review.
**Source:** 417-REVIEW.md

---

### Low: 0
**Low:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 417-REVIEW.md

---

### timeout 900s cargo test --locked -p vox-ml-cli --lib reasoning_compare →...
`timeout 900s cargo test --locked -p vox-ml-cli --lib reasoning_compare` → `exit 0` (`11 passed; 0 failed`, including `snapshot_with_all_indexed_shards_is_present` under default features without `--features gpu`).

**Context:** Verified in phase acceptance checks.
**Source:** 417-VERIFICATION.md

---

### timeout 900s cargo test --locked -p vox-populi --lib spoke_base → exit 0...
`timeout 900s cargo test --locked -p vox-populi --lib spoke_base` → `exit 0` (`29 passed; 0 failed` with zero features enabled).

**Context:** Verified in phase acceptance checks.
**Source:** 417-VERIFICATION.md

## Patterns

### Feature-independent contract test shims (#[cfg(all(test, not(feature = ....
Feature-independent contract test shims (`#[cfg(all(test, not(feature = ...)))] #[path = ...]`) for model-free filesystem and YAML validation

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 417-01-SUMMARY.md

---

### Structured owner:<domain> sunset:<date|never> <category>
Structured `owner:<domain> sunset:<date|never> <category>: <reason>` metadata on every `#[ignore]` attribute

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 417-01-SUMMARY.md

## Surprises

