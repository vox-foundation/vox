---
gsd_state_version: "1.0"
milestone: v5.7
milestone_name: Vox ↔ Python Bridge & Tolerant CLI Ergonomics
current_phase: 343
current_phase_name: Docs System SSOT and Automated Drift Control
status: executing
stopped_at: Phase 327 plans 01–04 done. Plan 05 blocked on human approval of polars==2.1.0. CONTEXT records two-tier full-Python-power rule.
last_updated: "2026-11-06T13:36:30.680Z"
last_activity: 2026-11-04
last_activity_desc: 327-04 SUMMARY committed; waiting on polars pin approval + Tier1/Tier2 LLM-target constraint
state_head: dac078de59f4535501117d8661c9c46e358b22ad
progress:
  total_phases: 11
  completed_phases: 1
  total_plans: 35
  completed_plans: 9
  percent: 0
---

# Project State: Unified Workstream

## Current Position

Phase: 343 (Docs System SSOT and Automated Drift Control) — READY TO EXECUTE
Plan: 327-05 (approve polars==2.1.0, then golden)
Status: CR-02/CR-03 stay open. Do not commit STATE/ROADMAP/REQUIREMENTS or mid.rs into 327 task commits. Polars stays uninstalled until resume signal `approved`. Operator locked two-tier rule: Tier 1 curated Vox allowlists (LLM corpus); Tier 2 `import py` (phase 329) for scipy/sklearn/pandas long tail — never mirror full library APIs into Vox syntax.
Last activity: 2026-11-04 — 327-04 complete (`087f292cb`); CONTEXT updated for full-Python-power without breaking LLM-target Vox.

## Milestone History (Recent)

- ✔ **v5.6 (`Phases 313–323`, `25/25` — executed and verified 2026-11-01; not yet formally closed via `/gsd-complete-milestone`; phase dirs not archived because 7 uncommitted changes exist in them)**: Reasoning Where the Lane Needs It
- ✅ **v5.5 (`Phases 307–312`, `24/24` requirements — Complete 2026-10-25):** T3 CI/TOESTUB Deep Enforcement ([Requirements](REQUIREMENTS.md))
- ✅ **v5.4 (`Phases 301–306`, `24/24` requirements — Complete 2026-10-25):** T3 CI/TOESTUB Enforcement Deep-Extraction & Harness Policy Hardening ([Requirements](REQUIREMENTS.md))

## Deferred Items

| Item | Reason | Deferred At |
|------|--------|-------------|
| audit-open v1.0 Phase 03 mens-hf-hub / sidecar / webhook notes | Already fixed or out-of-scope; carried forward at v5.5 close (override_closeout) | 2026-10-25 |

- Effect-TS-only oxlint rules; mobile Hermes/Uniwind/native static toolchain; CSS polish lints (see REQUIREMENTS Out of Scope).

## Next Step

Await operator `approved` for polars==2.1.0 on PyPI, then execute 327-05 (golden + null/missing-column py-plan: tests). Do not install polars before that. Do not fold STATE/ROADMAP/REQUIREMENTS, mid.rs, git-exec, or fan-in into 327 commits. After 327: discuss/plan/execute 328 → 329 (Tier 2 escape — this is where “all features” of scipy/sklearn/pandas land without polluting Tier 1 syntax).

## v5.5 Autonomous Progress (2026-10-25 America/Phoenix)

- Phases **307–312** executed on `cursor/t3-deeper-extraction-v55-510f`.
- 24 REQs checked in REQUIREMENTS.md; scorecard marks honest Partials for live CI upload wiring, knip dual, DOM pragma migration, surface-graph `inventory.vox` regen.
- PR create via ManagePullRequest may still fail (GitHub app); push branch + manual PR URL.

## Accumulated Context

### Roadmap Evolution

