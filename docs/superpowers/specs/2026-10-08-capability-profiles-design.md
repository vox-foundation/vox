---
title: "Capability Profiles (à-la-carte Vox server, headless, and remote Axis)"
description: "Named server profiles that expose one Vox capability slice (models, knowledge, research, mesh, attention, skills, orchestrator, …) over MCP and HTTP, boot only the subsystems that slice needs, and let Axis attach to a local or LAN server showing only the surfaces the profile enables."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Capability Profiles

## 1. Why

People want to run one part of Vox — the model selector, the mesh, the attention budget, the
knowledge base, auto-research — without adopting all of it. Today that is impossible in practice:

- **MCP surface is all-or-nothing.** 340 tools are generated from
  `contracts/operations/catalog.v1.yaml`. The schema has `product_lane` and `tier`, but every tool is
  `tier: core` and 308/340 have a null `side_effect_class`. Subsetting exists only as ad-hoc knobs
  (`VOX_MCP_HTTP_ALLOWED_TOOLS`, `VOX_MCP_TIERS`, Cargo features, per-chat-turn tool selection).
- **Bundles are install-time only.** `crates/vox-plugin-catalog/catalog.toml` bundles
  (`vox-server`, `vox-mesh`, `vox-edge`, …) choose which plugins get installed; their
  `exposes-tools` lists are metadata and do not filter what MCP advertises.
- **The daemon always boots everything.** `vox-orchestrator-d` (`src/bin/vox_orchestrator_d.rs`
  L161–383) runs one linear sequence: orchestrator → db → session manager → `ServerState` → agent
  fleet → Populi federation + A2A pollers → clarification inbox → Socrates research poller →
  flywheel → attention calibration → HTTP gateway.
- **Axis (the Tauri GUI) is local-only.** Of 60 command modules in `crates/vox-gui/src/commands`,
  ~12 go through the daemon, ~22 open `vox_db` directly, ~20 touch the local filesystem or spawn
  processes. There is no remote attach.

## 2. Decisions taken

| Question | Decision |
|---|---|
| Primary consumption channel | Run Vox as a server/daemon with a scoped MCP + HTTP **profile**. Rust-library embedding and crates.io publishing are follow-ups. |
| Network posture | Loopback by default; opt-in non-loopback bind with bearer tokens + TLS-or-tunnel; single tenant. |
| Axis | In scope, **including remote attach** to a LAN server. |
| Approach | Profiles + subsystem boot gating + exposure metadata; extract crates only where boot gating hits hard coupling. |

## 3. Exposure classes

Every op in `contracts/operations/catalog.v1.yaml` gains a **required** `exposure` field, and
`side_effect_class` becomes required:

| Class | Meaning | Served |
|---|---|---|
| `public` | Read-only, or writes only to the profile's own state (e.g. setting the active model). | Loopback; LAN with read or write token. |
| `operator` | Mutates workspace, repo, remote services, or other agents. | Loopback; LAN with write token only. Existing HITL approval matrix still applies. |
| `internal` | Never exposed through a named profile. | Loopback, `dev` profile only. |

Initial `internal` set: `vox_agy_*`, `vox_openclaw_*`, `vox_agent_gateway_call` and other
`*_gateway_call`, `vox_add_approval_allowlist_entry`, emergency-stop/drain controls,
`vox_secrets_doctor`, `vox_credentials_status`, `vox_gamify_*`.

`vox ci operations-sync` fails if any op lacks `exposure`, `side_effect_class`, or `requires`
(§5). The unused `tier` field is removed from the registry schema and `VOX_MCP_TIERS` is retired
(deprecation marker until removal).

## 4. Profile manifests

One file per profile: `contracts/profiles/<id>.v1.yaml`, generated into `vox-mcp-registry`
alongside the tool registry.

```yaml
id: models
description: "Model registry, selection/routing explain, leaderboard, chat via selected model."
requires: [model_registry, llm_facade]   # subsystems to boot (closure computed)
tools:
  include: [vox_list_models, vox_set_model, vox_get_active_model, vox_set_active_model,
            vox_benchmark_list, vox_chat_*]
  include_lanes: []
  max_exposure: operator
axis_surfaces: [models, chat]
```

Rules:

- `tools.include` accepts exact names and trailing-`*` prefixes; `include_lanes` selects by
  `product_lane`. The resolved set is clipped to `max_exposure`.
- Profiles compose: `--profile models,knowledge` is the union of tool sets and subsystem closures.
- `dev` = every op and every subsystem. It is the default for `vox mcp` and for the daemon Axis
  spawns locally, so current behaviour is unchanged.

**First-wave profiles:**

