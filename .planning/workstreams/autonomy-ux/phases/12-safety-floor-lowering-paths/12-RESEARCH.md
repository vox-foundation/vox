# Phase 12: Safety Floor & Lowering Paths — Research

**Researched:** 2026-10-08
**Domain:** MCP tool-dispatch approval gate, orchestrator task approval tiers, Socrates/grounding completion gates
**Confidence:** HIGH for code paths (all read directly); MEDIUM for shell-arg classification limits (by design best-effort)

## User Constraints (from REQUIREMENTS.md — locked, copied verbatim)

- D1: Three stops — Relaxed · Standard · Strict; new ids, legacy `high/moderate/low` mapped by explicit inversion table.
- D2: Relaxed auto-approves chat file edits; Standard and Strict ask.
- D3: Hard floor at every level = force-push / history rewrite, allowlist / policy grants, deploy / external publish, and secrets writes. Shell and delete are level-governed (not floored).
- D4: Level is per session, seeded from a saved per-user default.
- D5: Oversight is the only control — the Approvals view permission-mode selector is retired.
- D6: Full attention scope — inbox, "why this reached you", level-aware admission, per-agent fair-share quotas, batched digest.
- D7: Uncommitted Loquela/Chat/Popover/Tooltip edits belong to another agent — GUI phase must coordinate (land or rebase onto them), never overwrite.
- D8: Legacy drive-API `risk` key kept one release as a deprecated alias with a `vox-deprecated-since`/`retire-by` marker.

Out of scope (REQUIREMENTS.md): "New crates or new crate edges — logic lives in `vox-orchestrator`, consumers in `vox-orchestrator-mcp`, RPC constants in `vox-foundation`, `vox-gui` forwards only." FLOOR-04 (authenticated RPC for lowering a session level) is Phase 14. No oversight *level* exists yet in Phase 12 — the floor must be built so Phase 13's `oversight::resolve()` composes with it via `max`, not replaced by it.

<phase_requirements>

| ID | Requirement (abridged) | Research support |
|----|------------------------|------------------|
| FLOOR-01 | Force-push/history-rewrite and allowlist/policy-grant actions need human approval at every level; typed floor takes no level; effective = max(floor, level, trust) | §Architecture Patterns 1–2, §Inventory A/B, §Code Examples 1–3 |
| FLOOR-02 | Deploy/external-publish (incl. `vox_deploy`) and secrets writes floored; classified by action type, not description keywords | §Inventory C/D, §Pitfall 1 (phantom names), §Code Example 4 (YAML+Rust parity) |
| FLOOR-03 | No agent input can lower oversight: task hints/`risk`, `vox_preference_set`, trust gate relaxation only raise | §Lowering-path inventory L1–L8, §Code Examples 5–7 |
| FLOOR-05 | Every floor and lowering-path guard has a call-site test proven by mutation | §Validation Architecture |

</phase_requirements>

## Summary

The dangerous-tool gate in `dispatch.rs` is keyed **only on the canonical tool name**; args are never inspected [VERIFIED: crates/vox-orchestrator-mcp/src/dispatch.rs:235-362 — `if is_gated_tool(name_canonical) {`]. Worse, four of the six names in `RISK_CLASSES` (`vox_write_file`, `vox_multi_replace*`, `vox_delete_file`, `vox_run_shell`, `vox_deploy`) are **phantom**: they are absent from `contracts/mcp/tool-registry.canonical.yaml` and have no `"name" =>` dispatch arm, so a call parks and then falls through to "Unknown tool" [VERIFIED: rg found `vox_deploy` only in permission_modes.rs and vox-foundation agentos_mutation.rs; dispatch.rs:1930-1945 fallthrough]. Meanwhile the *real* mutators — `vox_apply_structured_edit` (writes any path; `state.repository.root.join(file_path)` accepts absolute paths), `vox_skill_run` (arbitrary `command` in a container sandbox), `vox_scientia_publication_publish`, `vox_news_approve`, `vox_publish_message`, `vox_openclaw_gateway_call`, `vox_agy_delegate` — are **not gated at all**. `scope_guard.rs` already hit and fixed this exact class ("the previous list never was [registered], so no gate ever fired") with an `every_write_tool_is_registered` test; reuse that pattern [VERIFIED: scope_guard.rs:9-15].

All three tool-call origins converge on one chokepoint: stdio/HTTP MCP clients (`handle_tool_call_with_mode`), the chat agent loop (agent_loop.rs:974-980), and autonomous runtime `@tool` intents via `McpToolDispatcher` (runtime.rs:1029-1038 → autonomous_tool_dispatch.rs:47). So the floor belongs in `dispatch_tool_call`, computed from `(tool, args)`, with the pure classifier in `vox-orchestrator`.

Lowering paths are broader than the requirement text: besides `risk` hints, `vox_preference_set`, and `trust_relax_gates`, the agent-reachable `vox_config_set` merges an **arbitrary JSON patch** into the live `OrchestratorConfig` (can flip `socrates_gate_enforce`, `trust_gate_relax_enabled`, `trust_auto_approve_min`, `agentos_guardrail_kernel_enabled`), `vox_trust_override` sets any agent's trust to 1.0, and `vox_resolve_approval` lets the caller resolve a pending approval — including the one the floor just parked. The chat agent loop dispatches **any** registry name the model emits, not just the offered set (agent_loop.rs:974-980), so "not offered to chat" is not a control.

