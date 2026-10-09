# Deferred items (Phase 3)

## 03-03

- `cargo nextest run -p vox-db --features legacy-import legacy_export_covers_all_baseline_tables` fails on HEAD for reasons that predate this plan: `LEGACY_EXPORT_TABLES` (crates/vox-db/src/codex_legacy.rs) lacks `kb_index_state` (Phase 2), `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events` and `web_cache` (baselines 91-93), none of which is in `LEGACY_EXPORT_SKIP_TABLES`. `kb_findings` IS present on both sides (this plan added it). Fix: add the five names in sorted position (or list them in the skip set) in a plan that owns the legacy export.

## 03-05

- **FIXED in `604a35abd` (03-04 follow-up, "typed count accessors for capture tests").** `vox ci sql-surface-guard --all` used to exit 1 on HEAD because `crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs:42` and `crates/vox-search/tests/web_capture_test.rs:65` issued raw `.connection().query(..)` row counts; both now use `VoxDb::search_corpus_row_counts`. Nothing left to do here.

## Gap closure (03-14..03-16), deferred by the 2026-10-08 gap-planning scope

These are not planned in the gap-closure plans. None of them blocks a ROADMAP Phase 3 success criterion (per 03-VERIFICATION.md dispositions).

- **WR-01** Crossref retraction check reads `update-to` instead of `updated-by` (`crates/vox-search/src/trust.rs`). It must be fixed before the first promotion consumer; owner Phase 9.
- **WR-02** findings `clean()` redacts before it sanitizes, so a secret split by zero-width characters survives redaction (`kb_tools/findings.rs`). Security hardening: reorder to sanitize-then-redact.
- **WR-03** the findings snapshot is raw text, and its hash covers the untruncated text (`kb_tools/findings.rs`). Hardening.
- **WR-04** `kb_findings.provenance` is unvalidated JSON that bypasses `SanitizedText`. Hardening.
- **WR-06** legacy migration skips orphaned `kb_entries` (inner JOIN) and aborts on the first per-entry error. Add a `failed` counter and a count invariant.
- **WR-07** inbox and snapshot growth is unbounded. Owner Phase 9 (triage/GC).
- **IN-01..IN-05** info-level review items.
- `legacy_export_covers_all_baseline_tables` is still red, as recorded under 03-03 and 03-12; it needs a plan that owns `codex_legacy.rs`.
- **D-07a (user decision, 2026-10-09):** the memory search leg is not re-tiered in Phase 3. That leg serves MEMORY.md and the daily logs, written by the model-callable `memory_store`/`memory_log`; its lines stay Advisory and keep feeding the chat preamble. Revisit with Phase 7 spotlighting. 03-15 pins the current behavior in its end-to-end test.
- Noted while planning 03-15: `knowledge_nodes.node_type` defaults to 'concept' (an Advisory type) in the DDL. No writer omits the type today, but a raw insert would land Advisory. Flipping the default is a DDL and digest change, so it is left for a schema-owning plan.

## 03-12

- `legacy_export_covers_all_baseline_tables` (feature `legacy-import`) still fails exactly as recorded under 03-03: the same five names (`kb_index_state`, `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events`, `web_cache`) and no others. 03-12 verified the three legacy VoxKB tables (`knowledge_bases`, `kb_entries`, `kb_routing_rules`) appear on both sides of the comparison, so the quarantine move did not widen the gap.
- Local build hazard (not a code problem): with the default `[profile.dev] incremental = true`, rustc hung at ~0% CPU on `vox_db` (and, in the first run, `vox_corpus`) on this host, deterministically, for 25+ minutes. `CARGO_INCREMENTAL=0` (and `LEAN_CTX_DISABLED=1` to bypass the output wrapper) made the same `cargo check -p vox-db` finish in 1 m 12 s. Later plans on this host should export `CARGO_INCREMENTAL=0`.
