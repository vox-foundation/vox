---
title: "Capability Profiles — shared contracts for all implementation plans"
description: "Exact type names, file locations, wire shapes, error codes, and env/flag names that every capability-profiles plan (01–07) must use verbatim."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Capability Profiles — Shared Contracts

Spec: [`docs/superpowers/specs/2026-10-08-capability-profiles-design.md`](../../specs/2026-10-08-capability-profiles-design.md).
Every plan in this directory consumes these names **verbatim**. If a plan needs a change here, it
changes this file in the same commit and says so.

## Plan map and order

| Plan | Delivers | Depends on |
|---|---|---|
| `01-exposure-metadata.md` | `Exposure`, `SideEffectClass`, `Subsystem` types; catalog fields; all 340 ops classified; CI gate; `tier` removed | — |
| `02-profiles.md` | Profile manifests, `ActiveProfile`, `tools/list` + dispatch filtering (stdio + HTTP), `--profile`, `VOX_PROFILE`, `vox serve`, `profile_mismatch`, bundle `profiles` | 01 |
| `03-subsystem-boot.md` | Conditioned daemon boot, `BootReport`, `subsystem_not_booted`, degraded boot | 01, 02 |
| `04-network-policy.md` | Bind policy, TLS/`--behind-tunnel`, exposure-by-role off loopback, `GET /v1/profile` | 02, 03 |
| `05-crate-extraction.md` | `vox-attention` (L2) and `vox-models` (L3) extracted from `vox-orchestrator` | **DEFERRED (D3)** — not a dependency of any other plan; runs only after user authorization of its edge diff |
| `06-axis-remote.md` | Axis `Backend` enum, `GatewayClient`, command-backends manifest + CI, surface gating, connect flow | 04 |
| `07-axis-db-migration.md` | Per-profile migration of `db`-class Axis commands behind daemon RPC / `/api/v2` | 06 |

Deviation from spec §5 (recorded here and in plan 01): `Subsystem` lives in **`vox-mcp-registry`**,
not `vox-orchestrator-types`, because `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-cli` already
depend on `vox-mcp-registry` and `vox-orchestrator-d` / `vox-gui` reach it through
`vox-orchestrator-mcp`. Zero new crate edges for plans 01–04, 06–07.

## Branch / worktree

- Worktree: `/Users/brbrainerd/dev/vox/.worktrees/capability-profiles`, branch
  `feat/capability-profiles`, based on `origin/main`.
- Never edit `/Users/brbrainerd/dev/vox` (main checkout) — it holds another session's uncommitted
  model-bench WIP in `crates/vox-orchestrator/src/models/**`, `crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs`, `crates/vox-gui/**`.

## Types (crate `vox-mcp-registry`)

New modules: `src/exposure.rs`, `src/subsystem.rs`, `src/profile.rs`, `src/boot.rs`; re-exported
from `lib.rs`. `vox-orchestrator-mcp` re-exports them as `vox_orchestrator_mcp::profiles::*`.

```rust
// src/exposure.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exposure { Public, Operator, Internal }          // Ord: Public < Operator < Internal
impl Exposure {
    pub const fn as_str(self) -> &'static str;              // "public" | "operator" | "internal"
    pub fn parse(s: &str) -> Option<Self>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass { None, ReadOnly, WritesFiles, SpawnsProcess, Network, RemoteMutation, Destructive }
impl SideEffectClass {
    pub const fn as_str(self) -> &'static str;              // snake_case of variant
    pub fn parse(s: &str) -> Option<Self>;
}

// src/subsystem.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    Db, Sessions, AgentFleet, ModelRegistry, LlmFacade, SearchIndex, Memory,
    Research, Mesh, Attention, Clarification, Flywheel, Compiler, Browser, Speech,
}
impl Subsystem {
    pub const ALL: &'static [Subsystem];
    pub const fn as_str(self) -> &'static str;              // snake_case: "db", "agent_fleet", ...
    pub fn parse(s: &str) -> Option<Self>;
    pub const fn deps(self) -> &'static [Subsystem];         // direct deps, table below
}
/// Transitive closure including the roots, deterministic order.
pub fn subsystem_closure(roots: &[Subsystem]) -> std::collections::BTreeSet<Subsystem>;
```