**Primary recommendation:** Add a pure `vox-orchestrator::safety_floor` module (`classify_floor(&FloorInput) -> Option<FloorCategory>`, no level parameter), enforce it in `dispatch_tool_call` so floored calls always park (bypassing `mode_auto_approves` and the allowlist), make every agent-reachable config/trust/preference/approval mutator either floored or refused, change hint composition to strictest-wins, and invert the two existing tests that currently *assert* the lowering behavior.

## Inventory — actions D3 floors

Structured identifier = classifiable from tool name or a typed arg. Free-text = only via a command string or description.

### A. Force-push / history rewrite

| Surface | Today | Structured? | Evidence |
|---|---|---|---|
| `vox_push`, `vox_force_push`, `vox_branch_delete` | Names exist only in vox-foundation vocabulary (`local_mutation`); no registry entry, no handler | Name only (phantom) | [VERIFIED: crates/vox-foundation/src/primitives/agentos_mutation.rs:8-75] |
| `vox_skill_run { command }` | Runs arbitrary command string in `SandboxedSkillRunner` (Docker/Podman via vox-container; OpenClaw sidecar fallback); ungated | Free-text | [VERIFIED: skills_tools.rs:417-421 `runner.run(&command, &limits)`; vox-skills/src/sandbox/mod.rs:1-14] |
| `vox_run_shell` | Gated by name (Destructive, `always_requires_approval: false`); no handler | Phantom; args never read | [VERIFIED: permission_modes.rs:56-106] |
| `git_exec::GitExec::run` (internal git used by MCP git tools) | `BANNED_PREFIXES` denies `stash`, `reset --hard`, `clean -f*`, `checkout .`…; **does not list `push --force`** | argv windows | [VERIFIED: git_exec.rs:48-67] |
| `vox_undo` / `vox_redo` / `vox_snapshot_restore` | Local jj-oplog/snapshot ops; ungated | Name | [VERIFIED: dispatch.rs:1040-1045] — whether these count as "history rewrite" is Open Question 2 |
| `vox_agy_delegate`, `vox_openclaw_gateway_call` | Delegate to an external agent/gateway that has its own tools; vox cannot see what they run | Name only | [VERIFIED: dispatch.rs:908, 970] |

### B. Allowlist / policy / oversight grants

| Surface | Today | Evidence |
|---|---|---|
| `vox_add_approval_allowlist_entry` | Correctly floored (`always_requires_approval: true`) and has call-site test | [VERIFIED: permission_modes.rs:93-105; pending_approvals_tests.rs:570-600] |
| `vox_config_set` | Ungated; merges arbitrary JSON into whole `OrchestratorConfig`, replaces live | [VERIFIED: dei_tools/control.rs:311-357; dispatch.rs:1644] |
| `vox_preference_set` | Ungated; live-mutates `socrates_gate_enforced`, `attention_enabled`, `attention_budget_ms`, `attention_alert_threshold`; no key validation | [VERIFIED: memory_tools/handlers_preferences.rs:36-106; dispatch.rs:1520-1522] |
| `vox_trust_override` | Ungated; `force_trust_score(agent_id, trust_score default 1.0)` | [VERIFIED: dispatch.rs:1608-1626] |
| `vox_attention_reset` | Ungated; offered in default chat turn set (lane ai) | [VERIFIED: dispatch.rs:1597-1607] |
| `vox_resolve_approval` | Ungated; resolves any pending approval by id with caller-supplied decision; no caller identity check | [VERIFIED: dispatch.rs:735-747] |
| `vox_set_agent_budget` | In HTTP `DEFAULT_ALLOWED_TOOLS` | [VERIFIED: http_gateway/mod.rs:56-91] |
| File writes to policy files (`Vox.toml`, `contracts/orchestration/*`, `.vox/agents/*.md` scope files, `.cursor/rules/*`) via `vox_apply_structured_edit` | Ungated; path-classifiable | [VERIFIED: compiler_tools.rs:905-979] |

### C. Deploy / external publish

| Surface | Today | Evidence |
|---|---|---|
| `vox_deploy` | Destructive, `always_requires_approval: false` in Rust **and** YAML; phantom (no handler) | [VERIFIED: permission_modes.rs:87-92; contracts/orchestration/permission-modes.v1.yaml:81-87] |
| `vox_scientia_publication_publish`, `…_submit_local`, `…_scholarly_pipeline_run`, `…_external_jobs_tick` | Ungated, lane data | [VERIFIED: scientia_tools/syndication.rs:78; dispatch.rs:1294, 1334] |
| `vox_news_approve` (live syndication) vs `vox_news_test_syndicate` (forces `dry_run = true`) | Ungated | [VERIFIED: news_tools.rs:56-62, 151] |
| `vox_publish_message` | Ungated, lane ai — confirm whether it leaves the process (Open Question 3) | [VERIFIED: dispatch.rs:896] |
| Background tasks | Only `task_description_suggests_external` substrings ("deploy", "publish ", "https://", "kubectl", "secret "…) or a manifest path containing "deploy" → `action.external` → `Blocked` | [VERIFIED: attention_fields.rs:11-23, 62-68; routing.rs:291-293] |

