---
phase: 451-direct-publication-flywheel-and-scientia-studio
plan: 01
subsystem: scientia-studio
tags: [rust, react, typescript, flywheel, approval-token, ssot, latex, arxiv, nanopub, vox-scientia, vox-gui]

requires:
  - phase: 450-calibrated-verification-anti-ouroboros-and-discovery
    provides: `WorthinessSignalsV2`, `AtomicNoveltyScorer`, and `OfflineAttested` scholarly fallback
provides:
  - Non-wiping `write_research_doc` (`artifact_json` preserved via `db.get_research_artifact`) and DB-hydrated SSOT synthesis (`doc_synthesizer.rs` 365 LoC, `architecture_ssot.rs`)
  - 4-layer human `ApprovalToken` `ExternalEmissionGate` (`flywheel_gate.rs` 378 LoC) + idempotent DB-backed `flywheel_advance_from_db` (`flywheel_exec.rs` 351 LoC) + hardened LaTeX/deterministic tar.gz bundles (`escape.rs`, `render.rs`, `bundle.rs`)
  - `/api/v2/scientia/*` Studio endpoints (`gui_ops/scientia_studio.rs`), 9 typed `BackendCalls` Tauri commands (`commands/scientia.rs`, `main.rs`), zero `execute_command` spawns, `<WorthinessRadarPanel />` (216 LoC) & `<DirectPublicationDrawer />` (338 LoC)
affects:
  - Phase 452 (`BeforeAfterAuditInspector` mounting in `ScientiaSurface` and `/scientia/v76-audit-diff` on `scientia_studio.rs`)

tech-stack:
  added: []
  patterns:
    - "Zero-manual-JSON flywheel: `flywheel_advance_from_db` executes all stages strictly from `publication_id` and `VoxDb` rows"
    - "4-layer emission gate: SHA3-256 content-digest `ApprovalToken` + `CriticGate` + `NoveltyVerdict` + `TrustLookup` / `OfflineAttested` without importing network-publish symbols in review flows"
    - "Pure BackendCalls IPC: `vox-gui` routes 100% of Scientia/Publications actions through `api_v2_*` Tauri commands with zero `VoxDb` handles or `execute_command` subprocesses"

key-files:
  created:
    - crates/vox-research-shim/src/research/doc_synthesizer.rs
    - crates/vox-scientia/src/review/flywheel_gate.rs
    - crates/vox-scientia/src/review/flywheel_exec.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/scientia_studio.rs
    - crates/vox-gui/ui/src/components/surfaces/Scientia/WorthinessRadarPanel.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/DirectPublicationDrawer.tsx
  modified:
    - crates/vox-research-shim/src/research/mod.rs
    - crates/vox-research-shim/src/research/discovery_bridge.rs
    - crates/vox-scientia/src/review/mod.rs
    - crates/vox-scientia/src/review_flow.rs
    - crates/vox-scientia/src/manuscript/latex/escape.rs
    - crates/vox-scientia/src/manuscript/latex/render.rs
    - crates/vox-scientia/src/manuscript/latex/bundle.rs
    - crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/mod.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/research.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs
    - crates/vox-cli-research/src/scientia_phase_handlers.rs
    - crates/vox-gui/src/commands/scientia.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/api/scientia.ts
    - crates/vox-gui/ui/src/components/surfaces/Publications/PublicationsView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Publications/PublicationsView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/ClaimsView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/ScientiaSurface.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/ScientiaSurface.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/DiscoveryInbox.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/DiscoveryInbox.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/DiscoveryReview.tsx
    - crates/vox-gui/ui/src/components/surfaces/Scientia/DiscoveryReview.test.tsx

key-decisions:
  - "Preserved existing `artifact_json` in `write_research_doc` by reading `db.get_research_artifact(session_id)` prior to storing doc-publish updates."
  - "Added Clavis resolution in `scientia_phase_handlers.rs` (`SecretId::VoxMinicheckEndpoint`, `VoxMinicheckModel`) during Wave F so `vox ci secret-env-guard` shows zero violations in v7.6 files."
  - "Maintained zero network-publish symbols in `flywheel_gate.rs` and `gui_ops/scientia.rs` (`no_network_publish_symbol_in_review_*`)."

