---
phase: 318-live-vram-selector-size-02
plan: 03
subsystem: inference-selection
status: complete
tags: [size-02, vram, selector, qwen3, auto-local, chat, gui, serve, vox-orchestrator]
requires:
  - "318-01 size_select (select_size, read_free_memory, load_ladder_overlay, resolve_base_pin, FreeMemory::cuda)"
provides:
  - "vox_orchestrator::models::auto_select: AutoModelSelection { selected_model_id, size, free_mb, floor_mb, reason }, select_for_free_memory, select_optimal_local_model_for_host -> Result<_, String> (runtime feature)"
  - "vox chat --model auto-local and GUI get_auto_model_recommendation return the live pick or the selector's refusal"
  - "vox mens serve: serve_fit_note re-checks the run's recorded base right before the worker loads"
affects:
  - "318-04 (GUI TypeScript type and mock still read detected_vram_gb / tier_reason; transport.ts and e2e tauriMock.ts)"
tech-stack:
  added: []
  patterns:
    - "Probe first, populi second: registered NVML free memory (x0.88) or_else size_select::read_free_memory"
    - "Inference consumers propagate the selector's Err; no fixed tier or default model"
key-files:
  created: []
  modified:
    - crates/vox-orchestrator/src/models/auto_select.rs
    - crates/vox-orchestrator/src/models/mod.rs
    - crates/vox-orchestrator/src/models/select.rs
    - crates/vox-orchestrator/src/models/registry.rs
    - crates/vox-cli/src/commands/chat.rs
    - crates/vox-gui/src/commands/models_host.rs
    - crates/vox-ml-cli/src/commands/ai/serve/mod.rs
decisions:
  - "serve_fit_note treats a base the ladder cannot resolve (a bare name with no slash) as off-ladder (Ok(None)), so such runs serve as before instead of failing on a resolver error"
  - "auto_select is gated on vox-orchestrator's runtime feature (where vox-populi/mens lives); every consumer already enables it"
metrics:
  duration: "~224 min wall (including a rate-limit pause and ~60 min of cold scoped builds)"
  completed: "2026-10-27"
  tests:
    auto_select: 4
    models_minus_premium_alias: 308
    ai_serve: 52
    vox_cli_chat_routing: 2
    vox_gui_auto_model_recommendation: 2
estimate:
  tokens: 90000
  tasks: 3
actuals:
  tokens: 8455
  tasks: 3
  commits: 3
plan_head_before: cd25219afe6f09c7ca31ddb21d366b253189f457
plan_head_after: 33c23515216c0301a2013478b8ac3c349d6107d4
---

# Phase 318 Plan 03: Inference Consumers Use the Live VRAM Selector Summary

`vox chat --model auto-local`, the GUI's `get_auto_model_recommendation` and the orchestrator's auto-select now get their model from the Plan 01 selector. It reads live free memory, using the registered NVML probe's minimum free MB x0.88 when a host registered one and vox-populi's Metal or nvidia-smi reading otherwise, and returns the largest Qwen3 ladder pin whose QLoRA floor fits. If nothing can be read or nothing fits, the caller gets the selector's error text. The six fake tiers (five `mens/runs/qwen3_27b_metal_check/*` runs and `vox-mens-8b-v0.6`) are gone, along with the total-memory probe and their registry entries. `vox mens serve` re-reads free memory right before the inference worker loads. If the run's recorded Qwen3 base no longer fits, serve refuses with the floor and the free MB, and it never steps the base down.

## Commits

| Task | Commit | Files |
|------|--------|-------|
| 1 Tracer: live pick reaches auto-select, chat and the GUI | `b28d04614` | auto_select.rs, models/mod.rs, models/select.rs, chat.rs, models_host.rs |
| 2 Fake `voxlocal` registry candidates deleted | `2fcd25db9` | registry.rs |
| 3 Serve fit check before load; M13-M16 | `33c235152` | serve/mod.rs |

`git rev-list --count cd25219af..HEAD` is 3. Each commit holds only its task's paths (`git commit --only`).

## Verification

