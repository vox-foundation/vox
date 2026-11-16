---
phase: 405
status: fixed
reviewer: antigravity
reviewed_at: 2026-11-13T06:50:00Z
---

# Phase 405 Code Review

- **WR-01 (Fixed in `405-REVIEW-FIX.md`):** `CargoDepHygieneDetector::detect` in `crates/vox-code-audit/src/detectors/cargo_dep_hygiene.rs` previously checked `[dependencies]` headers without stripping inline `#` comments and skipped TOML sub-table headers such as `[dependencies.vox-repository]` and `[target.'cfg(unix)'.dependencies.flate2]`. Hardened with inline-comment stripping, `subtable_dep_key`, unified `check_dep`, and `flags_subtable_and_commented_header_dependencies` unit test (`4 passed; 0 failed` in `0.00s`).
- **IN-01 (Verified):** Confirmed `contracts/reports/v68-structural-dag-speedup.v1.json` (85 lines), `contracts/ci/fan-in-snapshot.v1.json` (`vox-compiler: 20`, `vox-config: 40`, `vox-db: 27`, `vox-llm-egress: 4`, `vox-repository: 13`, `vox-sql: 3`), and `docs/src/architecture/layers.toml` (`max_workspace_deps = 50` matching 49 non-dev `vox-*` deps in `crates/vox-cli/Cargo.toml`) are consistent and exact.
- Verified zero `Cargo.lock` mutations (`00-core.md` §5 preserved).
- Verified `macos_linker_strip_missing` properly ignores `#` comment lines and strips TOML quotes so `[profile.release] panic = "abort"` is detected reliably without triggering false positives on comments in `.cargo/config.toml` (`7/7` `fan_in_budget` and `7/7` `dev_loop_guard` tests passing).
