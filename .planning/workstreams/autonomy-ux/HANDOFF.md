# Handoff: v1.2 Unified Autonomy Control (`autonomy-ux` workstream)

Written 2026-10-11 for the next IDE/agent. Read this whole file first. It assumes you know nothing about the earlier sessions.

## 1. What was asked

Original request (2026-10-08, voice-transcribed and run as `/gsd-new-milestone`). In the Loquela chat composer, the **Risk** chip and its inverted slider are confusing. Audit the GUI and backend, then plan one compact, visually human control for:

- how much oversight the user wants, and
- how often agents may interrupt them.

The plan should cover:

- what is and isn't controllable today;
- how the system decides when to interrupt and what to show;
- what must change, what each change achieves, and which pitfalls to avoid.

What happened next:

1. The request was run as a GSD milestone in a **parallel workstream**. v1.1 (`research-trial-flywheel`) was left untouched.
2. Research → requirements → roadmap.
3. `/gsd-plan-phase 12 --ws autonomy-ux` → research → context decisions → plans → checker → revision.

**Status: planning for Phase 12 is complete and committed. Execution has not started. No product code has been written for this workstream.**

## 2. Where everything is

| What | Path |
|---|---|
| Milestone research (audit + recommendations) | `.planning/workstreams/autonomy-ux/research/` — `AUDIT.md`, `STACK.md`, `FEATURES.md`, `ARCHITECTURE.md`, `PITFALLS.md`, `SUMMARY.md` |
| Requirements, decisions D1–D8, traceability | `.planning/workstreams/autonomy-ux/REQUIREMENTS.md` (35 requirements) |
| Roadmap, phases 12–19 | `.planning/workstreams/autonomy-ux/ROADMAP.md` |
| State | `.planning/workstreams/autonomy-ux/STATE.md` |
| Phase 12 | `.planning/workstreams/autonomy-ux/phases/12-safety-floor-lowering-paths/` — `12-RESEARCH.md`, `12-CONTEXT.md`, `12-VALIDATION.md`, `12-01`..`12-08-PLAN.md` |
| Project-level listing | `.planning/PROJECT.md` → `## Workstreams` |

**Branch:** `feat/research-trial-flywheel`. Nothing has been pushed by this workstream.

autonomy-ux commits, oldest first:

| Commit | Content |
|---|---|
| `0ea3d6767` | workstream + research |
| `99550b0f4` | requirements |
| `e9a650161` | roadmap |
| `42da4f32f` | state/roadmap fixes |
| `850f9fcd7` | Phase 12 research, context, validation |
| `ae4b938ec` | 8 plans, checker-revised; adds FLOOR-04b / BG-02b |

### Shared-repo hazards (read before touching git)

- **The phase-number collision is real.** Workstream `research-trial-flywheel` (v1.1) has its *own* "Phase 12". Another agent is executing it on this same branch right now (commits like `feat(12-07): ...`, `docs(12-08): ...` about eval-corpus/mens). Those are **not** autonomy-ux. When committing, scope messages as `docs(autonomy-ux)` / `feat(autonomy-ux 12-0N)` to keep them distinguishable.
- **`.planning/active-workstream` is a shared pointer that other agents keep rewriting.** It said `docs-freshness` at HEAD and `code-atlas` in the working tree. Do **not** edit it. Pass `--ws autonomy-ux` to every gsd command instead.
- **Uncommitted files you must not touch.** These belong to other agents/sessions (decision D7). Never stage, revert or reformat them.
  - `crates/vox-gui/ui/src/components/surfaces/Loquela/{DriveConsole,Loquela,RiskPopover,RiskPopover.test}.tsx`
  - `.../Chat/{ChatSurface,ChatSurface.test,GroundingCheckToggle,ResearchModeDropdown}.tsx`
  - `.../layout/{BreadcrumbBar,Sidebar}*.tsx`
  - `.../ui/Popover.tsx`
  - untracked `.../ui/Tooltip{,.test}.tsx`
  - `crates/vox-compiler/**` edits
  - `Cargo.lock`
  - root `package.json`
  - `.planning/research/.cache/`
  - `.planning/workstreams/code-atlas/state.json`
