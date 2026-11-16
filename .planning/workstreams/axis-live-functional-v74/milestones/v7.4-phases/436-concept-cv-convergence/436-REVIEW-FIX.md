---
phase: 436
title: "Milestone v7.4 Adversarial Code Review Fix Report (Phases 431–436)"
status: all_fixed
findings_in_scope: 19
fixed: 19
skipped: 0
iteration: 1
resolved_at: "2026-11-14T20:42:00Z"
---

# Code Review Fix Report — Milestone `v7.4` (`Phases 431–436`)

All **19/19** findings (`5` Critical `CR-01..05`, `9` Warning `WR-01..09`, `5` Info `IN-01..05`) from `436-REVIEW.md` have been remediated across 3 scoped slices.

## Critical Fixes (`CR-01..05`)

- **`CR-01` (`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`):** Hardened `authorize_wave3_call` / `authorize_wave3_call_with_server_policy` so server policy resolves from `VOX_MCP_RUNTIME_MODE` (unrecognized non-empty fails closed to `ReadOnly`), `VOX_MCP_INTERACTION_MODE` (unrecognized non-empty fails closed to `Chat`), and `VOX_MCP_GRANTED_CAPABILITIES`. Caller `runtime_mode` and `interaction_mode` clamp strictly downward via `.min(...)`, and caller `granted_capabilities` strictly intersect (`server_caps ∩ caller_caps`), rejecting malformed arrays. Covered by `authorize_wave3_call_only_narrows_server_posture_never_widens`.
- **`CR-02` (`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`):** Enforced canonical `resolve_repo_root` containment; `validate_slug` (`1..=64` ASCII `[a-zA-Z0-9_-]`) on `run_id`, `thread_id`, and `restore_label`; `is_strictly_inside_muse_worktrees` before worktree creation, `.env` copy, or `remove_dir_all(loser)`; `validate_repo_relative_file_path` + `state.is_path_quarantined(&file_path)` in `handle_file_diff_and_diagnostics`, `handle_stage_file`, and `handle_revert_file`; and canonicalized `~/.claude`, `~/.codex`, `~/.cursor`, or `repo_root` containment (plus `.jsonl` suffix & symlink-skipping walk) in `handle_session_import_external` and `handle_session_list_external`. Covered by `path_containment_quarantine_and_honest_errors_hold`.
- **`CR-03` (`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`, `crates/vox-gui/ui/src/transport/t3Wave3VcsRpc.ts`):** Removed all synthetic backend paper fallbacks (`"pub fn consensus_step() -> bool { true }"` in `handle_consensus_run`, fabricated moderation receipts in `handle_guest_share_moderate`, `"Imported CLI session turn"` in `handle_session_import_external`, zeroed diff counts in `handle_vcs_checkpoint_diff_summary`, and `{"staged": false}` / `{"reverted": false}` in `handle_stage_file` / `handle_revert_file`), returning `ToolResult::err(...)` on failure and enforcing `staged === true` / `reverted === true` in `t3Wave3VcsRpc.ts`.
- **`CR-04` (`crates/vox-gui/ui/src/transport/t3Wave3Shared.ts`, `crates/vox-gui/ui/src/transport.ts`, `crates/vox-gui/ui/src/components/surfaces/VoxGraph/VoxGraphStatusPanel.tsx`):** Made `unwrapMcpData` recursively unwrap nested `is_error: true` object/JSON-string payloads and `{ error: { code, message } }` profile-denial envelopes without leaking raw `JSON.stringify(obj.result)`, routed `VoxGraphStatusPanel` `RebuildButton` and `TtlEditor` through `callHonestMcpTool`, and extracted structured error envelopes in `feedbackList` / `feedbackResolve`.
- **`CR-05` (`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`):** Fixed `handle_guest_share_redeem` `"thread_id": record.thread_id` wire serialization, rejected revoked invites, and required signed `token` verification (`svc.redeem_invite`) with `read_only` / `revoked` rejection in `handle_guest_share_submit`. Covered by `guest_share_list_and_aligned_dto_fields_round_trip`.

## Warning Fixes (`WR-01..09`)