### D. Secrets writes

| Surface | Today | Evidence |
|---|---|---|
| MCP secrets tools | Only `vox_secrets_doctor` (read-only) exists; **no MCP secret-write tool** | [VERIFIED: secrets_tools: single `pub async fn secrets_doctor`; rg `store_secret|set_secret|keyring` in vox-orchestrator(-mcp)/src → no matches] |
| `vox_apply_structured_edit` to `.env*`, `*.pem`, `*.key`, `~/.ssh/*`, `~/.aws/credentials`, `.secrets` | Ungated; absolute `file_path` escapes repo root via `Path::join` | [VERIFIED: compiler_tools.rs:921 `let abs_path = state.repository.root.join(file_path);`] |
| `vox secrets set` / keychain via `vox_skill_run` command | Free-text | [ASSUMED: CLI exists; reachable only through command string] |

## Lowering-path inventory (FLOOR-03)

| # | Path | Mechanism today | Fix |
|---|---|---|---|
| L1 | `risk` hint on `vox_submit_task` / chat | `params.risk` → `TaskEnqueueHints.risk` → `task.risk_posture = Some(posture)` [VERIFIED: task_tools/submission.rs:181; types/tasks.rs:992-996] | Strictest-wins composition (below) |
| L2 | Hint beats configured policy | `effective_risk_for_task`: `task.risk_posture.or(category_risk).or(source_risk)` [VERIFIED: mode.rs:729-737] — hint `High` erases a configured category `Low` | New `strictest_risk_for_task` for oversight consumers only; leave the cost/Mode use alone (Phase 13 removes coupling) |
| L3 | Socrates/grounding enforce replaced by hint | `resolved.map(\|r\| r.grounding_enforce).unwrap_or(global_grounding_enforce)` [VERIFIED: socrates.rs:16-30]; `RiskPosture::High` resolves both to false [VERIFIED: mode.rs:242-266] | `hint_enforce \|\| configured_enforce` |
| L4 | Approval tier lowered by hint vs category | `effective_approval_tier` = `classified.max_strictness(risk.resolve()…)` [VERIFIED: attention_fields.rs:32-41] — max with *classified* is fine, but `risk` is already the hint-over-policy value from L2 | Feed strictest risk |
| L5 | `trust_relax_gates` | `trust_gate_relax_enabled && reliability >= min` [VERIFIED: success/mod.rs:426-432]; then `grounding_enforce && !trust_relax_gates` and `(socrates_enforce \|\| bypass_disallowed) && !trust_relax_gates` [VERIFIED: socrates.rs:119-121, 210-211] — also waives `bypass_disallowed` | Trust may waive only enforcement that a *hint* added above configured; never configured enforcement, never `bypass_disallowed` |
| L6 | `vox_preference_set` | Keys `socrates_gate_enforced` (note trailing **d**; config field is `socrates_gate_enforce`), `attention_*` live-mutate config [VERIFIED: handlers_preferences.rs:45-97]; rehydrated at startup [VERIFIED: server_state.rs:608] | Refuse oversight-affecting keys with an error |
| L7 | `vox_config_set` | Arbitrary patch [VERIFIED: control.rs:311-357]; default `trust_gate_relax_enabled = false` is one call away from `true` [VERIFIED: config/impl_default.rs:72] | Floor as policy grant (park), or refuse patches touching a denylist of safety fields — see Open Question 1 |
| L8 | `vox_trust_override`, `vox_resolve_approval` | Agent raises own trust / approves own park [VERIFIED: dispatch.rs:1608-1626, 735-747] | Floor `vox_trust_override` (policy grant). `vox_resolve_approval` caller authentication is FLOOR-04/Phase 14 — record as residual risk; at minimum refuse resolution from the dispatch path that registered the park (Open Question 4) |

Legitimate writers are unaffected by refusing L6/L7 keys over MCP: the GUI writes Socrates/trust settings by editing `Vox.toml` `[orchestrator]` directly [VERIFIED: vox-gui/src/commands/orchestrator.rs:440-536]; `vox attention` CLI writes `user_preferences` via vox-db directly [VERIFIED: vox-cli/src/commands/attention.rs:188-230]; the GUI's allowlist button calls `vox_add_approval_allowlist_entry` which already parks [VERIFIED: ApprovalsView.tsx:162].

## Architectural Responsibility Map

