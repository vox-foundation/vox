---
phase: 436
title: "Milestone v7.4 Adversarial Code Review (Phases 431–436)"
status: resolved
depth: deep
reviewed_at: "2026-11-14T19:48:09Z"
files_reviewed: 23
critical: 5
warning: 9
info: 5
total: 19
files_reviewed_list:
  - crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs
  - crates/vox-gui/src/commands/mercatus.rs
  - crates/vox-gui/src/remote/calls.rs
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-gui/ui/src/transport/t3Wave3Shared.ts
  - crates/vox-gui/ui/src/transport/t3Wave3ChatSessionRpc.ts
  - crates/vox-gui/ui/src/transport/t3Wave3VcsRpc.ts
  - crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts
  - crates/vox-gui/ui/src/index.css
  - crates/vox-gui/ui/src/components/ui/RomanRotaryKnob.tsx
  - crates/vox-gui/ui/src/components/ui/Select.tsx
  - crates/vox-gui/ui/src/components/ui/Slider.tsx
  - crates/vox-gui/ui/src/components/ui/BentoGrid.tsx
  - crates/vox-gui/ui/src/components/layout/SurfaceScrollHost.tsx
  - crates/vox-gui/ui/src/components/surfaces/Oratio/OratioStudioView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Populi/PopuliMeshBentoView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ContextWindowMeter.tsx
  - crates/vox-gui/ui/src/components/gamify/urbs/canvasVectorBadges.ts
  - crates/vox-gui/ui/src/components/surfaces/VoxGraph/VoxGraphStatusPanel.tsx
  - crates/vox-gui/ui/src/components/surfaces/Mercatus/Mercatus.tsx
  - crates/vox-gui/ui/e2e/review/audits.ts
  - crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts
  - crates/vox-gui/ui/e2e/review/live-controls-v74.spec.ts
---

# Adversarial Code Review — Milestone `v7.4` (`Phases 431–436`)

- **Critical (`CR-01..05`):** 5
- **Warning (`WR-01..09`):** 9
- **Info (`IN-01..05`):** 5
- **Total:** 19 across 23 reviewed files

## Critical Findings

### CR-01 — Caller-Controlled Authorization Widening in `authorize_wave3_call`
- **File:** `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs:58-97`
- **Issue:** `authorize_wave3_call` allowed `args["runtime_mode"]`, `args["interaction_mode"]`, and `args["granted_capabilities"]` to override server-configured `VOX_MCP_RUNTIME_MODE` / `VOX_MCP_INTERACTION_MODE` upward instead of clamping `args` strictly to narrow server policy.

### CR-02 — Unsanitized Repo/Worktree/External Session Paths in `t3_wave3_tools.rs`
- **File:** `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs:99-108, 493-520, 561-631, 924-930, 1239-1326, 1437-1441`
- **Issue:** `resolve_repo_root` accepted uncontained `args["repo_root"]`; `run_id` lacked slug validation before `.vox/muse-worktrees/` creation/deletion (`remove_dir_all`); `session_import_external` / `session_list_external` accepted uncontained host paths; `stage`/`revert` skipped Windows drive/component checks and `is_path_quarantined`.

### CR-03 — Backend Paper Fallbacks Synthesizing Fake Success on Failure
- **File:** `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs:203-239, 807-829, 928-935, 1196-1224, 1300-1326` & `crates/vox-gui/ui/src/transport/t3Wave3VcsRpc.ts:214-236`
- **Issue:** `handle_consensus_run`, `handle_guest_share_moderate`, `handle_session_import_external`, `handle_vcs_checkpoint_diff_summary`, and `handle_stage_file`/`handle_revert_file` returned synthetic success data when operations failed, and `t3Wave3VcsRpc.ts` did not verify `staged === true` / `reverted === true`.

### CR-04 — MCP Error Envelope `JSON.stringify` Leak & Unchecked `is_error` Calls
- **File:** `crates/vox-gui/ui/src/transport/t3Wave3Shared.ts:98-120`, `crates/vox-gui/ui/src/components/surfaces/VoxGraph/VoxGraphStatusPanel.tsx:36, 99-115`, `crates/vox-gui/ui/src/transport.ts:1322-1342`
- **Issue:** `unwrapMcpData` serialized `obj.result` via `JSON.stringify` on `is_error: true` instead of recursing to extract `result.error`, missed `{ error: { code, message } }` profile-denial envelopes, and `VoxGraphStatusPanel.tsx` called `invokeMcpTool` without `callHonestMcpTool` error unwrapping.