Dependency table (`deps`): `Sessions→[Db]`, `AgentFleet→[Sessions]`, `LlmFacade→[ModelRegistry]`,
`SearchIndex→[Db]`, `Memory→[Db, SearchIndex]`, `Research→[LlmFacade, SearchIndex, Db]`,
`Mesh→[Db]`, `Clarification→[Db]`, `Flywheel→[Db]`; all others `[]`.

```rust
// lib.rs — registry row (replaces the current struct; `tier` is REMOVED)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpToolRegistryEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub product_lane: &'static str,
    pub http_read_role_eligible: bool,
    pub exposure: Exposure,
    pub side_effect_class: SideEffectClass,
    pub requires: &'static [Subsystem],
}
pub fn registry_entry(name: &str) -> Option<&'static McpToolRegistryEntry>;

// src/profile.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileManifest {
    pub id: &'static str,
    pub description: &'static str,
    pub requires: &'static [Subsystem],
    pub tools: &'static [&'static str],      // fully resolved at build time (globs/lanes expanded, clipped to max_exposure)
    pub max_exposure: Exposure,
    pub axis_surfaces: &'static [&'static str],
}
pub const DEV_PROFILE_ID: &str = "dev";
pub static PROFILES: &[ProfileManifest];      // generated; includes synthesized `dev` (all tools, all subsystems, max_exposure Internal)

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ActiveProfile {
    pub ids: Vec<&'static str>,
    pub tools: std::collections::BTreeSet<&'static str>,
    pub subsystems: std::collections::BTreeSet<Subsystem>,   // closure
    pub axis_surfaces: std::collections::BTreeSet<&'static str>,
    pub max_exposure: Exposure,                               // max over composed profiles
}
impl ActiveProfile {
    pub fn dev() -> Self;
    pub fn resolve(ids: &[&str]) -> Result<Self, ProfileError>;   // empty slice => dev()
    pub fn from_env() -> Result<Self, ProfileError>;               // reads VOX_PROFILE; unset/empty => dev()
    pub fn contains_tool(&self, name: &str) -> bool;
    pub fn id_string(&self) -> String;                             // ids joined with ","; canonical sorted
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError { #[error("profile_unknown: {0}")] Unknown(String) }

// src/boot.rs
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootReport {
    pub booted: std::collections::BTreeSet<Subsystem>,
    pub failed: std::collections::BTreeMap<Subsystem, String>,
}
impl BootReport {
    pub fn is_up(&self, s: Subsystem) -> bool;                     // booted.contains(s)
    pub fn missing_for(&self, requires: &[Subsystem]) -> Option<Subsystem>; // first not-up requirement
}

/// Typed denial, shared by stdio MCP and HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDenial {
    ToolNotInProfile,
    SubsystemNotBooted(Subsystem),
    ExposureDenied,
    InsufficientRole,
}
impl ProfileDenial {
    pub const fn code(&self) -> &'static str;      // "tool_not_in_profile" | "subsystem_not_booted" | "exposure_denied" | "insufficient_role"
    pub const fn http_status(&self) -> u16;        // 404 | 503 | 403 | 403
}
```

Single decision function (plan 02 adds it, plan 03 feeds `BootReport`, plan 04 feeds caller context):

```rust
// src/profile.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerRole { Read, Write }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallContext { pub role: CallerRole, pub loopback: bool }
impl CallContext { pub const LOCAL_WRITE: CallContext = CallContext { role: CallerRole::Write, loopback: true }; }

pub fn authorize_tool(
    profile: &ActiveProfile,
    boot: &BootReport,
    ctx: CallContext,
    tool: &str,
) -> Result<&'static McpToolRegistryEntry, ProfileDenial>;
```

