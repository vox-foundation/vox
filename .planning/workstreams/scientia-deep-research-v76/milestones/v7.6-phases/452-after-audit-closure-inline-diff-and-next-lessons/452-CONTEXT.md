---
phase: 452
title: "After-Audit Closure, Inline Diff, and Next Lessons"
slug: "after-audit-closure-inline-diff-and-next-lessons"
milestone: "scientia-deep-research-v76"
status: "ready"
---

# Phase 452: After-Audit Closure, Inline Diff, and Next Lessons — Context

**Gathered:** 2026-11-15
**Status:** Ready for planning
**Requirements:** AUDT-02, AUDT-03, AUDT-04

<domain>
## Phase Boundary

Phase 452 closes Milestone `v7.6` (`scientia-deep-research-v76`) by proving empirical improvement across all 6 evaluation axes against Phase 447's frozen baseline (`AUDT-02`), exposing inline Before/After diff inspection in Axis GUI (`AUDT-03`), extracting `SanitizedText`-scrubbed quarantined "next lessons" (`AUDT-04`), and running batched Wave F verification across Phases 450–452:
- **Paired After-Audit & `vox_eval::paired_ni` Closure (`AUDT-02`):** `crates/vox-cli-research/src/scientia_v76_audit/{mod.rs, compare.rs}` (new `< 400`-LoC module), `crates/vox-cli-research/src/scientia_phase_handlers.rs`, `contracts/reports/scientia-v76-after-closure.v1.json`, `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`.
- **Inline Before/After Diff Inspector (`AUDT-03`):** `crates/vox-orchestrator-mcp/src/gui_ops/{scientia_studio.rs, research.rs}` (using existing `similar` workspace dep), `crates/vox-gui/src/commands/scientia.rs`, `crates/vox-gui/ui/src/{transport.ts, api/scientia.ts}`, `crates/vox-gui/ui/src/components/surfaces/Research/{BeforeAfterAuditInspector.tsx, BeforeAfterAuditInspector.test.tsx, ResearchView.tsx}` and `Scientia/ScientiaSurface.tsx`.
- **Quarantined Next-Lesson Flywheel (`AUDT-04`):** `crates/vox-scientia/src/lessons/{mod.rs, extractor.rs}` (`< 400` LoC), `crates/vox-db/src/store/ops_memory/kb_findings.rs`, `crates/vox-effort-route/src/pipeline.rs`.

Zero edits to `Cargo.toml`, `Cargo.lock`, `package.json`, `layers.toml`, or `crate-edges.allow.v1.json`.
</domain>

<decisions>
## Implementation Decisions

### Paired After-Audit & `vox_eval::paired_ni` Closure (`AUDT-02`)
- **Same frozen corpus + stratified live comparison:** `vox scientia v76-audit --stage after --compare contracts/reports/scientia-v76-before-baseline.v1.json` runs the identical deterministic Phase 447 cassette corpus and bug probes (`bug_probes.rs`, `cassettes.rs`), invokes `vox_eval::paired_ni` (already in `vox-cli-research/Cargo.toml`) to compute confidence intervals across all 6 axes, and writes `contracts/reports/scientia-v76-after-closure.v1.json` plus `vox audit`-compatible JSONL findings (`schema_version` + per-finding discriminator).
- **Strict regression invariants:** Verify `none_to_novel_false_positive_rate == 0.0`, `kv_cache_prefix_stability >= 0.99`, `probe_gui_cli_db_spawn_count == 0`, `probe_doc_publish_wipes_artifact == false`, `probe_mock_verifier_default == false`, and `0` regression across performance, quality, verification, integration, Socrates calibration, and novelty detection.
- **SSOT closure table:** Update `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md` with the measured Before → After table and verify with `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`.

