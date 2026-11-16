---
phase: 451-direct-publication-flywheel-and-scientia-studio
review_scope: plan 451-01 (Tasks 1, 2 & Wave F verification)
reviewed_at: 2026-11-15
reviewer: rust-verifier
status: approved
---

# Phase 451 Code & Architecture Review

## Scope Reviewed
- `crates/vox-research-shim/src/research/{mod.rs,discovery_bridge.rs,doc_synthesizer.rs}`
- `crates/vox-scientia/src/{review/{mod.rs,flywheel_gate.rs,flywheel_exec.rs},review_flow.rs,manuscript/{latex/{escape.rs,render.rs,bundle.rs},scaffold/architecture_ssot.rs}}`
- `crates/vox-orchestrator-mcp/src/gui_ops/{mod.rs,research.rs,scientia.rs,scientia_studio.rs}`
- `crates/vox-cli-research/src/scientia_phase_handlers.rs`
- `crates/vox-gui/src/{commands/scientia.rs,main.rs}` and `crates/vox-gui/ui/src/{transport.ts,api/scientia.ts,components/surfaces/{Publications,Scientia}/*}`

## Findings Summary
- **Blocking Issues:** `0`
- **Phase 451 Architectural / Policy Violations:** `0`
- **Seam Resolved in Coordination & Wave F:** Coordinator wired `pub mod doc_synthesizer;` into `crates/vox-research-shim/src/research/mod.rs`; `rust-verifier` aligned `flywheel_build_worthiness` to `vox_db_types::ScientiaClaimWithVerdict`, fixed `QueueSnapshot` field assertions in `review_flow.rs`, and migrated `scientia_phase_handlers.rs` lines 370–376 onto `vox_secrets::resolve_secret`.

## 1. Security, Publication Gate & Zero-`VoxDb` Studio
- **4-Layer Human Gate (`PUBL-01`):** `ExternalEmissionGate` validates SHA3-256 content-digest-bound `ApprovalToken`, `CriticGate`, `NoveltyVerdict`, and scholarly/offline attestation; `no_network_publish_symbol_in_review_flow` and `no_network_publish_symbol_in_review_ops` pass.
- **Hardened TeX & Bundles (`PUBL-03`):** `\end{verbatim}` injection neutralized, `-no-shell-escape` preserved, and tar/gzip bundles deterministically normalized with traversal rejection.
- **Zero-`VoxDb` / Zero-CLI-Spawn GUI (`STUD-01`, `STUD-02`):** `rg` across `Publications/` & `Scientia/` and `vox-gui` tests confirm `gui_cli_db_spawn_count == 0` and zero `VoxDb` handles in GUI handlers; UI guard suites pass (`101/101`).

## 2. File Size & Dependency Discipline (Phase 451 Additions)
| New Module | Observed LoC | Budget | Status |
|---|---|---|---|
| `crates/vox-research-shim/src/research/doc_synthesizer.rs` | `365` | `<400` | Pass |
| `crates/vox-scientia/src/review/flywheel_gate.rs` | `378` | `<400` | Pass |
| `crates/vox-scientia/src/review/flywheel_exec.rs` | `351` | `<400` | Pass |
| `crates/vox-orchestrator-mcp/src/gui_ops/scientia_studio.rs` (Phase 451 baseline) | `285` | `<400` | Pass (see Phase 452 review for post-diff expansion) |
| `crates/vox-gui/ui/src/components/surfaces/Scientia/WorthinessRadarPanel.tsx` | `216` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Scientia/DirectPublicationDrawer.tsx` | `338` | `<400` | Pass |

## Verdict
**APPROVED** — Phase 451 DB-hydrated flywheel, SSOT preservation, hardened artifact emitters, and zero-CLI-DB Axis Studio controls pass all backend, Tauri IPC, TypeScript, and UI guard checks.
