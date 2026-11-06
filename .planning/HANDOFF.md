# Handoff — v5.6 chat arc

**Written:** 2026-10-29 evening PT
**Repo:** `/Users/brbrainerd/dev/vox`
**Branch:** `cursor/t3-deeper-extraction-v55-510f`
**HEAD:** `b458794b8` (phase 320 plans; context remains `445cb8ba3`)
**For:** the next Claude session continuing this chat

This file is the handoff. An earlier version existed only as a chat message. "Chat arc" means this whole session, not the ChatARC feature. The ChatARC feature is phase 322 and is one section below.

## What this session is

`/gsd-autonomous` on milestone v5.6 "Reasoning Where the Lane Needs It". Root `.planning/`, phases 313–322. Not a workstream. Do not pass `--ws`. Do not push. Do not open a PR.

The operator answered grey areas with `1` (accept all). On 2026-10-29 they said accept all remaining questions and proceed. Do not stop to re-ask a grey area.

`query phase.complete` cannot write this tree: workstream mode is on and no workstream is active. Advance `ROADMAP.md`, `STATE.md`, and `REQUIREMENTS.md` with a Python script that reads and writes the whole file, then `gsd-tools commit` on named paths. `StrReplace` on `ROADMAP.md` is blocked by a shrink guard. Never `git add -A`. Never set `GSD_ALLOW_PLANNING_SHRINK`.

## Done

| Phase | What | Close |
|---|---|---|
| 313–315 | Lane prefix, held-out set, serve gate | Already complete when this arc resumed |
| 316 | `vox mens eval-reasoning-compare`. Real 8B run is an absence record: `mens/runs/reasoning_compare/`, schema `reasoning_before_after.v1`, status `not_compared`, reason `base_snapshot_absent`. No invented scores. | Complete 2026-10-28 |
| 317 | SIZE-01. Pins for Qwen3-1.7B `@70d244cc86cc` and Qwen3-4B `@1cfa9a720891` on `qwen3_code`, `floor_measured: false`, no `floor_mb`. `pick_base` skips them. `resolve_qwen3_size` names 0.6B–32B. | Complete 2026-10-28 |
| 318 | SIZE-02. One live free-memory selector on `pick_base`. CUDA free is nvidia-smi `memory.free`. Metal free is `monitor_metal` `memory_free_mb`. Largest measured QLoRA floor that fits wins. Explicit size, including `VOX_MENS_DEFAULT_MODEL`, is refused, not stepped down. Caller preset stays when the size does not change. Directory-tier ladder in `auto_select.rs` is gone. Verified 12/12. | Complete 2026-10-29 |
| 319 | MESH-01. `accelerator_free_mb` on the node record. Mesh pick routes the largest fitting floor. Stale or missing reading cannot win. One fresh node is a mesh pick. No fresh reading uses the phase 318 local selector. Explicit size on that local path is refused (`ceead59d7`). A free-memory reading that does not fit in `u32` is dropped before the max (`f2ab6c443`). Verified 12/12, security 7/7, Nyquist compliant. | `38001f8dc` |

`SIZE-01`, `SIZE-02`, and `MESH-01` are checked in `.planning/REQUIREMENTS.md`.

## In flight

Phase 320 plans are committed (`b458794b8`). `320-01-PLAN.md` writes one `size_train.v1` record per QLoRA size from an injected reading. `320-02-PLAN.md` registers `vox mens size-train` and takes that reading once. Execute 320-01, then 320-02. Do not reopen the decisions. Do not download weights or start a live train.

Execution of those plans is underway. `ROADMAP.md` still lists phase 320 as not started until the phase is verified and closed.

## Phase 320 decisions (locked)