### CR-05 — Guest Share `read_only` Scope Bypass, Tokenless Submit & `thread_id` Wire Bug
- **File:** `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs:1485-1562`
- **Issue:** `handle_guest_share_redeem` serialized the full `GuestInviteRecord` under `"thread_id"` instead of `record.thread_id`, and `handle_guest_share_submit` permitted missing `token` or `read_only` / `revoked` invites to submit prompts.

## Warning Findings

### WR-01 — Synchronous Non-Atomic Config Writes & Un-Debounced Keystroke Saves in `Mercatus`
- **File:** `crates/vox-gui/src/commands/mercatus.rs:42-57, 108` & `crates/vox-gui/ui/src/components/surfaces/Mercatus/Mercatus.tsx:46-60, 188`

### WR-02 — Out-of-Order Optimistic Rollback Races & Missing Unmount Guards
- **File:** `crates/vox-gui/ui/src/components/surfaces/Mercatus/Mercatus.tsx:25-44`, `crates/vox-gui/ui/src/components/surfaces/Oratio/OratioStudioView.tsx:108-178`, `crates/vox-gui/ui/src/components/surfaces/Populi/PopuliMeshBentoView.tsx:93-131`

### WR-03 — `RomanRotaryKnob` Mid-Drag `Shift` Jump, Missing `e.button === 0` / `onLostPointerCapture` & Small-Step Rounding
- **File:** `crates/vox-gui/ui/src/components/ui/RomanRotaryKnob.tsx:45-52, 84-90, 136-165`

### WR-04 — `MasterDetailWorkbench` Hydration Subtree Remount & Custom Min/Max Bounds
- **File:** `crates/vox-gui/ui/src/components/ui/BentoGrid.tsx:96-132, 173, 180-208, 286-291`

### WR-05 — Hardcoded Palette Colors in `PopuliMeshBentoView`, `OratioStudioView`, `ContextWindowMeter` & `Mercatus`
- **File:** `PopuliMeshBentoView.tsx:178-213`, `OratioStudioView.tsx:249-265`, `ContextWindowMeter.tsx:202, 235, 255`, `Mercatus.tsx:112-147, 204-208, 336, 369`
- **Note on `index.css` `--font-mono`:** Verified against Phase 376 (`themeContract.test.ts`), which intentionally enforces Roman serif typography on `--font-mono`; `index.css` `--font-mono` is preserved per project contract while component inline hexes are migrated to semantic CSS variables.

### WR-06 — Invalid `aria-selected` on `<tr>`, Keyboard Card/Row Selection, Unclamped `role="meter"` & Dead `SelectTrigger` `onChange`
- **File:** `Mercatus.tsx:157-169`, `VoxGraphStatusPanel.tsx:264-273`, `ContextWindowMeter.tsx:164-169, 304-313`, `Select.tsx:171-176`

### WR-07 — Rejected-Promise Cache Poisoning in `VoxTransport`, Missing `permissionMode` & Typed `oratioTranscribe`
- **File:** `crates/vox-gui/ui/src/transport.ts:375-408, 1322-1365`, `OratioStudioView.tsx:113, 148`, `PopuliMeshBentoView.tsx:103`

### WR-08 — VLM/CV Flywheel `0.82` Missing-Ref Fallback, `ImageBitmap` Leak, `oklch()` Leak Check & Rollback E2E Spec
- **File:** `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts`, `crates/vox-gui/ui/e2e/review/audits.ts`, `crates/vox-gui/ui/e2e/review/live-controls-v74.spec.ts`

### WR-09 — `git diff --numstat` Tab Split, `CompilerDiagnosticSpanDto` Fields & Idempotent-Only Local Retry
- **File:** `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs:1204-1215, 1266-1272`, `crates/vox-gui/src/remote/calls.rs:63-80`

## Info Findings (`IN-01..05`)
- **`IN-01`:** Error message sanitization for OS paths in `mercatus.rs`.
- **`IN-02`:** Empty-candidates guard in `getActiveTransportRouteStatus` (`t3Wave3Shared.ts`).
- **`IN-03`:** Preview reset helper, guarded preview `JSON.parse`, and `'mens-qwen3-8b'` slug alignment (`t3Wave3VcsRpc.ts`, `t3Wave3ChatSessionRpc.ts`).
- **`IN-04`:** Keyed `<TtlEditor>` instead of render-phase `setState` (`VoxGraphStatusPanel.tsx`).
- **`IN-05`:** Overflow-aware `tabIndex` in `BentoScrollRegion` (`BentoGrid.tsx`).
