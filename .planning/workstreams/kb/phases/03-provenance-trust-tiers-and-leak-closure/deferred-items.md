# Deferred items (Phase 3)

## 03-03

- `cargo nextest run -p vox-db --features legacy-import legacy_export_covers_all_baseline_tables` fails on HEAD for reasons that predate this plan: `LEGACY_EXPORT_TABLES` (crates/vox-db/src/codex_legacy.rs) lacks `kb_index_state` (Phase 2), `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events` and `web_cache` (baselines 91-93), none of which is in `LEGACY_EXPORT_SKIP_TABLES`. `kb_findings` IS present on both sides (this plan added it). Fix: add the five names in sorted position (or list them in the skip set) in a plan that owns the legacy export.

## 03-05

- **FIXED in `604a35abd` (03-04 follow-up, "typed count accessors for capture tests").** `vox ci sql-surface-guard --all` used to exit 1 on HEAD because `crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs:42` and `crates/vox-search/tests/web_capture_test.rs:65` issued raw `.connection().query(..)` row counts; both now use `VoxDb::search_corpus_row_counts`. Nothing left to do here.

## 03-12

- `legacy_export_covers_all_baseline_tables` (feature `legacy-import`) still fails exactly as recorded under 03-03: the same five names (`kb_index_state`, `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events`, `web_cache`) and no others. 03-12 verified the three legacy VoxKB tables (`knowledge_bases`, `kb_entries`, `kb_routing_rules`) appear on both sides of the comparison, so the quarantine move did not widen the gap.
- Local build hazard (not a code problem): with the default `[profile.dev] incremental = true`, rustc hung at ~0% CPU on `vox_db` (and, in the first run, `vox_corpus`) on this host, deterministically, for 25+ minutes. `CARGO_INCREMENTAL=0` (and `LEAN_CTX_DISABLED=1` to bypass the output wrapper) made the same `cargo check -p vox-db` finish in 1 m 12 s. Later plans on this host should export `CARGO_INCREMENTAL=0`.
