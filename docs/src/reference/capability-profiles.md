---
title: "Capability profiles"
description: "Reference for Vox capability profiles: the first-wave profile ids, how they compose, the --profile flag and VOX_PROFILE, how HTTP allowlists combine with profiles, and the typed profile errors."
category: "Language Reference"
status: "current"
training_eligible: true
---

# Capability profiles

A capability profile is a named slice of Vox's MCP tool surface plus the subsystems it needs.
Running a server with a profile makes MCP `tools/list`, HTTP `GET /v1/tools`, WebSocket
`list_tools`, and **every** dispatch path (stdio → daemon, daemon `orch.tool_call`, HTTP, WS,
autonomous agents) expose only that slice. Design: [capability profiles spec](../../superpowers/specs/2026-10-08-capability-profiles-design.md).

## Profiles

Manifests live in `contracts/profiles/<id>.v1.yaml` and are compiled into `vox-mcp-registry`
(`PROFILES`) by its build script, which expands `tools.include` (exact names or trailing-`*`
prefixes) and `include_lanes`, clips to `max_exposure`, and fails the build on unknown tools,
unknown subsystems, or a tool whose `requires` is not covered by the profile's subsystem closure.

| Profile | Contents | Subsystems (`requires`) | Max exposure |
|---|---|---|---|
| `dev` | Every tool (synthesized; the default) | all | internal |
| `models` | Model list/select/suggest, model manifest, benchmarks, chat | `db`, `model_registry`, `llm_facade` | operator |
| `knowledge` | `vox_memory_*`, `vox_session_*`, `vox_kb_*`, `vox_search_*`, `vox_knowledge_query` | `memory`, `sessions`, `search_index` | operator |
| `research` | `vox_research_*`, Scientia discovery/evaluation/prepare, news drafts — no publish/approve | `research` | operator |
| `research-publish` | Scientia/news publish, approve, submission, external jobs | `research` | operator |
| `mesh` | `vox_mesh_*`, `vox_populi_local_status`, `vox_a2a_ack` / `broadcast` / `history` / `inbox` / `send`, `vox_publish_message` | `mesh` | operator |
| `attention` | `vox_attention_history`, `vox_attention_reset`, `vox_budget_status`, `vox_trust_override` | `attention` | operator |
| `skills` | `vox_skill_*` | — | operator |
| `orchestrator` | Tasks, agents, queue, locks, plans, handoffs, approvals, clarifications | `agent_fleet`, `clarification`, `llm_facade`, `attention` | operator |
| `lang` | `vox_check`, `vox_validate_*`, tests, crate build/lint, grammar, compiler introspection | `compiler` | operator |
| `repo-read` | Read-only git/VCS/repo-index/snapshot views | — | public |
| `browser` | `vox_browser_*` (never a bundle default) | `browser`, `llm_facade` | operator |
| `speech` | `vox_oratio_*`, `vox_speech_to_code` | `speech`, `llm_facade` | operator |

The authoritative tool list for each profile is its manifest. A few catalog constraints the
manifests encode: `attention` stays DB-free and excludes `vox_attention_summary` and
`vox_cost_history`; `mesh` omits `vox_a2a_tasks` (that tool requires `agent_fleet`); `browser`
and `speech` include `llm_facade` because some of their tools need it; `orchestrator` includes
`attention` (for `vox_set_agent_budget`) and drops internal `vox_drain_agent`. Feature-gated
tools (for example `vox_scientia_*` without `news-publish`, `vox_browser_*` without
`heavy-browser`) are listed in a profile but only advertised when the build can dispatch them.

## Composition

`--profile models,knowledge` is the union of the tool sets and of the subsystem closures; the
resulting `max_exposure` is the maximum. Ids are canonicalized (sorted, de-duplicated): the
canonical form of `models,knowledge` is `knowledge,models`. Any list containing `dev` is `dev`.

## Selecting a profile

| Surface | How |
|---|---|
| `vox mcp --profile <ids>` | stdio MCP; the shared daemon is adopted or spawned with the same profile. |
| `vox serve --profile <ids>` | `vox-orchestrator-d` in the foreground with the HTTP gateway on, no stdio. Requires gateway auth; refuses if a daemon is already running. |
| `VOX_PROFILE=<ids>` | Read by `vox-orchestrator-d`, `vox mcp`, `vox serve`, and Axis when spawning its local daemon. |
| `~/.vox/config.toml` `VOX_PROFILE = "<ids>"` | Default for `vox mcp` / `vox serve`; written by `vox bundle apply` for bundles that declare `profiles`. Axis ignores it (its local daemon stays `dev` unless `VOX_PROFILE` is exported). |