| Gate | Command | Result | Required |
|------|---------|--------|----------|
| auto_select | `cargo nextest run -p vox-orchestrator --lib -E 'test(auto_select)'` | 4 passed | >= 4 |
| models:: | `... -E 'test(models::) - test(premium_alias)'` | 308 passed, 0 failed | >= 308 |
| ai::serve | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(ai::serve)'` | 52 passed | >= 52 |
| vox-cli probe | `cargo check -p vox-cli` | **compiled** (`Finished`, 5m 21s), so the tests ran: `test(chat_routing)` 2 passed (`chat_routing_ollama_vs_openrouter`, `chat_routing_auto_and_mens`) | compile branch: >= 2 |
| vox-gui probe | `cargo check -p vox-gui --tests` | **compiled** (`Finished`, 5m 32s), so the test ran: `test(auto_model_recommendation)` 2 passed (`auto_model_recommendation_is_a_live_pick_or_a_reason` in the `vox-gui` bin target and in the `search_probe_test` target) | compile branch: >= 1 |
| clippy | `cargo clippy -p vox-orchestrator --lib --tests`; `cargo clippy -p vox-ml-cli --features cloud --lib --tests` | both `Finished`; no finding in a touched file (pre-existing warnings only in vox-corpus, vox-eval, vox-search, vox-lsp, external_importer.rs, vox-populi) | as stated |
| rustfmt | `rustfmt --edition 2024 --check` on all seven Rust files | clean | clean |
| Task 1 greps | fake ids, `select_tier_for_vram`, `detect_hardware_vram_gb`, `vram_total_mb` in auto_select.rs 0; deleted names (`is_host_apple_silicon`, `select_optimal_local_model(` ...) under `crates/` 0; chat signature 1, selector call 1, caller `?` 1; GUI `Ok(...)` wrapper 0, selector call 1, old fields 0 | all pass | as stated |
| Task 2 greps | `register_mens_local_candidates\|qwen3_27b_metal_check` in vox-orchestrator 0 | pass | 0 |
| Task 3 order | `serve_fit_note(base.as_deref` (line 132) before `spawn_inference_worker(` (line 135); `serve_fit_note(` occurs 7 times | pass | before; >= 2 |
| size budgets | auto_select.rs non-blank lines before tests: 51 (< 120); chat.rs `git diff --numstat` vs anchor 22+17 = 39 (< 40); `git diff --stat 640f5b685 -- crates/vox-cli crates/vox-gui` shows only chat.rs and models_host.rs | pass | as stated |
| no edges | `git diff 640f5b685 -- contracts/ci crates/vox-orchestrator/Cargo.toml crates/vox-ml-cli/Cargo.toml crates/vox-cli/Cargo.toml` | empty | empty |

Both TDD tasks went RED before GREEN. Task 1's first build failed with 6 errors (`host_free_memory`, `select_for_free_memory` and the new re-exports were unresolved). Task 3's failed with 10 (`serve_fit_note`, `size_select`, `HashMap`, `TrainBase`).

## Mutation Results (M13-M16)

