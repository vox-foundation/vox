---
phase: 534-honest-ground-truth-resolver-and-drift-gate
workstream: openwiki-self-healing-docs-ssot-v91
status: passed
verified_at: "2026-11-19T09:54:16Z"
requirements:
  - TRUTH-01
  - TRUTH-02
  - TRUTH-03
  - TRUTH-04
---

# Phase 534 Verification Report: Honest Ground-Truth Resolver, Unmasked Reality Audit & Expanded Drift Gate

## Verdict: PASSED

All 4 Phase 534 requirements (`TRUTH-01`..`TRUTH-04`) verified with observed tool output and test runs after the final code/doc edits.

## Verified Checks & Observed Output

| Check | Command | Outcome | Observed Result |
|---|---|---|---|
| Architecture Layers | `timeout 900s cargo run -p vox-arch-check` | PASSED (`exit 0`) | `vox-doc-pipeline` (L3) -> `vox-graph-reader` (L1) complies with `layers.toml` |
| Claims Unit Tests | `timeout 900s cargo test -p vox-doc-pipeline --lib claims` | PASSED (`exit 0`) | `36 passed; 0 failed` (after final `resolver.rs` edit) |
| Full Pipeline Tests | `timeout 900s cargo test -p vox-doc-pipeline` | PASSED (`exit 0`) | `147` lib + `4` production audit + `1` fence_issues + `1` frontmatter parity + `5` `openwiki_e2e_integration` passed |
| OpenWiki E2E Suite | `timeout 900s cargo test -p vox-doc-pipeline --test openwiki_e2e_integration` | PASSED (`exit 0`) | `5 passed; 0 failed` (`baseline.v1.json` remains `0` deficits) |
| Full Corpus Reality Audit | `timeout 900s ./target/debug/vox audit docs --update-ledger --write-findings --seed-corpus-ratchet` | PASSED (`exit 0`) | `1,102` pages, `39,705` mentions (`25,677` resolved, `768` retired, `4,273` missing, `8,987` ambiguous), `1,076` actionable deficits; ledger: `1,210` pages (`180` verified, `253` drifting, `209` broken, `568` historical, `0` unanchored prior to `llms.txt` attestations); `corpus-ratchet.v1.json` seeded with `970` deficits (`baseline.v1.json` kept at `0`) |
| `llms.txt` Ledger Parity | `rg` across `contracts/reports/docs-freshness/ledger.v1.json` | PASSED (`exit 0`) | All `19` `docs/src/` pages linked by `docs/src/.well-known/llms.txt` have `"state": "verified"` |
| Corpus Drift Gate | `timeout 900s ./target/debug/vox audit docs --check-drift` | PASSED (`exit 0`) | `✅ Documentation Drift Gate PASSED` (`Active ratcheted/baselined deficits: 970, Resolved: 0, Expired suppressions: 0`) |
| Docs Reality Audit Contract | `timeout 900s ./target/debug/vox ci docs-reality-audit verify` | PASSED (`exit 0`) | `docs-reality-audit verify OK (10 claims, 0 findings)` |
| Retired Symbol Guard | `timeout 900s ./target/debug/vox ci retired-symbol-check` | PASSED (`exit 0`) | `retired-symbol-check OK` |
| Pipeline Markdown Lint | `timeout 900s ./target/debug/vox-doc-pipeline --lint-only --paths architecture/openwiki-self-healing-docs-ssot-2026.md,contributors/agent-instruction-architecture.md,reference/mcp-tool-registry-contract.md,reference/cli.md` | PASSED (`exit 0`) | `vox-doc-pipeline lint complete — no hard errors.` |

## Requirement Traceability

- **TRUTH-01:** Hardcoded 58-crate alias allowlist removed from `crates/vox-doc-pipeline/src/claims/resolver.rs`. Crates loaded from `crates/*/Cargo.toml` (`[package]` and `[[bin]]` targets); bundle IDs loaded from `crates/vox-plugin-catalog/catalog.toml`; repository/graph IDs loaded from `.vox/repositories.yaml` and `contracts/retrieval/vox-graph-corpora.v1.yaml`; MCP tool names loaded from `contracts/mcp/tool-registry.canonical.yaml`; deleted crates mapped via `RETIRED_CRATE_REPLACEMENTS`. (Full migration of pre-existing deleted crate strings into `retired-symbols.v1.yaml` deferred to Phase 539 cleanup so `retired-symbol-check` stays green.)
- **TRUTH-02:** `FilePath` prefix bypass removed; `CliCommand` restricted to active `vox-cli` commands plus live clap aliases and parent-group subcommand completeness; `RustSymbol` verified via cached `vox-graph-reader` AST/module/`pub use` inspection (`CrateSymbolIndex`); `extractor.rs` downgrades `/archive/` and frontmatter `status: legacy|deprecated|archived|research|roadmap|superseded` to `MentionRole::Mentions`.
- **TRUTH-03:** `findings.rs` emits live UTC `Utc::now().to_rfc3339()` timestamps; `vox audit docs --write-findings` writes directly to `contracts/reports/docs-freshness/{findings,metrics}.v1.json`; `gate.rs` + `audit.rs` enforce `evaluate_corpus_ratchet` with shrink-only `contracts/reports/docs-freshness/corpus-ratchet.v1.json`; `ledger.rs` evaluates broken mentions across all `is_active_corpus_page` `status: "current"` pages.
- **TRUTH-04:** `docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md` updated to document `docs/src/` (`voxlang.org`) as canonical database plus honest resolver and full-corpus ratchet; broken/drifting `llms.txt` targets remediated and attested.
