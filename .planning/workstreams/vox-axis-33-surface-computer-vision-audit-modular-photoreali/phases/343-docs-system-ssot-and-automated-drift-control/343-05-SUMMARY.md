---
phase: 343-docs-system-ssot-and-automated-drift-control
plan: 05
status: complete
wave: 5
selected_options:
  task-2: option-a
---

# Phase 343 Plan 05 Summary

Hardened docs guard inputs, eliminated silently dead branches, enforced mandatory inputs for link checking and retired symbol scanning, and applied the fatal allowlist decision (Option A).

## 1. Key Accomplishments

1. **Strict Guard Inputs (`docs.rs`):**
   - Extracted `fn workspace_crate_names(root: &Path) -> Result<HashSet<String>>` to strictly read package names from `crates/*/Cargo.toml`, bubbling errors instead of silently skipping unreadable manifests.
   - Removed the dead check on deleted `docs/src/architecture/orphan-surface-inventory.md`, along with `parse_workspace_crate_block` and `CRATE_LINE_RE`. Crate-roster parity is handled by `vox-arch-check` Rule 12.
   - Replaced the retired architecture index check with an immediate recreation guard on `docs/src/architecture/architecture-index.md`, matching the `research-index.md` check.
   - Removed the stale `training_eligible` warning keyed on hand-written `last_updated` (which the linter forbids).

2. **Strict Guard Inputs (`retired_symbol_check.rs`):**
   - Made `AGENTS.md`, `GEMINI.md`, `CLAUDE.md`, and `.github/copilot-instructions.md` mandatory root guides (missing file causes fatal error).
   - Added scanning of `.github/copilot-instructions.md` with markdown table and retired-section carve-outs.
   - Made `.cursor/rules` mandatory and required at least one `.mdc` file.
   - Converted directory walk errors in `docs/` to fatal errors rather than silently skipping.

3. **Strict Inputs & Fatal Allowlist (`check_links.rs`):**
   - Target not found or missing `docs/src` now returns a fatal error rather than skipping.
   - Replaced warn-only allowlist with Option A (fatal allowlist semantics):
     - Existence check runs first.
     - Broken link with expired allowlist entry fails with `allowlist_expired`.
     - Broken link with unexpired allowlist entry is suppressed and marked used.
     - Valid link with matching allowlist entry fails with `allowlist_unneeded`.
     - In repo mode, unused allowlist entries fail with `allowlist_unused`.
   - Cleared all 96 stale/unused/unneeded allowlist entries from `contracts/documentation/link-allowlist.v1.yaml` (0 broken links across 9,183 markdown links in docs and root guides).
   - Updated `link-allowlist.v1.yaml` header noting fatal expiry and unused/unneeded failure behavior.

4. **Documentation & Ledger:**
   - Updated `docs/src/architecture/docs-system-ssot.md` §4 (`vox ci check-links` and `Retired-symbol check` rows) and deleted the "Silently dead check" bullet from §6.
   - Added `check_links` / `retired_symbol_check` / `check_docs_ssot` input checks to `docs/src/contributors/detector-coverage-ledger.md`.
   - Verified doc lint passes with `cargo run -p vox-doc-pipeline -- --lint-only`.

## 2. Decision Checkpoint (Task 2)

- **Selected Option:** `option-a` ("Recommended: fatal expiry, and resolve every current entry at its source now (rewrite the dead link as an inline-code path or point it at the current file, then delete the entry); also fail on entries whose link now resolves or that match no link").
- **Resolution:** All 96 allowlist entries were either unneeded (3 resolved to live targets) or unused (93 were already converted to code spans or never referenced as markdown links). All entries deleted from `link-allowlist.v1.yaml`, leaving 0 entries.

## 3. Mutation Proofs

1. **Root Guide Missing (Task 1):**
   - Temporarily moved `GEMINI.md` to `GEMINI.md.343bak`.
   - Ran `cargo test -p vox-cli-ci retired_symbol_check::tests::real_tree`.
   - Verified fatal failure with message: `missing mandatory guide: /Users/brbrainerd/dev/vox/GEMINI.md`.
   - Restored `GEMINI.md` and confirmed byte-identical restoration.

2. **Allowlist Expiry (Task 3):**
   - Appended `[broken-test-link](no_such_file_mut_343.md)` to `AGENTS.md` and added an expired allowlist entry (`expires: "2000-01-01"`) in `link-allowlist.v1.yaml`.
   - Ran `cargo test -p vox-cli-ci check_links::tests::real_tree`.
   - Verified fatal failure with message: `[allowlist_expired]`.
   - Restored `AGENTS.md` and `link-allowlist.v1.yaml` and verified byte-identical restoration.

## 4. Verification Results

- `cargo test -p vox-cli-ci check_links`: 10 passed, 0 failed.
- `cargo test -p vox-cli-ci retired_symbol`: 9 passed, 0 failed.
- `cargo test -p vox-cli --lib run_body_helpers::docs`: 13 passed, 0 failed.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/docs-system-ssot.md contributors/detector-coverage-ledger.md`: passed.
