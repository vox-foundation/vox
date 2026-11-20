---
phase: 534-honest-ground-truth-resolver-and-drift-gate
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/vox-doc-pipeline/Cargo.toml
  - crates/vox-doc-pipeline/src/claims/extractor.rs
  - crates/vox-doc-pipeline/src/claims/resolver.rs
  - crates/vox-doc-pipeline/src/claims/gate.rs
  - crates/vox-doc-pipeline/src/claims/ledger.rs
  - crates/vox-doc-pipeline/src/claims/findings.rs
  - crates/vox-cli-research/src/audit.rs
  - contracts/documentation/retired-symbols.v1.yaml
  - contracts/reports/docs-freshness/corpus-ratchet.v1.json
  - contracts/reports/docs-freshness/ledger.v1.json
  - contracts/reports/docs-freshness/findings.v1.json
  - contracts/reports/docs-freshness/metrics.v1.json
  - docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md
autonomous: true
requirements:
  - TRUTH-01
  - TRUTH-02
  - TRUTH-03
  - TRUTH-04
must_haves:
  truths:
    - "Deleted crates resolve as ResolutionStatus::Retired with canonical replacements; workspace crates and catalog.toml bundle IDs resolve as Resolved."
    - "Missing workspace file paths, invalid multi-token CLI commands, and non-existent Rust module/export symbols resolve as Missing rather than prefix-bypassed Resolved."
    - "Frontmatter status legacy|deprecated|archived|research|roadmap and /archive/ paths downgrade extracted mentions to MentionRole::Mentions."
    - "Non-reader-facing status: current docs are gated by shrink-only contracts/reports/docs-freshness/corpus-ratchet.v1.json while baseline.v1.json remains zero-deficit and llms.txt targets remain verified."
    - "vox audit docs --write-findings writes live UTC timestamps and artifacts under contracts/reports/docs-freshness/ without corrupting contracts/reports/docs-reality-audit/."
  artifacts:
    - path: "crates/vox-doc-pipeline/src/claims/resolver.rs"
      provides: "Honest ground-truth registry with catalog.toml/repositories.yaml loading, retired crate mapping, real FilePath/CliCommand/RustSymbol AST verification"
    - path: "crates/vox-doc-pipeline/src/claims/gate.rs"
      provides: "is_active_corpus_page + CorpusRatchetFile shrink-only ratchet gate"
    - path: "contracts/reports/docs-freshness/corpus-ratchet.v1.json"
      provides: "Shrink-only deficit ratchet for non-reader-facing current docs"
    - path: "docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md"
      provides: "Updated SSOT documenting honest resolver, corpus ratchet, and docs/src/ voxlang.org canonical DB"
  key_links:
    - from: "crates/vox-doc-pipeline/Cargo.toml"
      to: "crates/vox-graph-reader"
      via: "L3 -> L1 dependency for AST symbol resolution"
    - from: "crates/vox-cli-research/src/audit.rs"
      to: "contracts/reports/docs-freshness/{findings,metrics,corpus-ratchet}.v1.json"
      via: "vox audit docs --write-findings / --check-drift / --tighten-baseline"
---

# Phase 534 Plan 01: Honest Ground-Truth Resolver, Unmasked Reality Audit & Expanded Drift Gate

<objective>
Close all resolution blind spots and stale report paths in `vox-doc-pipeline` and `vox audit docs` using TDD, refresh `contracts/reports/docs-freshness/` with honest baseline + shrink-only corpus ratchet, and update `docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md`.
</objective>

<tasks>

<task type="auto">
  <name>Task 1: Wire vox-graph-reader into vox-doc-pipeline and inspect exact symbol/CLI helpers</name>
  <files>crates/vox-doc-pipeline/Cargo.toml</files>
  <action>
    Inspect `crates/vox-graph-reader/src/ast.rs` public extraction APIs and CLI claim/CI inspection paths, add `vox-graph-reader = { workspace = true }` to `crates/vox-doc-pipeline/Cargo.toml`, and run `vox-arch-check` to verify `layers.toml` (L3 -> L1) compliance.
  </action>
  <verify>`timeout 900s cargo run -p vox-arch-check` and `timeout 900s cargo check -p vox-doc-pipeline` succeed.</verify>
</task>

