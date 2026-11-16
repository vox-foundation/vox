# Deferred items (Phase 3)

## 03-03

- **Status:** resolved
- RESOLVED in `crates/vox-db/src/codex_legacy.rs`: `kb_index_state`, `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events` added to `LEGACY_EXPORT_TABLES`, and `web_cache` added to `LEGACY_EXPORT_SKIP_TABLES`.

## 03-05

- **Status:** resolved
- **FIXED in `604a35abd` (03-04 follow-up, "typed count accessors for capture tests").** `vox ci sql-surface-guard --all` passes via `VoxDb::search_corpus_row_counts`.

## Gap closure (03-14..03-16)

- **Status:** resolved
- **WR-01 (RESOLVED):** `crates/vox-search/src/trust.rs` checks both `updated-by` and `update-to` on Crossref work records.
- **WR-02 (RESOLVED):** `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs` sanitizes zero-width characters via `SanitizedText::new` before running `vox_redact::redact_owned`.
- **WR-03 & WR-04 (RESOLVED):** `capture_finding` sanitizes `provenance` JSON strings recursively through `clean()`.
- **WR-06 & WR-07 (RESOLVED in Phase 42):** `crates/vox-search/src/kb_inbox.rs` implements inbox promotion, rejection, and GC; `codex_legacy.rs` table list is complete.
- **D-07a (RESOLVED):** memory search leg pinned as Advisory and verified by end-to-end tests.

## 03-12

- **Status:** resolved
- `legacy_export_covers_all_baseline_tables` resolved in `crates/vox-db/src/codex_legacy.rs`; build hazard notes documented.