Precedence for `vox mcp` / `vox serve`: `--profile` > `VOX_PROFILE` > config file > `dev`.

Bundle defaults (`crates/vox-plugin-catalog/catalog.toml`): `vox-server` → `orchestrator,knowledge,mesh`;
`vox-mesh` → `mesh`; `vox-edge` → `lang,skills`; `vox-fullstack`, `vox-ml`, `vox-ml-metal`, `vox-dev` → `dev`;
`vox-base` and `vox-mobile` leave the configured profile alone.

## Daemon adoption

`orch.ping` reports `"profile": "<canonical ids>"` (a daemon predating profiles counts as `dev`).
A client asking for a different profile does not adopt that daemon and gets `profile_mismatch`;
stop the daemon or request its profile.

## HTTP gateway allowlists

The effective HTTP tool set is `VOX_MCP_HTTP_ALLOWED_TOOLS` ∩ the profile. When
`VOX_MCP_HTTP_ALLOWED_TOOLS` is unset, `dev` keeps the historical default allowlist and a named
profile defaults to its own tools. Unknown names in the env var still fail gateway startup; names
outside the profile are dropped with a warning. Read-token narrowing
(`http_read_role_eligible`, `VOX_MCP_HTTP_READ_ROLE_ALLOWED_TOOLS`) applies on top, unchanged.
Every browser tool, `vox_skill_use`, and `vox_search_structural` is denied to the read role.

Workspace `@tool`s and skill macro tools are not in the static registry, so they are served only
under `dev`.

## Errors

Denials use one JSON shape on stdio MCP (tool result with `isError: true`), WebSocket, and HTTP
(with the listed status):

```text
{"error":{"code":"subsystem_not_booted","message":"tool vox_kb_query requires subsystem search_index, which is not running","subsystem":"search_index"}}
```

| Code | HTTP | When |
|---|---|---|
| `tool_not_in_profile` | 404 | Tool exists but the active profile excludes it (or a non-registry tool under a named profile). |
| `subsystem_not_booted` | 503 | A subsystem the tool requires is not running (plan 03's conditioned boot). |
| `exposure_denied` | 403 | `internal` tool off loopback, or `operator` tool with a read token (plan 04). |
| `insufficient_role` | 403 | Read token for a tool that is not read-role eligible (plan 04). |
| `profile_unknown` | 400 | `--profile` / `VOX_PROFILE` names a manifest that does not exist; `vox-orchestrator-d` exits non-zero. |
| `profile_mismatch` | 409 | An adoptable daemon serves a different profile. |

The decision order is fixed and implemented once, in `vox_mcp_registry::authorize_tool`:
membership → internal off loopback → operator for read role → read-role eligibility → booted subsystems.

## Boot and degraded mode

`vox-orchestrator-d` boots only the subsystems in the active profile's closure (the profile's
`requires` plus their dependencies). Steps run in a fixed order; `dev` runs every step in the
same order as before profiles existed.

| Subsystem | Boot step(s) | Notes |
|---|---|---|
| `db` | `db` | Opens the workspace journey store and syncs the orchestrator schema. |
| `model_registry` | `model_registry`, `routing_preferences` | Persisted routing preferences need `db`; without it the scorer defaults stand. |
| `llm_facade`, `search_index`, `memory`, `compiler`, `browser`, `speech` | one step each | Initialised lazily per call; the step declares them available. |
| `sessions` | `sessions` | |
| `agent_fleet` | `agent_fleet` | Still honours `VOX_MCP_AGENT_FLEET`. |
| `mesh` | `populi_federation_poller`, `a2a_remote_result_poller`, `a2a_remote_worker_poller` | |
| `clarification` | `clarification_inbox` | |
| `research` | `socrates_research_poller` (+ scientia mesh jobs) | |
| `flywheel` | `flywheel` | |
| `attention` | `attention_calibration` | |

**Failure degrades, it does not abort.** A failed step puts its subsystem in the boot report's
`failed` map with the error, and boot continues. Under any named profile, subsystems that depend
on a failed one are not started and are recorded as `dependency <x> failed`. Under `dev`, they
still start, matching pre-profile behaviour (for example, sessions fall back to file persistence
when the store is unavailable). The daemon exits non-zero only when **every** subsystem the
profile `requires` failed.

**What clients see.** Tools whose `requires` include a subsystem that is not up are dropped from
`tools/list`. Calling one anyway returns `subsystem_not_booted` (HTTP 503) with the subsystem
name. Stdio `vox mcp` reads the daemon's report through the `orch.boot_report` RPC, and the
daemon-only RPCs `research.run` and `research.persist_claims` apply the same check.
