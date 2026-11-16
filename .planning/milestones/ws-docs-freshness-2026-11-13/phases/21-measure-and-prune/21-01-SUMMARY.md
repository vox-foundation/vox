---
phase: "21-measure-and-prune"
plan: "01"
status: completed
date: "2026-10-13"
requirements_met:
  - MEASURE-01
  - MEASURE-02
---

# Plan 21-01 Summary: Mention Extractor & Resolvers

## Execution Summary
Implemented the deterministic documentation mention extractor and ground truth resolvers within `crates/vox-doc-pipeline::claims`.

## Delivered Components
1. **Dependencies & Architecture (`crates/vox-doc-pipeline/Cargo.toml`, `src/lib.rs`)**:
   - Added workspace dependencies `pulldown-cmark`, `regex`, `serde`, `serde_yaml`, `walkdir`, and `sha2`.
   - Exposed `pub mod claims;` in library root.
2. **Core Types (`crates/vox-doc-pipeline/src/claims/types.rs`)**:
   - `MentionKind`: `FilePath`, `CrateName`, `RustSymbol`, `CliCommand`, `EnvVar`, `ConfigKey`.
   - `MentionRole`: `Prescribes` (normative, actionable deficits) vs `Mentions` (illustrative, descriptive, `vox:skip`, `text` fence).
   - `ResolutionStatus`: `Resolved`, `Missing`, `Retired`, `Ambiguous`.
   - `ExtractedMention` & `ResolvedMention` with deficit classification helper `is_actionable_deficit()`.
3. **AST Extractor (`crates/vox-doc-pipeline/src/claims/extractor.rs`)**:
   - Uses `pulldown-cmark::Parser` with `into_offset_iter()` and line offset indexing.
   - Accurately tokenizes inline backticked code and code fence contents.
   - Detects `// vox:skip`, `# vox:skip`, and `text`/`plaintext` fences, marking all enclosed mentions as `MentionRole::Mentions`.
   - Identifies descriptive/historical prose context to downgrade historical mentions.
4. **Deterministic Resolvers (`crates/vox-doc-pipeline/src/claims/resolver.rs`)**:
   - `GroundTruthRegistry` loads repository filesystem, `command-registry.yaml`, `env-vars.v1.yaml`, `registry.v1.yaml`, and `retired-symbols.v1.yaml`.
   - Resolves mentions against ground truth with precedence: checks `retired-symbols.v1.yaml` first, then validates paths, crates, CLI subcommands, env vars, config keys, and Rust symbol modules.
5. **Corpus Scanner (`crates/vox-doc-pipeline/src/claims/mod.rs`)**:
   - `scan_file` and `scan_corpus` scanning `docs/src` markdown files with support for targeted path filtering.

## Verification
- Unit & integration tests: `timeout 120s cargo test -p vox-doc-pipeline claims` passed (3/3 tests ok).
- Full crate test suite: `timeout 120s cargo test -p vox-doc-pipeline` passed (27/27 tests ok).
- Architecture & layers check: `timeout 120s cargo run -p vox-arch-check` passed (exit code 0).