### Inline Before/After Diff Inspector (`AUDT-03`)
- **Backend diff projection without new Cargo deps:** `crates/vox-orchestrator-mcp/Cargo.toml` already depends on `similar = { workspace = true }`. Compute structured unified/inline hunks (`Before`, `After`, `6-axis delta`, claim/report diffs) in `gui_ops/scientia_studio.rs` and serve them over `/api/v2/scientia/audit-diff` + typed `BackendCalls` Tauri IPC (`0` `VoxDb` handles in `vox-gui`).
- **UI component (`< 400` LoC):** Implement `<BeforeAfterAuditInspector />` using `@tanstack/react-virtual` and Radix `data-slot` primitives (`0` raw `<select>`/`<input type="range">`, `0` emojis/dingbats) mounted in `ResearchView.tsx`, `ScientiaSurface.tsx`, and Chat turn receipt views.

### Quarantined Next-Lesson Flywheel (`AUDT-04`)
- **Model-agnostic extraction + `SanitizedText` quarantine:** Extract structured "next lessons" from post-audit and post-research runs strictly via `vox_actor_runtime::llm::infer_with_retry`. Scrub all text fields into `vox_db::kb_text::SanitizedText` and persist into `VoxDb::kb_findings` (`FindingOrigin::Untrusted` / pending quarantine) until `>= 2`-run/domain corroboration in `kb_inbox`.
- **Proposed-only routing & MENS Auto-DPO:** Emit `.proposed` enforcement candidates through `vox-effort-route` and verified repair/grounding pairs for MENS Auto-DPO. **Never** auto-mutate `AGENTS.md` or canonical `docs/src/architecture/` files from unreviewed lessons.

### Architectural Invariants & Batched Wave F
- Zero `Cargo.toml`/`package.json` edits; `< 400` LoC per new file; Clavis secrets; `vox-crypto` hashing; `vox_actor_runtime::llm::infer_with_retry`; batched Wave F serialized verification across Phases 450–452.
</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-cli-research/src/scientia_v76_audit/mod.rs` already defines `AuditStage::{Before, After}`, `ScientiaV76AuditReport`, `build_v76_audit_report`, and schema validation against `contracts/reports/scientia-v76-audit.v1.schema.json`.
- `crates/vox-eval/src/paired_ni.rs` provides the paired non-inferiority/superiority statistical helpers already linked into `vox-cli-research/Cargo.toml`.
- `crates/vox-db/src/store/ops_memory/kb_findings.rs` (`NewKbFinding`, `KbFindingRow`) already enforces `SanitizedText` and `FindingOrigin` / `FindingStatus` quarantine semantics.
- `crates/vox-orchestrator-mcp/Cargo.toml` already includes `similar = { workspace = true }` for backend inline text diff hunks.
</code_context>

<specifics>
## Parallel Wave Boundaries & Batched Wave F

- **Execution split:** File-disjoint tracks for (1) `scientia_v76_audit` paired `paired_ni` closure + SSOT table, (2) `BeforeAfterAuditInspector` backend/UI diff viewer, and (3) `SanitizedText`-quarantined lesson extractor + `vox-effort-route` / MENS Auto-DPO emission.
- **Wave F (single batched verification across Phases 450–452):** Run serialized `cargo test` per touched crate (`vox-secrets`, `vox-search`, `vox-scientia`, `vox-research-shim`, `vox-publisher`, `vox-orchestrator-mcp`, `vox-cli-research`, `vox-gui`), `tsc --noEmit`, Vitest component + guard suites, `vox scientia v76-audit --stage after --compare ...`, `vox audit code`, `vox ci secret-env-guard`, `vox ci secrets-parity`, `vox-arch-check`, and `vox-doc-pipeline --lint-only`.
</specifics>

<deferred>
## Deferred Ideas

- Cross-peer Populi iroh QUIC mesh gossip for `NoveltyEvidenceBundle` and `FindingCandidate` digests (`DISC-F01` in `v8.0+`).
</deferred>

---
*Phase: 452-after-audit-closure-inline-diff-and-next-lessons*
*Context gathered: 2026-11-15*