| Profile | Tool groups (approx. count today) |
|---|---|
| `models` | models (5), routing explain, benchmark, chat |
| `knowledge` | `vox_memory_*` (7), `vox_session_*` (6), `vox_kb_*` (12), `vox_search_*` (10), `vox_knowledge_query` |
| `research` | `vox_research_*`, `vox_scientia_*` **minus publish/approve** |
| `research-publish` | publish/approve/external jobs (operator; explicit opt-in) |
| `mesh` | `vox_mesh_*`, `vox_populi_local_status`, `vox_a2a_*`, `vox_publish_message` |
| `attention` | `vox_attention_*`, `vox_budget_status`, `vox_trust_override` |
| `skills` | `vox_skill_*` (11) |
| `orchestrator` | tasks, agents, queue, locks, plan, handoff |
| `lang` | `vox_validate_*`, `vox_check_*`, `vox_run_tests`, `vox_test_*` |
| `repo-read` | `vox_git_*` (read), `vox_vcs_status`, `vox_repo_*` (read), snapshots list |
| `browser` | `vox_browser_*` (32); never in any composite default |
| `speech` | `vox_oratio_*`, `vox_speech_*` |

**Deferred:** `code-intel` (needs new `vox graph` MCP tools; none exist today).

**Activation:**

- `vox serve --profile <ids> [--bind <addr>] [--tls-cert … --tls-key … | --behind-tunnel]` —
  daemon + HTTP gateway, no stdio.
- `vox mcp --profile <ids>` — stdio MCP, daemon spawned or adopted with the same profile.
- Bundles in `catalog.toml` gain `profiles = [...]` (e.g. `vox-server` →
  `[orchestrator, knowledge, mesh]`, `vox-mesh` → `[mesh]`), used by `vox bundle apply` to write
  the default profile into config.

An adopted daemon whose profile differs from the requested one is not reused; the caller gets
`profile_mismatch` and must stop it or request the matching profile.

## 5. Subsystem graph and conditioned boot

A `Subsystem` enum in `vox-orchestrator-types` (L0) with declared dependencies:

| Subsystem | Depends on | Today's boot step |
|---|---|---|
| `db` | — | `connect_workspace_journey_optional` + `init_db` |
| `sessions` | `db` | `SessionManager::new` |
| `agent_fleet` | `sessions` | `spawn_agent_fleet_if_enabled_with_dispatcher` |
| `model_registry` | — | none in `main` (lazy inside orchestrator build); becomes an explicit step |
| `llm_facade` | `model_registry` | none in `main` (lazy `vox_actor_runtime::llm`); becomes an explicit step |
| `search_index` | `db` | none in `main` (lazy); becomes an explicit step |
| `memory` | `db`, `search_index` | MemoryManager bootstrap inside `build_repo_scoped_orchestrator` |
| `research` | `llm_facade`, `search_index`, `db` | Socrates research poller, Scientia |
| `mesh` | `db` | Populi federation poller, A2A result/worker pollers |
| `attention` | — | attention calibration |
| `clarification` | `db` | clarification inbox poller |
| `flywheel` | `db` | `FlywheelMonitor` |
| `compiler` | — | compiler/validate handles |
| `browser` | — | browser plugin |
| `speech` | — | oratio plugin |

- The daemon resolves the profile's `requires` into a closure and runs each existing boot step only
  if its subsystem is in it. `dev` resolves to all subsystems and reproduces today's order exactly.
- Each catalog op declares `requires: [..]`. The registry generator fails if a profile includes a
  tool whose `requires` is not covered by the profile's closure.
- At runtime `tools/list` and dispatch both filter on *in profile* **and** *required subsystems
  actually booted*.

**Extraction forced by boot gating** (and only this, in this project):

- **`vox-models`** (new, L3) ← `vox-orchestrator/src/models/` (~14k LoC). Deps:
  `vox-llm-config`, `vox-llm-egress`, `vox-orchestrator-types`, `vox-telemetry`. `ModelSpec`,
  `TaskCategory`, `CostPreference`, `RemainingBudget` move to `vox-orchestrator-types`.
  `vox-research-shim::selection` depends on `vox-models` instead of `vox-orchestrator`.
- **`vox-attention`** (new, L2) ← `vox-orchestrator/src/attention/` + `budget/` + `budget_gate.rs`
  (~3.4k LoC) as pure logic. Live state is reached through a `BudgetSource` trait that the
  orchestrator implements and the `attention` subsystem provides standalone.

Both crates get `layers.toml` entries and `where-things-live.md` rows in the same PR. New crate
edges are **proposed** in the PR description for user authorization per AGENTS.md §Dependency
Discipline; no `exceptions` entries are self-added.

Profiles are a runtime concept: one binary serves every profile. A compile-time slim server build
is out of scope.

## 6. Network, auth, discovery

**Bind policy** (enforced at gateway startup, reported by `vox doctor`):

- Default bind `127.0.0.1`.
- Non-loopback bind requires: write token **and** read token (Clavis:
  `VoxMcpHttpBearerToken`, `VoxMcpHttpReadBearerToken`) **and** either TLS (`--tls-cert`,
  `--tls-key`) or explicit `--behind-tunnel`. Missing any → refuse to start with an error naming
  what is missing.