- **Always commit planning files with `git commit --only <paths>`.** Otherwise you sweep other agents' staged work into your commit. The pre-commit hook has hung once in this repo; if it does, kill the hook processes, don't `--no-verify`.

## 3. Audit findings: what exists today

Full detail with file:line evidence is in `research/AUDIT.md` and `phases/12-.../12-RESEARCH.md`. Re-verify line numbers before editing; the repo moves fast.

**The "Risk" chip is inverted and narrow.**

- **Inverted:** high risk means *fewer* checks.
- **Narrow:** it only affects background tasks.
- **It also hijacks model choice.** At Low risk it overrides the model axes (`vox-orchestrator/src/mode.rs` `effective_axes`).

**"Check replies"** (`GroundingCheckToggle`) is a separate, non-blocking mechanism.

**Dead or display-only controls:**

- `safetyTokenBudget` isn't wired to anything, and its inputs are always 0.0.
- The attention budget (`attention/budget.rs` `AttentionBudget`) is shown read-only.

**The dispatch gate** (`vox-orchestrator-mcp/src/dispatch.rs` ~L235–260; the file is over 2,374 lines, so put no new logic there) keys on tool names that mostly **don't exist**, such as `vox_deploy` and `vox_run_shell`.

**Real mutators with no gate:**

- `vox_snapshot_restore`, `vox_undo`, `vox_workspace_merge`
- `vox_config_set`, `vox_trust_override`, `vox_preference_set`
- `vox_scientia_publication_publish`, `vox_mesh_dispatch`
- plugin/skill installs

`permission_modes.rs` (~L87–92) auto-approves `vox_deploy` under `accept_all`.

**Ways oversight can be lowered** (all closed by Phase 12):

- A task's own `risk_posture` hint wins: `effective_risk_for_task = task.or(category).or(source)`. A self-assigned High turns off grounding and Socrates.
- `socrates.rs` `hint.unwrap_or`.
- `success/mod.rs` `trust_relax_gates`.
- Ungated `vox_preference_set` (`handlers_preferences.rs`) live-mutates `socrates_gate_enforce`, `attention_*`, and `approval_allowlist.*`. Writing that last key fakes an allowlist grant.
- The chat agent loop (`agent_loop.rs`) runs any tool name the model emits, not only the offered set.
- `message.rs:351` passes permission mode `None`.
- Any caller of `vox_resolve_approval` can approve any park, so an agent could approve its own floored action. This was found by the plan checker and is fixed by 12-07/12-08.

**Contract parity tests** (`control/mod.rs:23`, `protocol.rs:337`) compare ids only.

**Where the knobs live:**

- Approval tier: `attention_fields.rs` `effective_approval_tier`, `max_strictness`.
- Routing: `routing.rs` (trust auto-approve; external → Blocked).
- `interruption_policy.rs`, `gates.rs`.
- Contracts: `contracts/gui/drive-console.v1.yaml`, `contracts/orchestration/permission-modes.v1.yaml` (becomes v2 in 12-05).

## 4. Decisions (locked; do not reopen without the user)

**Milestone decisions (REQUIREMENTS.md):**

- **D1:** One Oversight control with three stops, **Relaxed · Standard · Strict**. More oversight means safer, so the scale is no longer inverted. It gets new ids plus a legacy inversion table.
- **D2:** Relaxed auto-approves chat edits.
- **D3:** The **safety floor** always asks a human, at every level:
  - force-push / history rewrite
  - allowlist / policy grants
  - deploy / external publish
  - secrets writes

  Shell and delete are governed by the level, not the floor.
- **D4:** The level is per-session, seeded from a per-user default.
- **D5:** Retire the Approvals view's permission-mode selector. Oversight becomes the only control.
- **D6:** Full attention scope (cross-agent attention inbox).
- **D7:** The Loquela/Chat/Popover/Tooltip WIP belongs to another agent. Coordinate; never overwrite. This is resolved as the first task of Phase 18.
- **D8:** Keep the legacy `risk` key for one release, with a retire-by marker.
- **Mode stays separate.** Mode (cost ↔ intelligence) remains its own axis. Safety tokens and delegate-to-free go to the Mode backlog.

**Phase 12 decisions (`12-CONTEXT.md`):**

