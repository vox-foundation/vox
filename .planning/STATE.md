---
gsd_state_version: "1.0"
milestone: v5.6
milestone_name: Reasoning Where the Lane Needs It
current_phase: 320
current_phase_name: Train the Sizes
status: ready_to_plan
stopped_at: Phase 320 context written — ready to plan
last_updated: "2026-10-30T04:05:00.000Z"
last_activity: 2026-10-29
last_activity_desc: Phase 320 smart discuss context written
state_head: f2ab6c443
progress:
  total_phases: 10
  completed_phases: 7
  total_plans: 16
  completed_plans: 16
  percent: 70
---

# Project State: Unified Workstream

## Current Position

Phase: 320 (Train the Sizes) — not started
Plan: none yet
Status: Phase 319 verified passed (12/12). Security threats_open 0. Nyquist compliant. v5.6 continues through 320–322; not done until two sizes have real trained scores and 322 is planned/executed or deferred.
Last activity: 2026-10-29 — Phase 319 closed

## Milestone History (Recent)

- ✅ **v5.5 (`Phases 307–312`, `24/24` requirements — Complete 2026-10-25):** T3 CI/TOESTUB Deep Enforcement ([Requirements](REQUIREMENTS.md))
- ✅ **v5.4 (`Phases 301–306`, `24/24` requirements — Complete 2026-10-25):** T3 CI/TOESTUB Enforcement Deep-Extraction & Harness Policy Hardening ([Requirements](REQUIREMENTS.md))

## Deferred Items

| Item | Reason | Deferred At |
|------|--------|-------------|
| audit-open v1.0 Phase 03 mens-hf-hub / sidecar / webhook notes | Already fixed or out-of-scope; carried forward at v5.5 close (override_closeout) | 2026-10-25 |

- Effect-TS-only oxlint rules; mobile Hermes/Uniwind/native static toolchain; CSS polish lints (see REQUIREMENTS Out of Scope).

## Next Step

Phase 320 context is written. Plan from the locked decisions. Do not re-open them. Do not download weights or start a live train. Phases 321–322 remain.

## v5.5 Autonomous Progress (2026-10-25 America/Phoenix)

- Phases **307–312** executed on `cursor/t3-deeper-extraction-v55-510f`.
- 24 REQs checked in REQUIREMENTS.md; scorecard marks honest Partials for live CI upload wiring, knip dual, DOM pragma migration, surface-graph `inventory.vox` regen.
- PR create via ManagePullRequest may still fail (GitHub app); push branch + manual PR URL.

## Accumulated Context

### Roadmap Evolution

- Phase 322 audited 2026-10-28: pack feeds `build_selection_request` (not PreferFree); ChatARC composes existing catalogs; parallel to 317–321.
- Phase 322 added: Calibrated turn decisions and dynamic action catalog (System One/RLCD + ChatARC; folds standalone Jev plan into v5.6 after Phase 321)

## Performance Metrics

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 316 P01 | 1h 26m | 3 tasks | 9 files |
| Phase 317 P01 | 41min | 2 tasks | 5 files |
| Phase 318 P01–P03 | — | 9 tasks | selector, training, chat, GUI |
| Phase 319 P01–P03 | — | 7 tasks | mesh pick, heartbeat, local fallback |

## Decisions

- [Phase 316]: Detected::Ready carries the resolved BaseLabel so the compared report can name the pin
- [Phase 316]: Weight-index shards that are absolute or contain .. are an incomplete snapshot
- [Phase 316]: The how-to paragraph names eval-reasoning-compare and mens/runs/reasoning_compare/ and contains no digits
- [Phase 317]: 1.7B and 4B are unmeasured qwen3_code QLoRA rows with no floor_mb; pick_base cannot select them
- [Phase 317]: floor_measured defaults true so existing measured floors stay eligible
- [Phase 317]: Size names resolve through resolve_qwen3_size; capability tags stay on the memory picker
- [Phase 318]: Unmeasured 1.7B and 4B stay ineligible; the live pick is the largest measured qwen3_code QLoRA floor that fits
- [Phase 318]: CUDA free memory is nvidia-smi memory.free; Metal free memory is monitor_metal memory_free_mb
- [Phase 318]: An explicit size, including VOX_MENS_DEFAULT_MODEL, is refused when it does not fit and is not stepped down
- [Phase 318]: A caller preset stays when the size does not change; a step-down uses the smaller size's preset
- [Phase 319]: accelerator_free_mb is the mesh fit number; gpu_vram_total_mb does not decide the size
- [Phase 319]: An explicit size on the local fallback is refused, not stepped down or up
- [Phase 319]: A free-memory reading that does not fit in u32 is dropped before the max
- [Phase 319]: One fresh node is a mesh pick; no fresh reading uses the phase 318 local selector

## Session

**Last session:** 2026-10-29
**Stopped at:** Phase 320 context written, ready to plan
**Resume file:** None