Order of checks in `authorize_tool` (tests pin this order):
1. Unknown tool or not in `profile.tools` → `ToolNotInProfile`.
2. `!ctx.loopback && entry.exposure == Internal` → `ExposureDenied`.
3. `ctx.role == Read && entry.exposure == Operator` → `ExposureDenied`.
4. `ctx.role == Read && !entry.http_read_role_eligible` → `InsufficientRole`.
5. `boot.missing_for(entry.requires)` is `Some(s)` → `SubsystemNotBooted(s)`.

Advertising (`tools/list`, `GET /v1/tools`) = the tools for which `authorize_tool` returns `Ok`.

## Contract files

- `contracts/operations/catalog.v1.yaml`: on every op with `mcp != null`, the `mcp:` block gains
  `exposure: public|operator|internal` and `requires: [<subsystem ids>]` (may be `[]`); the op's
  existing top-level `side_effect_class` becomes **required non-null** for those ops. `mcp.tier` is
  removed.
- `contracts/mcp/tool-registry.canonical.yaml` (generated by `vox ci operations-sync --target mcp --write`):
  each tool row gains `exposure`, `side_effect_class`, `requires`; `tier` removed.
- `contracts/mcp/tool-registry.schema.json`: updated accordingly.
- `contracts/profiles/<id>.v1.yaml` (plan 02) — format:

```yaml
id: models
description: "..."
requires: [db, model_registry, llm_facade]   # D1
tools:
  include: [vox_list_models, vox_chat_*]   # exact names or trailing-* prefix
  include_lanes: []                        # product_lane values
max_exposure: operator
axis_surfaces: [models, chat]
```

- `contracts/gui/command-backends.v1.yaml` (plan 06): `schema_version: 1` + `commands: [{ module: <file stem>, class: daemon|db|host|shell, surfaces: [..], notes?: <string> }]`.
  `shell` = Axis-local, same meaning in local and remote mode, never gated. `surfaces` and profile
  `axis_surfaces` are `view_key`s from `contracts/gui/surface-registry.v1.yaml`.

## Env vars and flags (add to `contracts/config/env-vars.v1.yaml` + `docs/src/reference/env-vars.md`)

| Name | Plan | Meaning |
|---|---|---|
| `VOX_PROFILE` | 02 | Comma-separated profile ids; unset ⇒ `dev`. Spawners (CLI, GUI) pass it to `vox-orchestrator-d`. |
| `VOX_MCP_HTTP_HOST` (existing) | 04 | Bind host; non-loopback triggers bind policy. |
| `VOX_MCP_HTTP_TLS_CERT`, `VOX_MCP_HTTP_TLS_KEY` | 04 | PEM paths. |
| `VOX_MCP_HTTP_BEHIND_TUNNEL` | 04 | `1` = operator attests a TLS-terminating tunnel/proxy. |
| `VOX_MCP_WORKSPACE_TOOLS` | 01 | `1` = also advertise workspace-federated `@tool`s in `tools/list` (only served under `dev`). Registered in `env-vars.v1.yaml` **and** `contracts/config/registry.v1.yaml`. |
| `VOX_MCP_TIERS` | 01 | Deprecated alias of `VOX_MCP_WORKSPACE_TOOLS` (only `workspace`/`all` opt in, as today); warns once when set; ignored when `VOX_MCP_WORKSPACE_TOOLS` is set. |

CLI: `vox mcp --profile <ids>`; `vox serve --profile <ids> [--bind <host:port>] [--tls-cert <p> --tls-key <p> | --behind-tunnel]`
(register in `contracts/cli/command-registry.yaml`, then `cargo run -p vox-cli -- ci command-sync`).

