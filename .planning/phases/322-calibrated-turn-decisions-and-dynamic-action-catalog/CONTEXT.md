# Phase 322 — Context (audited 2026-10-28)

## Critique of the first draft

| Claim in the first draft | Code / research | Correction |
| --- | --- | --- |
| Pack sets `allow_free` so PreferFree / `free_floor` wins | `PreferFree` is a `SelectionPolicy` step (`policy_for_profile`). `free_floor` is `vox_config::model_defaults::FREE_FLOOR` / `OPENROUTER_FREE_FALLBACK_MODELS`. `allow_free_in_performance_mode` only stops the scorer from dropping `:free` specs under Performance (`ranking.rs`). | Do not describe the pack as invoking PreferFree or reordering `free_floor`. |
| Primary site is `lifecycle_hooks` `for_task(CodeGen)` | Those functions are `resolve_mens_fast_apply_target` and `consult_oracle_model_advisory`. Chat routing builds intent in `llm_bridge/model_route_policy/resolve.rs::build_selection_request`, which already sets complexity, prefer_local, cacheable, axes from clutch/risk, and `allow_free` when `prefer_local` or `ClutchProfile::Free`. | Insert the pack **upstream** of `build_selection_request`. Do not override clutch. |
| New loop enum including `draft_pr` | Repair already re-selects via `apply_repair_escalation_to_config` (`repair_loop` + prefer_local + allow_free). Stop and post-tool hooks already exist. | Extend those functions. PR is a tool in the shortlist, not a pack primitive. |
| ChatARC discovers skills/MCP/CLI from scratch | Skill discovery + tier-1 catalog, MCP deferred tool hints, and `command_catalog::build_catalog` + scored search already exist. `get_skill_reliability` gates **external skill install**, not chat ranking (graph-audit "never read" is stale for install). | Compose those catalogs. Choice is the missing consumer. |
| Hard-depends on phase 321 | Size ladder / harness before-after does not produce decision labels. | Parallel with 317–321. Do not fork SIZE-02. |
| Zero-shot model routing | Still anti-fit (arxiv 2610.02267). | Unchanged. |

## Intent

1. Turn decision packs patch only fields `build_selection_request` does not already own, then the existing `select` / `decide` path runs.
2. Post-tool confidence re-enters `apply_repair_escalation_to_config`.
3. ChatARC = one shortlist over skills + MCP tools + command catalog entries, with calibrated Choice consumed by activation. Fail open to current lexical/deferred behavior.

## Do not

- Replace `select`, clutch/risk `effective_axes`, PreferFree profiles, `FREE_FLOOR`, SIZE-02, MESH-01, or MENS spoke weights.
- Emit model ids from the pack.
- Distill Jev into MENS chat SFT.
- Add a third discovery registry or a permanent unused facade.

## Anchors

- `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs`
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/batch_and_eval_gate.rs` (`apply_repair_escalation_to_config`)
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/lifecycle_hooks.rs` (fast-apply + oracle + stop/post-tool hooks)
- `crates/vox-orchestrator-mcp/src/chat_tools/skill_catalog.rs`
- `crates/vox-cli/src/command_catalog` (via `cli_dispatch`)
- `crates/vox-plugin-host/src/skill_registry.rs` (install reliability gate)
- Research §§11–17: `docs/src/architecture/jev-system-one-harness-integration-research-2026.md`

## Example (corrected)

User: fix a dead free-tier slug.

Epoch A may set `task=CodeGen` and a low `complexity` **if clutch is unset**. It must not flip Performance clutch into free eligibility by itself. If the operator's clutch is already `Free`, `build_selection_request` already sets `allow_free_in_performance_mode`. `select` may then rank a `:free` model; the ordered `FREE_FLOOR` list is a separate fallback, not this pack's output.

After a failed verify, Epoch B low `model_still_ok` calls the existing repair escalation (re-`decide` with `repair_loop`), not a new `draft_pr` state. Opening a PR happens if ChatARC's shortlist Choice selects an existing PR/command tool.
