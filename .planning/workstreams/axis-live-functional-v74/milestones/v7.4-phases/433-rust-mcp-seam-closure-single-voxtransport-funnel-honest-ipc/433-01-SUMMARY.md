# Phase 433 Summary — Plan 433-01

- **Phase**: `433` (`Rust/MCP Seam Closure, Single voxTransport Funnel & Honest IPC Contract`)
- **Status**: `Complete`
- **Requirements Closed**: `AXIS-IPC-HONESTY-01`, `AXIS-IPC-HONESTY-02`, `AXIS-IPC-HONESTY-03`, `AXIS-IPC-HONESTY-04`

## Key Deliverables
1. **Rust/MCP & Catalog Seam Closure (`AXIS-IPC-HONESTY-01`, `AXIS-IPC-HONESTY-03`, `AXIS-IPC-HONESTY-04`)**:
   - Implemented `GuestShareService::list_invites` and `list_pending_messages` in `crates/vox-orchestrator/src/guest_share.rs`.
   - Wired `vox_guest_share_list` (`guest.share.list`) across `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`, `input_schemas.rs`, `input_schemas/browser_and_gui.rs`, `dispatch_domains/compiler_analysis.rs`, and `contracts/operations/catalog.v1.yaml` (+ `vox ci operations-sync --target all --write`).
   - Aligned wire DTO fields for `vox_guest_share_mint` (`token`, `session_id`, `permission`, `scope`, `revoked`), `vox_guest_share_redeem`, `vox_guest_share_submit`, `vox_guest_share_moderate`, and `vox_muse_pool_adopt` (`adopted_variant_id`, `adopted_branch_name`, `pruned_variant_ids`).
   - Added `std::fs::create_dir_all(parent)` before `std::fs::write` in `crates/vox-gui/src/commands/mercatus.rs` (`save_mercatus_config_to_path`).
   - Added `"vox_consensus_run" => Duration::from_secs(120)` in `crates/vox-gui/src/remote/calls.rs` (`deadline_for_tool`).
2. **100% Single `voxTransport` Funnel (`AXIS-IPC-HONESTY-02`)**:
   - Exported `invokeCmd`, `listenEvent`, and `type UnlistenFn` from `crates/vox-gui/ui/src/transport.ts` (delegating exclusively through `safeInvoke` / `safeListen` inside the single `__VOX_RAW_IPC_BEGIN__..END__` region and threading active `PermissionMode` onto `invoke_mcp_tool`).
   - Migrated all 40 non-`transport.ts` source files previously importing `@tauri-apps/api/core` or `@tauri-apps/api/event` and tightened `crates/vox-gui/ui/src/guards/ipcBoundaries.test.ts` to `ALLOW_DIRECT_INVOKE = new Set(['transport.ts'])` and `ALLOW_DIRECT_LISTEN = new Set(['transport.ts'])`.
3. **Honest Live IPC vs Explicit Browser Preview (`AXIS-IPC-HONESTY-01..04`)**:
   - Split 1,109-line `t3Wave3Rpc.ts` into `<410`-line modules (`t3Wave3Shared.ts`, `t3Wave3VcsRpc.ts`, `t3Wave3ChatSessionRpc.ts`, barrel `t3Wave3Rpc.ts`), adding `guestShareList` and rethrowing sanitized errors whenever `backendAvailable() === true`.
   - Added `crates/vox-gui/ui/src/guards/noPaperFallbackGuard.test.ts`, surfaced honest `consensus-error-banner` alerts in `ChatTranscript.tsx` via `sanitizeErrorForToast`, and synced `crates/vox-gui/ui/e2e/lib/tauriMock.ts` across all 29 Wave 3/4 tools + `mercatus_save_config`.

## Verification Receipts
- `cargo test -p vox-orchestrator --lib guest_share` → `2 passed; 0 failed`
- `cargo test -p vox-orchestrator-mcp --lib t3_wave3_tools` & `input_schemas` → `17 passed; 0 failed`
- `cargo test -p vox-gui --bins` → `379 passed; 0 failed`
- `cargo run -p vox-cli -- ci gui-catalog-parity` → `passed`
- `pnpm --dir crates/vox-gui/ui test -- --maxWorkers=6` → `364 passed (364)` (`2,278 passed (2,278)`)
- `pnpm --dir crates/vox-gui/ui build` & `pnpm --dir crates/vox-gui/ui knip` → `exit 0`
- `./target/debug/toestub crates/vox-gui/ui/src/transport --min-severity error` → `0 issues`
