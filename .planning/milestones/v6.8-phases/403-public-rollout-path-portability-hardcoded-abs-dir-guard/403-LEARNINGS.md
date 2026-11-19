---
phase: 403
phase_name: "public-rollout-path-portability-hardcoded-abs-dir-guard"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "STATE.md"
---

# Phase 403 Learnings: public-rollout-path-portability-hardcoded-abs-dir-guard

## Decisions

### Scrubbed machine-local hardcoded absolute paths (/Users/<user>, /home/<u...
Scrubbed machine-local hardcoded absolute paths (`/Users/<user>`, `/home/<user>`, `C:\Users\<user>`, `.gemini/antigravity/brain/`, `/private/tmp/`, `/private/var/folders/`) across workspace scripts, GUI CV flywheel specs & checked-in reports (`vox` and `../vox-axis`), Rust tests (`std::env::temp_dir()`), contracts, and 17 active `docs/src/` files.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 403-01-SUMMARY.md

---

### Preserved synthetic cross-OS test fixtures (/home/alice, /Users/bob, C:\...
Preserved synthetic cross-OS test fixtures (`/home/alice`, `/Users/bob`, `C:\Users\bob`) inside unit-test contexts.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 403-01-SUMMARY.md

---

### Added FS_HARDCODED_ABS_DIR = "vox/fs/hardcoded-abs-dir" in crates/vox-co...
Added `FS_HARDCODED_ABS_DIR = "vox/fs/hardcoded-abs-dir"` in `crates/vox-code-audit/src/diagnostics/catalog.rs` and `HardcodedAbsDirDetector` in `crates/vox-code-audit/src/detectors/hardcoded_abs_dir.rs` (registered in `crates/vox-code-audit/src/detectors/mod.rs`, `rule_count() == 67`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 403-01-SUMMARY.md

---

### Added O(1) file-level and line-level byte pre-filters so clean files ski...
Added O(1) file-level and line-level byte pre-filters so clean files skip allocation/pattern scanning; classified machine-local home/scratch paths as `Severity::Error` (Tier 1) and unanchored OS root paths in non-test/non-const production code as `Severity::Warning` (Tier 2).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 403-01-SUMMARY.md

---

### Optimized MagicValueDetector (crates/vox-code-audit/src/detectors/magic_...
Optimized `MagicValueDetector` (`crates/vox-code-audit/src/detectors/magic_value.rs`) by replacing 3 per-line regexes (`comment_re`, `const_def_re`, `test_attr_re`) with `trim_start()` prefix checks and gating `port_rule`/`ip_rule`/`path_rule`/`db_conn_rule` behind byte checks.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 403-01-SUMMARY.md

## Lessons

### total: 4 passed: 4 issues: 0 pending: 0 skipped: 0
total: 4 passed: 4 issues: 0 pending: 0 skipped: 0

**Context:** Observed during user acceptance testing.
**Source:** 403-UAT.md

---

### [none]
[none]

**Context:** Observed during user acceptance testing.
**Source:** 403-UAT.md

## Patterns

## Surprises

