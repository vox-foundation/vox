---
phase: "22-deterministic-drift-gate-and-pr-impact"
plan: "01"
status: completed
date: "2026-10-13"
requirements_met:
  - DRIFT-02
  - DRIFT-03
---

# Plan 22-01 Summary: Freshness Ledger Data Model, Frontmatter Governance & Symbol-Span Hasher

## Execution Summary
Implemented the Freshness Ledger data model, normalized symbol-span content hasher (formatting/comment invariant), frontmatter governance extensions (`owner`, `review_cadence`, `code_anchors`), and hard linter enforcement rejecting hand-authored `last_verified` / `last_verified_commit`. Generated the initial `contracts/reports/docs-freshness/ledger.v1.json` across all 972 corpus pages.

## Delivered Components
1. **Freshness Ledger Schema & Model (`contracts/reports/docs-freshness/ledger.v1.schema.json`, `crates/vox-doc-pipeline/src/claims/ledger.rs`)**:
   - Machine-readable ledger schema defining per-page state (`verified`, `drifting`, `broken`, `unanchored`, `historical`), anchors, symbol-span hashes, and corpus metrics.
   - Initialized and generated `contracts/reports/docs-freshness/ledger.v1.json` (972 pages evaluated).
   - Marked `contracts/reports/docs-freshness/ledger.v1.json linguist-generated=true -diff` in `.gitattributes`.
2. **Normalized Symbol-Span Hasher (`crates/vox-doc-pipeline/src/claims/hasher.rs`)**:
   - Token-based whitespace and punctuation normalization eliminating false-positive drift churn caused by `rustfmt`, line breaks, and comments.
   - Canonical recursive JSON key sorting for contract targets (`.json`, `.yaml`, `.yml`).
   - AST regex symbol-span extraction (`fn`, `struct`, `enum`, `trait`, `type`, `actor`, `workflow`, `component`).
   - Target resolution for crates (`crates/<name>/Cargo.toml`), CLI commands (`contracts/cli/command-registry.yaml`), and env vars (`contracts/config/env-vars.v1.yaml`).
3. **Frontmatter Governance & Lint (`crates/vox-doc-pipeline/src/pipeline/lint.rs`, `types.rs`, `mod.rs`)**:
   - Allowed optional frontmatter fields `owner`, `review_cadence`, and `code_anchors`.
   - Hard error `LintKind::HandAuthoredLastVerified` if a contributor hand-authors `last_verified:` or `last_verified_commit:`.
   - Updated `docs/src/contributors/documentation-governance.md` with Code Anchors & Freshness Governance subsection.
   - Parity test `crates/vox-doc-pipeline/tests/frontmatter_governance_parity.rs` ensuring doc and linter stay in sync.
4. **CLI Integration (`crates/vox-cli/src/commands/audit.rs`)**:
   - Added `--update-ledger` flag to `vox audit docs` to recompute and write the Freshness Ledger with git HEAD commit attribution.

## Verification
- Unit & integration tests: `cargo test -p vox-doc-pipeline` passed (35/35 unit tests, 1/1 parity test).
- Parity test: `cargo test -p vox-doc-pipeline --test frontmatter_governance_parity` passed.
- Architecture check: `cargo run -p vox-arch-check` passed with 0 errors.
- Linter verification: `cargo run -p vox-doc-pipeline -- --lint-only` passed with 0 errors.
- Ledger generation: `cargo run -p vox-cli -- audit docs --update-ledger` completed with exit 0 and generated valid `contracts/reports/docs-freshness/ledger.v1.json`.