- **P12-D1:** Floor `vox_snapshot_restore`, `vox_undo`, `vox_workspace_merge`, and force-push shell arguments.
- **P12-D2:** Floor `vox_deploy`, `vox_scientia_publication_publish`, plugin/skill installs, and `vox_mesh_dispatch`.
- **P12-D3:** Match secrets by key/path pattern, with the limits documented.
- **P12-D4:** Keep Socrates' no-context skip.
- **P12-D5:** `vox_attention_reset` is governed by the level.
- **P12-D6:** `permission-modes` v1 → **v2**.
- **P12-D7:** Floor **every** `vox_config_set`.
- **Also floored:** policy-shaped `vox_preference_set` keys (`approval_allowlist.*`, `socrates_gate_enforce`, `attention_*`, `oversight.*`).

Constraints:

- Effective oversight = max(floor, level, trust).
- The classifier sees only the tool name and arguments.
- Floor checks live in the dispatch path, implemented as new modules (`vox_orchestrator::oversight::floor`, `vox-orchestrator-mcp/src/floor_gate.rs`).
- Floored task-level actions use the Review tier.
- Add entries to `SKIP_DISPATCH_PROBE`.
- Invert the two existing lowering tests.

**Checker-driven rulings (2026-10-11):**

- Agents must not be able to approve their own floored park, and this is fixed **in Phase 12**: 12-07 covers the in-process dispatchers, 12-08 the stdio `vox mcp` server and the HTTP gateway.
- Two residuals were moved to later phases:
  - **T-12-25 → FLOOR-04b (Phase 14).** A raw daemon-socket or token-holding process can still resolve as if human until the authenticated human channel exists.
  - **T-12-23 → BG-02b (Phase 15).** A floor-lifted Review can still be cleared by agent self-attestation (`checks_passed` keys) until the attention inbox exists.

## 5. Roadmap (phases 12–19)

12. Safety Floor and lowering paths (planned)
13. Contract and resolver
14. Session state and RPC (FLOOR-04, FLOOR-04b, the authenticated human channel)
15. Background tasks (BG-02, BG-02b); runs in parallel with 16
16. Sync chat; runs in parallel with 15
17. Attention inbox
18. GUI (first task: settle D7 WIP ownership; also fix the GUI comments still citing `permission-modes.v1.yaml`)
19. Claims ledger / verification close-out

## 6. Phase 12 plans: what to execute

Covers requirements FLOOR-01, FLOOR-02, FLOOR-03, FLOOR-05.

| Plan | Wave | Depends on | Autonomous | Objective |
|---|---|---|---|---|
| 12-01 | 1 | — | **no** | Floor classifier `oversight/floor.rs` + `floor_gate.rs` hook. Tracer: `vox_config_set` parks under accept_all. Summaries redacted to key names. |
| 12-02 | 1 | — | yes | Task risk hint / category can only *raise* strictness. Configured posture is OR'd with global. `trust_relax_gates` can't lower. |
| 12-03 | 1 | — | yes | `vox_preference_set` refuses oversight/policy keys. |
| 12-04 | 2 | 01, 02 | yes | Background floor tier (Review). Call-site tests at every origin. Registered-or-reserved test. |
| 12-05 | 2 | 01 | **no** | `permission-modes` v2 contract; `vox_deploy` always-ask; `floor:` column agrees with `classify_floor`. |
| 12-07 | 3 | 01, 04 | yes | Refuse approve/modify of a floored park from in-process agent dispatchers; origin set by construction (`AGENT_ORIGIN`, `refuse_agent_floor_resolution`). |
| 12-08 | 4 | 07 | yes | Same refusal for the stdio `vox mcp` server and HTTP gateway entry points. |
| 12-06 | 5 | all | yes | Mutation proofs (M1…M2x, `grep -c` before/after each mutation, harmless fixtures), phase gate. Updates `12-VALIDATION.md` to real test names and sets `nyquist_compliant: true`. |

**Two blocking decision checkpoints** (the executor stops and asks the user):