- Off loopback: read token sees `public` ∩ `http_read_role_eligible`; write token sees
  `public` + `operator`; `internal` is never served regardless of profile.
- `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` is rejected for non-loopback binds.

**Discovery:** `GET /v1/profile` (read token) returns active profile ids, booted and failed
subsystems, the served tool names, `axis_surfaces`, and server version. Added to
`contracts/mcp/http-gateway.openapi.yaml`.

Multi-tenant isolation is out of scope: one operator, many clients.

## 7. Remote Axis

**Backend abstraction (Rust side only).** `vox-gui` gets
`enum Backend { Local(PersistentDaemon), Remote(GatewayClient) }` behind the existing Tauri
commands. `ui/src/transport.ts` is unchanged — the UI still uses `invoke`/`listen`.
`GatewayClient` speaks the gateway's WebSocket RPC (`/v1/ws`) and `/api/v2/*` dashboard routes.

**Command backend manifest.** `contracts/gui/command-backends.v1.yaml` classifies every command
module:

| Class | Today | Remote behaviour |
|---|---|---|
| `daemon` | ~12 | Works via `GatewayClient`. |
| `db` | ~22 | Local-only until migrated behind daemon RPC or `/api/v2`. |
| `host` | ~20 | Local by nature (fs, process, VCS isolation, CLI sidecar); hidden remotely. |

A CI check fails when a command module is unclassified.

**Surface visibility:** a surface renders iff it is in the profile's `axis_surfaces` **and** (in
remote mode) all of its commands are `daemon`-class. Otherwise it shows a "not available on this
server" state naming the reason (not in profile / needs local machine / not yet remote-capable).

**Connect flow:** Settings → "Connect to server" → URL + token. Token stored in the OS keyring via
the secrets crate; TLS certificate pinned on first connect (TOFU, re-prompt on change); major.minor
version must match Axis or connection is refused with `version_mismatch`.

**Migration order** for `db`-class commands follows the profiles: models → knowledge → research →
mesh → attention → skills → orchestrator, one small PR each.

## 8. Errors

Typed, stable codes on both MCP and HTTP:

| Code | When |
|---|---|
| `tool_not_in_profile` | Tool exists but the active profile excludes it. |
| `subsystem_not_booted` | Tool is in profile but a required subsystem failed or was not started; carries the subsystem name. |
| `exposure_denied` | `internal` off loopback, or `operator` with a read token. |
| `insufficient_role` | Token role cannot call this tool. |
| `profile_unknown` | `--profile` names a manifest that does not exist. |
| `profile_mismatch` | Adoptable daemon is running a different profile. |
| `version_mismatch` | Axis ↔ server major.minor differ. |

Subsystem boot failure degrades rather than aborts (matching today's "continue without persisted
Codex" behaviour): the failure is reported in `/v1/profile` and dependent tools drop from
`tools/list`. If **every** subsystem a profile requires fails, the daemon exits non-zero.

## 9. Testing (failing tests first)

1. **Contract tests** in the registry generator: every op has `exposure`, `side_effect_class`,
   `requires`; every profile's tools are covered by its subsystem closure; `dev` = all ops.
2. **Mutation-verified guards** (AGENTS.md §PR & Review Discipline): disable the exposure check and
   the bind policy in turn and confirm their tests fail — `internal` tool over a LAN bind must be
   rejected; non-loopback bind without tokens/TLS must refuse to start.
3. **Boot tests** under `vox-test-harness`: each first-wave profile boots and asserts the exact
   booted-subsystem set (e.g. `models` opens no turso connection and starts no Populi pollers).
4. **Axis Playwright** (`crates/vox-gui/ui/e2e/`, `installTauriMock` in remote mode): surface
   gating, "not available on this server" states, connect flow, with screenshots in
   `review-bundle/latest/`.
5. **Back-compat:** `vox mcp` with no flags and Axis against a local daemon produce today's tool
   list and surfaces (`dev`).

## 10. Rollout (each step ships independently)

1. Exposure / side-effect / `requires` metadata on all 340 ops + CI gate.
2. Profile manifests + `tools/list` and dispatch filtering (on today's full boot).
3. Subsystem graph + conditioned boot; extract `vox-models` and `vox-attention`.
4. Bind/auth policy + `GET /v1/profile`.
5. Axis `Backend` abstraction, command-backend manifest, remote connect.
6. Per-profile migration of `db`-class Axis commands.
7. Bundle `profiles = [...]`; docs: running Vox as a server, profile reference, remote Axis.

## 11. Out of scope (follow-up specs)

- Multi-tenant auth, per-user keys, quotas.
- Compile-time slim `vox-server` binary.
- crates.io publishing / Rust-library embedding of capability crates.
- Extracting knowledge, research, and mesh out of `vox-orchestrator`.
- `vox graph` MCP tools and the `code-intel` profile.
- Relaxing the plugin version-equals-workspace gate for third-party plugins.
