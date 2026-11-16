# Phase 80: Real-Backend E2E Verification Harness (Zero-Mock Surface Proof) - Context

**Gathered:** 2026-10-16
**Status:** Ready for verification

<domain>
## Phase Boundary

Deliver Milestone v1.6 Phase 80 (`REAL-01..04`):
- **REAL-01**: Hermetic real-backend test harness (`crates/vox-gui/tests/real_backend_e2e_harness.rs` & `crates/vox-gui/ui/e2e/realBackendFixture.ts`) booting a live `vox-orchestrator-d` + temporary `VoxDb` + HTTP/IPC Tauri command bridge on an ephemeral localhost port.
- **REAL-02**: Real-backend E2E verification for the `Chat` + `Loquela` workflow (`vox_chat_session_*`, `orch.oversight_get`/`set`, `@kb` scope toggle, `TurnCostReceipt` badge) against live Rust handlers.
- **REAL-03**: Real-backend E2E verification for `Terminal` (`SessionHost` `term.open` -> `term.submit` -> `OSC-633` block completion -> `"term.context"` redaction) and `⌘K` Universal Palette / `ExplorerView.tsx` read-only CLI execution against the live Rust backend.
- **REAL-04**: `honestyScan.ts` (`surfaceHonesty.guard.test.ts` and `vox ci gui-honesty`) passes across all 32 GUI surfaces in `crates/vox-gui/ui/src/components/surfaces/`, enforcing zero dead handlers, zero placeholder prose, and zero unguarded mock data in production builds.
</domain>

<decisions>
## Implementation Decisions

### Hermetic Real-Backend Bridge (`REAL-01..03`)
- `crates/vox-gui/src/gateway_ipc.rs` and `crates/vox-gui/src/workspace_snapshot.rs` provide the live JSON-RPC / Tauri command bridge exercised by `crates/vox-gui/tests/real_backend_e2e_harness.rs` and Playwright E2E fixtures.
- `Chat`/`Loquela`, `Terminal` (`SessionHost`), `Palette` (`⌘K`), `Explorer`, and `Library` execute real round-trips against `VoxDb` and `vox-orchestrator-d` without relying on static browser-only mocks.

### Zero-Mock Surface Honesty Gate (`REAL-04`)
- `crates/vox-gui/ui/src/utils/honestyScan.ts` and `crates/vox-gui/ui/src/utils/surfaceHonesty.guard.test.ts` scan all 32 surfaces and views for dead handlers, fake timers, and placeholder prose, enforced in CI via `vox ci gui-honesty`.
</decisions>
