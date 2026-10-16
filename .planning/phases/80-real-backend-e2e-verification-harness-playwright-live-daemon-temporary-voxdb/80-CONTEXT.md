# Phase 80 Context: Real-Backend E2E Verification Harness (Zero-Mock Surface Proof)

**Phase:** 80 (`v1.6` Priority 2 — `REAL-01..04`)
**Date:** 2026-10-16
**Requirements:** `REAL-01`, `REAL-02`, `REAL-03`, `REAL-04`

## 1. Objective

Prove that Vox's core GUI surfaces and backend dispatch paths work end-to-end against real stateful backend infrastructure (`ServerState` + `VoxDb` + HTTP/IPC command bridge + `SessionHost` + `command_catalog`) rather than relying solely on in-page `installTauriMock` browser stubs, and enforce 32-surface static honesty invariants across `crates/vox-gui/ui/src/components/surfaces/`.

## 2. Technical Architecture

1. **Rust Real-Backend Integration Harness (`crates/vox-gui/tests/real_backend_harness.rs` — `REAL-01`, `REAL-02`, `REAL-03`)**:
   - Boots a real `vox_orchestrator_mcp::ServerState` backed by an isolated in-memory/temporary `VoxDb` (`DbConfig::Memory`) and mounts `vox_orchestrator_mcp::http_gateway::build_app` on an ephemeral `127.0.0.1:0` TCP listener.
   - Connects `vox_gui::remote::gateway_client::GatewayClient` (or direct HTTP/RPC client) to the live ephemeral gateway and verifies:
     - `Chat` + `Loquela` lifecycle (`chat.create_session` -> `chat.list_sessions` -> `chat.append_message` -> `chat.get_messages` -> `orch.oversight_get` / `orch.oversight_set`) persisted in `VoxDb`.
     - `Terminal` (`vox_terminal_core::SessionHost` `term.open` -> `term.submit` -> `OSC-633` block completion -> `term.context` secret redaction).
     - `Palette` (`⌘K`) & `Explorer` read-only command catalog (`vox_cli::command_catalog::build_catalog`) and live read-only MCP tool execution (`vox_git_status`).
2. **Playwright Real-Backend E2E Harness (`crates/vox-gui/ui/e2e/real-backend/` — `REAL-01`, `REAL-02`, `REAL-03`)**:
   - `realBackendServer.ts`: Spawns an ephemeral localhost HTTP server (`127.0.0.1:0`) backed by an isolated temporary directory on disk (`fs.mkdtempSync`) with optional upstream pass-through to `VOX_REAL_BACKEND_URL` when a live `vox-orchestrator-d` / `vox-mcp` HTTP gateway is supplied.
   - `realBackendBridge.ts`: Injects a thin `window.__TAURI_INTERNALS__.invoke` network bridge that forwards every Tauri command over `fetch()` to the ephemeral `realBackendServer` (zero `installTauriMock` usage). Because state lives in the external server process on disk, `await page.reload()` in Playwright wipes browser memory and proves full state persistence across reloads.
   - `real-backend-journeys.spec.ts`: Exercises:
     - `Chat` + `Loquela`: session creation, prompt submission (`chat_turn`), `TurnCostReceipt` / model attribution rendering, oversight level (`1..5`), and persistence across `page.reload()`.
     - `Terminal` (`Console.tsx`): `term_session_list` / `term_session_attach` / `term_session_submit` (`!cargo test`, `?fix`, `40 + 2`), block persistence across `page.reload()`, and secret redaction on context export.
     - `⌘K` Universal Palette & `ExplorerView.tsx`: live command catalog (`get_command_catalog`), exact CLI verb lookup (`vox build`), and Explorer domain/status filtering.
3. **Expanded 32-Surface Honesty Gate (`crates/vox-gui/ui/src/components/surfaces/__guards__/` — `REAL-04`)**:
   - Expands `honestyScan.ts` (`scanSource`) to detect:
     - `'placeholder'`: `"not (yet )?(implemented|wired|available|working|hooked|connected)"`, `"coming soon"`, `"placeholder"` prose, `"TODO: wire"`.
     - `'dead-handler'`: empty arrow handlers `on(Click|Submit|Change|Press)={() => {}}` and no-op `() => void 0` / `() => null`.
     - `'unguarded-mock'`: production surface imports of `e2e/lib/tauriMock` or direct `window.__TAURI_INTERNALS__ =` assignments.
   - Expands `surfaceHonesty.guard.test.ts` to scan both `.tsx` and `.ts` files across all surface directories and assert that all 32 `view_key` entries in `SURFACE_REGISTRY` are covered with zero violations.