| Responsibility | Crate (layer) | File | Rationale |
|---|---|---|---|
| Floor categories + pure classifier (`classify_floor`) and shell/argv/path classifiers | `vox-orchestrator` (L3) | new `src/safety_floor.rs` (or `attention/floor.rs`) | Pure logic; reused by background admission; `vox-orchestrator-mcp` already depends on it [VERIFIED: layers.toml:210 `vox-orchestrator = { layer = 3, … }`] |
| Enforcement at tool-call chokepoint | `vox-orchestrator-mcp` (L3) | `dispatch.rs` gate block, `permission_modes.rs` | Single convergence point for MCP, chat loop, autonomous runtime [VERIFIED: layers.toml:234] |
| Name vocabulary (already exists) | `vox-foundation` (L1) | `agentos_mutation.rs` | Do not move; may be read by classifier only if vox-orchestrator already depends on vox-foundation (it does via umbrella — confirm with `cargo tree -p vox-orchestrator -e normal --depth 1`) |
| Background task floor | `vox-orchestrator` | `attention_fields.rs::populate_task_attention_fields` | Set `action.external`/floor from manifest paths + tool_hints, keep substring as supplementary escalator |
| Hint/trust composition | `vox-orchestrator` | `mode.rs`, `socrates.rs`, `success/mod.rs` | |
| Preference/config refusal | `vox-orchestrator-mcp` | `handlers_preferences.rs`, `dei_tools/control.rs` | |
| GUI | none in Phase 12 | — | D7: do not touch GUI files |

No new crate edges are needed. Do not add a `vox-orchestrator-types` floor type unless GUI needs it later (it would be L0 and fine, but YAGNI now).

## Project Constraints (from .cursor/rules/ and AGENTS.md)

- **Crate edges:** no new workspace edges; `exceptions` ledger entries are user-authorized-only; check with `vox ci crate-edges` [CITED: AGENTS.md §Dependency Discipline].
- **Test-first:** every new `pub fn` in `crates/*/src/**` needs a test in the same file before commit; `tdd-guard` pre-commit hook [CITED: AGENTS.md §Test-First Policy].
- **Mutation proof:** "Break the guard deliberately and confirm the test fails, then restore… Guard the mutation with a before/after presence check" [CITED: AGENTS.md §PR & Review Discipline].
- **Formatting:** never `cargo fmt --all`; use `vox run scripts/fmt.vox` (dirty files) [CITED: AGENTS.md].
- **Secrets:** no `std::env::var` for secrets; Phase 12 adds no secret reads [CITED: .cursor/rules/secrets-policy.mdc].
- **Contracts:** `permission-modes.v1.yaml` is `lifecycle: stable`, `x-vox-version: 1`; flipping a row's `always_requires_approval` is a tightening, not a breaking shape change — no rename, but note the change in the file [VERIFIED: contracts/orchestration/permission-modes.v1.yaml header]. If a new `floor:` column is added, bump per data-storage policy [CITED: .cursor/rules/data-storage-policy.mdc].
- **Registry SSOT:** any new tool or registry change goes through `contracts/mcp/tool-registry.canonical.yaml`; `yaml_registry_tools_have_dispatch_match_arms` requires an arm per entry; newly-parking registry tools must be added to `SKIP_DISPATCH_PROBE` [VERIFIED: dispatch.rs:2138, 2150-2161].
- **Pre-push clippy gap:** run `cargo clippy -p vox-orchestrator -p vox-orchestrator-mcp -- -D warnings` before handoff [CITED: AGENTS.md §Perennial Bug Patterns].
- **Build broker:** plain `cargo` on PATH only [CITED: AGENTS.md §Build Broker].
- **Parallel workstream:** do not touch `research-trial-flywheel` files (STATE.md).
- Project skill `.agents/skills/drive-task` exists; not relevant to this phase's code.

## Standard Stack

No new packages. Use what the crates already depend on: `serde_json::Value` for args, `std::path::{Path, Component}` for lexical path normalization, `shlex`-style splitting only if `shlex` is already in `vox-orchestrator`'s dependency tree (check `Cargo.toml`; otherwise a ~20-line whitespace/quote tokenizer is acceptable per defactor policy), `serial_test` + `IsolatedDbEnv` (already used by pending_approvals_tests.rs).

## Architecture Patterns

### 1. Floor is a function of the action only

```text
effective_tier = max_strictness(floor_tier(action), level_tier(level), trust_tier(agent))
                  ^ Phase 12            ^ Phase 13 (today: PermissionMode)   ^ classify_tier
```

The floor signature must not accept `PermissionMode`, `RiskPosture`, trust, or any oversight level, so no future level can lower it. This is checkable by code review and by a compile-time shape test (call it with only `(tool, args)`).

### 2. Enforcement placement in `dispatch_tool_call`

Today [VERIFIED: dispatch.rs:235-362]:

```text
if is_gated_tool(name) {
    mode_auto_approved = mode.auto_approves(name)        // false when always_requires_approval
    allowlisted = allowlist_eligible(name) && in_allowlist
    if !mode_auto_approved && !allowlisted { park }
}
```

Target:

```text
let floor = vox_orchestrator::safety_floor::classify_floor(&FloorInput::tool(name, &args));
if is_gated_tool(name) || floor.is_some() {
    mode_auto_approved = floor.is_none() && mode.auto_approves(name)
    allowlisted        = floor.is_none() && allowlist_eligible(name) && in_allowlist
    if !mode_auto_approved && !allowlisted { park (record floor category in approval summary) }
}
```

Place it **after** budget/scope/lock/skill-permission checks (those deny; the floor parks) and keep it outside the `agentos_guardrail_kernel_enabled` conditional so a config flip cannot disable it.