<task type="auto" tdd="true">
  <name>Task 2: TDD Red — add failing tests for TRUTH-01, TRUTH-02, and TRUTH-03</name>
  <files>crates/vox-doc-pipeline/src/claims/extractor.rs, crates/vox-doc-pipeline/src/claims/resolver.rs, crates/vox-doc-pipeline/src/claims/gate.rs</files>
  <action>
    Add unit tests in `extractor.rs`, `resolver.rs`, and `gate.rs` proving current bypasses fail expectations:
    - Retired crates (`vox-arca`, `vox-codex`, `vox-schola`, `vox-oratio`, `vox-ludus`, `vox-codegen-rust`, `vox-pm`) -> `Retired` with canonical replacement; real workspace crates & `catalog.toml` bundle IDs (`vox-ml`, `vox-dev`) -> `Resolved`.
    - Missing file paths (`crates/vox-cli/src/totally_missing_file.rs`, `docs/src/missing_page.md`) -> `Missing`.
    - Invalid multi-token CLI commands (`vox audit docs-reality`, `vox mens fake-subcommand`) -> `Missing`; valid commands (`vox audit docs`, `vox ci ssot-drift`) -> `Resolved`.
    - Missing Rust symbols (`vox_compiler::codegen_rust`) -> `Missing`; real exported symbols (`vox_actor_runtime::llm`) -> `Resolved`.
    - Frontmatter `status: legacy|deprecated|archived|research|roadmap` downgrades extracted claims to `MentionRole::Mentions`.
    - `CorpusRatchetFile` / `evaluate_corpus_ratchet` detects regressions in non-reader-facing `status: current` pages.
    Run `timeout 900s cargo test -p vox-doc-pipeline --lib claims` and observe RED failures.
  </action>
  <verify>Tests compile and new assertions fail on pre-fix code.</verify>
</task>

<task type="auto">
  <name>Task 3: Implement honest extractor, ground-truth resolver, corpus ratchet, and CLI wiring</name>
  <files>crates/vox-doc-pipeline/src/claims/extractor.rs, crates/vox-doc-pipeline/src/claims/resolver.rs, crates/vox-doc-pipeline/src/claims/gate.rs, crates/vox-doc-pipeline/src/claims/ledger.rs, crates/vox-doc-pipeline/src/claims/findings.rs, crates/vox-cli-research/src/audit.rs, contracts/documentation/retired-symbols.v1.yaml</files>
  <action>
    1. `extractor.rs`: Parse frontmatter `status:` and downgrade `legacy|deprecated|archived|research|roadmap` (plus `/archive/` paths) to `MentionRole::Mentions`.
    2. `resolver.rs`: Drop 58 hardcoded crate aliases; dynamically load bundle IDs from `crates/vox-plugin-catalog/catalog.toml` and repo IDs from `.vox/repositories.yaml`; add `RETIRED_CRATE_REPLACEMENTS`; migrate any retired symbol already clear of active docs into `contracts/documentation/retired-symbols.v1.yaml`; verify `FilePath` against real disk existence (handling existing directory/file targets without blind prefix bypasses or `"nonexistent"` hack); tighten multi-token `CliCommand` validation against discovered subcommands; verify `RustSymbol` via workspace crate `src/` module & `vox-graph-reader` AST `pub` symbol scan.
    3. `gate.rs`, `ledger.rs`, `findings.rs`: Implement `is_active_corpus_page` + `CorpusRatchetFile` (`corpus-ratchet.v1.json`) shrink-only gating alongside `baseline.v1.json`; replace hardcoded `2026-10-13T00:00:00Z` with `chrono::Utc::now().to_rfc3339()`; check non-historical `status: current` pages in `ledger.rs`.
    4. `crates/vox-cli-research/src/audit.rs`: Update `--write-findings` paths to `contracts/reports/docs-freshness/{findings,metrics}.v1.json` and integrate `corpus-ratchet.v1.json` into `--check-drift`, `--generate-baseline`, and `--tighten-baseline`.
  </action>
  <verify>`rustfmt --edition 2024` on all changed `.rs` files; `timeout 900s cargo test -p vox-doc-pipeline` passes (including `openwiki_e2e_integration`); `timeout 900s cargo build -p vox-cli` succeeds.</verify>
</task>

<task type="auto">
  <name>Task 4: Refresh docs-freshness reports, fix any reader-facing/llms drift, and update SSOT docs</name>
  <files>docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md, contracts/reports/docs-freshness/ledger.v1.json, contracts/reports/docs-freshness/findings.v1.json, contracts/reports/docs-freshness/metrics.v1.json, contracts/reports/docs-freshness/corpus-ratchet.v1.json</files>
  <action>
    Update `docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md` (`TRUTH-04`), run `./target/debug/vox audit docs --update-ledger --write-findings` and seed/tighten `corpus-ratchet.v1.json` (remediating any minor reader-facing or `llms.txt` broken mentions so `baseline.v1.json` remains `[]` and `llms.txt` targets stay `verified`), then verify all drift/CI/lint gates pass.
  </action>
  <verify>Run `vox audit docs --check-drift`, `vox ci docs-reality-audit verify`, `vox ci retired-symbol-check`, and `vox-doc-pipeline --lint-only --paths architecture/openwiki-self-healing-docs-ssot-2026.md`.</verify>
</task>

</tasks>