requirements-completed: [PUBL-01, PUBL-02, PUBL-03, PUBL-04, STUD-01, STUD-02]

duration: 50min
completed: 2026-11-15
---

# Phase 451 Plan 01: Direct Publication Flywheel & Zero-`VoxDb` Scientia Studio Summary

**Phase 451 Plan 01 shipped the DB-hydrated Direct Publication (`> Nanopublication`) flywheel (`PUBL-01..04`) behind a 4-layer human `ApprovalToken` gate and eliminated all CLI-DB `execute_command` subprocess spawns across Axis Scientia/Publications Studio (`STUD-01..02`).**

## Accomplishments

### Task 1 (`p451-flywheel`): SSOT Preservation, 4-Layer Gate & Hardened Flywheel (`PUBL-01..04`)
- Fixed `write_research_doc` in `crates/vox-orchestrator-mcp/src/gui_ops/research.rs` to preserve `artifact_json` via `db.get_research_artifact(session_id)` (`write_research_doc_preserves_existing_artifact_json`), and created `crates/vox-research-shim/src/research/doc_synthesizer.rs` (`365` LoC, `< 400`, wired into `research/mod.rs`) + upgraded `architecture_ssot.rs` to emit `vox-doc-pipeline`-clean SSOT docs with `kb:summary` and `vox-diagram` fences.
- Updated `discovery_bridge.rs` to preserve `EventCandidateClass`, `novelty_evidence_bundle_id`, and `confidence_json`; created `review/flywheel_gate.rs` (`378` LoC, 4-layer `ExternalEmissionGate`) and `review/flywheel_exec.rs` (`351` LoC, idempotent `flywheel_advance_from_db`); hydrated `nanopub_build`, queue reply windows, and token-attributed USD cost in `review_flow.rs` and `scientia_phase_handlers.rs`.
- Hardened LaTeX `\end{verbatim}` neutralization, Unicode normalization (`escape.rs`, `render.rs`), and deterministic `mtime=0` tar/gzip bundling + traversal checks (`bundle.rs`).
- Mounted `/api/v2/scientia/publications/{id}/claims`, `/scientia/venue-recommend`, and `/scientia/publications/{id}/flywheel-advance` via `gui_ops/scientia_studio.rs`.

### Task 2 (`p451-studio-ui`): Zero-`execute_command` Tauri IPC & Radix Studio (`STUD-01`, `STUD-02`)
- Eliminated all `'execute_command'` CLI-DB spawns from `PublicationsView.tsx` and `ClaimsView.tsx`; wired 9 typed `BackendCalls` Tauri commands in `commands/scientia.rs` & `main.rs`, `transport.ts`, and `api/scientia.ts`.
- Created `<WorthinessRadarPanel />` (`216` LoC, `< 400`) and `<DirectPublicationDrawer />` (`338` LoC, `< 400`) using Radix `data-slot` primitives and mounted them across `ScientiaSurface.tsx`, `DiscoveryReview.tsx`, `DiscoveryInbox.tsx`, and `PublicationsView.tsx`.

## Verification Receipts
- `vox-orchestrator-mcp` (`gui_ops`: `156 passed`, `scientia`: `18 passed`, `scientia_studio`: `4 passed`), `vox-scientia --lib` (`376 passed; 1 ignored`), `vox-research-shim --lib` (`171 passed`), `vox-cli-research --lib scientia` (`30 passed`), `vox-gui --bins scientia` (`9 passed`).
- `pnpm --dir crates/vox-gui/ui exec tsc --noEmit`: `exit 0`; combined Vitest (`17` suites, `101` tests, including `PublicationsView`, `ClaimsView`, `DiscoveryInbox`, `DiscoveryReview`, `ScientiaSurface`, `noRawNativeControlsGuard`, `zeroEmojiAndDingbat`, `transportIpcGuard`): `17 passed (17)`, `101 passed (101)`.

---
*Phase: 451-direct-publication-flywheel-and-scientia-studio*
*Completed: 2026-11-15*