Protocol: back up both files and record their `shasum`, `grep -cF` the guard, mutate with the Edit tool, count again, run the named test with `--no-fail-fast`, restore with `cp` from the backup and compare the hash (`cmp` is blocked by this host's shell allowlist), count again, then re-run the test. After all four, `shasum -c` matched both pre-mutation hashes (auto_select.rs `b68b058b9e98`, serve/mod.rs `75bfe55d56a5`).

| ID | File | Guard | Count before / mutated / restored | Red run (named test) | Restore | Green re-run |
|----|------|-------|-----------------------------------|----------------------|---------|--------------|
| M13 | auto_select.rs | `.map(FreeMemory::cuda)` (3-line expression replaced by `size_select::read_free_memory()`) | 1 / 0 / 1 | exit 100, FAIL `registered_probe_free_memory_wins_and_keeps_the_margin` | hash identical | PASS |
| M14 | auto_select.rs | `select_size(&overlay, free, None, false)`; `free` became `free.or(Some(&FreeMemory { mb: u64::MAX, source: "default" }))` | 1 / 0 / 1 | exit 100, FAIL `auto_selection_refuses_without_a_reading` | hash identical | PASS |
| M15 | serve/mod.rs | `select_size(overlay, free, Some(base), false)`; `false` became `true` | 1 / 0 / 1 | exit 100, FAIL `serve_fit_note_refuses_a_ladder_base_that_does_not_fit` | hash identical | PASS |
| M16 | serve/mod.rs | `let Some(base) = base_model else` (3-line let-else replaced by `let base = base_model.unwrap_or("Qwen/Qwen3-32B@9216db…");`) | 1 / 0 / 1 | exit 100, FAIL `serve_fit_note_passes_unknown_and_missing_bases` | hash identical | PASS |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Serve would have refused runs whose base is a bare unresolvable name**
- **Found during:** Task 3
- **Issue:** `select_size` with an explicit base that has no `/` and is not a ladder size (for example `custom-base`) returns the resolver's error, not `PassThrough`. A run like that would have stopped serving, but the plan says off-ladder bases serve as before.
- **Fix:** `serve_fit_note` returns `Ok(None)` when `size_select::resolve_base_pin(Some(base), overlay)` fails. `custom-base` was added to `serve_fit_note_passes_unknown_and_missing_bases`.
- **Files modified:** crates/vox-ml-cli/src/commands/ai/serve/mod.rs
- **Commit:** `33c235152`

### Other adjustments

2. **M13 guard after rustfmt.** rustfmt's chain width puts `free_vram_mb_from_probe().map(FreeMemory::cuda).or_else(size_select::read_free_memory)` on three lines, so the acceptance grep for the single-line form counts 0. The expression itself is unchanged. The guard became `.map(FreeMemory::cuda)` (count 1), and the mutation replaced the whole three-line expression, as the coordinator's rustfmt note allows. M15 and M16 guards are unique substrings of rustfmt's output.
3. **chat.rs diff budget (< 40 lines).** Wrapping each of the three returns in `Ok(..)` and using `.expect("routes")` came to 46 changed lines. The body now ends `let cfg = if ... ; Ok(cfg)`, the seven routing calls use `.unwrap()`, and the auto-local branch asserts `model.split('@').nth(1).map(str::len) == Some(40)`. It checks the revision's length, not that it is hex. The selector only returns 40-hex pins (Plan 01 `split_pin`). Final diff: 39 lines.
4. **Compile probes took the compile branch.** `vox-cli` and `vox-gui` compile on this branch (fixed in `371c198fe`), so both probes ran their tests. The GUI filter matched two instances of the same test because `vox-gui` compiles `models_host` into two test targets.
5. **Doc-lint.** This plan changes no docs, so there was no doc-lint gate.

No Rule 4 issues came up. There were no package installs, crate edges, Cargo.toml, contracts/ci or exceptions-ledger changes. STATE.md and ROADMAP.md were not touched, as the coordinator asked.

## Known Stubs

None. The GUI's TypeScript `AutoModelSelection` (`crates/vox-gui/ui/src/transport.ts:895-896`) and the e2e mock (`crates/vox-gui/ui/e2e/lib/tauriMock.ts:297-299`) still use the old `detected_vram_gb` / `tier_reason` shape and a fake tier id. Plan 04 updates them, as this plan's objective says.

## Threat Flags

None. T-318-13 through T-318-17 and T-318-20 are mitigated as planned: total-memory sizing is deleted (grep gate) and the probe keeps its margin (M13); no reading means refusal (M14); serve refuses right before load (M15, call-order gate); the manifest goes through the existing serde loader, and its base is only resolved against the ladder (M16); the fake registry candidates are deleted; the GUI returns the selector's error unchanged. There is no new network, auth or file-write surface.

## Self-Check: PASSED

- FOUND: `pub fn select_for_free_memory` in auto_select.rs; `fn serve_fit_note` in serve/mod.rs; `select_optimal_local_model_for_host()` in chat.rs and models_host.rs
- FOUND commits on HEAD: b28d04614, 2fcd25db9, 33c235152