Daemon ping response gains `"profile": "<id_string>"`; a spawner requesting a different profile
gets `profile_mismatch` (HTTP 409 where applicable) instead of adopting.

## Wire shapes

MCP tool error (stdio + WS), as the tool result `isError: true` with JSON text content:

```json
{"error":{"code":"subsystem_not_booted","message":"tool vox_kb_query requires subsystem search_index, which is not running","subsystem":"search_index"}}
```

HTTP: same JSON body with `ProfileDenial::http_status()`. Profile resolution errors: 400
`profile_unknown`. Adopt mismatch: 409 `profile_mismatch`. Axis client-side (Tauri command errors,
same `{"error":{"code","message",...}}` envelope): `version_mismatch`, `local_only`,
`not_remote_capable`, `tls_pin_required`, `tls_pin_mismatch`, `unauthorized`, `unreachable`,
`invalid_url`, `insecure_url`. Per-surface reasons in `axis_backend_status`:
`not_in_profile | needs_local_machine | not_remote_capable`.
`axis_connect_remote(url, token?, accept_fingerprint?)`.

`GET /v1/profile` (read or write token; loopback unauthenticated allowed only where today's
gateway allows it):

```json
{
  "profiles": ["models"],
  "max_exposure": "operator",
  "subsystems": { "booted": ["llm_facade", "model_registry"], "failed": [{"id": "db", "error": "..."}] },
  "tools": ["vox_list_models", "..."],
  "axis_surfaces": ["chat", "models"],
  "server_version": "0.x.y"
}
```

`tools` here = tools advertisable to the **calling** token's context.

## Global constraints (apply to every task)

- TDD: failing test first; every new `pub fn` has a test in the same file (`skeleton/untested-pub-api`).
- Security guards are mutation-verified: break the guard, see the test fail, restore, with a
  before/after presence check (AGENTS.md §PR & Review Discipline).
- No new workspace crate edges in plans 01–04, 06–07. Plan 05's new edges are **proposed for user
  authorization** and are not added to `contracts/ci/crate-edges.allow.v1.json` by an agent.
- Secrets via `vox_secrets::resolve_secret`; no `std::env::var` for tokens. Non-secret `VOX_*`
  config via existing `vox-config` patterns.
- Never `cargo fmt --all`; format dirty files with `vox run scripts/fmt.vox`.
- Generated files (`tool-registry.canonical.yaml`, `*.generated.md`) are regenerated, never hand-edited.
- New `.md` under `docs/src/` has frontmatter; lint with `cargo run -p vox-doc-pipeline -- --lint-only --paths <path relative to docs/src>`.
- Per-crate checks before each commit: `cargo check -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`, targeted `cargo nextest run -p <crate> <filter>`.
- `dev` profile must reproduce today's behaviour exactly (back-compat tests in plans 02 and 03).

## Amendments (2026-10-08, after plan drafting) — these override anything above

### User decisions
- **D1 (DB for small profiles):** `models` profile `requires` includes `db` (scoreboard, bench evidence, chat history). `attention` and `skills` stay DB-free. In remote mode Axis stores GUI preferences **client-side** (Axis local settings), never via server DB. Plan 03's "models opens no turso" boot test becomes "attention opens no turso"; a separate test asserts `models` boots `db`.
- **D2 (eval sandbox):** no public-eval exemption. The operator adds `VOX_MCP_HTTP_READ_BEARER_TOKEN` in Coolify before plan 04 Task 4 deploys; plan 04 Task 10 changes only deployment config and docs: both compose files (`vox-eval.compose.yml`, `docker/vox-eval.compose.yml`) pass the read-token secret through (`VOX_MCP_HTTP_READ_BEARER_TOKEN=${VOX_MCP_HTTP_READ_BEARER_TOKEN}`) and set `VOX_MCP_HTTP_BEHIND_TUNNEL=1` (Traefik terminates TLS), plus comments and the deployment doc. No policy or code change.
- **D3 (plan 05):** deferred. Runs only after the main-checkout model-bench WIP is committed and rebased onto #598; the exact edge diff is presented to the user for authorization at that time. Plans 01–04, 06, 07 must not depend on plan 05.
- **D4:** `vox-models` → `vox-populi` stays an **optional** edge (identical behaviour).