- Candidates: measured `qwen3_code` QLoRA rows whose `floor_mb` fits one phase-318 free-memory reading, taken once. Floors: 0.6B 2000, 8B 12000, 14B QLoRA 20000, 32B 60000. Smallest floor first. Every candidate gets a record.
- 1.7B and 4B stay unmeasured. Reason `floor_unmeasured`. No adapter. The 14B LoRA row is not a candidate. A floor above the reading is `does_not_fit`.
- Missing or incomplete snapshot: `base_snapshot_absent`. Ready means `config.json`, `tokenizer.json`, and a weight file. Do not download weights.
- Do not start a live train. A Ready snapshot this phase does not train is `train_not_run`. This machine's Hugging Face cache has Qwen3-0.6B snapshot `c1899de289a0` (config, tokenizer, `model.safetensors`), plus 1.7B and 4B. It does not have 8B, 14B, or 32B.
- Records: `mens/runs/size_train/<size>-<rev12>.json` (`mens/runs/` is gitignored). Schema `size_train.v1`. Fields: `status` `trained` or `not_trained`, `hf_id`, `size_token`, `free_mb`, `floor_mb` or `floor_measured: false`, and `reason` when not trained. No accuracy, think-initiation, or intelligence keys.
- Tests inject the reading and a Ready fixture versus an absent fixture. No network. No GPU. A 0-test exit 0 is not a pass. `vox-ml-cli` modules under `gpu` need `--features gpu`. Follow the `reasoning_compare` command shape. No new crate edge.

## Still to do after 320

321 has no directory. Lock these without asking, then plan and execute:

- The harness serves a chosen size's base or adapter through the existing serve path, behind the collateral gate.
- One report shows before/after reasoning and intelligence scores only for sizes that were actually trained. `train_not_run`, `base_snapshot_absent`, `floor_unmeasured`, and `does_not_fit` are listed with no numbers.
- The milestone is not complete until two sizes have real trained scores. This tree does not have those two trains. Do not invent scores. Do not download weights to manufacture them. Say so in the report.

322 already has context and no plans. Skip discuss. Plan, then execute. Context: `.planning/phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/CONTEXT.md`. Research: `docs/src/architecture/jev-system-one-harness-integration-research-2026.md`. The cursor plan `labeled_router_plus_jev` is folded here. Do not build it as a standalone facade.

After 320–322 pass or are explicitly deferred: `gsd-audit-milestone`, `gsd-complete-milestone v5.6`, `gsd-cleanup`. Not before. Two real trained scores are still missing, so the milestone stays open unless the operator defers that bar.

## Phase 322 (the feature named ChatARC)

ChatARC means Chat Action Routing Catalog. Requirements `DEC-01`..`DEC-04`, `CAT-01`, `CAT-02`, all still Pending.

- A versioned turn pack runs before `build_selection_request` in `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs`. It may set `task`, `complexity`, and `prefer_local` only when the caller has not set them. It may set `allow_free_in_performance_mode` only when clutch is unset. It never emits model ids. Flag off matches today's request bytes-for-bytes.
- `allow_free_in_performance_mode` only lets `:free` specs compete in the scorer under Performance. It does not invoke `PreferFree` and it does not reorder `FREE_FLOOR` / `OPENROUTER_FREE_FALLBACK_MODELS`.
- Post-tool low `model_still_ok` re-enters `apply_repair_escalation_to_config`. Stop and verify stay on `evaluate_stop_hook` and `evaluate_post_tool_batch_hook`. `draft_pr` is not a pack directive unless an existing PR tool is already on the shortlist.
- ChatARC composes skill roots, MCP tool lists including deferred hints, and `command_catalog::build_catalog`. No third registry. Choice is over K≤10 including `none`, consumed by skill activation, tool offering, or command hinting. Fail open to today's lexical and deferred behavior. `get_skill_reliability` stays the install gate.
- Hosted Jev is gated by `inference_privacy_local_only_from_env`. No MENS chat distillation. No unused facade. Do not fork SIZE-02 or MESH-01. Parallel to 317–321.

## Traps

- `lean-ctx -c 'cargo …'` dies at 600 seconds or 32 MB (exit 137) during compile. Redirect Cargo output to a file. `nohup` and `sh` are blocked. `python3 -c` and `node -e` are blocked; use a heredoc.
- Route Cargo through PATH `cargo` so the build broker sees it. Do not call `~/.cargo/bin/cargo`. Other checkouts (`vox-voxdiagram`) share the broker.
- `vox-populi` picker tests need `--features mens`. Orchestrator `vram_admission_*` needs `--features populi-transport`. `vox-gui` tests are on the binary target.
- Never `cargo fmt --all`. `rustfmt` only touched files. GSD worktrees are off. Executors use `--no-transition` and do not edit `STATE.md` or `ROADMAP.md`.
- UI gates for 318–320 were `frontend: false`. Skip UI specs unless a later gate says otherwise.
- The working tree has a large unrelated dirty set. Stage named paths only.
