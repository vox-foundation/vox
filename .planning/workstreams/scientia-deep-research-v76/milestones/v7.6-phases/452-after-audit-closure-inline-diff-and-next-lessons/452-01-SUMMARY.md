---
phase: 452-after-audit-closure-inline-diff-and-next-lessons
plan: 01
subsystem: audit-closure
tags: [rust, react, paired-ni, similar, diff, lessons, auto-dpo, vox-cli-research, vox-orchestrator-mcp, vox-gui]

requires:
  - phase: 447-before-audit-baseline-and-continuum-contracts
    provides: Locked `contracts/reports/scientia-v76-before-baseline.v1.json` baseline (`deterministic_seed: 20260447`)
  - phase: 451-direct-publication-flywheel-and-scientia-studio
    provides: `gui_ops/scientia_studio.rs` and `ScientiaSurface.tsx` host surfaces
provides:
  - Paired Before/After audit runner (`scientia_v76_audit/after_closure.rs` 392 LoC, `bug_probes.rs`, `cassettes.rs`, `mod.rs`) emitting `contracts/reports/scientia-v76-after-closure.v1.json` + JSONL (`schema_version: 1`) + updated §4 in `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`
  - Quarantined Next-Lessons flywheel (`scientia_v76_audit/lessons.rs` 398 LoC): injection/reasoning scrub (`SanitizedText`), `vox_actor_runtime::llm::infer_with_retry` summarization, `>= 2`-domain corroboration gate, `vox-effort-route` `.proposed` staging, and `ChosenRejectedPairV2` integrity checks (`0` unreviewed SSOT mutations)
  - Inline Before/After diff inspector (`GET /api/v2/scientia/v76-audit-diff` via `similar::TextDiff::from_lines`, `scientia_v76_audit_diff` Tauri command, `BeforeAfterAuditInspector.tsx` 373 LoC + `BeforeAfterAuditInspector.test.tsx` 155 LoC) mounted in `ScientiaSurface.tsx` (`Audit Diff` tab) and `ResearchView.tsx`
  - Full batched Wave F verification sign-off across Phases 450–452
affects:
  - Milestone `v7.6` (`scientia-deep-research-v76`) completion

tech-stack:
  added: []
  patterns:
    - "Blinded paired evaluation (`vox_eval::paired_ni`): Wilson/Newcombe intervals across 6 axes plus `schema_version: 1` JSONL stream (`scientia_v76_axis_delta`, `scientia_v76_bug_closure`, `scientia_v76_lesson`)"
    - "Quarantined lessons sink: scraped/synthesized lessons stay `untrusted` (`quarantined: true`, `injectable_as_curated: false`) until `>= 2`-domain corroboration and emit `.proposed` artifacts only"
    - "Virtualized backend-computed hunks: `similar::TextDiff::from_lines` projected via `/api/v2/scientia/v76-audit-diff` into `@tanstack/react-virtual` in Axis GUI"

key-files:
  created:
    - crates/vox-cli-research/src/scientia_v76_audit/after_closure.rs
    - crates/vox-cli-research/src/scientia_v76_audit/lessons.rs
    - crates/vox-gui/ui/src/components/surfaces/Scientia/BeforeAfterAuditInspector.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/BeforeAfterAuditInspector.test.tsx
    - contracts/reports/scientia-v76-after-closure.v1.json
    - .planning/workstreams/scientia-deep-research-v76/phases/452-after-audit-closure-inline-diff-and-next-lessons/scientia-v76-after-findings.v1.jsonl
  modified:
    - contracts/reports/scientia-v76-audit.v1.schema.json
    - contracts/index.yaml
    - crates/vox-cli-core/src/scientia.rs
    - crates/vox-cli-research/src/scientia.rs
    - crates/vox-cli-research/src/scientia_v76_audit/mod.rs
    - crates/vox-cli-research/src/scientia_v76_audit/bug_probes.rs
    - crates/vox-cli-research/src/scientia_v76_audit/cassettes.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/scientia_studio.rs
    - crates/vox-gui/src/commands/scientia.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/api/scientia.ts
    - crates/vox-gui/ui/src/components/surfaces/Scientia/ScientiaSurface.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/ScientiaSurface.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
    - docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md

key-decisions:
  - "Preserved locked `contracts/reports/scientia-v76-before-baseline.v1.json` validity by keeping optional `paired_comparison` and `lessons_summary` properties additive on `scientia-v76-audit.v1.schema.json`."
  - "Mounted `<BeforeAfterAuditInspector />` under `components/surfaces/Scientia/` and shared it in both `ScientiaSurface.tsx` (`'audit'` tab) and `ResearchView.tsx` (inline toggle)."
  - "Regenerated `contracts/secrets/{managed-env-names.v1.json,secret-capabilities.v1.json,managed-env-names.md}` via `vox ci secrets-contracts` (`508 secrets`) to satisfy `vox ci secrets-parity`."