### From plan 01
- Shared validator: `vox-mcp-registry/src/classification.rs` with `ToolClassification` and `classify_tool_row`, included by `build.rs` via `#[path]` (together with `exposure.rs`, `subsystem.rs`) so build-time and `operations-sync` gates share one implementation.
- Tool `_meta` keys: `vox_tier` → `vox_source` (`workspace` | `skill`); new `vox_exposure`, `vox_side_effect_class`, `vox_requires`.
- Internal set additionally includes the `vox_agent_*` gateway family and browser cookie export/import tools.
- `remote_mutation` added to `capability-registry.schema.json`.
- `side_effect_class` precedence when a tool does several things: `destructive` > `remote_mutation` > `spawns_process` > `network` > `writes_files` > `read_only` > `none`.
- Gate invariants: `http_read_role_eligible ⇒ side_effect_class ∈ {none, read_only}`; `exposure == operator ⇒ side_effect_class ∉ {none, read_only}`; `exposure == public ⇒ side_effect_class ∉ {spawns_process, remote_mutation, destructive}`.
- Federated workspace tools: today's default (hidden) is preserved. The opt-in moves from `VOX_MCP_TIERS` to `VOX_MCP_WORKSPACE_TOOLS=1` (register in env-vars contract); `VOX_MCP_TIERS` is honoured as a deprecated alias (with a one-time warning) **only for the values that reveal workspace tools today: `workspace` and `all`**; `dev`, `advanced` and other values keep current behaviour (workspace tools hidden). The alias is not widened. Workspace/skill tools are only ever served under `dev`.
- Catalog stays `catalog.v1.yaml` (no rename): the new fields are enforced by our generator; bump the schema's `x-vox-version` and note it in the plan.

### From plan 02
- `dev` has `axis_surfaces: ["*"]` (wildcard = every surface); plan 06 treats `*` accordingly.
- Helpers reused by later plans: in `vox_orchestrator_mcp::profiles` — `guard_tool`, `denial_error_json`, `full_boot_report`, `is_dev`; in `vox-cli-core` — `ping_profile_matches`, `profile_mismatch_error`, `OrchestratorDaemonEnsure::for_profile`.
- `ProfileDenial` lives in `boot.rs` (re-exported from `lib.rs` either way).
- `VOX_MCP_HTTP_ALLOWED_TOOLS` composes by intersection with the profile; unset ⇒ `dev` keeps today's default list, a named profile defaults to its own tool set.

