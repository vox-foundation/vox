# Phase 12 Context — Safety Floor & Lowering Paths

Decisions locked by the user on 2026-10-11, after `12-RESEARCH.md`. Milestone decisions D1–D8 in `../../REQUIREMENTS.md` also apply.

## Locked decisions

- **P12-D1 — History rewrite floor:** `vox_snapshot_restore`, `vox_undo`, `vox_workspace_merge`, and force-push / history-rewrite shell invocations are floored (always require human approval).
- **P12-D2 — External effects floor:** deploy/external publish (`vox_deploy`, `vox_scientia_publication_publish`), plugin/skill installs, and mesh/gateway dispatch (`vox_mesh_dispatch`) are floored.
- **P12-D3 — Secrets writes:** floored by key/path pattern on generic config and file writes (secrets-shaped `vox_config_set` keys, `.env*` and credential paths). The fact that no dedicated secrets tool exists, and the limits of pattern matching, are documented in the floor module.
- **P12-D4 — Socrates without context:** tasks with no Socrates context keep skipping the Socrates check. This is not treated as a lowering path.
- **P12-D5 — `vox_attention_reset`:** governed by the oversight level, not floored.
- **P12-D6 — Contract:** `permission-modes.v1.yaml` is bumped to `permission-modes.v2.yaml`, adding always-ask flags and floor categories, with the YAML↔Rust parity test updated to match by id.
- **P12-D7 — `vox_config_set`:** every call is floored, because no first-party caller uses it.

## Policy decisions (from milestone D3)

- Policy/allowlist grants are floored: `vox_config_set`, `vox_trust_override`, `vox_add_approval_allowlist_entry`, and `vox_preference_set` writes to policy-shaped keys (`approval_allowlist.*`, `socrates_gate_enforce`, `attention_*`, `oversight.*`). Non-policy preference keys remain writable.
- Shell and delete are NOT floored (level-governed), except force-push / history-rewrite shell arguments (P12-D1).

## Constraints from research

- The floor classifier takes only tool name and arguments — no oversight level or trust input.
- Floored calls are never auto-approved and never allowlisted; effective = max(floor, level, trust).
- Floored calls must be checked in the dispatch path itself, not only in the offered tool list (the chat loop runs any tool the model names).
- Task-level floored actions use the Review tier (Blocked rejects instead of waiting).
- New logic lives in its own module (`vox_orchestrator::oversight::floor` plus a small dispatch hook); `dispatch.rs` is already over the file-size limit.
- Newly floored tools go in `SKIP_DISPATCH_PROBE` so the registry test does not wait 300 s per tool.
- The two existing tests that assert lowering behavior are inverted, not deleted.
- Every guard gets a call-site test with a recorded mutation proof (`grep -c` before and after).