### 3. Background tasks

In `populate_task_attention_fields`, compute `classify_floor(&FloorInput::task(&task))` from structured signals (write manifest paths, `tool_hints`, `task_category`) and lift `effective_tier` to at least `Review` (or `Blocked` to match current external behavior — Open Question 5). Keep `task_description_suggests_external` as a supplementary escalator only [CITED: research/PITFALLS.md].

### 4. Strictest-wins composition

`RiskPosture` strictness is inverted relative to naming: `Low` (strictest) > `Moderate` > `High` (loosest) [VERIFIED: mode.rs:242-266]. Implement `RiskPosture::strictness_rank()` and `strictest(a, b)`; compose hint, category, source with it.

### Anti-patterns

- Testing `classify_floor` alone and calling FLOOR-05 done — PITFALLS C8: helper tests pass with the call site deleted.
- Adding names to `RISK_CLASSES` that are not in the registry (repeats the phantom bug).
- Relying on lane/offer filtering as a control (agent_loop.rs dispatches any name).
- Treating the description substring check as the classifier.

## Don't Hand-Roll

| Problem | Don't build | Use instead |
|---|---|---|
| Parking + waiting for human | New approval queue | `state.pending_approvals` + existing gate block (dispatch.rs:235-362) |
| Tool name vocabulary | New mutation-kind list | `vox_foundation::…::mutation_kind_for_tool` as a seed; canonical registry yaml as SSOT |
| Registry drift check | New ad-hoc test | Mirror `scope_guard::every_write_tool_is_registered` for `RISK_CLASSES` and floor name lists |
| Argv window matching | New matcher | Pattern from `git_exec::is_banned` (window equality after `-c k=v`) |
| Tier max | Custom ordering | `ApprovalTier::max_strictness` [VERIFIED: attention/budget.rs:98-133] |
| YAML↔Rust parity | New test | Extend `risk_classes_yaml_matches_rust_table` [VERIFIED: pending_approvals_tests.rs:500-560] |

## Common Pitfalls

1. **Phantom gated names.** Flipping `vox_deploy` to `always_requires_approval: true` (success criterion 2) is required but has *zero* runtime effect today because nothing dispatches `vox_deploy`. The plan must say so and also floor the real publish tools (Inventory C). Add a test: every `RISK_CLASSES` name is in `TOOL_REGISTRY` **or** in an explicit `RESERVED_GATED_NAMES` list with a comment.
2. **Existing tests encode the bug.** `socrates.rs:369-394 explicit_risk_posture_still_wins_over_category_policy` asserts High hint turns enforcement *off*; `attention_fields.rs:~285-316` asserts "explicit risk_posture must win over category policy" yielding `AutoApprove`. Both must be inverted, not deleted — they become the FLOOR-03 regression tests.
3. **Key spelling.** Refuse both `socrates_gate_enforced` and `socrates_gate_enforce`, plus prefixes `attention_`, `oversight.`, `trust_`, `approval`, `permission`, `tier_gate`. Prefer an allowlist of permitted preference keys if the set of legitimate keys is small (check `rg 'set_user_preference' crates/`).
4. **Path escape.** Classify the *resolved* path: `root.join(p)` with absolute `p` returns `p`; normalize `..` lexically. Classifying the raw arg misses `/Users/x/.ssh/id_ed25519` and `../../.env`.
5. **Shell classification is defeatable.** Aliases, `sh -c "$(echo …)"`, scripts, `GIT_*` env, git config `push.default`/`remote.*.push` with `+` refspecs, base64. It is an escalator, not a boundary; D3 already makes shell level-governed. Document limits in the module doc comment.
6. **Mutation tests that never ran.** Use the before/after `grep -c` presence check around the deleted guard line; a concurrent fmt or agent can revert the mutation.
7. **Timeout = deny, not approve.** A floored call that times out (300s `APPROVAL_TIMEOUT`) must deny; verify the existing timeout branch does.
8. **LoC budgets.** vox-orchestrator `max_loc = 70_000`, vox-orchestrator-mcp `max_loc = 40_000` [VERIFIED: layers.toml:210, 234]; keep the new module small and run `cargo run -q -p vox-arch-check`.

## Code Examples

### 1. Floor types (new, `vox-orchestrator/src/safety_floor.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorCategory { HistoryRewrite, PolicyGrant, ExternalPublish, SecretsWrite }

pub enum FloorInput<'a> {
    Tool { name: &'a str, args: &'a serde_json::Value, repo_root: &'a std::path::Path },
    Task { write_paths: &'a [std::path::PathBuf], tool_hints: &'a [String] },
}