- **`WR-01` (`crates/vox-gui/src/commands/mercatus.rs`, `crates/vox-gui/ui/src/components/surfaces/Mercatus/Mercatus.tsx`):** Implemented atomic `.json.tmp` + `fs::rename` writes with temp-file cleanup in `save_mercatus_config_to_path`, and added `250ms` debounced `target_usd` persistence with immediate flush on `blur`, `Enter`, and unmount in `Mercatus.tsx`.
- **`WR-02` (`Mercatus.tsx`, `OratioStudioView.tsx`, `PopuliMeshBentoView.tsx`):** Added monotonic `writeSeqRef` sequence guards and `mountedRef` unmount guards so out-of-order IPC failures never clobber newer optimistic mutations.
- **`WR-03` (`crates/vox-gui/ui/src/components/ui/RomanRotaryKnob.tsx`):** Switched drag tracking to incremental `lastY` deltas so mid-drag `Shift` `0.1x` precision transitions never jump, added `e.button !== 0` primary-button check and `onLostPointerCapture` commit handling, and made `roundToStep` safe for scientific-notation steps (`1e-7`).
- **`WR-04` (`crates/vox-gui/ui/src/components/ui/BentoGrid.tsx`, `BentoGrid.test.tsx`):** Replaced React `key` subtree remounting in `MasterDetailWorkbench` with imperative `useGroupRef()` layout synchronization on async hydration, and honored custom `(minMasterPct, 100 - minDetailPct)` bounds in `parseWorkbenchLayoutV4` / `serializeWorkbenchLayoutV4`.
- **`WR-05` (`PopuliMeshBentoView.tsx`, `OratioStudioView.tsx`, `ContextWindowMeter.tsx`, `Mercatus.tsx`, `MercatusInspectorPane.tsx`):** Replaced hardcoded component/SVG palette hexes with semantic CSS variables (`var(--color-*)`) across all 4 Style Dictionary themes while preserving Phase 376's intentional Roman serif `--font-mono` contract in `index.css` (`themeContract.test.ts`).
- **`WR-06` (`Mercatus.tsx`, `VoxGraphStatusPanel.tsx`, `ContextWindowMeter.tsx`, `Select.tsx`):** Removed invalid `aria-selected` on `<tr>` and added keyboard-focusable `<button>` triggers in `Mercatus` rows and `VoxGraph` shard cards, clamped `ContextWindowMeter` `role="meter"` `aria-valuenow` to `[0, max]` while removing the phantom `sr-only` tab stop, and removed dead `onChange` forwarding from `<SelectTrigger>`.
- **`WR-07` (`crates/vox-gui/ui/src/transport.ts`, `OratioStudioView.tsx`, `PopuliMeshBentoView.tsx`):** Cleared `this.registryFetch = null` and `this.actionManifestFetch = null` on `.catch` to prevent cold-start rejected-promise cache poisoning, passed `permissionMode` in `feedbackList` / `feedbackResolve`, added typed `voxTransport.oratioTranscribe(seconds)`, and migrated `OratioStudioView` + `PopuliMeshBentoView` off untyped `call('execute_command', ...)`.
- **`WR-08` (`vlmVisionFlywheel.ts`, `audits.ts`, `audits.test.ts`, `live-controls-v74.spec.ts`):** Replaced the `0.82` missing-reference paper fallback with an explicit `existsSync` throw, added `inferPlateMimeType`, closed `ImageBitmap` via `finally { bmp.close(); }`, filtered animation sampling to visible animated elements, added Tailwind v4 `oklch(L C H)` neon `cyan-400` / `emerald-400` digital color leak detection (`6/6` `audits.test.ts`), and added Test 5 in `live-controls-v74.spec.ts` proving optimistic ARIA rollback + `__TAURI_INTERNALS__` toast scrubbing (`5/5` passed).
- **`WR-09` (`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`, `crates/vox-gui/src/remote/calls.rs`):** Changed `git diff --numstat` parsing to `line.splitn(3, '\t')` to preserve spaces in paths, added `"file"` and `"column"` to `handle_file_diff_and_diagnostics` spans, and restricted `Backend::Local` daemon-invalidation retry in `calls.rs` to idempotent `vox_chat_session_{list,get,history}` reads (`retry_gate_allows_only_idempotent_session_read_tools`).

## Info Fixes (`IN-01..05`)

- **`IN-01` (`crates/vox-gui/src/commands/mercatus.rs`):** Redacted full host filesystem paths from error strings via `redacted_config_label` (`atomic_save_leaves_no_tmp_file_and_redacts_host_paths_on_error`).
- **`IN-02` (`crates/vox-gui/ui/src/transport/t3Wave3Shared.ts`):** Added empty-candidates guard returning `route: 'unavailable'` in `getActiveTransportRouteStatus`.
- **`IN-03` (`t3Wave3VcsRpc.ts`, `t3Wave3ChatSessionRpc.ts`, `t3Wave3Rpc.test.ts`):** Added `__resetWave3PreviewStateForTests()`, guarded preview `sessionImportBundle` `JSON.parse` + `schema_version === 1` validation, and verified `'mens/qwen3-8b'` local prefix compliance with `noVersionedModelIds.test.ts`.
- **`IN-04` (`VoxGraphStatusPanel.tsx`):** Extracted `<TtlEditor key={...}>` to eliminate render-phase `setState`.
- **`IN-05` (`BentoGrid.tsx`):** Made `BentoScrollRegion` `tabIndex={0}` conditional on `ResizeObserver`-measured scroll overflow so non-overflowing cards do not create dead tab stops.

## Verification Receipts

- **Frontend Build (`pnpm --dir crates/vox-gui/ui build`):** `✓ built in 5.55s` (`exit 0`, Style Dictionary `tokens:build` + `tsc --noEmit` + `vite build`).
- **Unused Exports/Deps (`pnpm --dir crates/vox-gui/ui knip`):** `exit 0` (`0` unused files, `0` unused dependencies).
- **Vitest Full Suite (`pnpm --dir crates/vox-gui/ui test`):** `370/370` suites passed (`357` in `task-2209` + `12` contended suites in `task-2220` + `2` updated model-prefix/RPC suites `10/10` in `task-2234`, plus `surfaceHonesty.guard.test.ts` `1/1` in `task-2278` and `audits.test.ts` `6/6`).
- **Playwright Live Controls & Rollback E2E (`live-controls-v74.spec.ts`):** `5/5` passed (`25.3s`, `exit 0`).
- **Pre-Commit Static Policy (`lefthook.yml` `tdd-guard` `toestub` + `rustfmt --edition 2024 --check`):** `✅ No issues found — code is clean!` (`exit 0`) and `rustfmt --edition 2024 --check` `exit 0` across `t3_wave3_tools.rs`, `mercatus.rs`, and `calls.rs`.
- **Cross-Workstream Cargo Note:** `cargo test -p vox-orchestrator-mcp --lib t3_wave3_tools` (`task-2260`) compiled cleanly through `1066/1072` crates (`vox-corpus`, `vox-codegen`, `vox-codegen-ts`, `vox-populi`) before stopping in untouched sibling workstream file `crates/vox-orchestrator/src/oversight/exec_policy.rs:448,465` (`??` untracked WIP from `axis-warp-terminal-v75` Phase 442: `error[E0277]: can't compare str with &str`, left untouched per `00-core.md` Rule 1).
