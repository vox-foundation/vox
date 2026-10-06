# Deferred items (Phase 3)

## 03-03

- `cargo nextest run -p vox-db --features legacy-import legacy_export_covers_all_baseline_tables` fails on HEAD for reasons that predate this plan: `LEGACY_EXPORT_TABLES` (crates/vox-db/src/codex_legacy.rs) lacks `kb_index_state` (Phase 2), `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events` and `web_cache` (baselines 91-93), none of which is in `LEGACY_EXPORT_SKIP_TABLES`. `kb_findings` IS present on both sides (this plan added it). Fix: add the five names in sorted position (or list them in the skip set) in a plan that owns the legacy export.