1. **12-01 Task 1, "human clicks park until Phase 14".**
   - Today's first-party human paths will also require approval: GUI Undo, Skills add/install, Plugin install, Mesh dispatch, and CLI `vox orchestrator undo` / `vox dei workspace merge` / `vox dei snapshot restore`.
   - **No CLI approval command exists.** CLI-only users can't complete those actions without the GUI Approvals view.
   - Recommended answer: **`accept-park`**. The alternative, `defer-human-tools`, contradicts the locked P12-D1/D2/D7 and sends the phase back to discuss-phase.
2. **12-05 Task 1, contract rename (one-way door).**
   - `rename` (recommended): `git mv` v1 → v2 and note `migration_window: none`.
   - `side-by-side`: keep v1 frozen and deprecated.

**Checker history:**

- **Iteration 1:** 1 blocker (agent self-approval) and 8 warnings. The warnings: M12 test couldn't fail; Review self-attestation; v2 row `floor: policy_grant`; configured posture must be OR'd with global; M13/M14 tested helpers instead of call sites; missing mutation rows for `path_is_policy_file` and redaction; checkpoint wording; secret values in summaries.
- **Revision:** the planner revised the plans (adding 12-07/12-08) and committed `ae4b938ec`, whose message says "checker-verified".
- **Caveat:** that revision ran in a session that was interrupted, so this handoff's author did not see the final checker output. If you want certainty, re-run the checker; it's cheap (§8).

## 7. Known loose ends

- `STATE.md` progress counters predate the plans. The execute-phase tooling updates them.
- `12-VALIDATION.md` still has `nyquist_compliant: false` and some stale test-binary names. 12-06 Task 3 fixes both. Floor tests live in a separate binary, `pending_approvals_floor_tests`.
- **Deferred:**
  - checking the chat agent loop's calls against the offered tool set (optional);
  - GUI comments citing the v1 contract path (Phase 18);
  - 12-08's `entry_point_callers_are_classified` scan only matches literal `TOOL_CALL` / `"orch.tool_call"` in Rust, so frontend `invoke_mcp_tool` callers aren't enumerated (noted under FLOOR-04b for Phase 14).
- Phases 13–19 have no research, context, or plans yet.

## 8. How to continue

```bash
cd ~/dev/vox
G=~/.cursor/gsd-core/bin/gsd-tools.cjs
node $G check verify-command-paths 12 --raw --ws autonomy-ux       # pre-checker probe (was clean)
node $G check verify-failure-directions 12 --raw --ws autonomy-ux  # pre-checker probe (was clean)
```

- Optional re-check: spawn `gsd-plan-checker` on `phases/12-safety-floor-lowering-paths/12-0*-PLAN.md`, with REQUIREMENTS, CONTEXT, RESEARCH and VALIDATION as required reading.
- Execute: `/gsd-execute-phase 12 --ws autonomy-ux`. Answer the two checkpoints (§6).
- Then: `/gsd-plan-phase 13 --ws autonomy-ux`, and so on through 19.

Executing Rust work in this repo:

- **Tests first.** Every new `pub fn` gets a test in the same file.
- **Formatting:** `cargo fmt -p <crate>` or `vox run scripts/fmt.vox`. Never `cargo fmt --all`.
- **Before pushing:** `cargo clippy -p <crate> -- -D warnings`.
- **Use plain `cargo` on PATH.** It goes through the build broker.
- **No new crate dependency edges.** `exceptions` and ledger entries are user-authorized only.
- **Verify security guards by mutation:** break the guard, confirm the test fails, restore it, and confirm restoration with `grep -c`.
- **GUI changes (Phase 18)** need Playwright specs plus `review-bundle/latest/` screenshots.
- **Don't push or open PRs** unless the user asks.

## 9. Tooling gotchas seen in this environment

- The **lean-ctx** shell hook blocks `perl -pi`, `node -e`, `od`, and the native Read tool outside the repo. Use `sed -i ''`, `rg`, or the Write tool.
- lean-ctx shows *compressed* file previews. StrReplace or rewrites based on them have corrupted `PROJECT.md` before. For big edits, read the raw file with `sed -n`, and diff afterwards.
- `gsd query commit` run from a `cd`'d subshell with relative paths committed nothing. Use `git add` plus `git commit --only`.
- Subagent spawns occasionally return `resource_exhausted`. A fresh spawn (not a resume) has worked.