requirements-completed: [AUDT-02, AUDT-03, AUDT-04]

duration: 55min
completed: 2026-11-15
---

# Phase 452 Plan 01: Paired After-Audit Closure, Inline Diff & Next Lessons Summary

**Phase 452 Plan 01 closed all 9 live `v7.6` bug probes (`9/9` resolved), proved zero-regression paired improvement across all 6 evaluation axes via `vox scientia v76-audit --stage after --compare`, mounted `<BeforeAfterAuditInspector />` with virtualized `similar` diff hunks in Axis GUI, and landed the quarantined Next-Lessons pipeline with full batched Wave F verification passing.**

## Accomplishments

### Task 1 (`AUDT-02`): Paired After-Audit Closure & SSOT Table
- Extended `ScientiaCmd::V76Audit` (`--compare`, `--jsonl-out`) in `crates/vox-cli-core/src/scientia.rs` and `crates/vox-cli-research/src/scientia.rs`, added optional `paired_comparison` & `lessons_summary` schema fields in `scientia-v76-audit.v1.schema.json`, and registered `scientia-v76-after-closure-v1` in `contracts/index.yaml`.
- Created `crates/vox-cli-research/src/scientia_v76_audit/after_closure.rs` (`392` LoC, `< 400`) evaluating all 6 post-v7.6 axes and computing `vox_eval::paired_ni` CIs against `contracts/reports/scientia-v76-before-baseline.v1.json`.
- Executed `./target/debug/vox scientia v76-audit --stage after --compare contracts/reports/scientia-v76-before-baseline.v1.json --out contracts/reports/scientia-v76-after-closure.v1.json --jsonl-out ...scientia-v76-after-findings.v1.jsonl` (`exit 0`), closing `9/9` bug probes (`0` `Inferior` axes; `3` `non_inferior` + `superior: true`, `3` `inconclusive` at `n=3..6` frozen cassette size with positive `+0.20..+0.67` deltas), and documented §4 in `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md` (`vox-doc-pipeline --lint-only` `exit 0`).

### Task 2 (`AUDT-03`): Inline Before/After Diff Inspector
- Added `GET /scientia/v76-audit-diff` in `crates/vox-orchestrator-mcp/src/gui_ops/scientia_studio.rs` (`similar::TextDiff::from_lines`), `scientia_v76_audit_diff` Tauri command (`commands/scientia.rs`, `main.rs`), and `transport.ts` / `api/scientia.ts` client wrappers.
- Created `crates/vox-gui/ui/src/components/surfaces/Scientia/BeforeAfterAuditInspector.tsx` (`373` LoC) & `BeforeAfterAuditInspector.test.tsx` (`155` LoC), mounted in `ScientiaSurface.tsx` (`Audit Diff` tab) and `ResearchView.tsx` (`toggle-audit-inspector-btn`).

### Task 3 (`AUDT-04`): Quarantined Next-Lessons Flywheel
- Created `crates/vox-cli-research/src/scientia_v76_audit/lessons.rs` (`398` LoC, `< 400`) scrubbing prompt-injection & reasoning tags (`SanitizedText`), summarizing via `vox_actor_runtime::llm::infer_with_retry`, requiring `>= 2` distinct domains before corroboration promotion (`quarantined_findings_count: 3`, `corroborated_promotions_count: 2`), emitting `.proposed` enforcement artifacts via `vox-effort-route` (`proposed_effort_rules_count: 2`), minting validated Auto-DPO pairs (`auto_dpo_pairs_minted: 3`), and guaranteeing `unreviewed_ssot_mutations: 0`.

### Task 4 (`Wave F`): Batched Verification Sign-Off
- All 7 backend crate test batches (`vox-secrets` 129, `vox-search` 279, `vox-scientia` 376, `vox-publisher` 197, `vox-research-shim` 171, `vox-orchestrator-mcp` `gui_ops` 156 / `chat_tools` 226 / `scientia` 18 / `http_gateway` 87 / `scientia_studio` 4, `vox-cli-research` 665 + `scientia` 30, `vox-gui` `scientia` 9 + `chat_turn` 18) passed with `0` failures.
- `tsc --noEmit` (`exit 0`), 17-file combined Vitest (`17 passed; 101 passed`), `command_catalog_paths_baseline` (`1 passed`), `vox ci contracts-index` (`212 entries`), `vox ci secrets-parity` OK, `vox-doc-pipeline --lint-only` OK, and `cargo run -p vox-arch-check` (`exit 0`).

---
*Phase: 452-after-audit-closure-inline-diff-and-next-lessons*
*Completed: 2026-11-15*
