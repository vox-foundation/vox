---
phase: 405
status: fixed
fixed_at: 2026-11-13T06:50:00Z
---

# Phase 405 Code Review Fixes

## Fixed Items

1. **`WR-01` (`crates/vox-code-audit/src/detectors/cargo_dep_hygiene.rs`):**
   - Added `subtable_dep_key(header)` to extract package names from TOML `[dependencies.<pkg>]` and `[target.<cfg>.dependencies.<pkg>]` sub-tables.
   - Stripped trailing `#` inline comments from `[...]` headers before section matching.
   - Extracted shared `CargoDepHygieneDetector::check_dep` so standard table entries and sub-table headers share identical validation across `vox-repository` forbids, pure `-types` crates, `vox-cli`, `vox-compiler`, and `vox-codegen`.
   - Added unit test `flags_subtable_and_commented_header_dependencies`.
   - Formatted with `rustfmt --edition 2024 crates/vox-code-audit/src/detectors/cargo_dep_hygiene.rs`.
2. **`IN-01` (`contracts/ci/fan-in-snapshot.v1.json`, `docs/src/architecture/layers.toml`, `contracts/reports/v68-structural-dag-speedup.v1.json`):**
   - Verified all three artifacts on disk match actual workspace state (`vox-sql` has `3` workspace dependents: `vox-codegen`, `vox-codegen-ts`, `vox-cli-research`; `vox-cli` has `49` non-dev `vox-*` dependencies under `[dependencies]`, fitting `max_workspace_deps = 50`).

## Verification

- `timeout 900s cargo test -p vox-code-audit --lib cargo_dep_hygiene`: **4 passed; 0 failed** (`0.00s`).
- `timeout 900s cargo test -p vox-cli-ci --lib fan_in_budget`: **7 passed; 0 failed** (`0.03s`).
