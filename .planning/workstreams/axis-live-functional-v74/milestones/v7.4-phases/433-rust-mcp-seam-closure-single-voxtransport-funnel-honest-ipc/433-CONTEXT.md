# Phase 433 Context — Rust/MCP Seam Closure, Single `voxTransport` Funnel & Honest IPC Error Architecture

- **Milestone**: `v7.4` (`axis-live-functional-v74`)
- **Requirements**: `AXIS-IPC-HONESTY-01`, `AXIS-IPC-HONESTY-02`, `AXIS-IPC-HONESTY-03`, `AXIS-IPC-HONESTY-04`
- **Status**: Ready for Execution (`Smart Discuss` 9/9 decisions accepted)

## Locked Decisions

### Area 1 — Rust/MCP Missing Read Endpoints & DTO Alignment (`AXIS-IPC-HONESTY-01`, `AXIS-IPC-HONESTY-03`, `AXIS-IPC-HONESTY-04`)
1. **`vox_guest_share_list` (`guest.share.list`)**:
   - Add `GuestShareService::list_invites(&self, thread_id: Option<&str>) -> Vec<GuestInviteRecord>` and `GuestShareService::list_pending_messages(&self, thread_id: Option<&str>) -> Vec<GuestPendingMessage>` in `crates/vox-orchestrator/src/guest_share.rs`.
   - Register `vox_guest_share_list` across `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` (`access_requirement_for_wave3_tool` under `reads()`, `handle_guest_share_list` returning `{ "invites": [...], "submissions": [...] }`), `input_schemas.rs`, `input_schemas/browser_and_gui.rs`, and `dispatch_domains/compiler_analysis.rs`.
   - Add `- id: guest.share.list` (`mcp.name: vox_guest_share_list`, `side_effect_class: pure_read`, `product_lane: ai`, `exposure: operator`) at its sorted spot in `contracts/operations/catalog.v1.yaml` and run `cargo run -p vox-cli -- ci operations-sync --write`.
2. **DTO Wire Alignment**:
   - `handle_guest_share_mint`: include `"token": format!("{}:{}", record.invite_id, record.signature_hex)` and `"session_id": record.thread_id`, `"permission": ...` alongside `GuestInviteRecord` fields so both Rust callers and TypeScript `guestShareMint` see a single canonical payload.
   - `handle_muse_pool_adopt`: include `"adopted_variant_id": winner` alongside `"winner": winner` and `"adopted_branch_name"`, `"pruned_variant_ids"`.
   - `sessionListExternal` in TypeScript: accept both bare array `DiscoveredExternalSessionDto[]` (actual Rust `vox_session_list_external` `ToolResult` `data` shape) and `{ sessions: [...] }`.
3. **Mercatus Config Directory Creation (`AXIS-IPC-HONESTY-03`)**:
   - In `crates/vox-gui/src/commands/mercatus.rs` (`mercatus_save_config`), ensure `if let Some(parent) = config_path.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory {}: {e}", parent.display()))?; }` runs before `std::fs::write(&config_path, ...)`.
4. **Remote Multi-Model Consensus Deadline (`AXIS-IPC-HONESTY-04`)**:
   - In `crates/vox-gui/src/remote/calls.rs` (`deadline_for_tool`), add `"vox_consensus_run"` to the long-deadline (`Duration::from_secs(120)`) match arm so remote 3-leg fan-out + synthesis never hits the 15s default RPC cutoff.
5. **L5 Binary Boundary (`CORE-04`)**:
   - `crates/vox-gui` maintains zero persistent `VoxDb` handles.

### Area 2 — 100% Single `voxTransport` Funnel & Zero Boundary Exemptions (`AXIS-IPC-HONESTY-02`)
1. **Single Raw IPC Hub (`transport.ts`)**:
   - Keep `@tauri-apps/api/core` (`invoke`) and `@tauri-apps/api/event` (`listen`) exclusively inside `src/transport.ts`'s marked `// __VOX_RAW_IPC_BEGIN__` ... `// __VOX_RAW_IPC_END__` region (`transportIpcGuard.test.ts` remains strict).
   - Export `invokeCmd<T>(cmd, args?)` and `listenEvent<T>(event, handler)` (plus `type UnlistenFn`) from `src/transport.ts`, automatically threading `permissionMode: currentPermissionMode` on `invoke_mcp_tool` calls whenever `currentPermissionMode !== null` and `permissionMode` is not already specified.
   - Add cohesive `<500` non-blank-line domain helper modules under `src/transport/` where helpful, importing `invokeCmd` / `listenEvent` from `../transport` (never importing `@tauri-apps/api/*` directly).
2. **Zero Surface Exemptions**:
   - Migrate all 40 non-`transport.ts` files importing `@tauri-apps/api/core` (`38` allowlisted + `GsdDocumentsPane.tsx` + `GsdStudioView.tsx`) and all 8 non-`transport.ts` files importing `@tauri-apps/api/event` to `transport.ts` / `transport/*`.
   - Tighten `crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts` so `ALLOW_DIRECT_INVOKE = new Set(['transport.ts'])` and `ALLOW_DIRECT_LISTEN = new Set(['transport.ts'])`.

### Area 3 — Honest Live IPC Errors vs Explicit Browser Preview (`AXIS-IPC-HONESTY-01`)
1. **Eliminate Catch-and-Fake Dishonesty (`t3Wave3Rpc.ts` -> modular `<500`-line split)**:
   - Split the 1,109-line `src/transport/t3Wave3Rpc.ts` into `<500` non-blank-line modules (`t3Wave3Shared.ts`, `t3Wave3VcsRpc.ts`, `t3Wave3ChatSessionRpc.ts`, and barrel `t3Wave3Rpc.ts`).
   - When `backendAvailable() === true`: every helper calls `voxTransport.invokeMcpTool(...)`, unwraps via `unwrapMcpData`, validates the schema, and **rethrows** sanitized errors (`new Error(sanitizeErrorForToast(err))`) on failure — zero silent catch-and-fake `_demo` tokens, fake Git OIDs, fake consensus synthesis, or fake `500 BU` battery numbers.
   - When `backendAvailable() === false` (browser preview without Tauri): return explicit preview envelopes carrying `preview_reason: 'browser_preview_no_desktop_backend'` (never `_demo` tokens when live).
   - Sync `crates/vox-gui/ui/e2e/lib/tauriMock.ts` (`mcpResult` + `mercatus_save_config`) so Playwright mock harnesses return honest structured DTOs for all Wave 3/4 tools.
   - Add `crates/vox-gui/ui/src/guards/noPaperFallbackGuard.test.ts` enforcing zero silent `catch {}` fake-success fallbacks across `src/transport/`.