/// Deliberately takes no oversight level, permission mode, risk posture, or trust.
pub fn classify_floor(input: &FloorInput<'_>) -> Option<FloorCategory> { /* … */ }

pub fn floor_tier(c: Option<FloorCategory>) -> ApprovalTier {
    if c.is_some() { ApprovalTier::Review } else { ApprovalTier::AutoApprove }
}
```

### 2. Name table (seed)

```rust
const POLICY_GRANT_TOOLS: &[&str] = &[
    "vox_add_approval_allowlist_entry", "vox_config_set", "vox_trust_override",
];
const EXTERNAL_PUBLISH_TOOLS: &[&str] = &[
    "vox_deploy", "vox_scientia_publication_publish", "vox_scientia_publication_submit_local",
    "vox_news_approve", /* + confirmed: vox_publish_message, scholarly_pipeline_run, external_jobs_tick */
];
const HISTORY_REWRITE_TOOLS: &[&str] = &["vox_force_push"];
```

### 3. Force-push classification from a command string (best-effort)

```rust
fn command_rewrites_history(cmd: &str) -> bool {
    let toks: Vec<&str> = cmd.split(|c: char| c.is_whitespace() || c == ';' || c == '&' || c == '|')
        .filter(|t| !t.is_empty()).collect();
    toks.windows(2).enumerate().any(|(i, w)| {
        let rest = &toks[i + 2..];
        match w {
            ["git", "push"] => rest.iter().any(|t| matches!(*t, "-f" | "--force" | "--force-with-lease" | "--force-if-includes" | "--mirror" | "--delete" | "-d")
                || t.starts_with("--force-with-lease=") || t.starts_with('+') || t.starts_with(":")),
            ["git", "filter-branch"] | ["git", "filter-repo"] | ["git", "replace"] => true,
            ["jj", "git"] => rest.first() == Some(&"push") && rest.iter().any(|t| matches!(*t, "--allow-new" | "--force" | "--deleted")),
            _ => false,
        }
    })
}
```

Also skip leading `-c k=v`/`-C dir` before the subcommand (as `git_exec::is_banned` does). `rebase`, `reset --hard`, `commit --amend` rewrite *local* history only and stay level-governed (D3: "force-push / history rewrite" — published history). Open Question 2 records the decision.

### 4. `vox_deploy` flip (both sides; parity test enforces)

```yaml
# contracts/orchestration/permission-modes.v1.yaml
  - tool: vox_deploy
    class: destructive
    reversible: false
    always_requires_approval: true
```

```rust
// permission_modes.rs RISK_CLASSES
RiskClass { tool: "vox_deploy", class: SafetyClass::Destructive, reversible: false, always_requires_approval: true },
```

### 5. Socrates hint can only add

```rust
pub fn resolve_grounding_socrates_enforce(task: &AgentTask, ov: &TaskPolicyOverrides, global_g: bool, global_s: bool) -> (bool, bool) {
    let configured = configured_risk(task, ov).map(|r| r.resolve());          // category/source only
    let hinted     = task.risk_posture.map(|r| r.resolve());
    let cfg_g = configured.map_or(global_g, |r| r.grounding_enforce || global_g);
    let cfg_s = configured.map_or(global_s, |r| r.socrates_enforce || global_s);
    (cfg_g || hinted.is_some_and(|r| r.grounding_enforce),
     cfg_s || hinted.is_some_and(|r| r.socrates_enforce))
}
```

### 6. Trust relaxation waives only hint-added enforcement

Return `(configured_enforce, hinted_extra)` and gate on `configured_enforce || (hinted_extra && !trust_relax_gates)`; remove `!trust_relax_gates` from the `bypass_disallowed` arm [VERIFIED: socrates.rs:210-211].

### 7. Preference refusal

```rust
const OVERSIGHT_PREF_PREFIXES: &[&str] = &["socrates_gate_enforce", "attention_", "oversight.", "trust_", "approval", "permission"];
if OVERSIGHT_PREF_PREFIXES.iter().any(|p| key.starts_with(p)) {
    return ToolResult::<String>::err(format!("preference '{key}' affects oversight and cannot be set via MCP; use the GUI/CLI")).to_json();
}
```

(`starts_with("socrates_gate_enforce")` covers the `…d` spelling.) Note: this breaks any agent workflow that toggled attention via MCP; the CLI path remains.

## Assumptions Log

| # | Claim | Risk if wrong |
|---|---|---|
| A1 | `vox_publish_message` leaves the process (external publish) | Over-flooring an internal message bus; confirm in handler |
| A2 | `vox_skill_run` container can reach a git remote with credentials | If fully network/creds-isolated, force-push via skill_run is moot (still classify) |
| A3 | `vox-orchestrator` can see `vox-foundation` vocabulary without a new edge | If not, duplicate the ~20-name list with `// vox:defactored-from` |
| A4 | Approval timeout path denies | If it approves, floor is defeated by waiting |
| A5 | No agent workflow legitimately needs `vox_config_set` unattended | Parking it may stall automation; alternative is field denylist |

## Open Questions

1. **`vox_config_set`: park (policy grant) or refuse safety fields?** Recommendation: floor the whole tool (PolicyGrant) — any config field can affect oversight indirectly, and humans have the GUI.
2. **Does "history rewrite" include local `vox_undo`/`vox_snapshot_restore`/`git rebase`/`reset --hard`?** Recommendation: no — floor only published-history rewrites (force push, ref delete, filter-repo); local rewrites are recoverable via jj oplog and stay level-governed (consistent with D3 "shell and delete are level-governed").
3. Which scientia/news tools actually reach external services vs local DB rows? Confirm per handler before adding to `EXTERNAL_PUBLISH_TOOLS`.
4. **Self-approval via `vox_resolve_approval`.** Full fix is FLOOR-04 (Phase 14). Interim: should Phase 12 floor or refuse `vox_resolve_approval` from the chat agent loop / autonomous dispatcher origins? Recommendation: yes — refuse when the call originates from `McpToolDispatcher` or the chat agent loop (both are in-process origins vox controls), leave GUI/HTTP to Phase 14.
5. Background floored tasks: `Review` (reaches inbox in Phase 15/BG-02) or `Blocked` (current external behavior)? Recommendation: `Blocked` until BG-02 lands, then `Review`.

## Environment Availability

| Dependency | Available | Notes |
|---|---|---|
| Rust toolchain / cargo (via broker shim) | yes | macOS host |
| `serial_test`, `IsolatedDbEnv` | yes | used by pending_approvals_tests.rs |
| `cargo mutants` | not required in Phase 12 | Phase 19 runs it; Phase 12 uses manual mutation |
| Docker/Podman | not required | `vox_skill_run` tests should assert parking, not execution |

`.planning/config.json` has no `workflow.nyquist_validation` key → treated as enabled.

## Validation Architecture

### Test framework

| Property | Value |
|---|---|
| Framework | Rust `#[test]` / `#[tokio::test]` + `serial_test` |
| Call-site harness | `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs` (`IsolatedDbEnv::new()`, `ServerState::new_full(load_config())`, spawn `handle_tool_call_with_mode(..., Some("accept_all"))`, poll `state.pending_approvals.list()` until `D_15S`) [VERIFIED: :570-600] |
| Quick run | `cargo test -p vox-orchestrator safety_floor` |
| Full phase run | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests && cargo test -p vox-orchestrator -- safety_floor socrates attention_fields` |

### Requirement → test map

| Req | Behavior | Type | Command | Exists? |
|---|---|---|---|---|
| FLOOR-01 | `vox_skill_run {"command":"git push --force origin main"}` parks under `accept_all` | call-site | `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests floor_force_push_parks_under_accept_all` | ❌ Wave 0 |
| FLOOR-01 | `vox_config_set`, `vox_trust_override` park under `accept_all` | call-site | `… pending_approvals_tests floor_policy_grant_tools_park_under_accept_all` | ❌ |
| FLOOR-01 | Floored call ignores allowlist entry | call-site | `… floor_ignores_allowlist` | ❌ |
| FLOOR-01 | Autonomous runtime origin (`McpToolDispatcher`) parks floored call | call-site | `cargo test -p vox-orchestrator-mcp autonomous_dispatch_floor_parks` | ❌ |
| FLOOR-01 | Classifier table (argv/path/name) | unit | `cargo test -p vox-orchestrator safety_floor` | ❌ |
| FLOOR-01 | Every `RISK_CLASSES`/floor name is registered or reserved | unit | `cargo test -p vox-orchestrator-mcp every_gated_tool_is_registered_or_reserved` | ❌ |
| FLOOR-02 | `vox_deploy` YAML↔Rust parity incl. `always_requires_approval: true` | parity | `… pending_approvals_tests risk_classes_yaml_matches_rust_table` | ✅ (data change) |
| FLOOR-02 | `vox_scientia_publication_publish` parks under `accept_all` | call-site | `… floor_external_publish_parks_under_accept_all` | ❌ |
| FLOOR-02 | `vox_apply_structured_edit` on `.env` / absolute `~/.ssh` path parks | call-site | `… floor_secrets_path_write_parks` | ❌ |
| FLOOR-02 | Background task with manifest write to `.env` gets floor tier without keyword in description | unit (call-site of `populate_task_attention_fields`) | `cargo test -p vox-orchestrator attention_fields floor_` | ❌ |
| FLOOR-03 | High hint cannot disable configured Socrates/grounding | unit (inverted) | `cargo test -p vox-orchestrator explicit_risk_posture_cannot_lower` | ⚠ invert existing |
| FLOOR-03 | High hint cannot lower category Low (Review) tier | unit (inverted) | `cargo test -p vox-orchestrator attention_fields risk_hint_cannot_lower_category` | ⚠ invert existing |
| FLOOR-03 | Trust relax does not waive configured enforce or `bypass_disallowed` | integration via `complete_task` | `cargo test -p vox-orchestrator trust_relax_cannot_waive_configured` | ❌ |
| FLOOR-03 | `vox_preference_set` refuses `socrates_gate_enforced`, `attention_enabled`, `oversight.level`; config unchanged | call-site via `handle_tool_call` | `cargo test -p vox-orchestrator-mcp preference_set_refuses_oversight_keys` | ❌ |
| FLOOR-05 | Mutation proof for each guard above | manual protocol | see below | — |

### Mutation-proof protocol (per guard)

1. `grep -c '<guard expression>' <file>` → expect 1.
2. Delete/neutralize the guard (e.g. replace `floor.is_none() &&` with `true &&`).
3. `grep -c` → expect 0 (proves the edit is present).
4. Run the mapped test → **must fail**. Record the failing assertion in the PLAN/SUMMARY.
5. `git checkout -- <file>` (or re-apply), `grep -c` → 1, rerun → passes.

Guards to mutate: dispatch floor condition; allowlist bypass under floor; `vox_deploy` flag (YAML and Rust separately); `classify_floor` call in `populate_task_attention_fields`; strictest-wins in `resolve_grounding_socrates_enforce`; strictest-wins in `effective_approval_tier` input; trust-relax narrowing; preference key refusal; config_set/trust_override floor entries.

### Sampling

- Per task commit: `cargo test -p <touched crate> <filter>` (< 60s).
- Per wave: `cargo test -p vox-orchestrator-mcp --test pending_approvals_tests` + `cargo test -p vox-orchestrator -- safety_floor socrates attention_fields` + `cargo clippy -p vox-orchestrator -p vox-orchestrator-mcp -- -D warnings`.
- Phase gate: above plus `cargo run -q -p vox-arch-check`, `vox ci crate-edges`, `vox ci pre-push`.

### Wave 0 gaps

- New `safety_floor.rs` unit tests (written first, failing).
- Call-site tests in `pending_approvals_tests.rs` (written first, failing).
- Inverted versions of the two lowering-asserting tests.
- `every_gated_tool_is_registered_or_reserved` test.

## Security Domain — threat model

| Actor | Capability | Attack | Mitigation in Phase 12 | Residual |
|---|---|---|---|---|
| **Sub-agent** (autonomous runtime emitting `@tool` lines) | Any registry tool via `McpToolDispatcher`, mode `None` (Ask) | Submit task with `risk:"high"`; `vox_config_set {"trust_gate_relax_enabled":true}`; `vox_trust_override` self; `vox_resolve_approval` own park | Strictest-wins hints; config/trust tools floored; trust relax narrowed; refuse resolve from in-process origins (OQ4) | Description-only intent; delegation to `vox_agy_delegate`/OpenClaw is opaque |
| **Prompt-injected tool output** (web page, file, MCP result steering the chat model) | Model calls any registry name — agent loop does not check the offered set (agent_loop.rs:974-980) | Injected "call `vox_preference_set socrates_gate_enforced=false`" or `vox_skill_run "git push -f"` | Preference refusal; floor on skill_run argv; floor on policy grants. Recommend (not required) rejecting names not in the turn's offered set | Obfuscated shell commands bypass argv classifier |
| **Compromised MCP client** (HTTP gateway / stdio peer) | `DEFAULT_ALLOWED_TOOLS` incl. `vox_preference_set`, `vox_trust_override`, `vox_attention_reset`, `vox_set_agent_budget`; can pass `permission_mode` via transport | Set `accept_all`, then call floored tools; resolve its own approvals | Floor ignores mode and allowlist; preference refusal; trust_override floored | Can call `vox_resolve_approval` → **FLOOR-04 / Phase 14** (authenticated RPC) must close this |

STRIDE coverage: Elevation of privilege (lowering paths) — primary; Tampering (policy files via structured edit) — path floor; Repudiation — floored parks already emit `hitl_approval_record` + oplog `ApprovalRequested` [VERIFIED: dispatch.rs:235-362].

## Sources

### Primary (HIGH — read directly this session)
- `crates/vox-orchestrator-mcp/src/{dispatch.rs, permission_modes.rs, scope_guard.rs, git_exec.rs, compiler_tools.rs, skills_tools.rs, autonomous_tool_dispatch.rs, memory_tools/handlers_preferences.rs, dei_tools/control.rs, http_gateway/mod.rs, llm_bridge/tool_selection.rs, chat_tools/chat/agent_loop.rs, task_tools/submission.rs, news_tools.rs, scientia_tools/syndication.rs}`
- `crates/vox-orchestrator/src/{runtime.rs, mode.rs, attention/{routing.rs,budget.rs}, orchestrator/task_dispatch/submit/attention_fields.rs, orchestrator/task_dispatch/complete/success/{mod.rs,socrates.rs}, config/{orchestrator_fields.rs,impl_default.rs}, types/tasks.rs, orchestrator/core/lineage.rs}`
- `crates/vox-foundation/src/primitives/agentos_mutation.rs`, `crates/vox-skills/src/sandbox/{mod.rs,runner.rs}`
- `contracts/orchestration/permission-modes.v1.yaml`, `contracts/mcp/tool-registry.canonical.yaml`, `docs/src/architecture/layers.toml`
- `crates/vox-orchestrator-mcp/tests/pending_approvals_tests.rs`
- GUI/CLI legitimate writers: `vox-gui/src/commands/orchestrator.rs`, `vox-gui/ui/.../ApprovalsView.tsx`, `vox-cli/src/commands/attention.rs`

### Secondary (CITED)
- `.planning/workstreams/autonomy-ux/{REQUIREMENTS.md, ROADMAP.md, STATE.md, research/SUMMARY.md, research/PITFALLS.md}`
- `AGENTS.md`, `.cursor/rules/*.mdc`

## Metadata

- Confidence: code-path findings HIGH; shell-classifier coverage MEDIUM (inherently incomplete); Open Questions 1–5 need planner/user decision.
- Valid until: next change to `dispatch.rs` gate block or `tool-registry.canonical.yaml`.