### From plan 03
- `BootReport` derives `serde::Deserialize` as well as `Serialize`.
- New daemon RPC method `orch.boot_report` returning the `BootReport` JSON (stdio `vox mcp` reads it).
- Marker subsystems (`llm_facade`, `search_index`, `memory`, `compiler`, `browser`, `speech`; `model_registry` until plan 05): "booted" = permitted, not pre-warmed.
- `dev` uses lenient dependency handling (today's behaviour: dependents still start when `db` fails); named profiles are strict (dependents of a failed subsystem are recorded failed with `dependency <x> failed`).
- Daemon methods that are not tool calls (`research.run`, `research.persist_claims`, …) are also gated by the profile (their subsystem must be booted).

### From plan 04
- `--behind-tunnel` (or `VOX_MCP_HTTP_BEHIND_TUNNEL=1`) on a loopback bind counts as network-exposed: tokens required and every caller is treated as `loopback: false`.
- `VOX_MCP_HTTP_TLS_CERT`, `VOX_MCP_HTTP_TLS_KEY`, `VOX_MCP_HTTP_BEHIND_TUNNEL` are registered as `vox_secrets::SecretId`s, matching existing `VOX_MCP_HTTP_*` handling.
- `GET /v1/profile` `tools` == the set `GET /v1/tools` returns for the same caller (env allowlist ∩ `guard_tool`).
- `--bind` accepts only an IP `host:port`.
- `/api/v2/mesh/nodes/{id}/kill` is loopback-only.
- TLS reuses the pinned `rustls`/`tokio-rustls` on `ring`; never call `ServerConfig::builder()` (use the explicit-provider builder). Any lockfile package/provider change is a STOP for user authorization.

### From plan 06
- Command-backend classes are `daemon | db | host | shell` (`shell` = Axis-local code: build info, identity, logging, settings). Manifest has `schema_version: 1` and optional `notes` per row.
- `surfaces` and profile `axis_surfaces` use view keys from the GUI surface registry (`surface-registry.v1.yaml`).
- Axis client error codes: `local_only`, `not_remote_capable`, `tls_pin_required`, `tls_pin_mismatch`, `unauthorized`, `unreachable`, `invalid_url`, `insecure_url`, `version_mismatch`.
- Per-surface availability reasons: `not_in_profile`, `needs_local_machine`, `not_remote_capable`.
- `axis_connect_remote(url, token, accept_fingerprint: Option<String>)`.
- Gateway origin guard: a WebSocket upgrade with **no** `Origin` header is allowed only when a valid bearer token is presented (native clients); browser `Origin`s keep today's rules. Mutation-verified in plan 06 Task 3.

### From plan 07
- Local and remote Axis reach the same server code: new GET-only daemon RPC `orch.api_v2_get` mirrors `/api/v2` reads locally.
- Narrow preference tools replace `vox_preference_get/set` for Axis (model keys and `gui.*` keys only); safety settings such as `socrates_gate_enforced` stay unreachable from them. Per D1, `gui.*` preferences are client-side in remote mode.
- Shared-file protocol for parallel group tasks: `dashboard_api.rs` route wiring happens once in plan 07 Task 1; groups append only under their own marker comments in `dispatch.rs`, `input_schemas.rs`, `catalog.v1.yaml`; generated files are regenerated after merge, never hand-merged.

### From the reconciliation pass (2026-10-08)
- New non-secret env reads (`VOX_PROFILE` in plan 02, `VOX_MCP_WORKSPACE_TOOLS` in plan 01) are registered in **both** `contracts/config/env-vars.v1.yaml` and `contracts/config/registry.v1.yaml` (`knobs:`), because `vox ci config-hygiene` / `config-registry-parity` read registered names from `registry.v1.yaml` + Clavis + `CONFIG_KEYS`, not from `env-vars.v1.yaml`. Plan 04's settings are Clavis `SecretId`s and need no `registry.v1.yaml` row.
- GUI preferences (plan 07 Task 5): ops `vox_gui_preference_get` / `vox_gui_preference_set` are `exposure: internal` (loopback + `dev` only, `requires: [db]`); Axis uses them only in local mode (a denial or unreachable daemon is surfaced as an error, never silently redirected); remote mode uses only the client-side `axis-preferences.json`. Accepted limitations: no cross-machine sync in remote mode; values set remotely before plan 07 Task 5 are not migrated. `preferences` stays `shell` in `command-backends.v1.yaml`.
- Plan 07 consumes plan 06's names as written: `crate::remote::backend::{Backend, BackendState}` (managed `Arc<BackendState>`), `GatewayClient::{call_daemon_method, api_get}`, `axis_backend_status`, and the `gui_command_backends::problems` validator.
- Console is **host-only in remote mode** for now: `pty` and `terminal_core` are `host`-class modules on the `console` surface, so plan 06's real gate reports `console` as `needs_local_machine` against any remote server. Plan 07 Task 8 migrates `console_a2a`'s db commands and asserts that honest unavailable state; un-gating console remotely needs a separate surface split (follow-up).