- Phase 322 audited 2026-10-28: pack feeds `build_selection_request` (not PreferFree); ChatARC composes existing catalogs; parallel to 317–321.
- Phase 322 added: Calibrated turn decisions and dynamic action catalog (System One/RLCD + ChatARC; folds standalone Jev plan into v5.6 after Phase 321)
- Phase 323 added and completed (2026-11-01): High-Value Starlight MDX Conversions and Ingest Parity (`vox-kb-facets` + `vox-search` + `vox-doc-pipeline` + `vox-corpus` MDX parity, plus `docs/src/index.mdx` and `docs/src/tutorials/tut-fullstack-walkthrough.mdx`)
- Phase 342 added (2026-11-05): Docs Authoring Parity and Starlight 0.42 Upgrade (`DOCS-PARITY-01..04` — 01 include-plugin upgrade landed in-session; 02 Starlight ≥0.42/Astro ≥7.2.10 gates 03 GitHub alerts and 04 links validator; blueprint pillars 2–4 mostly rejected after verification; SSOT: `docs/src/architecture/docs-site-asciidoc-parity-findings-2026.md`)
- Phase 342 planned (2026-11-05): 4 plans / 4 serial waves (01 link fixes + baseline freeze → 02 package-legitimacy checkpoint + Astro 7.4/Starlight 0.42 upgrade → 03 GitHub alerts → 04 links validator + mutation probe); research trial-built 907 pages on the new stack; checker 0 blockers/0 warnings. Execution halts at 01-T1 until other sessions' uncommitted `docs-astro/` + `crates/vox-doc-pipeline/` edits are committed.
- Phase 343 added (2026-11-06): Docs System SSOT and Automated Drift Control (`DOCS-DRIFT-01..06`). SSOT map `docs/src/architecture/docs-system-ssot.md` landed; its §6 known-gaps list is the phase backlog (regen-list parity, overlay parity, prose-vs-CLI, dead guards/orphans, scheduled auto-correct PRs, meta-gate).
- Phase 342 complete (2026-11-06): all 4 plans executed (`9829418e9`, `1f6e2512b`..`93c6973f3`), verification passed 7/7 (`09c272e99`). Astro 7.4.5 / Starlight 0.42.9 installed mature-only (operator answer "Approved mature-only"); GitHub alerts render as asides; `starlight-links-validator` 0.27.0 makes broken internal links fatal (mutation-probed). Test failures unchanged vs frozen baseline (unit 3, Playwright 20 — pre-existing trust-signals/links).
- Phase 331 added (2026-11-01): Tolerant CLI Ergonomics & Precomputed Semantic Routing (`CLI-TOL-01..04` — zero-runtime-LLM prefix unfolding, order-agnostic verb↔noun token permutation, `side_effect_class` + TTY/LLM dual-persona execution gate, and CI-precomputed `contracts/cli/semantic-aliases.v1.json` with cosine margin & antonym banlist enforcement; SSOT: `docs/src/architecture/cli-semantic-tolerance-research-2026.md`)
- Phase 333 added (2026-11-03): Gated training for every model size. Every published size must clear the same release review; 0.6B leaves the smoke recipe; cards stay plain-language and do not call an unfinished run finished.
- Phase 333 extended (2026-11-03): A ready size is stored and distributed as the `vox mens merge-qlora` checkpoint. Cards change in that same step. Until then they describe the trained add-on. 4B is a full shared-recipe run, not a smoke run. Local Qwen bases exist for 0.6B, 1.7B, 4B, 8B, and 14B. 32B is not downloaded. 14B has no trained adapter.

## Performance Metrics

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 316 P01 | 1h 26m | 3 tasks | 9 files |
| Phase 317 P01 | 41min | 2 tasks | 5 files |
| Phase 318 P01–P03 | — | 9 tasks | selector, training, chat, GUI |
| Phase 319 P01–P03 | — | 7 tasks | mesh pick, heartbeat, local fallback |
| Phase 324 P01 | 1h 48m | 4 tasks | effect parser, HIR caps, determinism lint |
| Phase 326 P08 | 44min | 2 tasks | parity checker, python-bridge-parity CI arm |

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
- [Phase 324]: @uses and uses share parse_effect_item; unknown names are Declaration errors
- [Phase 324]: python and python_unverified are flat effects lowered on fn, activity, and workflow
- [Phase 324]: py:allow grants py; py:unverified grants py and py.unverified; spawn stays a keyword token mapped to the spawn effect
- [Phase 333]: The published Hub file is the trained add-on. The complete model is the merge of the local Qwen base and that add-on, stored and distributed only after the release receipt passes. The card updates in that same step.
- [Phase 333]: 4B is a full shared-recipe run uploaded before the checklist passed. The smoke preset applies to the published 0.6B run only.
- [Phase 326]: A comment that names an op is not a handler. python-bridge-parity returns Err on a non-zero cargo status.

## Session

**Last session:** 2026-11-03T23:23:00Z
**Stopped at:** Completed 326-08-PLAN.md (`23d2fc815`). 326-04 is next. Phase 333 context gathered (`REL-01..04`); planning not started.
**Resume file:** .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-04-PLAN.md
**324-01:** Executed 2026-11-03T09:25:02Z — effect parser, HIR capabilities, caps grammar, determinism lint. Commits 03e07f96e..53065172f.
**324 verification:** 2026-11-03T09:32:09Z — gaps_found, 4/5. EFF-02 runtime refusal was missing.
**324 re-verification:** 2026-11-03T10:26:00Z — passed, 5/5, after 324-02. `vox run --caps` without `py` exits 77 before the Python body. Report: `.planning/phases/324-effect-gate-determinism-eff-01-03/324-VERIFICATION.md`.
**324 code review:** 2026-11-03T09:36:04Z — issues_found (0 critical, 3 warnings, 1 info). `vox run` still does not enforce `py` caps. Report: `.planning/phases/324-effect-gate-determinism-eff-01-03/324-REVIEW.md`.
**324-02:** Executed 2026-11-03T10:18:52Z — runtime py denial before the body. Commits ea13019f9..620daa9d0.
