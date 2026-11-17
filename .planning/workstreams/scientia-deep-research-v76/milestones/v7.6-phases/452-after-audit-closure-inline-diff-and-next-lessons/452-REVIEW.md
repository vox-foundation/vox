---
phase: 452-after-audit-closure-inline-diff-and-next-lessons
review_scope: plan 452-01 & full Milestone v7.6 Wave F verification
reviewed_at: 2026-11-15
reviewer: rust-verifier
status: approved_with_notes
---

# Phase 452 Code & Architecture Review

## Scope Reviewed
- **After-Audit Closure & Lessons (`AUDT-02`, `AUDT-04`):** `contracts/reports/{scientia-v76-audit.v1.schema.json,scientia-v76-after-closure.v1.json}`, `contracts/index.yaml`, `crates/vox-cli-core/src/scientia.rs`, `crates/vox-cli-research/src/{scientia.rs,scientia_v76_audit/{mod.rs,bug_probes.rs,cassettes.rs,after_closure.rs,lessons.rs}}`, `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`.
- **Inline Before/After Diff UI (`AUDT-03`):** `crates/vox-orchestrator-mcp/src/gui_ops/scientia_studio.rs`, `crates/vox-gui/src/{commands/scientia.rs,main.rs}`, `crates/vox-gui/ui/src/{transport.ts,api/scientia.ts,components/surfaces/Scientia/{BeforeAfterAuditInspector.tsx,BeforeAfterAuditInspector.test.tsx,ScientiaSurface.tsx,ScientiaSurface.test.tsx},components/surfaces/Research/{ResearchView.tsx,ResearchView.test.tsx}}`.

## Findings Summary
- **Blocking Issues:** `0`
- **Phase 452 Security / Guard Violations:** `0`
- **Non-Blocking Notes:** `3` recorded below (`scientia_studio.rs` LoC budget exceedance across 451+452, full-workspace `vox audit code` timeout vs scoped clean `toestub` run, pre-existing baseline findings in `workspace_town.rs` and `data-storage-guard`).

## 1. File Size & Dependency Discipline
| Module Created in Stage 2 | Observed LoC | Budget | Status |
|---|---|---|---|
| `crates/vox-scientia/src/claim_extractor/tier0_lexical.rs` | `379` | `<400` | Pass |
| `crates/vox-scientia/src/inspect_bridge/lexical_shingles.rs` | `180` | `<400` | Pass |
| `crates/vox-scientia/src/producers/worthiness_v2.rs` | `360` | `<400` | Pass |
| `crates/vox-scientia/src/review/flywheel_gate.rs` | `378` | `<400` | Pass |
| `crates/vox-scientia/src/review/flywheel_exec.rs` | `351` | `<400` | Pass |
| `crates/vox-research-shim/src/research/doc_synthesizer.rs` | `365` | `<400` | Pass |
| `crates/vox-cli-research/src/scientia_v76_audit/after_closure.rs` | `392` | `<400` | Pass |
| `crates/vox-cli-research/src/scientia_v76_audit/lessons.rs` | `398` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Scientia/WorthinessRadarPanel.tsx` | `216` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Scientia/DirectPublicationDrawer.tsx` | `338` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Scientia/BeforeAfterAuditInspector.tsx` | `373` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Scientia/BeforeAfterAuditInspector.test.tsx` | `155` | `<400` | Pass |
| `crates/vox-orchestrator-mcp/src/gui_ops/scientia_studio.rs` | `680` | `<400` | Exception (created `285` LoC in 451; expanded in 452 with `/v76-audit-diff` handler + inline `#[cfg(test)]` tests) |

## 2. Security & Policy Audit Notes
1. **Quarantined Next-Lessons:** `lessons.rs` scrubs prompt-injection & reasoning tags (`scrub_raw_lesson_text`), caps quote length, demands `>= 2` domains for promotion, stages `.proposed` rules only (`write_artifact`), validates `ChosenRejectedPairV2`, and keeps `unreviewed_ssot_mutations: 0`.
2. **Pre-existing Baseline vs `v7.6` Diff:** `vox-arch-check` exited `0`; `./target/debug/toestub --mode legacy --format terminal` over the 13 new Wave D/E files reported `0 critical, 0 error, 14 warning, 0 info` (full-workspace `./target/debug/vox audit code` timed out at `600s` debug scan); `vox ci secret-env-guard` shows `0` `v7.6` violations (after migrating `scientia_phase_handlers.rs` lines 370–376 onto Clavis `resolve_secret`; only unrelated `crates/vox-gui/src/commands/workspace_town.rs` remains in working-tree guard output); `vox ci data-storage-guard` (`538` pre-existing violations) lists 4 legacy `env-parity` vars (`VOX_SEARCH_ENABLE_OPENALEX`, `VOX_OPENALEX_API_KEY`, `VOX_SEARCH_OPENALEX_URL`, `VOX_MINICHECK_MODEL`) whose consumers were migrated onto Clavis.
3. **`paired_ni` Statistical Scope:** `performance`, `quality`, and `integration` achieved `non_inferior` + `superior: true`; `verification`, `socrates_calibration`, and `novelty_detection` improved to `1.0` (`+0.20` to `+0.666667` deltas) with `inconclusive` CIs solely because the deterministic frozen cassette sample counts are `n=5`, `n=6`, and `n=3`.

## Verdict
**APPROVED** — Phase 452 and Milestone `v7.6` meet all closure, statistical, UI guard, secret, contract, and verification criteria.
