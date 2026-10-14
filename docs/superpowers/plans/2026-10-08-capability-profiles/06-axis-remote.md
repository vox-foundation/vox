---
title: "Capability Profiles plan 06 — Remote Axis"
description: "Implementation plan for attaching the Axis Tauri GUI to a remote vox serve gateway: command-backend manifest + CI gate, Backend enum and GatewayClient with TOFU certificate pinning, remote-mode command gate, surface gating, and the Settings connect flow."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Remote Axis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Axis (the `vox-gui` Tauri app) can attach to a `vox serve` gateway on the LAN. It then shows only the surfaces the server's profile serves *and* that can run remotely, explains every hidden surface with a typed reason, and behaves exactly as today when no remote connection is active.

**Architecture:**

- `contracts/gui/command-backends.v1.yaml` classifies every command module in `crates/vox-gui/src/commands/` as `daemon | db | host | shell`, together with the UI surfaces (`view_key`s) that call it. A new CI check (run by `vox ci gui-catalog-parity`, and therefore by `ssot-drift`) fails when a module is unclassified or the manifest names a module that does not exist.
- A new Rust module tree, `crates/vox-gui/src/remote/`, contains:
  - `enum Backend { Local(Arc<PersistentDaemon>), Remote(Arc<GatewayClient>) }`, held in a managed `BackendState`;
  - `GatewayClient`, which uses `GET /v1/profile` and `/api/v2/*` through reqwest and the WebSocket RPC on `/v1/ws` through tokio-tungstenite, with a TOFU certificate-pinning rustls verifier;
  - an invoke gate wrapped around `tauri::generate_handler!`. In remote mode it rejects `db`- and `host`-class commands with `local_only`.
- `daemon`-class commands call `BackendState::call`. In remote mode, daemon methods with no remote route return `not_remote_capable`.
- `ui/src/transport.ts` keeps its role as the only IPC hub. A `BackendStatusProvider` feeds the surface gate (`SurfaceGate` in `renderSurfaceContent`), the Sidebar, the BreadcrumbBar and a status pill. The connect flow lives in a new Settings section, "Server".

**Tech Stack:** Rust (Tauri 2, reqwest 0.12 with rustls 0.23 on the `ring` provider, tokio-tungstenite 0.24, serde_yaml), `vox-secrets` registry tokens (OS keyring), `vox-config` user config, `vox-crypto` (SHA3-256 fingerprint), React 18 + TypeScript, vitest, Playwright.

## Global Constraints

Copied verbatim from [`00-contracts.md`](./00-contracts.md) §Global constraints:

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

Plan-specific constraints:

- Work only in `/Users/brbrainerd/dev/vox/.worktrees/capability-profiles` (branch `feat/capability-profiles`).
- **Transport-crypto ledger (`contracts/crypto/transport-providers.v1.json`) is USER-AUTHORIZED-ONLY.** This plan adds no TLS provider. Task 4 contains a STOP checkpoint: if adding the dependencies changes the set of packages in `Cargo.lock`, stop and ask the user.
- GUI changes must pass `pnpm --dir crates/vox-gui/ui typecheck`, `pnpm --dir crates/vox-gui/ui test` and `pnpm --dir crates/vox-gui/ui test:e2e` (AGENTS.md §GUI Visual Verification Invariant), with screenshots saved to `crates/vox-gui/ui/review-bundle/latest/`.
- New UI IPC goes through `transport.ts` (`src/guards/ipcBoundaries.test.ts`). No new `vox://` event channels: adding one would also require `contracts/channels.v1.yaml` parity.

## Contract changes this plan makes to `00-contracts.md`

These contract changes are already applied to `00-contracts.md` (body + Amendments "From plan 06") by the reconciliation pass; Task 1 Step 3 only verifies them:

1. The `command-backends.v1.yaml` row gains a fourth class, `shell`, and an optional `notes`; the file gains a `schema_version: 1` header.
   - `shell` means: Axis-local, with the same meaning in local and remote mode (app chrome, build info, UI preferences, Remote Axis connect/disconnect). It is never gated.
   - Without `shell`, every surface would be hidden in remote mode, because `get_build_info`, `get_identity_summary` and `log_frontend` would be classed `host`.
2. Rows' `surfaces` and profile `axis_surfaces` (plan 02) are `view_key`s from `contracts/gui/surface-registry.v1.yaml`.
3. New Axis client-side error codes (Tauri command errors, same JSON envelope as the MCP tool error): `local_only`, `not_remote_capable`, `tls_pin_required`, `tls_pin_mismatch`, `unauthorized`, `unreachable`, `invalid_url`, `insecure_url`, `version_mismatch` (already listed).
4. New per-surface unavailability reasons: `not_in_profile | needs_local_machine | not_remote_capable`.
5. `axis_connect_remote` takes a third optional argument, `accept_fingerprint`. TOFU needs a confirm round-trip.

---

## Command-module inventory (source for `contracts/gui/command-backends.v1.yaml`)

The inventory was built by reading every module in `crates/vox-gui/src/commands/` (60 files plus `mod.rs`).

- **Class tests:**
  - `daemon` = only reaches `PersistentDaemon` / `OrchDaemonClient`;
  - `db` = `vox_db` / `connect_workspace*` / `GuiDbPool` / `Codex::connect`;
  - `host` = `std::fs` / `tokio::fs` / process spawn / tauri-plugin-shell / CLI sidecar / git / local config files or OS.
- **Mixed modules** take the most restrictive class (`host` > `db` > `daemon`), and splitting candidates are noted.
- **Surfaces** were traced from `invoke('<cmd>')` / `safeInvoke('<cmd>')` in `ui/src`, through `transport.ts` wrappers and hooks, to the surface that renders them (`surfaceComponents.tsx`, `decoratorRegistry.ts`). `[]` = app chrome (palette, drawers, banners) or a helper module.

| # | module | class | surfaces | notes |
|---|---|---|---|---|
| 1 | action_manifest | host | [] | reads `contracts/operations/catalog.v1.yaml` from the local repo (omnibar index) |
| 2 | activity | db | activity, chat | `connect_workspace_journey_optional` |
| 3 | app_state | shell | [] | initial view |
| 4 | browser | host | browser | **split:** `browser_*` session commands are orch RPC; `preview_*` spawn local processes; frames read local cache |
| 5 | build_info | shell | [] | compile-time version |
| 6 | catalog | shell | catalog | in-process static CLI catalog |
| 7 | chat | db | chat | **split:** `chat_send_message`/`secretary_confirm_task` are orch RPC; sessions/messages are `GuiDbPool` |
| 8 | chat_turn | db | chat | orch RPC + `GuiDbPool` persistence |
| 9 | coderabbit | host | coderabbit | CLI sidecar + `.coderabbit/` files |
| 10 | console_a2a | db | console | `Codex::connect` (mesh DB) |
| 11 | control_plane | daemon | chat, dashboard, tasks | no remote route yet for `orch.*` task methods |
| 12 | daemon | daemon | [] | readiness / version banner |
| 13 | devlog | shell | [] | frontend log bridge |
| 14 | discovery | db | console | `Codex::connect` + static catalog |
| 15 | docs_index | host | [] | local `docs/` tree (DocReader drawer, omnibar) |
| 16 | dynamic_mapping | host | [] | reads `contracts/cli/command-registry.yaml`; no UI callers today |
| 17 | event_annotate | daemon | [] | helper: annotates agent-event frames |
| 18 | execute | host | approvals, catalog, oratio, populi, skills | tauri-plugin-shell |
| 19 | gamify | db | gamify, settings | `Codex::connect` |
| 20 | graphify | host | [] | local graph cache status |
| 21 | gui_db_pool | db | [] | helper: the `GuiDbPool` |
| 22 | harness | host | chat | **split:** `get_task_diff` = orch tool_call; `list_repo_files` = local git |
| 23 | harness_eval | db | harness-health | `GuiDbPool` |
| 24 | harness_issues | host | chat, scientia | **split:** golden-corpus scan/apply = `tokio::fs`; issue rows = db |
| 25 | harness_town | host | gamify | spawns `git`/`gh` |
| 26 | identity | shell | [] | local OS user for sidebar footer |
| 27 | knowledge | db | memory, research | `ingest_url` fetches from this machine |
| 28 | llm_settings | host | settings | local `VoxConfig` + local endpoint probes |
| 29 | mcp | daemon | approvals, chat, dashboard, mesh, needs-you, settings, skills, tasks, vox-search | `orch.tool_call`; routed remotely over `/v1/ws` |
| 30 | memory | db | memory | `connect_workspace_journey_optional` + `MemoryManager` |
| 31 | mens_run_reports | host | mens | reads run reports from disk |
| 32 | mens_serve | host | mens | spawns `vox mens serve` |
| 33 | mercatus | host | mercatus | local config file |
| 34 | mesh | host | settings | `~/.vox/trusted_nodes.json` |
| 35 | mic | host | chat | local audio capture (Loquela composer) |
| 36 | mission_control | daemon | sub-agents | no remote route yet |
| 37 | models | db | chat, models, runs, settings | `connect_workspace_journey_optional`; `set_routing_priority` also sets this process's env |
| 38 | oauth | host | [] | OpenRouter PKCE + local secret store (onboarding) |
| 39 | oratio | host | chat | local STT |
| 40 | orchestrator | host | chat, console, dashboard, flow, needs-you, settings, tasks | **split:** status/agent streams + `get_orchestrator_status` are orch RPC; `hopper_*` = db; config/task-policy writes `Vox.toml` |
| 41 | plan_panel | db | chat | `GuiDbPool` |
| 42 | policy | host | policies | local policy registry + status files |
| 43 | preferences | shell | memory, settings | **deliberate exception (D1):** Axis-local `gui.*` UI preferences (theme, layout) are **client-side in remote mode** and never go to the server. Until plan 07 Task 5 they read the client machine's local workspace DB; plan 07 moves remote mode to a client-side settings file and local mode to loopback-only daemon ops. They must keep working when remote. |
| 44 | process_util | host | [] | helper: quiet `Command` builders |
| 45 | pty | host | console | local PTY |
| 46 | research | host | research | **split:** `start_research_async` = orch RPC; flag/persist = db; `save_research_doc` writes `docs/` locally |
| 47 | routing_explain | db | models | registry + scoreboard from the workspace DB |
| 48 | runs | db | runs | `connect_workspace_journey_optional` |
| 49 | scientia | db | publications, research, scientia | `GuiDbPool` |
| 50 | scientia_review | host | scientia | **split:** only `detect_repo_license`/`detect_workspace_version` read local files; the rest is db |
| 51 | search | host | memory | **split:** `vox_search_query` = db; `open_locator` spawns the local editor |
| 52 | search_probe | host | research | writes local search config TOML + db quota |
| 53 | secrets | host | settings | local Clavis vault/keyring |
| 54 | signing | host | settings | local signing key |
| 55 | speech_plugin_backend | host | [] | helper: plugin host registration |
| 56 | stt_config | host | settings | local user config |
| 57 | terminal_core | host | console | PTY block model |
| 58 | user_config | host | settings | `~/.vox/config.toml` |
| 59 | vcs_isolation | daemon | repository | routed remotely via `/api/v2/vcs/isolation[/strategy]` |
| 60 | workspace_town | host | gamify | scans local repo files |

**Counts (existing 60):** daemon **6**, db **15**, host **33**, shell **6**. After this plan the new `axis_remote` module adds one `shell` row (61 rows).

**db-class modules (input for plan 07):** activity, chat, chat_turn, console_a2a, discovery, gamify, gui_db_pool, harness_eval, knowledge, memory, models, plan_panel, routing_explain, runs, scientia.

**db-dominant `host` split candidates for plan 07:** harness_issues, research, scientia_review, search, search_probe, orchestrator (`hopper_*`).

**Surfaces usable remotely after this plan** (given a profile that lists them): `mesh`, `repository`, `vox-search`, plus the Axis-local surfaces `settings` and `coverage`. Every other surface shows a typed "not available on this server" state.

---

## File Structure

**Create**

| Path | Responsibility |
|---|---|
| `contracts/gui/command-backends.v1.yaml` | Module → class + surfaces manifest (inventory above) |
| `crates/vox-cli/src/commands/ci/gui_command_backends.rs` | CI check: every command module classified, no stale rows, valid classes/surfaces |
| `crates/vox-gui/src/remote/mod.rs` | Module root |
| `crates/vox-gui/src/remote/error.rs` | `AxisError` + JSON envelope |
| `crates/vox-gui/src/remote/tls_pin.rs` | SHA3-256 leaf fingerprint, `PinVerifier` (rustls), pinned `ClientConfig` |
| `crates/vox-gui/src/remote/manifest.rs` | Embedded manifest + surface registry, `surface_availability` |
| `crates/vox-gui/src/remote/gateway_client.rs` | URL policy, version check, `WsRpc`, `GatewayClient`, remote route table |
| `crates/vox-gui/src/remote/backend.rs` | `Backend`, `BackendState`, `BackendStatusDto` |
| `crates/vox-gui/src/remote/gate.rs` | Remote-mode invoke gate (`local_only`) |
| `crates/vox-gui/src/remote/credentials.rs` | Keyring token + pinned-fingerprint persistence |
| `crates/vox-gui/src/commands/axis_remote.rs` | `axis_connect_remote`, `axis_disconnect_remote`, `axis_backend_status`, agent-event relay |
| `crates/vox-gui/ui/src/lib/axisError.ts` (+ `.test.ts`) | Decode Axis error envelopes |
| `crates/vox-gui/ui/src/hooks/useBackendStatus.tsx` (+ `.test.tsx`) | Backend status context, `surfaceAvailability` |
| `crates/vox-gui/ui/src/components/layout/SurfaceUnavailable.tsx` (+ `.test.tsx`) | Unavailable state + `SurfaceGate` |
| `crates/vox-gui/ui/src/components/layout/BackendStatusPill.tsx` (+ `.test.tsx`) | Remote indicator |
| `crates/vox-gui/ui/src/components/surfaces/Settings/ServerConnectionSection.tsx` (+ `.test.tsx`) | Connect / TOFU confirm / disconnect |
| `crates/vox-gui/ui/src/components/surfaces/Settings/settingsBackends.ts` (+ `.test.ts`) | Which Settings sections need the local machine |
| `crates/vox-gui/ui/e2e/axis-remote.spec.ts` | Playwright: gating, connect flow, local back-compat, screenshots |

**Modify**

- `contracts/index.yaml`
- `crates/vox-cli/src/commands/ci/mod.rs`
- `crates/vox-cli/src/commands/ci/gui_catalog_parity.rs`
- `crates/vox-orchestrator-mcp/src/http_gateway/origin_guard.rs`
- `crates/vox-gui/Cargo.toml`
- `crates/vox-gui/src/main.rs`
- `crates/vox-gui/src/commands/{mod,control_plane,mission_control,vcs_isolation,mcp,daemon,chat_turn,orchestrator}.rs`
- `crates/vox-gui/ui/src/{transport.ts,types/tauri.ts,main.tsx}`
- `crates/vox-gui/ui/src/components/layout/{surfaceComponents.tsx,Sidebar.tsx,BreadcrumbBar.tsx}`
- `crates/vox-gui/ui/src/components/surfaces/Settings/{SettingsView.tsx,settingsIndex.ts}`
- `docs/src/how-to/how-to-run-vox-as-a-server.md`
- `docs/src/reference/capability-profiles.md`
- `docs/src/architecture/where-things-live.md`

---

### Task 0: Worktree setup (vox-gui sidecar + UI dist)

**Files:** none (build artifacts only; nothing is committed).

**Interfaces:** none.

A fresh worktree has neither the release `vox` sidecar that `tauri-build` bundles nor `crates/vox-gui/ui/dist` (AGENTS.md §Perennial Bug Patterns). `cargo check -p vox-gui` and clippy fail until both exist.

- [ ] **Step 1: Build the sidecar.**

```bash
cd /Users/brbrainerd/dev/vox/.worktrees/capability-profiles
vox run scripts/gui-build.vox || ./scripts/vox-dev.sh run scripts/gui-build.vox
```

Expected: the script finishes with the triple-suffixed sidecar path under `crates/vox-gui/` printed, and exit code 0.

- [ ] **Step 2: Build the UI dist.**

```bash
pnpm --dir crates/vox-gui/ui install
pnpm --dir crates/vox-gui/ui build
```

Expected: `crates/vox-gui/ui/dist/index.html` exists.

- [ ] **Step 3: Baseline.**

```bash
cargo check -p vox-gui
pnpm --dir crates/vox-gui/ui typecheck
```

Expected: both exit 0. If not, fix the environment, not the code. Do not commit anything from this task.

---

### Task 1: Command-backend manifest + contract amendment

**Files:**
- Create: `contracts/gui/command-backends.v1.yaml`
- Modify: `contracts/index.yaml`

**Interfaces:** the manifest format consumed by Task 2 (CI) and Task 5 (runtime).

**Parallel-safe with:** Task 3.

- [ ] **Step 1: Write the manifest.** `axis_remote` is listed already. Task 2's check fails until Task 8 creates `crates/vox-gui/src/commands/axis_remote.rs`, so Task 2 Step 5 creates an empty stub for it.

```yaml
# Backend class + UI surfaces of every Axis command module (crates/vox-gui/src/commands/*.rs).
# class: daemon = orchestrator daemon RPC only (works remotely through GatewayClient where routed)
#        db     = opens the workspace DB directly (local-only until plan 07 migrates it)
#        host   = local filesystem / processes / VCS / CLI sidecar / OS (local by nature)
#        shell  = Axis-local, same meaning in both modes (never gated)
# surfaces: view_keys from contracts/gui/surface-registry.v1.yaml whose UI calls this module.
# Enforced by `vox ci gui-catalog-parity` (gui_command_backends check).
schema_version: 1
commands:
- { module: action_manifest, class: host, surfaces: [], notes: "reads contracts/operations/catalog.v1.yaml from the local repo" }
- { module: activity, class: db, surfaces: [activity, chat] }
- { module: app_state, class: shell, surfaces: [] }
- { module: axis_remote, class: shell, surfaces: [settings], notes: "Remote Axis connect/disconnect/status" }
- { module: browser, class: host, surfaces: [browser], notes: "split: browser_* = orch RPC; preview_* spawn processes; frames read local cache" }
- { module: build_info, class: shell, surfaces: [] }
- { module: catalog, class: shell, surfaces: [catalog] }
- { module: chat, class: db, surfaces: [chat], notes: "split: chat_send_message/secretary_confirm_task = orch RPC" }
- { module: chat_turn, class: db, surfaces: [chat] }
- { module: coderabbit, class: host, surfaces: [coderabbit] }
- { module: console_a2a, class: db, surfaces: [console] }
- { module: control_plane, class: daemon, surfaces: [chat, dashboard, tasks], notes: "no remote route yet" }
- { module: daemon, class: daemon, surfaces: [] }
- { module: devlog, class: shell, surfaces: [] }
- { module: discovery, class: db, surfaces: [console] }
- { module: docs_index, class: host, surfaces: [] }
- { module: dynamic_mapping, class: host, surfaces: [] }
- { module: event_annotate, class: daemon, surfaces: [] }
- { module: execute, class: host, surfaces: [approvals, catalog, oratio, populi, skills] }
- { module: gamify, class: db, surfaces: [gamify, settings] }
- { module: graphify, class: host, surfaces: [] }
- { module: gui_db_pool, class: db, surfaces: [] }
- { module: harness, class: host, surfaces: [chat], notes: "split: get_task_diff = orch tool_call; list_repo_files = local git" }
- { module: harness_eval, class: db, surfaces: [harness-health] }
- { module: harness_issues, class: host, surfaces: [chat, scientia], notes: "split: corpus scan/apply = tokio::fs; rows = db" }
- { module: harness_town, class: host, surfaces: [gamify] }
- { module: identity, class: shell, surfaces: [] }
- { module: knowledge, class: db, surfaces: [memory, research] }
- { module: llm_settings, class: host, surfaces: [settings] }
- { module: mcp, class: daemon, surfaces: [approvals, chat, dashboard, mesh, needs-you, settings, skills, tasks, vox-search] }
- { module: memory, class: db, surfaces: [memory] }
- { module: mens_run_reports, class: host, surfaces: [mens] }
- { module: mens_serve, class: host, surfaces: [mens] }
- { module: mercatus, class: host, surfaces: [mercatus] }
- { module: mesh, class: host, surfaces: [settings] }
- { module: mic, class: host, surfaces: [chat] }
- { module: mission_control, class: daemon, surfaces: [sub-agents], notes: "no remote route yet" }
- { module: models, class: db, surfaces: [chat, models, runs, settings] }
- { module: oauth, class: host, surfaces: [] }
- { module: oratio, class: host, surfaces: [chat] }
- { module: orchestrator, class: host, surfaces: [chat, console, dashboard, flow, needs-you, settings, tasks], notes: "split: streams/status = orch RPC; hopper_* = db; config writes Vox.toml" }
- { module: plan_panel, class: db, surfaces: [chat] }
- { module: policy, class: host, surfaces: [policies] }
- { module: preferences, class: shell, surfaces: [memory, settings], notes: "Axis-local gui.* UI preferences; client-side in remote mode, never via the server DB (D1)" }
- { module: process_util, class: host, surfaces: [] }
- { module: pty, class: host, surfaces: [console] }
- { module: research, class: host, surfaces: [research], notes: "split: start_research_async = orch RPC; save_research_doc writes docs/ locally" }
- { module: routing_explain, class: db, surfaces: [models] }
- { module: runs, class: db, surfaces: [runs] }
- { module: scientia, class: db, surfaces: [publications, research, scientia] }
- { module: scientia_review, class: host, surfaces: [scientia], notes: "split: only license/version detection reads local files" }
- { module: search, class: host, surfaces: [memory], notes: "split: vox_search_query = db; open_locator spawns editor" }
- { module: search_probe, class: host, surfaces: [research] }
- { module: secrets, class: host, surfaces: [settings] }
- { module: signing, class: host, surfaces: [settings] }
- { module: speech_plugin_backend, class: host, surfaces: [] }
- { module: stt_config, class: host, surfaces: [settings] }
- { module: terminal_core, class: host, surfaces: [console] }
- { module: user_config, class: host, surfaces: [settings] }
- { module: vcs_isolation, class: daemon, surfaces: [repository], notes: "routed via /api/v2/vcs/isolation" }
- { module: workspace_town, class: host, surfaces: [gamify] }
```

- [ ] **Step 2: Register the contract.** In `contracts/index.yaml`, insert directly after the `frontend-dependency-policy-v1` entry (it ends with `- vox ci contracts-index` before `- id: completion-policy-v1`):

```yaml
  - id: gui-command-backends-v1
    path: contracts/gui/command-backends.v1.yaml
    owner: platform-gui
    kind: yaml
    description: Backend class (daemon|db|host|shell) and UI surfaces of every Axis command module; drives Remote Axis gating.
    enforced_by:
      - vox ci gui-catalog-parity
```

- [ ] **Step 3: Confirm the contract text is already amended.** The reconciliation pass applied this plan's contract changes to `00-contracts.md` (§Contract files `command-backends.v1.yaml` line and §Wire shapes Axis client-side codes). Verify, do not re-edit:

```bash
rg -n 'class: daemon\|db\|host\|shell' docs/superpowers/plans/2026-10-08-capability-profiles/00-contracts.md
rg -n 'axis_connect_remote\(url, token\?, accept_fingerprint\?\)' docs/superpowers/plans/2026-10-08-capability-profiles/00-contracts.md
```

Expected: one hit each.

- [ ] **Step 4: Validate the index.**

```bash
cargo run -q -p vox-cli -- ci contracts-index
```

Expected: exit 0.

- [ ] **Step 5: Commit.**

```bash
git add contracts/gui/command-backends.v1.yaml contracts/index.yaml
git commit -m "feat(gui): classify every Axis command module by backend class"
```

---

### Task 2: CI check `gui_command_backends` (run by `gui-catalog-parity` / `ssot-drift`)

**Files:**
- Create: `crates/vox-cli/src/commands/ci/gui_command_backends.rs`
- Modify: `crates/vox-cli/src/commands/ci/mod.rs`
- Modify: `crates/vox-cli/src/commands/ci/gui_catalog_parity.rs`
- Create (stub, replaced in Task 8): `crates/vox-gui/src/commands/axis_remote.rs`

**Interfaces:**

```rust
pub fn problems(modules_on_disk: &BTreeSet<String>, manifest: &serde_yaml::Value, view_keys: &BTreeSet<String>) -> Vec<String>;
pub fn run(repo_root: &Path) -> anyhow::Result<()>;
```

Wiring: `gui_catalog_parity::run` already runs inside `run_ssot_drift` (`ds!("gui_catalog_parity", …)` in `run_body_helpers/docs.rs`) and as `vox ci gui-catalog-parity`. Calling the new check from it needs no new CLI subcommand, no `command-registry.yaml` row and no catalog op.

**Parallel-safe with:** Task 3, Task 4.

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-cli/src/commands/ci/gui_command_backends.rs` containing only the test module and the signatures with `todo!()` bodies:

```rust
//! `contracts/gui/command-backends.v1.yaml` must classify every Axis command module exactly once.

use anyhow::{Context, Result, bail};
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn problems(
    _modules_on_disk: &BTreeSet<String>,
    _manifest: &Value,
    _view_keys: &BTreeSet<String>,
) -> Vec<String> {
    todo!()
}

pub fn run(_repo_root: &Path) -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn manifest(yaml: &str) -> Value {
        serde_yaml::from_str(yaml).expect("fixture yaml")
    }

    #[test]
    fn consistent_manifest_has_no_problems() {
        let m = manifest(
            "commands:\n- { module: mcp, class: daemon, surfaces: [mesh] }\n- { module: pty, class: host, surfaces: [] }\n",
        );
        assert!(problems(&set(&["mcp", "pty"]), &m, &set(&["mesh"])).is_empty());
    }

    #[test]
    fn unclassified_module_is_reported() {
        let m = manifest("commands:\n- { module: mcp, class: daemon, surfaces: [] }\n");
        let found = problems(&set(&["mcp", "new_thing"]), &m, &set(&[]));
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("new_thing:"), "{found:?}");
        assert!(found[0].contains("is not classified"), "{found:?}");
    }

    #[test]
    fn row_for_missing_module_is_reported() {
        let m = manifest("commands:\n- { module: gone, class: db, surfaces: [] }\n");
        let found = problems(&set(&[]), &m, &set(&[]));
        assert!(found.iter().any(|p| p.starts_with("gone:") && p.contains("does not exist")), "{found:?}");
    }

    #[test]
    fn bad_class_duplicate_and_unknown_surface_are_reported() {
        let m = manifest(
            "commands:\n- { module: a, class: remote, surfaces: [nope] }\n- { module: a, class: db, surfaces: [] }\n",
        );
        let found = problems(&set(&["a"]), &m, &set(&["mesh"]));
        assert!(found.iter().any(|p| p.contains("class `remote`")), "{found:?}");
        assert!(found.iter().any(|p| p.contains("classified 2 times")), "{found:?}");
        assert!(found.iter().any(|p| p.contains("surface `nope`")), "{found:?}");
    }

    #[test]
    fn empty_manifest_is_reported() {
        let found = problems(&set(&[]), &manifest("schema_version: 1\n"), &set(&[]));
        assert!(found.iter().any(|p| p.contains("`commands` is missing or empty")), "{found:?}");
    }

    #[test]
    fn repository_manifest_is_consistent() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        run(&root).expect("contracts/gui/command-backends.v1.yaml matches crates/vox-gui/src/commands");
    }
}
```

In `crates/vox-cli/src/commands/ci/mod.rs`, add `mod gui_command_backends;` on the line after `mod gui_catalog_parity;`.

- [ ] **Step 2: Run the tests and watch them fail.**

```bash
cargo nextest run -p vox-cli gui_command_backends
```

Expected: 6 tests FAIL with `not yet implemented`.

- [ ] **Step 3: Implement.** Replace the two `todo!()` functions with:

```rust
const MANIFEST_PATH: &str = "contracts/gui/command-backends.v1.yaml";
const COMMANDS_DIR: &str = "crates/vox-gui/src/commands";
const SURFACE_REGISTRY_PATH: &str = "contracts/gui/surface-registry.v1.yaml";
const CLASSES: [&str; 4] = ["daemon", "db", "host", "shell"];

/// Every disagreement between the manifest and the source tree, sorted; empty = consistent.
pub fn problems(
    modules_on_disk: &BTreeSet<String>,
    manifest: &Value,
    view_keys: &BTreeSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    let rows = manifest
        .get("commands")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        out.push(format!("{MANIFEST_PATH}: `commands` is missing or empty"));
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for row in &rows {
        let module = row.get("module").and_then(Value::as_str).unwrap_or_default();
        if module.is_empty() {
            out.push(format!("{MANIFEST_PATH}: row without `module`"));
            continue;
        }
        *seen.entry(module.to_string()).or_default() += 1;
        let class = row.get("class").and_then(Value::as_str).unwrap_or_default();
        if !CLASSES.contains(&class) {
            out.push(format!("{module}: class `{class}` is not one of daemon|db|host|shell"));
        }
        if !modules_on_disk.contains(module) {
            out.push(format!(
                "{module}: listed in {MANIFEST_PATH} but {COMMANDS_DIR}/{module}.rs does not exist"
            ));
        }
        for surface in row.get("surfaces").and_then(Value::as_sequence).into_iter().flatten() {
            let surface = surface.as_str().unwrap_or_default();
            if !view_keys.contains(surface) {
                out.push(format!(
                    "{module}: surface `{surface}` is not a view_key in {SURFACE_REGISTRY_PATH}"
                ));
            }
        }
    }
    for (module, count) in &seen {
        if *count > 1 {
            out.push(format!("{module}: classified {count} times"));
        }
    }
    for module in modules_on_disk {
        if !seen.contains_key(module) {
            out.push(format!(
                "{module}: {COMMANDS_DIR}/{module}.rs is not classified in {MANIFEST_PATH}"
            ));
        }
    }
    out.sort();
    out
}

pub fn run(repo_root: &Path) -> Result<()> {
    let modules = modules_on_disk(&repo_root.join(COMMANDS_DIR))?;
    let manifest = read_yaml(&repo_root.join(MANIFEST_PATH))?;
    let registry = read_yaml(&repo_root.join(SURFACE_REGISTRY_PATH))?;
    let found = problems(&modules, &manifest, &view_keys(&registry));
    if !found.is_empty() {
        bail!(
            "gui-command-backends: {} problem(s):\n  {}",
            found.len(),
            found.join("\n  ")
        );
    }
    tracing::info!(modules = modules.len(), "gui-command-backends: every Axis command module is classified");
    Ok(())
}

fn read_yaml(path: &Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_yaml::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn modules_on_disk(dir: &Path) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if stem != "mod" {
                out.insert(stem.to_string());
            }
        }
    }
    Ok(out)
}

fn view_keys(registry: &Value) -> BTreeSet<String> {
    registry
        .get("surfaces")
        .and_then(Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("view_key").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}
```

- [ ] **Step 4: Wire it into `gui_catalog_parity::run`.** In `crates/vox-cli/src/commands/ci/gui_catalog_parity.rs`, directly after `vox_cli_ci::gui_version_sync::run(repo_root, false)?;`, add:

```rust
    super::gui_command_backends::run(repo_root)?;
```

- [ ] **Step 5: Stub the `axis_remote` module** so the repository manifest is consistent before Task 8. Create `crates/vox-gui/src/commands/axis_remote.rs`:

```rust
//! Remote Axis connect / disconnect / status commands (implemented in plan 06 Task 8).
```

Add `pub mod axis_remote;` to `crates/vox-gui/src/commands/mod.rs`, between `pub mod app_state;` and `pub mod browser;`.

- [ ] **Step 6: Run the tests and the check.**

```bash
cargo nextest run -p vox-cli gui_command_backends
cargo run -q -p vox-cli -- ci gui-catalog-parity
cargo check -p vox-gui
```

Expected: 6 tests PASS. `gui-catalog-parity` exits 0. The log line `gui-command-backends: every Axis command module is classified modules=61` appears with `RUST_LOG=info`.

- [ ] **Step 7: Mutation check.** Delete the `{ module: pty, … }` row from the manifest. Run `cargo run -q -p vox-cli -- ci gui-catalog-parity`. Expected: non-zero exit naming `pty: crates/vox-gui/src/commands/pty.rs is not classified`. Restore the row and confirm with `rg -c 'module: pty,' contracts/gui/command-backends.v1.yaml` (expected `1`). Then rerun and confirm exit 0.

- [ ] **Step 8: Lint and commit.**

```bash
cargo clippy -p vox-cli --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-cli/src/commands/ci/gui_command_backends.rs crates/vox-cli/src/commands/ci/mod.rs crates/vox-cli/src/commands/ci/gui_catalog_parity.rs crates/vox-gui/src/commands/axis_remote.rs crates/vox-gui/src/commands/mod.rs
git commit -m "feat(ci): fail when an Axis command module is unclassified"
```

---

### Task 3: Gateway accepts bearer-authenticated WebSocket upgrades without `Origin`

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/origin_guard.rs`

**Interfaces:** `is_origin_allowed(public_eval_enabled, path, headers) -> bool` keeps its signature.

**Why:** `is_origin_allowed` rejects a WebSocket upgrade that has no `Origin` header. That defends against browsers, which always send `Origin`. Axis's native tungstenite client sends no `Origin`, so `/v1/ws` is unreachable from Remote Axis. A browser cannot attach an `Authorization` header to a `WebSocket` handshake. So "no Origin **and** a bearer header" is not a browser request, and `enforce_auth` still validates the token itself. If plan 04 already changed this guard, Step 2 passes immediately. In that case skip Steps 3–4 and record that in the commit message of the next task.

**Parallel-safe with:** Tasks 1, 2, 4, 5, 9.

- [ ] **Step 1: Add the failing test** to the `tests` module in `origin_guard.rs`:

```rust
    #[test]
    fn test_ws_upgrade_without_origin_allowed_only_with_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        headers.insert("authorization", HeaderValue::from_static("Bearer t0ken"));
        assert!(is_origin_allowed(false, "/v1/ws", &headers));

        let mut basic = HeaderMap::new();
        basic.insert("upgrade", HeaderValue::from_static("websocket"));
        basic.insert("authorization", HeaderValue::from_static("Basic Zm9vOmJhcg=="));
        assert!(!is_origin_allowed(false, "/v1/ws", &basic));

        // A browser always sends Origin; a non-loopback Origin stays denied even with a bearer.
        let mut browser = HeaderMap::new();
        browser.insert("upgrade", HeaderValue::from_static("websocket"));
        browser.insert("authorization", HeaderValue::from_static("Bearer t0ken"));
        browser.insert("origin", HeaderValue::from_static("https://attacker.example"));
        assert!(!is_origin_allowed(false, "/v1/ws", &browser));
    }
```

- [ ] **Step 2: Run the test and watch it fail.**

```bash
cargo nextest run -p vox-orchestrator-mcp origin_guard
```

Expected: `test_ws_upgrade_without_origin_allowed_only_with_bearer` FAILS on the first assertion. `test_ws_upgrade_missing_origin` still PASSES.

- [ ] **Step 3: Implement.** In `is_origin_allowed`, replace

```rust
    } else if is_upgrade {
```

with

```rust
    } else if is_upgrade && !has_bearer_authorization(headers) {
```

and add below `is_loopback_host`:

```rust
/// Native (non-browser) clients such as Remote Axis send `Authorization: Bearer` on the upgrade
/// and no `Origin`; browsers cannot set `Authorization` on a WebSocket handshake. The token itself
/// is validated by `enforce_auth`, not here.
fn has_bearer_authorization(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with(vox_http_client::BEARER_PREFIX))
}
```

- [ ] **Step 4: Run the tests again.** Same command. Expected: all `origin_guard` tests PASS.

- [ ] **Step 5: Mutation check.** Change `!has_bearer_authorization(headers)` to `true` (always deny). Expected: the new test fails. Then change it to `false` (never deny). Expected: `test_ws_upgrade_missing_origin` fails. Restore, then confirm with `rg -c '!has_bearer_authorization\(headers\)' crates/vox-orchestrator-mcp/src/http_gateway/origin_guard.rs` (expected `1`) and rerun to green.

- [ ] **Step 6: Lint and commit.**

```bash
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/http_gateway/origin_guard.rs
git commit -m "fix(gateway): allow bearer-authenticated native WebSocket upgrades without Origin"
```

---

### Task 4: Dependencies, `AxisError`, TOFU pin verifier

**Files:**
- Modify: `crates/vox-gui/Cargo.toml`
- Create: `crates/vox-gui/src/remote/mod.rs`
- Create: `crates/vox-gui/src/remote/error.rs`
- Create: `crates/vox-gui/src/remote/tls_pin.rs`
- Modify: `crates/vox-gui/src/main.rs` (add `mod remote;`)

**Interfaces:**

```rust
// error.rs
pub enum AxisError { Daemon(String), DaemonUnavailable(String), LocalOnly { command: String, module: String, class: &'static str },
    NotRemoteCapable { method: String }, VersionMismatch { server: String, gui: String }, TlsPinRequired { fingerprint: String },
    TlsPinMismatch { pinned: String, fingerprint: String }, Unauthorized(String), Unreachable(String), InvalidUrl(String),
    InsecureUrl(String), Remote { code: String, message: String } }
impl AxisError { pub fn code(&self) -> &str; pub fn message(&self) -> String; pub fn to_json(&self) -> serde_json::Value; }
impl From<AxisError> for String;
// tls_pin.rs
pub fn cert_fingerprint(der: &[u8]) -> String;                  // lowercase hex SHA3-256 (vox_crypto::compliance_hash)
pub enum PinOutcome { Match, FirstUse { fingerprint: String }, Changed { pinned: String, fingerprint: String } }
pub fn pin_outcome(expected: Option<&str>, seen: &str) -> PinOutcome;
pub struct PinVerifier;  impl PinVerifier { pub fn new(expected: Option<String>) -> Self; pub fn seen(&self) -> Option<String>; pub fn outcome(&self) -> Option<PinOutcome>; }
pub fn pinned_client_config(verifier: Arc<PinVerifier>) -> Result<rustls::ClientConfig, rustls::Error>;
```

**TLS stack finding (verified against this worktree's `Cargo.lock`):**

- `reqwest 0.12` is pinned with `rustls-tls` (provider `ring`); `rustls 0.23` is a workspace dependency with `features = ["ring"]`.
- `tokio-tungstenite 0.24` with `connect` + `rustls-tls-webpki-roots` is already resolved through `vox-openclaw-runtime`, including `tokio-rustls`, `rustls` and `webpki-roots 0.26`.
- `futures-util 0.3` is a workspace dependency.
- Pinning needs only rustls' `danger::ServerCertVerifier` plus `reqwest::ClientBuilder::use_preconfigured_tls` and `tokio_tungstenite::Connector::Rustls`.
- **No new crate, no new provider, no version bump** — only new dependency *edges* from `vox-gui` to crates already in the graph. The fingerprint uses `vox_crypto::compliance_hash` (SHA3-256), per AGENTS.md §Cryptography Policy.

**Parallel-safe with:** Tasks 2, 3, 5, 9.

- [ ] **Step 1: Add the dependencies.** In `crates/vox-gui/Cargo.toml` under `[dependencies]`, after `reqwest = { workspace = true }`:

```toml
rustls = { workspace = true }
tokio-tungstenite = { workspace = true, default-features = false, features = ["connect", "rustls-tls-webpki-roots"] }
futures-util = { workspace = true }
```

- [ ] **Step 2: STOP checkpoint — the provider ledger stays untouched.**

```bash
cargo tree -p vox-gui -i aws-lc-rs -e normal 2>&1 | head -3 > /tmp/awslc.after
git stash push crates/vox-gui/Cargo.toml && cargo tree -p vox-gui -i aws-lc-rs -e normal 2>&1 | head -3 > /tmp/awslc.before; git stash pop
diff /tmp/awslc.before /tmp/awslc.after && echo "aws-lc-rs reachability unchanged"
cargo check -p vox-gui
git diff Cargo.lock | rg '^\+name = ' && echo "NEW PACKAGE IN LOCKFILE — STOP" || echo "no new packages"
```

Expected: `aws-lc-rs reachability unchanged` and `no new packages`. The `Cargo.lock` diff touches only the `vox-gui` package's `dependencies = [` list. **If any new `[[package]]` appears, or `aws-lc-rs`/`ring` reachability changes: STOP. Report to the user and request authorization before going further. Do not edit `contracts/crypto/transport-providers.v1.json`.** If `cargo hakari` is installed, run `cargo hakari generate --diff` and apply its output with `cargo hakari generate` when it reports drift (nightly gates on it).

- [ ] **Step 3: Write the failing tests.** Create `crates/vox-gui/src/remote/mod.rs`:

```rust
//! Remote Axis: attach the GUI to a `vox serve` gateway instead of the local daemon.

pub mod error;
pub mod tls_pin;
```

Add `mod remote;` to `crates/vox-gui/src/main.rs` directly after `mod drive;`.

Create `crates/vox-gui/src/remote/error.rs` with the type, `todo!()` bodies, and tests:

```rust
//! Typed Remote Axis errors. Tauri commands return them as the JSON envelope
//! `{"error":{"code","message",...}}` (the same shape as MCP tool errors); local daemon errors
//! pass through as their original text so local mode is unchanged.

use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisError {
    /// Local daemon RPC failed after the daemon was reached.
    Daemon(String),
    /// Local daemon could not be adopted or spawned.
    DaemonUnavailable(String),
    LocalOnly { command: String, module: String, class: &'static str },
    NotRemoteCapable { method: String },
    VersionMismatch { server: String, gui: String },
    TlsPinRequired { fingerprint: String },
    TlsPinMismatch { pinned: String, fingerprint: String },
    Unauthorized(String),
    Unreachable(String),
    InvalidUrl(String),
    InsecureUrl(String),
    Remote { code: String, message: String },
}

impl AxisError {
    pub fn code(&self) -> &str {
        todo!()
    }
    pub fn message(&self) -> String {
        todo!()
    }
    pub fn to_json(&self) -> Value {
        todo!()
    }
}

impl From<AxisError> for String {
    fn from(err: AxisError) -> String {
        let _ = err;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_daemon_errors_pass_through_unchanged() {
        assert_eq!(String::from(AxisError::Daemon("conn refused".into())), "conn refused");
        assert_eq!(String::from(AxisError::DaemonUnavailable("spawn failed".into())), "spawn failed");
    }

    #[test]
    fn remote_errors_serialize_as_the_error_envelope() {
        let s = String::from(AxisError::LocalOnly {
            command: "pty_spawn".into(),
            module: "pty".into(),
            class: "host",
        });
        let v: Value = serde_json::from_str(&s).expect("json");
        assert_eq!(v["error"]["code"], "local_only");
        assert_eq!(v["error"]["command"], "pty_spawn");
        assert_eq!(v["error"]["module"], "pty");
        assert_eq!(v["error"]["class"], "host");
        assert!(v["error"]["message"].as_str().unwrap().contains("disconnect"));
    }

    #[test]
    fn pin_errors_carry_fingerprints() {
        let v = AxisError::TlsPinMismatch { pinned: "aa".into(), fingerprint: "bb".into() }.to_json();
        assert_eq!(v["error"]["code"], "tls_pin_mismatch");
        assert_eq!(v["error"]["pinned"], "aa");
        assert_eq!(v["error"]["fingerprint"], "bb");
        let v = AxisError::TlsPinRequired { fingerprint: "cc".into() }.to_json();
        assert_eq!(v["error"]["code"], "tls_pin_required");
        assert_eq!(v["error"]["fingerprint"], "cc");
    }

    #[test]
    fn codes_are_the_contract_codes() {
        assert_eq!(AxisError::NotRemoteCapable { method: "orch.x".into() }.code(), "not_remote_capable");
        assert_eq!(AxisError::VersionMismatch { server: "0.7.0".into(), gui: "0.6.0".into() }.code(), "version_mismatch");
        assert_eq!(AxisError::Unauthorized("x".into()).code(), "unauthorized");
        assert_eq!(AxisError::Unreachable("x".into()).code(), "unreachable");
        assert_eq!(AxisError::InvalidUrl("x".into()).code(), "invalid_url");
        assert_eq!(AxisError::InsecureUrl("x".into()).code(), "insecure_url");
        assert_eq!(AxisError::Remote { code: "tool_not_in_profile".into(), message: "m".into() }.code(), "tool_not_in_profile");
    }
}
```

Create `crates/vox-gui/src/remote/tls_pin.rs` with `todo!()` bodies and tests:

```rust
//! TOFU certificate pinning for Remote Axis. The server's leaf certificate is pinned by its
//! SHA3-256 fingerprint; the CA chain and hostname are deliberately not consulted (LAN servers use
//! self-signed certificates), so the pin is the only trust anchor.

use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};

pub fn cert_fingerprint(der: &[u8]) -> String {
    let _ = der;
    todo!()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinOutcome {
    Match,
    FirstUse { fingerprint: String },
    Changed { pinned: String, fingerprint: String },
}

pub fn pin_outcome(expected: Option<&str>, seen: &str) -> PinOutcome {
    let _ = (expected, seen);
    todo!()
}

#[derive(Debug)]
pub struct PinVerifier {
    expected: Option<String>,
    seen: Mutex<Option<String>>,
    provider: Arc<CryptoProvider>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf() -> CertificateDer<'static> {
        CertificateDer::from(vec![0x30, 0x03, 0x02, 0x01, 0x01])
    }

    fn verify(v: &PinVerifier) -> Result<ServerCertVerified, rustls::Error> {
        let name = ServerName::try_from("vox.lan").expect("server name");
        v.verify_server_cert(&leaf(), &[], &name, &[], UnixTime::now())
    }

    #[test]
    fn fingerprint_is_lowercase_hex_sha3_256() {
        let fp = cert_fingerprint(b"abc");
        assert_eq!(fp.len(), 64);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(fp, vox_crypto::hex_encode(&vox_crypto::compliance_hash(b"abc")));
        assert_ne!(fp, cert_fingerprint(b"abd"));
    }

    #[test]
    fn pin_outcome_cases() {
        assert_eq!(pin_outcome(Some("ABCD"), "abcd"), PinOutcome::Match);
        assert_eq!(pin_outcome(None, "abcd"), PinOutcome::FirstUse { fingerprint: "abcd".into() });
        assert_eq!(
            pin_outcome(Some("1111"), "abcd"),
            PinOutcome::Changed { pinned: "1111".into(), fingerprint: "abcd".into() }
        );
    }

    #[test]
    fn verifier_rejects_first_use_and_records_the_fingerprint() {
        let v = PinVerifier::new(None);
        assert!(verify(&v).is_err());
        let fp = cert_fingerprint(leaf().as_ref());
        assert_eq!(v.seen(), Some(fp.clone()));
        assert_eq!(v.outcome(), Some(PinOutcome::FirstUse { fingerprint: fp }));
    }

    #[test]
    fn verifier_accepts_only_the_pinned_certificate() {
        let fp = cert_fingerprint(leaf().as_ref());
        assert!(verify(&PinVerifier::new(Some(fp))).is_ok());
        let changed = PinVerifier::new(Some("00".repeat(32)));
        assert!(verify(&changed).is_err());
        assert!(matches!(changed.outcome(), Some(PinOutcome::Changed { .. })));
    }

    #[test]
    fn pinned_client_config_builds_on_the_ring_provider() {
        let cfg = pinned_client_config(Arc::new(PinVerifier::new(None))).expect("config");
        assert!(cfg.alpn_protocols.is_empty());
    }
}
```

- [ ] **Step 4: Run the tests and watch them fail.**

```bash
cargo nextest run -p vox-gui remote::
```

Expected: compile errors for the missing `PinVerifier` methods and `pinned_client_config`. That counts as the failing state. Add the Step 5 code for `PinVerifier`/`pinned_client_config`, and the remaining tests fail with `not yet implemented` until every body is in place.

- [ ] **Step 5: Implement `error.rs`:**

```rust
impl AxisError {
    pub fn code(&self) -> &str {
        match self {
            Self::Daemon(_) => "daemon_error",
            Self::DaemonUnavailable(_) => "daemon_unavailable",
            Self::LocalOnly { .. } => "local_only",
            Self::NotRemoteCapable { .. } => "not_remote_capable",
            Self::VersionMismatch { .. } => "version_mismatch",
            Self::TlsPinRequired { .. } => "tls_pin_required",
            Self::TlsPinMismatch { .. } => "tls_pin_mismatch",
            Self::Unauthorized(_) => "unauthorized",
            Self::Unreachable(_) => "unreachable",
            Self::InvalidUrl(_) => "invalid_url",
            Self::InsecureUrl(_) => "insecure_url",
            Self::Remote { code, .. } => code,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::LocalOnly { command, module, class } => format!(
                "{command} needs this machine's local Vox ({class}-class module `{module}`); disconnect from the server to use it"
            ),
            Self::NotRemoteCapable { method } => {
                format!("daemon method {method} has no remote route yet")
            }
            Self::VersionMismatch { server, gui } => {
                format!("server runs Vox {server}, Axis is {gui}; major.minor must match")
            }
            Self::TlsPinRequired { .. } => {
                "first connection to this server: confirm its certificate fingerprint".to_string()
            }
            Self::TlsPinMismatch { .. } => {
                "the server certificate changed since it was pinned".to_string()
            }
            Self::Daemon(m)
            | Self::DaemonUnavailable(m)
            | Self::Unauthorized(m)
            | Self::Unreachable(m)
            | Self::InvalidUrl(m)
            | Self::InsecureUrl(m) => m.clone(),
            Self::Remote { message, .. } => message.clone(),
        }
    }

    pub fn to_json(&self) -> Value {
        let mut err = json!({ "code": self.code(), "message": self.message() });
        match self {
            Self::LocalOnly { command, module, class } => {
                err["command"] = json!(command);
                err["module"] = json!(module);
                err["class"] = json!(class);
            }
            Self::NotRemoteCapable { method } => err["method"] = json!(method),
            Self::VersionMismatch { server, gui } => {
                err["server"] = json!(server);
                err["gui"] = json!(gui);
            }
            Self::TlsPinRequired { fingerprint } => err["fingerprint"] = json!(fingerprint),
            Self::TlsPinMismatch { pinned, fingerprint } => {
                err["pinned"] = json!(pinned);
                err["fingerprint"] = json!(fingerprint);
            }
            _ => {}
        }
        json!({ "error": err })
    }
}

impl From<AxisError> for String {
    fn from(err: AxisError) -> String {
        match err {
            AxisError::Daemon(m) | AxisError::DaemonUnavailable(m) => m,
            other => other.to_json().to_string(),
        }
    }
}
```

**Implement `tls_pin.rs`** (replace the stubs, keep the tests):

```rust
pub fn cert_fingerprint(der: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::compliance_hash(der))
}

pub fn pin_outcome(expected: Option<&str>, seen: &str) -> PinOutcome {
    match expected {
        Some(pin) if pin.eq_ignore_ascii_case(seen) => PinOutcome::Match,
        Some(pin) => PinOutcome::Changed {
            pinned: pin.to_ascii_lowercase(),
            fingerprint: seen.to_string(),
        },
        None => PinOutcome::FirstUse {
            fingerprint: seen.to_string(),
        },
    }
}

impl PinVerifier {
    pub fn new(expected: Option<String>) -> Self {
        Self {
            expected,
            seen: Mutex::new(None),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }

    pub fn seen(&self) -> Option<String> {
        self.seen.lock().expect("pin verifier mutex").clone()
    }

    pub fn outcome(&self) -> Option<PinOutcome> {
        self.seen()
            .map(|fp| pin_outcome(self.expected.as_deref(), &fp))
    }
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let fingerprint = cert_fingerprint(end_entity.as_ref());
        *self.seen.lock().expect("pin verifier mutex") = Some(fingerprint.clone());
        match pin_outcome(self.expected.as_deref(), &fingerprint) {
            PinOutcome::Match => Ok(ServerCertVerified::assertion()),
            _ => Err(rustls::Error::General(
                "axis remote: server certificate is not pinned".into(),
            )),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub fn pinned_client_config(verifier: Arc<PinVerifier>) -> Result<rustls::ClientConfig, rustls::Error> {
    let provider = verifier.provider.clone();
    Ok(rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth())
}
```

- [ ] **Step 6: Run the tests again.** Same command. Expected: 9 tests PASS (4 in `error`, 5 in `tls_pin`).

- [ ] **Step 7: Mutation check (security guard).** In `verify_server_cert`, change `PinOutcome::Match => Ok(…)` to `_ => Ok(ServerCertVerified::assertion())`, which accepts any certificate. Run `cargo nextest run -p vox-gui remote::tls_pin`. Expected: `verifier_rejects_first_use_and_records_the_fingerprint` and `verifier_accepts_only_the_pinned_certificate` FAIL. Restore, then confirm with `rg -c 'PinOutcome::Match => Ok\(ServerCertVerified::assertion\(\)\)' crates/vox-gui/src/remote/tls_pin.rs` (expected `1`) and rerun to green.

- [ ] **Step 8: Lint and commit.**

```bash
cargo clippy -p vox-gui --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-gui/Cargo.toml Cargo.lock crates/vox-gui/src/main.rs crates/vox-gui/src/remote/
git commit -m "feat(gui): AxisError envelope and TOFU certificate pin verifier"
```

---

### Task 5: Embedded manifest + surface availability

**Files:**
- Create: `crates/vox-gui/src/remote/manifest.rs`
- Modify: `crates/vox-gui/src/remote/mod.rs` (`pub mod manifest;`)

**Interfaces:**

```rust
pub enum ModuleClass { Daemon, Db, Host, Shell }            // serde snake_case
pub struct CommandBackendRow { pub module: String, pub class: ModuleClass, pub surfaces: Vec<String> }
pub struct CommandBackends { pub commands: Vec<CommandBackendRow> }
pub fn command_backends() -> &'static CommandBackends;      // parsed once from the embedded YAML
pub fn module_class(module: &str) -> Option<ModuleClass>;
pub fn registry_surfaces() -> &'static [String];            // non-null view_keys of surface-registry.v1.yaml
pub const REMOTE_ROUTED_DAEMON_MODULES: &[&str];
pub const REMOTE_UNROUTED_DAEMON_MODULES: &[&str];
pub const AXIS_LOCAL_SURFACES: &[&str];
pub enum UnavailableReason { NotInProfile, NeedsLocalMachine, NotRemoteCapable }   // serde snake_case
pub struct SurfaceAvailability { pub surface: String, pub available: bool, pub reason: Option<UnavailableReason> }
pub fn surface_availability(backends: &CommandBackends, surfaces: &[String], remote_axis_surfaces: Option<&[String]>) -> Vec<SurfaceAvailability>;
```

Rule (spec §7): a surface renders iff it is in the profile's `axis_surfaces` **and**, in remote mode, every module listing it is `daemon` (with a remote route) or `shell`. `settings` and `coverage` configure this Axis install, so they are always available.

**Parallel-safe with:** Tasks 3, 4, 9.

- [ ] **Step 1: Write the failing tests.** Create `manifest.rs` with the type definitions and the tests below, with `todo!()` bodies for the functions:

```rust
//! Embedded `contracts/gui/command-backends.v1.yaml` + `surface-registry.v1.yaml`, and the
//! per-surface availability rule for Remote Axis.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const COMMAND_BACKENDS_YAML: &str =
    include_str!("../../../../contracts/gui/command-backends.v1.yaml");
const SURFACE_REGISTRY_YAML: &str =
    include_str!("../../../../contracts/gui/surface-registry.v1.yaml");

/// Daemon-class modules whose every daemon method has a `RemoteRoute` (see `gateway_client::remote_route`).
pub const REMOTE_ROUTED_DAEMON_MODULES: &[&str] = &["daemon", "event_annotate", "mcp", "vcs_isolation"];
/// Daemon-class modules with at least one method the gateway does not expose yet.
pub const REMOTE_UNROUTED_DAEMON_MODULES: &[&str] = &["control_plane", "mission_control"];
/// Surfaces that configure this Axis install itself; available in both modes.
pub const AXIS_LOCAL_SURFACES: &[&str] = &["settings", "coverage"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleClass {
    Daemon,
    Db,
    Host,
    Shell,
}

impl ModuleClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Db => "db",
            Self::Host => "host",
            Self::Shell => "shell",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommandBackendRow {
    pub module: String,
    pub class: ModuleClass,
    #[serde(default)]
    pub surfaces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommandBackends {
    pub commands: Vec<CommandBackendRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotInProfile,
    NeedsLocalMachine,
    NotRemoteCapable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurfaceAvailability {
    pub surface: String,
    pub available: bool,
    pub reason: Option<UnavailableReason>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(module: &str, class: ModuleClass, surfaces: &[&str]) -> CommandBackendRow {
        CommandBackendRow {
            module: module.into(),
            class,
            surfaces: surfaces.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn reason_of(out: &[SurfaceAvailability], surface: &str) -> Option<UnavailableReason> {
        out.iter().find(|a| a.surface == surface).expect("surface listed").reason
    }

    #[test]
    fn local_mode_makes_every_surface_available() {
        let b = CommandBackends { commands: vec![row("pty", ModuleClass::Host, &["console"])] };
        let out = surface_availability(&b, &strings(&["console", "mesh"]), None);
        assert!(out.iter().all(|a| a.available && a.reason.is_none()));
    }

    #[test]
    fn remote_mode_reasons_follow_the_most_restrictive_module() {
        let b = CommandBackends {
            commands: vec![
                row("mcp", ModuleClass::Daemon, &["mesh", "tasks"]),
                row("control_plane", ModuleClass::Daemon, &["tasks"]),
                row("models", ModuleClass::Db, &["models"]),
                row("pty", ModuleClass::Host, &["console"]),
                row("preferences", ModuleClass::Shell, &["memory"]),
            ],
        };
        let surfaces = strings(&["console", "mesh", "models", "tasks", "memory", "runs", "settings"]);
        let profile = strings(&["console", "mesh", "models", "tasks", "memory"]);
        let out = surface_availability(&b, &surfaces, Some(&profile));
        assert_eq!(reason_of(&out, "mesh"), None);
        assert_eq!(reason_of(&out, "memory"), None);
        assert_eq!(reason_of(&out, "console"), Some(UnavailableReason::NeedsLocalMachine));
        assert_eq!(reason_of(&out, "models"), Some(UnavailableReason::NotRemoteCapable));
        assert_eq!(reason_of(&out, "tasks"), Some(UnavailableReason::NotRemoteCapable));
        assert_eq!(reason_of(&out, "runs"), Some(UnavailableReason::NotInProfile));
        assert_eq!(reason_of(&out, "settings"), None, "Axis-local surfaces are always available");
    }

    /// `dev` serves `axis_surfaces: ["*"]` (00-contracts Amendments "From plan 02"): every surface is in profile.
    #[test]
    fn wildcard_axis_surfaces_put_every_surface_in_profile() {
        let out = surface_availability(command_backends(), registry_surfaces(), Some(&strings(&["*"])));
        assert!(out.iter().all(|a| a.reason != Some(UnavailableReason::NotInProfile)), "{out:?}");
        assert_eq!(reason_of(&out, "mesh"), None);
    }

    #[test]
    fn embedded_manifest_parses_and_covers_axis_remote() {
        assert_eq!(module_class("axis_remote"), Some(ModuleClass::Shell));
        assert_eq!(module_class("pty"), Some(ModuleClass::Host));
        assert_eq!(module_class("mcp"), Some(ModuleClass::Daemon));
        assert_eq!(module_class("no_such_module"), None);
        assert!(registry_surfaces().iter().any(|s| s == "mesh"));
        assert!(!registry_surfaces().iter().any(|s| s.is_empty()));
    }

    #[test]
    fn every_daemon_module_has_a_routing_decision() {
        for row in &command_backends().commands {
            if row.class == ModuleClass::Daemon {
                let decided = REMOTE_ROUTED_DAEMON_MODULES.contains(&row.module.as_str())
                    || REMOTE_UNROUTED_DAEMON_MODULES.contains(&row.module.as_str());
                assert!(decided, "daemon module `{}` needs a remote routing decision", row.module);
            }
        }
    }

    #[test]
    fn real_inventory_gates_known_surfaces() {
        let profile = strings(&["mesh", "repository", "models", "console", "vox-search"]);
        let out = surface_availability(command_backends(), registry_surfaces(), Some(&profile));
        assert_eq!(reason_of(&out, "mesh"), None);
        assert_eq!(reason_of(&out, "repository"), None);
        assert_eq!(reason_of(&out, "vox-search"), None);
        assert_eq!(reason_of(&out, "models"), Some(UnavailableReason::NotRemoteCapable));
        assert_eq!(reason_of(&out, "console"), Some(UnavailableReason::NeedsLocalMachine));
        assert_eq!(reason_of(&out, "chat"), Some(UnavailableReason::NotInProfile));
    }
}
```

- [ ] **Step 2: Run the tests and watch them fail.**

```bash
cargo nextest run -p vox-gui remote::manifest
```

Expected: 5 FAIL (`not yet implemented`).

- [ ] **Step 3: Implement:**

```rust
#[derive(Deserialize)]
struct SurfaceRegistry {
    surfaces: Vec<SurfaceRegistryRow>,
}

#[derive(Deserialize)]
struct SurfaceRegistryRow {
    view_key: Option<String>,
}

pub fn command_backends() -> &'static CommandBackends {
    static PARSED: OnceLock<CommandBackends> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_yaml::from_str(COMMAND_BACKENDS_YAML)
            .expect("contracts/gui/command-backends.v1.yaml parses (enforced by vox ci gui-catalog-parity)")
    })
}

pub fn module_class(module: &str) -> Option<ModuleClass> {
    command_backends()
        .commands
        .iter()
        .find(|r| r.module == module)
        .map(|r| r.class)
}

pub fn registry_surfaces() -> &'static [String] {
    static PARSED: OnceLock<Vec<String>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let registry: SurfaceRegistry = serde_yaml::from_str(SURFACE_REGISTRY_YAML)
            .expect("contracts/gui/surface-registry.v1.yaml parses (enforced by vox ci gui-surface-registry)");
        registry
            .surfaces
            .into_iter()
            .filter_map(|r| r.view_key)
            .filter(|k| !k.is_empty())
            .collect()
    })
}

pub fn surface_availability(
    backends: &CommandBackends,
    surfaces: &[String],
    remote_axis_surfaces: Option<&[String]>,
) -> Vec<SurfaceAvailability> {
    surfaces
        .iter()
        .map(|surface| {
            let reason = remote_axis_surfaces.and_then(|served| remote_reason(backends, surface, served));
            SurfaceAvailability {
                surface: surface.clone(),
                available: reason.is_none(),
                reason,
            }
        })
        .collect()
}

fn remote_reason(backends: &CommandBackends, surface: &str, served: &[String]) -> Option<UnavailableReason> {
    if AXIS_LOCAL_SURFACES.contains(&surface) {
        return None;
    }
    if !served.iter().any(|s| s == surface || s == "*") {
        return Some(UnavailableReason::NotInProfile);
    }
    let rows = || backends.commands.iter().filter(|r| r.surfaces.iter().any(|s| s == surface));
    if rows().any(|r| r.class == ModuleClass::Host) {
        return Some(UnavailableReason::NeedsLocalMachine);
    }
    let not_remote_capable = rows().any(|r| match r.class {
        ModuleClass::Db => true,
        ModuleClass::Daemon => !REMOTE_ROUTED_DAEMON_MODULES.contains(&r.module.as_str()),
        ModuleClass::Host | ModuleClass::Shell => false,
    });
    not_remote_capable.then_some(UnavailableReason::NotRemoteCapable)
}
```

Add `pub mod manifest;` to `remote/mod.rs`.

- [ ] **Step 4: Run the tests again.** Same command. Expected: 5 PASS.

- [ ] **Step 5: Lint and commit.**

```bash
cargo clippy -p vox-gui --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-gui/src/remote/manifest.rs crates/vox-gui/src/remote/mod.rs
git commit -m "feat(gui): per-surface remote availability from the command-backend manifest"
```

---

### Task 6: `GatewayClient` (profile, version check, WebSocket RPC, `/api/v2`, route table)

**Files:**
- Create: `crates/vox-gui/src/remote/gateway_client.rs`
- Modify: `crates/vox-gui/src/remote/mod.rs` (`pub mod gateway_client;`)

**Interfaces:**

```rust
pub struct ServerProfile { pub profiles: Vec<String>, pub max_exposure: String, pub subsystems: ServerSubsystems,
    pub tools: Vec<String>, pub axis_surfaces: Vec<String>, pub server_version: String }      // = GET /v1/profile (00-contracts)
pub fn parse_server_url(raw: &str) -> Result<Url, AxisError>;   // https anywhere; http only for loopback; normalized to origin
pub fn ws_url(base: &Url) -> Result<Url, AxisError>;            // https→wss, http→ws, path /v1/ws
pub fn major_minor(version: &str) -> Option<(u64, u64)>;
pub fn check_version(server: &str, gui: &str) -> Result<(), AxisError>;
pub enum RemoteRoute { ToolCall, ApiGet(&'static str), ApiPost(&'static str) }
pub fn remote_route(method: &str) -> Option<RemoteRoute>;
pub fn unwrap_v2_envelope(body: Value) -> Result<Value, AxisError>;
pub struct WsRpc; impl WsRpc { pub fn spawn<S>(ws: WebSocketStream<S>) -> Self; pub async fn call_tool(&self, name: &str, args: Value, deadline: Duration) -> Result<Value, AxisError>; pub fn subscribe(&self) -> broadcast::Receiver<Value>; }
pub struct ConnectRequest { pub url: Url, pub token: String, pub expected_pin: Option<String> }
pub struct GatewayClient;
impl GatewayClient {
    pub async fn connect(req: ConnectRequest) -> Result<(Self, Option<String>), AxisError>;   // Some(pin) for https
    pub fn profile(&self) -> &ServerProfile; pub fn base_url(&self) -> &Url;
    pub async fn call_daemon_method(&self, method: &str, params: Value, deadline: Duration) -> Result<Value, AxisError>;
    pub async fn api_get(&self, path: &str) -> Result<Value, AxisError>;
    pub async fn api_post(&self, path: &str, body: Value) -> Result<Value, AxisError>;
    pub fn subscribe_events(&self) -> broadcast::Receiver<Value>;
}
pub(crate) async fn fetch_profile(http: &reqwest::Client, base: &Url, token: &str) -> Result<ServerProfile, AxisError>;
```

**Wire facts used (read from `crates/vox-orchestrator-mcp/src/http_gateway/`):**

- WS frames are `{"id","type","name","args"}`. The types are `call_tool` / `list_tools` / `subscribe`, and replies are `{"id","type":"call_tool_result","success","is_error","data","error"}`.
- `data` is the parsed tool JSON envelope, the same value the daemon's `orch.tool_call` returns. So `invoke_mcp_tool` wraps identical shapes in both modes.
- The server pushes `{"type":"agent_event","data":<AgentEvent>}`.
- The upgrade authenticates from the `Authorization` header (`resolve_access_role`).
- `/api/v2` returns `{"v":1,"data":…}` or `{"v":1,"error":{"code","message"}}` (`services::routes::{ok,err}`).
- `GET /api/v2/vcs/isolation` and `POST /api/v2/vcs/isolation/strategy` take the same body keys as `vcs_isolation::build_set_strategy_params` (`strategy_default`, `agent_id`, `strategy`) and return `json_vcs_facade::isolation_status_json`, which is what `orch.vcs_isolation_status` returns locally.

**Parallel-safe with:** Tasks 3, 5, 9.

- [ ] **Step 1: Write the failing tests.** Create `gateway_client.rs` with the type definitions (`ServerProfile`, `ServerSubsystems`, `FailedSubsystem`, `RemoteRoute`, `ConnectRequest`, `WsRpc` fields, `GatewayClient` fields as in Step 3), the function signatures with `todo!()` bodies, and this test module:

```rust
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use tokio::io::DuplexStream;
    use tokio_tungstenite::tungstenite::protocol::Role;

    pub(crate) fn profile(axis_surfaces: &[&str]) -> ServerProfile {
        ServerProfile {
            profiles: vec!["mesh".into()],
            max_exposure: "operator".into(),
            subsystems: ServerSubsystems { booted: vec!["db".into(), "mesh".into()], failed: vec![] },
            tools: vec!["vox_mesh_status".into()],
            axis_surfaces: axis_surfaces.iter().map(|s| s.to_string()).collect(),
            server_version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    /// A `GatewayClient` wired to an in-memory websocket; the returned half plays `/v1/ws`.
    pub(crate) async fn duplex_client(profile: ServerProfile) -> (GatewayClient, WebSocketStream<DuplexStream>) {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let client_ws = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let server_ws = WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let base = Url::parse("https://vox.lan:3921/").expect("url");
        (GatewayClient::from_parts(base, "t0ken".into(), reqwest::Client::new(), profile, WsRpc::spawn(client_ws)), server_ws)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn url_policy() {
        assert_eq!(parse_server_url("https://vox.lan:3921/x?y=1").unwrap().as_str(), "https://vox.lan:3921/");
        assert!(parse_server_url("http://127.0.0.1:3921").is_ok());
        assert!(parse_server_url("http://localhost:3921").is_ok());
        assert!(parse_server_url("http://[::1]:3921").is_ok());
        assert_eq!(parse_server_url("http://vox.lan:3921").unwrap_err().code(), "insecure_url");
        assert_eq!(parse_server_url("ftp://vox.lan").unwrap_err().code(), "invalid_url");
        assert_eq!(parse_server_url("not a url").unwrap_err().code(), "invalid_url");
    }

    #[test]
    fn ws_url_swaps_scheme_and_path() {
        assert_eq!(ws_url(&Url::parse("https://vox.lan:3921/").unwrap()).unwrap().as_str(), "wss://vox.lan:3921/v1/ws");
        assert_eq!(ws_url(&Url::parse("http://127.0.0.1:3921/").unwrap()).unwrap().as_str(), "ws://127.0.0.1:3921/v1/ws");
    }

    #[test]
    fn version_check_compares_major_minor_only() {
        assert_eq!(major_minor("0.6.3+build.12 (abc)"), Some((0, 6)));
        assert_eq!(major_minor("1.2"), Some((1, 2)));
        assert_eq!(major_minor("garbage"), None);
        assert!(check_version("0.6.9", "0.6.0").is_ok());
        assert_eq!(check_version("0.7.0", "0.6.0").unwrap_err().code(), "version_mismatch");
        assert_eq!(check_version("garbage", "0.6.0").unwrap_err().code(), "version_mismatch");
    }

    #[test]
    fn route_table() {
        use vox_foundation::protocol::orch_daemon_method as m;
        assert_eq!(remote_route(m::TOOL_CALL), Some(RemoteRoute::ToolCall));
        assert_eq!(remote_route(m::VCS_ISOLATION_STATUS), Some(RemoteRoute::ApiGet("/api/v2/vcs/isolation")));
        assert_eq!(remote_route(m::VCS_ISOLATION_SET_STRATEGY), Some(RemoteRoute::ApiPost("/api/v2/vcs/isolation/strategy")));
        assert_eq!(remote_route(m::SUBMIT_TASK), None);
        assert_eq!(remote_route(m::SUBAGENT_TREE), None);
    }

    #[test]
    fn v2_envelope() {
        assert_eq!(unwrap_v2_envelope(json!({"v":1,"data":{"a":1}})).unwrap(), json!({"a":1}));
        let err = unwrap_v2_envelope(json!({"v":1,"error":{"code":"lock","message":"busy"}})).unwrap_err();
        assert_eq!(err, AxisError::Remote { code: "lock".into(), message: "busy".into() });
    }

    #[tokio::test]
    async fn fetch_profile_sends_bearer_and_maps_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/profile"))
            .and(header("authorization", "Bearer good"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "profiles": ["mesh"], "max_exposure": "operator",
                "subsystems": {"booted": ["db", "mesh"], "failed": [{"id": "research", "error": "no key"}]},
                "tools": ["vox_mesh_status"], "axis_surfaces": ["mesh"], "server_version": "0.6.0"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET")).and(path("/v1/profile")).respond_with(ResponseTemplate::new(401)).mount(&server).await;
        let base = parse_server_url(&server.uri()).unwrap();
        let http = reqwest::Client::new();
        let p = fetch_profile(&http, &base, "good").await.expect("profile");
        assert_eq!(p.axis_surfaces, vec!["mesh".to_string()]);
        assert_eq!(p.subsystems.failed[0].id, "research");
        assert_eq!(fetch_profile(&http, &base, "bad").await.unwrap_err().code(), "unauthorized");
    }

    #[tokio::test]
    async fn ws_rpc_matches_replies_by_id_and_forwards_agent_events() {
        let (client, mut server) = duplex_client(profile(&["mesh"])).await;
        let mut events = client.subscribe_events();
        let answer = tokio::spawn(async move {
            let Some(Ok(Message::Text(text))) = server.next().await else { panic!("no request") };
            let req: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(req["type"], "call_tool");
            assert_eq!(req["name"], "vox_mesh_status");
            server.send(Message::Text(json!({"type":"agent_event","success":true,"is_error":false,"data":{"id":7}}).to_string())).await.unwrap();
            server.send(Message::Text(json!({"id":req["id"],"type":"call_tool_result","success":true,"is_error":false,"data":{"success":true,"data":{"nodes":[]}}}).to_string())).await.unwrap();
            server
        });
        let out = client
            .call_daemon_method(
                vox_foundation::protocol::orch_daemon_method::TOOL_CALL,
                json!({"name":"vox_mesh_status","args":{}}),
                Duration::from_secs(5),
            )
            .await
            .expect("tool result");
        assert_eq!(out, json!({"success":true,"data":{"nodes":[]}}));
        assert_eq!(events.recv().await.unwrap(), json!({"id":7}));
        drop(answer.await.unwrap());
    }

    #[tokio::test]
    async fn unrouted_method_is_not_remote_capable() {
        let (client, _server) = duplex_client(profile(&["mesh"])).await;
        let err = client
            .call_daemon_method(vox_foundation::protocol::orch_daemon_method::SUBAGENT_TREE, json!({}), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "not_remote_capable");
    }

    #[tokio::test]
    async fn closed_socket_fails_pending_calls() {
        let (client, server) = duplex_client(profile(&["mesh"])).await;
        drop(server);
        let err = client
            .call_daemon_method(
                vox_foundation::protocol::orch_daemon_method::TOOL_CALL,
                json!({"name":"vox_mesh_status"}),
                Duration::from_secs(5),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "unreachable");
    }
}
```

- [ ] **Step 2: Run the tests and watch them fail.**

```bash
cargo nextest run -p vox-gui remote::gateway_client
```

Expected: 9 FAIL (`not yet implemented`).

- [ ] **Step 3: Implement** (full file body above the test modules):

```rust
//! Remote Axis client for a `vox serve` gateway: `GET /v1/profile` and `/api/v2/*` over reqwest,
//! MCP tool calls and agent events over the `/v1/ws` WebSocket RPC. TLS uses the TOFU
//! `PinVerifier`; plain http is accepted only for loopback.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use vox_foundation::protocol::orch_daemon_method;

use super::error::AxisError;
use super::tls_pin::{PinOutcome, PinVerifier, pinned_client_config};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerProfile {
    pub profiles: Vec<String>,
    pub max_exposure: String,
    pub subsystems: ServerSubsystems,
    pub tools: Vec<String>,
    pub axis_surfaces: Vec<String>,
    pub server_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSubsystems {
    pub booted: Vec<String>,
    pub failed: Vec<FailedSubsystem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedSubsystem {
    pub id: String,
    pub error: String,
}

pub fn parse_server_url(raw: &str) -> Result<Url, AxisError> {
    let url = Url::parse(raw.trim()).map_err(|e| AxisError::InvalidUrl(format!("{raw}: {e}")))?;
    let host = url
        .host_str()
        .ok_or_else(|| AxisError::InvalidUrl(format!("{raw}: no host")))?;
    match url.scheme() {
        "https" => {}
        "http" if is_loopback_host(host) => {}
        "http" => {
            return Err(AxisError::InsecureUrl(format!(
                "{raw}: use https:// (plain http is only allowed for loopback)"
            )));
        }
        other => return Err(AxisError::InvalidUrl(format!("{raw}: unsupported scheme {other}"))),
    }
    let mut origin = url;
    origin.set_path("/");
    origin.set_query(None);
    origin.set_fragment(None);
    Ok(origin)
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost"
        || host
            .trim_matches(|c| c == '[' || c == ']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

pub fn ws_url(base: &Url) -> Result<Url, AxisError> {
    let mut url = base
        .join("/v1/ws")
        .map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
    let scheme = if base.scheme() == "https" { "wss" } else { "ws" };
    url.set_scheme(scheme)
        .map_err(|()| AxisError::InvalidUrl(format!("cannot derive a websocket url from {base}")))?;
    Ok(url)
}

pub fn major_minor(version: &str) -> Option<(u64, u64)> {
    let core = version.split(['+', '-', ' ']).next()?;
    let mut parts = core.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

pub fn check_version(server: &str, gui: &str) -> Result<(), AxisError> {
    match (major_minor(server), major_minor(gui)) {
        (Some(a), Some(b)) if a == b => Ok(()),
        _ => Err(AxisError::VersionMismatch {
            server: server.to_string(),
            gui: gui.to_string(),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteRoute {
    ToolCall,
    ApiGet(&'static str),
    ApiPost(&'static str),
}

/// Gateway equivalent of an orchestrator daemon method; `None` = not exposed remotely yet.
pub fn remote_route(method: &str) -> Option<RemoteRoute> {
    match method {
        orch_daemon_method::TOOL_CALL => Some(RemoteRoute::ToolCall),
        orch_daemon_method::VCS_ISOLATION_STATUS => Some(RemoteRoute::ApiGet("/api/v2/vcs/isolation")),
        orch_daemon_method::VCS_ISOLATION_SET_STRATEGY => {
            Some(RemoteRoute::ApiPost("/api/v2/vcs/isolation/strategy"))
        }
        _ => None,
    }
}

pub fn unwrap_v2_envelope(body: Value) -> Result<Value, AxisError> {
    if let Some(err) = body.get("error") {
        return Err(AxisError::Remote {
            code: err.get("code").and_then(Value::as_str).unwrap_or("error").to_string(),
            message: err.get("message").and_then(Value::as_str).unwrap_or_default().to_string(),
        });
    }
    Ok(body.get("data").cloned().unwrap_or(Value::Null))
}

#[derive(Debug, Deserialize)]
struct WsReply {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<String>,
}

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<WsReply>>>>;

pub struct WsRpc {
    out: mpsc::UnboundedSender<Message>,
    pending: Pending,
    events: broadcast::Sender<Value>,
    next_id: AtomicU64,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl WsRpc {
    pub fn spawn<S>(ws: WebSocketStream<S>) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (mut sink, mut stream) = ws.split();
        let (out, mut out_rx) = mpsc::unbounded_channel::<Message>();
        let pending: Pending = Arc::default();
        let (events, _) = broadcast::channel(256);
        let writer = tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        });
        let reader_pending = pending.clone();
        let reader_events = events.clone();
        let reader = tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let Message::Text(text) = msg else { continue };
                let Ok(reply) = serde_json::from_str::<WsReply>(&text) else { continue };
                if let Some(id) = reply.id.clone() {
                    if let Some(tx) = reader_pending.lock().await.remove(&id) {
                        let _ = tx.send(reply);
                        continue;
                    }
                }
                if reply.msg_type == "agent_event" {
                    let _ = reader_events.send(reply.data);
                }
            }
            // Dropping the senders fails every waiter with RecvError -> Unreachable.
            reader_pending.lock().await.clear();
        });
        Self {
            out,
            pending,
            events,
            next_id: AtomicU64::new(1),
            tasks: vec![writer, reader],
        }
    }

    pub async fn call_tool(&self, name: &str, args: Value, deadline: Duration) -> Result<Value, AxisError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);
        let frame = serde_json::json!({ "id": id, "type": "call_tool", "name": name, "args": args });
        if self.out.send(Message::Text(frame.to_string())).is_err() {
            self.pending.lock().await.remove(&id);
            return Err(AxisError::Unreachable("websocket closed".into()));
        }
        match tokio::time::timeout(deadline, rx).await {
            Ok(Ok(reply)) if reply.msg_type == "call_tool_result" => Ok(reply.data),
            Ok(Ok(reply)) => Err(AxisError::Remote {
                code: "ws_error".into(),
                message: reply.error.unwrap_or(reply.msg_type),
            }),
            Ok(Err(_)) => Err(AxisError::Unreachable("websocket closed".into())),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(AxisError::Unreachable(format!("tool {name} timed out")))
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.events.subscribe()
    }
}

impl Drop for WsRpc {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

pub struct ConnectRequest {
    pub url: Url,
    pub token: String,
    pub expected_pin: Option<String>,
}

pub struct GatewayClient {
    base: Url,
    token: String,
    http: reqwest::Client,
    profile: ServerProfile,
    ws: WsRpc,
}

impl GatewayClient {
    /// Connects, checks the version, and opens `/v1/ws`. Returns the pin actually used (https only).
    pub async fn connect(req: ConnectRequest) -> Result<(Self, Option<String>), AxisError> {
        let verifier = Arc::new(PinVerifier::new(req.expected_pin.clone()));
        let mut builder = vox_http_client::client_builder();
        let mut connector = None;
        if req.url.scheme() == "https" {
            let cfg = pinned_client_config(verifier.clone())
                .map_err(|e| AxisError::Unreachable(e.to_string()))?;
            connector = Some(Connector::Rustls(Arc::new(cfg.clone())));
            builder = builder.use_preconfigured_tls(cfg);
        }
        let http = builder.build().map_err(|e| AxisError::Unreachable(e.to_string()))?;
        let profile = match fetch_profile(&http, &req.url, &req.token).await {
            Ok(p) => p,
            Err(e) => return Err(pin_error(&verifier).unwrap_or(e)),
        };
        check_version(&profile.server_version, env!("CARGO_PKG_VERSION"))?;
        let ws = connect_ws(&req.url, &req.token, connector)
            .await
            .map_err(|e| pin_error(&verifier).unwrap_or(e))?;
        let pin = verifier.seen();
        Ok((Self::from_parts(req.url, req.token, http, profile, WsRpc::spawn(ws)), pin))
    }

    pub(crate) fn from_parts(base: Url, token: String, http: reqwest::Client, profile: ServerProfile, ws: WsRpc) -> Self {
        Self { base, token, http, profile, ws }
    }

    pub fn profile(&self) -> &ServerProfile {
        &self.profile
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    pub async fn call_daemon_method(&self, method: &str, params: Value, deadline: Duration) -> Result<Value, AxisError> {
        match remote_route(method) {
            Some(RemoteRoute::ToolCall) => {
                let name = params.get("name").and_then(Value::as_str).ok_or_else(|| AxisError::Remote {
                    code: "bad_request".into(),
                    message: "params.name (string) required".into(),
                })?;
                let args = params.get("args").cloned().unwrap_or_else(|| serde_json::json!({}));
                self.ws.call_tool(name, args, deadline).await
            }
            Some(RemoteRoute::ApiGet(path)) => self.api_get(path).await,
            Some(RemoteRoute::ApiPost(path)) => self.api_post(path, params).await,
            None => Err(AxisError::NotRemoteCapable { method: method.to_string() }),
        }
    }

    pub async fn api_get(&self, path: &str) -> Result<Value, AxisError> {
        let url = self.base.join(path).map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
        let resp = self.http.get(url).bearer_auth(&self.token).send().await;
        read_v2(resp).await
    }

    pub async fn api_post(&self, path: &str, body: Value) -> Result<Value, AxisError> {
        let url = self.base.join(path).map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
        let resp = self.http.post(url).bearer_auth(&self.token).json(&body).send().await;
        read_v2(resp).await
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Value> {
        self.ws.subscribe()
    }
}

fn pin_error(verifier: &PinVerifier) -> Option<AxisError> {
    match verifier.outcome()? {
        PinOutcome::Match => None,
        PinOutcome::FirstUse { fingerprint } => Some(AxisError::TlsPinRequired { fingerprint }),
        PinOutcome::Changed { pinned, fingerprint } => Some(AxisError::TlsPinMismatch { pinned, fingerprint }),
    }
}

async fn read_v2(resp: reqwest::Result<reqwest::Response>) -> Result<Value, AxisError> {
    let resp = resp.map_err(|e| AxisError::Unreachable(e.to_string()))?;
    match resp.status().as_u16() {
        401 | 403 => Err(AxisError::Unauthorized(format!("server rejected the token ({})", resp.status()))),
        _ => unwrap_v2_envelope(
            resp.json::<Value>()
                .await
                .map_err(|e| AxisError::Remote { code: "bad_response".into(), message: e.to_string() })?,
        ),
    }
}

pub(crate) async fn fetch_profile(http: &reqwest::Client, base: &Url, token: &str) -> Result<ServerProfile, AxisError> {
    let url = base.join("/v1/profile").map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
    let resp = http
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| AxisError::Unreachable(e.to_string()))?;
    match resp.status().as_u16() {
        200 => resp
            .json::<ServerProfile>()
            .await
            .map_err(|e| AxisError::Remote { code: "bad_profile".into(), message: e.to_string() }),
        401 | 403 => Err(AxisError::Unauthorized(format!("server rejected the token ({})", resp.status()))),
        status => Err(AxisError::Remote {
            code: format!("http_{status}"),
            message: resp.text().await.unwrap_or_default(),
        }),
    }
}

async fn connect_ws(
    base: &Url,
    token: &str,
    connector: Option<Connector>,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, AxisError> {
    let mut request = ws_url(base)?
        .as_str()
        .into_client_request()
        .map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
    let bearer = vox_http_client::bearer_auth_header(token)
        .map_err(|_| AxisError::Unauthorized("token contains characters not allowed in a header".into()))?;
    request.headers_mut().insert(AUTHORIZATION, bearer);
    let (ws, _response) = tokio_tungstenite::connect_async_tls_with_config(request, None, false, connector)
        .await
        .map_err(|e| AxisError::Unreachable(e.to_string()))?;
    Ok(ws)
}
```

If `request.headers_mut().insert(AUTHORIZATION, bearer)` does not type-check, `reqwest` and `tungstenite` resolved different `http` majors. Check with `cargo tree -p vox-gui -i http`. Build the value with `tokio_tungstenite::tungstenite::http::HeaderValue::from_str(&vox_http_client::bearer_auth_header_string(token))` instead. Do not add an `http` dependency.

Add `pub mod gateway_client;` to `remote/mod.rs`.

- [ ] **Step 4: Run the tests again.** Same command. Expected: 9 PASS.

- [ ] **Step 5: Lint and commit.**

```bash
cargo clippy -p vox-gui --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-gui/src/remote/gateway_client.rs crates/vox-gui/src/remote/mod.rs
git commit -m "feat(gui): GatewayClient with pinned TLS, profile/version check, WS RPC"
```

---

### Task 7: `Backend` enum, `BackendState`, invoke gate, daemon-class routing

**Files:**
- Create: `crates/vox-gui/src/remote/backend.rs`, `crates/vox-gui/src/remote/gate.rs`
- Modify: `crates/vox-gui/src/remote/mod.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Modify: `crates/vox-gui/src/commands/{control_plane,mission_control,vcs_isolation,mcp,daemon,chat_turn,orchestrator}.rs`

**Interfaces:**

```rust
// backend.rs
pub enum Backend { Local(Arc<PersistentDaemon>), Remote(Arc<GatewayClient>) }
pub struct BackendStatusDto { pub mode: &'static str, pub url: Option<String>, pub profile: Option<ServerProfile>, pub surfaces: Vec<SurfaceAvailability> }
pub struct BackendState;
impl BackendState {
    pub fn new(local: Arc<PersistentDaemon>) -> Self; pub fn local(&self) -> Arc<PersistentDaemon>; pub fn is_remote(&self) -> bool;
    pub async fn current(&self) -> Backend;
    pub async fn set_remote(&self, client: Arc<GatewayClient>, relay: Option<tokio::task::JoinHandle<()>>); pub async fn set_local(&self);
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, AxisError>;
    pub async fn call_with_deadline(&self, method: &str, params: Value, deadline: Duration, permission_mode: Option<String>) -> Result<Value, AxisError>;
    pub async fn status(&self) -> BackendStatusDto;
}
// gate.rs
pub type CommandTable = [(&'static str, &'static str, &'static str)];        // (root, module, command)
pub fn local_only_denial(table: &CommandTable, command: &str, remote: bool, class_of: impl Fn(&str) -> Option<ModuleClass>) -> Option<AxisError>;
pub fn typed_handler<F>(f: F) -> F where F: Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static;
pub fn gate_invoke(table: &CommandTable, invoke: tauri::ipc::Invoke<tauri::Wry>, inner: &dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool) -> bool;
```

Local mode must be byte-for-byte today's behaviour:

- `BackendState::call` in local mode runs the same `ensure()` → `OrchDaemonClient::{with_token,new}` → `call_with_deadline` sequence the four modules hand-roll today.
- Errors pass through unchanged as `AxisError::{DaemonUnavailable,Daemon}` → `String`.
- In local mode the gate returns `inner(invoke)` untouched.

`permission_mode` is not forwarded remotely. The gateway's `call_tool` has no such field, so a remote dangerous tool follows the server's HITL default (`ask`) — the fail-safe direction.

- [ ] **Step 1: Write the failing tests.** Create `gate.rs`:

```rust
//! Remote-mode invoke gate: in remote mode, commands from `db`/`host`-class modules are rejected with
//! `local_only` before they run; `daemon`/`shell` modules and non-`commands` roots pass through.

use tauri::Manager;
use tauri::ipc::Invoke;

use super::backend::BackendState;
use super::error::AxisError;
use super::manifest::{self, ModuleClass};

pub type CommandTable = [(&'static str, &'static str, &'static str)];

pub fn local_only_denial(
    table: &CommandTable,
    command: &str,
    remote: bool,
    class_of: impl Fn(&str) -> Option<ModuleClass>,
) -> Option<AxisError> {
    let _ = (table, command, remote, class_of);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &CommandTable = &[
        ("commands", "pty", "pty_spawn"),
        ("commands", "models", "list_model_cards"),
        ("commands", "mcp", "invoke_mcp_tool"),
        ("commands", "build_info", "get_build_info"),
        ("commands", "brand_new", "brand_new_cmd"),
        ("drive", "bridge", "get_drive_mode"),
    ];

    fn class_of(module: &str) -> Option<ModuleClass> {
        match module {
            "pty" => Some(ModuleClass::Host),
            "models" => Some(ModuleClass::Db),
            "mcp" => Some(ModuleClass::Daemon),
            "build_info" => Some(ModuleClass::Shell),
            _ => None,
        }
    }

    #[test]
    fn local_mode_never_denies() {
        for (_, _, cmd) in TABLE {
            assert_eq!(local_only_denial(TABLE, cmd, false, class_of), None, "{cmd}");
        }
    }

    #[test]
    fn remote_mode_denies_db_and_host_commands() {
        let err = local_only_denial(TABLE, "pty_spawn", true, class_of).expect("denied");
        assert_eq!(err, AxisError::LocalOnly { command: "pty_spawn".into(), module: "pty".into(), class: "host" });
        let err = local_only_denial(TABLE, "list_model_cards", true, class_of).expect("denied");
        assert_eq!(err.code(), "local_only");
    }

    #[test]
    fn remote_mode_allows_daemon_shell_and_drive() {
        assert_eq!(local_only_denial(TABLE, "invoke_mcp_tool", true, class_of), None);
        assert_eq!(local_only_denial(TABLE, "get_build_info", true, class_of), None);
        assert_eq!(local_only_denial(TABLE, "get_drive_mode", true, class_of), None);
        assert_eq!(local_only_denial(TABLE, "not_registered", true, class_of), None);
    }

    #[test]
    fn unclassified_module_fails_closed_remotely() {
        let err = local_only_denial(TABLE, "brand_new_cmd", true, class_of).expect("denied");
        assert_eq!(err, AxisError::LocalOnly { command: "brand_new_cmd".into(), module: "brand_new".into(), class: "unclassified" });
    }
}
```

Create `backend.rs` with the types and `todo!()` bodies, plus:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::gateway_client::test_support::{duplex_client, profile};
    use crate::remote::manifest::UnavailableReason;
    use vox_foundation::protocol::orch_daemon_method;

    #[tokio::test]
    async fn starts_local_with_every_surface_available() {
        let state = BackendState::new(Arc::new(PersistentDaemon::default()));
        assert!(!state.is_remote());
        assert!(matches!(state.current().await, Backend::Local(_)));
        let status = state.status().await;
        assert_eq!(status.mode, "local");
        assert_eq!(status.url, None);
        assert!(status.profile.is_none());
        assert!(!status.surfaces.is_empty());
        assert!(status.surfaces.iter().all(|s| s.available && s.reason.is_none()));
    }

    #[tokio::test]
    async fn remote_mode_reports_profile_gates_surfaces_and_refuses_unrouted_methods() {
        let (client, _server) = duplex_client(profile(&["mesh", "models"])).await;
        let state = BackendState::new(Arc::new(PersistentDaemon::default()));
        state.set_remote(Arc::new(client), None).await;
        assert!(state.is_remote());

        let status = state.status().await;
        assert_eq!(status.mode, "remote");
        assert_eq!(status.url.as_deref(), Some("https://vox.lan:3921/"));
        let reason = |s: &str| status.surfaces.iter().find(|a| a.surface == s).unwrap().reason;
        assert_eq!(reason("mesh"), None);
        assert_eq!(reason("models"), Some(UnavailableReason::NotRemoteCapable));
        assert_eq!(reason("tasks"), Some(UnavailableReason::NotInProfile));

        let err = state.call(orch_daemon_method::SUBAGENT_TREE, serde_json::json!({})).await.unwrap_err();
        assert_eq!(err, AxisError::NotRemoteCapable { method: orch_daemon_method::SUBAGENT_TREE.into() });

        state.set_local().await;
        assert!(!state.is_remote());
        assert!(matches!(state.current().await, Backend::Local(_)));
    }
}
```

Add `pub mod backend;` and `pub mod gate;` to `remote/mod.rs`.

- [ ] **Step 2: Run the tests and watch them fail.**

```bash
cargo nextest run -p vox-gui remote::gate remote::backend
```

Expected: 6 FAIL (`not yet implemented`).

- [ ] **Step 3: Implement `gate.rs`:**

```rust
pub fn local_only_denial(
    table: &CommandTable,
    command: &str,
    remote: bool,
    class_of: impl Fn(&str) -> Option<ModuleClass>,
) -> Option<AxisError> {
    if !remote {
        return None;
    }
    let (root, module, _) = table.iter().find(|(_, _, cmd)| *cmd == command)?;
    if *root != "commands" {
        return None;
    }
    let class = match class_of(module) {
        Some(ModuleClass::Daemon | ModuleClass::Shell) => return None,
        Some(c @ (ModuleClass::Db | ModuleClass::Host)) => c.as_str(),
        None => "unclassified",
    };
    Some(AxisError::LocalOnly {
        command: command.to_string(),
        module: module.to_string(),
        class,
    })
}

/// Pins `tauri::generate_handler!`'s closure to the Wry runtime so it can be wrapped.
pub fn typed_handler<F>(f: F) -> F
where
    F: Fn(Invoke<tauri::Wry>) -> bool + Send + Sync + 'static,
{
    f
}

// toestub-ignore(skeleton/untested-pub-api) — Tauri IPC adapter; the decision is `local_only_denial` (tested above) and the e2e spec drives it end to end
pub fn gate_invoke(
    table: &CommandTable,
    invoke: Invoke<tauri::Wry>,
    inner: &dyn Fn(Invoke<tauri::Wry>) -> bool,
) -> bool {
    let remote = invoke
        .message
        .webview()
        .state::<std::sync::Arc<BackendState>>()
        .is_remote();
    let command = invoke.message.command().to_string();
    if let Some(err) = local_only_denial(table, &command, remote, manifest::module_class) {
        invoke.resolver.reject(String::from(err));
        return true;
    }
    inner(invoke)
}
```

**Implement `backend.rs`:**

```rust
//! The GUI's backend selector: the local orchestrator daemon (default, unchanged behaviour) or a
//! remote `vox serve` gateway.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use vox_orchestrator::orch_daemon::{ORCH_CLIENT_READ_DEADLINE, OrchDaemonClient};

use super::error::AxisError;
use super::gateway_client::{GatewayClient, ServerProfile};
use super::manifest::{self, SurfaceAvailability};
use crate::commands::daemon::PersistentDaemon;

pub enum Backend {
    Local(Arc<PersistentDaemon>),
    Remote(Arc<GatewayClient>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BackendStatusDto {
    pub mode: &'static str,
    pub url: Option<String>,
    pub profile: Option<ServerProfile>,
    pub surfaces: Vec<SurfaceAvailability>,
}

pub struct BackendState {
    local: Arc<PersistentDaemon>,
    remote: tokio::sync::RwLock<Option<Arc<GatewayClient>>>,
    remote_flag: AtomicBool,
    relay: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl BackendState {
    pub fn new(local: Arc<PersistentDaemon>) -> Self {
        Self {
            local,
            remote: tokio::sync::RwLock::new(None),
            remote_flag: AtomicBool::new(false),
            relay: std::sync::Mutex::new(None),
        }
    }

    pub fn local(&self) -> Arc<PersistentDaemon> {
        self.local.clone()
    }

    /// Synchronous mode check for the invoke gate and the local stream loops.
    pub fn is_remote(&self) -> bool {
        self.remote_flag.load(Ordering::Acquire)
    }

    pub async fn current(&self) -> Backend {
        match self.remote.read().await.as_ref() {
            Some(client) => Backend::Remote(client.clone()),
            None => Backend::Local(self.local.clone()),
        }
    }

    pub async fn set_remote(&self, client: Arc<GatewayClient>, relay: Option<tokio::task::JoinHandle<()>>) {
        *self.remote.write().await = Some(client);
        self.replace_relay(relay);
        self.remote_flag.store(true, Ordering::Release);
    }

    pub async fn set_local(&self) {
        self.remote_flag.store(false, Ordering::Release);
        self.replace_relay(None);
        *self.remote.write().await = None;
    }

    fn replace_relay(&self, next: Option<tokio::task::JoinHandle<()>>) {
        let previous = std::mem::replace(&mut *self.relay.lock().expect("relay mutex"), next);
        if let Some(task) = previous {
            task.abort();
        }
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, AxisError> {
        self.call_with_deadline(method, params, ORCH_CLIENT_READ_DEADLINE, None).await
    }

    pub async fn call_with_deadline(
        &self,
        method: &str,
        params: Value,
        deadline: Duration,
        permission_mode: Option<String>,
    ) -> Result<Value, AxisError> {
        match self.current().await {
            Backend::Local(daemon) => {
                let addr = daemon.ensure().await.map_err(AxisError::DaemonUnavailable)?;
                let mut client = match daemon.token().await {
                    Some(token) => OrchDaemonClient::with_token(addr, token),
                    None => OrchDaemonClient::new(addr),
                };
                if let Some(mode) = permission_mode {
                    client = client.with_permission_mode(mode);
                }
                client
                    .call_with_deadline(method, params, deadline)
                    .await
                    .map_err(|e| AxisError::Daemon(e.to_string()))
            }
            Backend::Remote(gateway) => gateway.call_daemon_method(method, params, deadline).await,
        }
    }

    pub async fn status(&self) -> BackendStatusDto {
        let backends = manifest::command_backends();
        let surfaces = manifest::registry_surfaces();
        match self.current().await {
            Backend::Local(_) => BackendStatusDto {
                mode: "local",
                url: None,
                profile: None,
                surfaces: manifest::surface_availability(backends, surfaces, None),
            },
            Backend::Remote(gateway) => {
                let profile = gateway.profile().clone();
                BackendStatusDto {
                    mode: "remote",
                    url: Some(gateway.base_url().to_string()),
                    surfaces: manifest::surface_availability(backends, surfaces, Some(&profile.axis_surfaces)),
                    profile: Some(profile),
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run the tests again.** Same command. Expected: 6 PASS.

- [ ] **Step 5: Route the daemon-class modules.** In each of `control_plane.rs`, `mission_control.rs` and `vcs_isolation.rs`:
  - Replace `use crate::commands::daemon::PersistentDaemon;` with `use crate::remote::backend::BackendState;`.
  - Replace the whole `async fn call_orchestrator_daemon(…) { … }` with:

    ```rust
    async fn call_orchestrator_daemon(
        backend: &BackendState,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        backend.call(method, params).await.map_err(String::from)
    }
    ```

  - Replace every `tauri::State<'_, Arc<PersistentDaemon>>` with `tauri::State<'_, Arc<BackendState>>` (StrReplace with `replace_all`). Before → after with `rg -c "State<'_, Arc<PersistentDaemon>>"`: control_plane 13 → 0, mission_control 3 → 0, vcs_isolation 2 → 0.
  - Any remaining `daemon: &PersistentDaemon` helper parameter becomes `daemon: &BackendState`.
  - Delete `use vox_orchestrator::orch_daemon::OrchDaemonClient;` when `rg -n OrchDaemonClient <file>` reports no other use.

In `mcp.rs`, replace the body of `invoke_mcp_tool` and its `daemon` parameter:

```rust
#[tauri::command]
pub async fn invoke_mcp_tool(
    tool: String,
    args: Value,
    permission_mode: Option<String>,
    daemon: tauri::State<'_, Arc<crate::remote::backend::BackendState>>,
) -> Result<Value, String> {
    let value = daemon
        .call_with_deadline(
            orch_daemon_method::TOOL_CALL,
            serde_json::json!({ "name": tool, "args": args }),
            deadline_for_tool(&tool),
            permission_mode,
        )
        .await
        .map_err(|e| match e {
            crate::remote::error::AxisError::Daemon(msg) => format!("MCP tool '{tool}' failed: {msg}"),
            other => String::from(other),
        })?;

    let is_error = value.get("success") == Some(&serde_json::Value::Bool(false));

    Ok(serde_json::json!({
        "tool": tool,
        "is_error": is_error,
        "result": value,
    }))
}
```

Remove the now-unused `use vox_orchestrator::orch_daemon::OrchDaemonClient;` from `mcp.rs`. Leave the `deadline_for_tool` tests untouched.

In `daemon.rs`, replace the two commands:

```rust
#[tauri::command]
pub async fn orchestrator_daemon_ready(
    state: tauri::State<'_, std::sync::Arc<crate::remote::backend::BackendState>>,
) -> Result<bool, String> {
    Ok(match state.current().await {
        crate::remote::backend::Backend::Local(daemon) => daemon.ensure_live().await.is_ok(),
        crate::remote::backend::Backend::Remote(_) => true,
    })
}

#[tauri::command]
pub async fn orchestrator_version_mismatch(
    state: tauri::State<'_, std::sync::Arc<crate::remote::backend::BackendState>>,
) -> Result<Option<VersionMismatch>, String> {
    Ok(match state.current().await {
        crate::remote::backend::Backend::Local(daemon) => daemon.last_version_mismatch.read().await.clone(),
        // Remote major.minor is enforced at connect time (`version_mismatch`).
        crate::remote::backend::Backend::Remote(_) => None,
    })
}
```

In `chat_turn.rs`:
- add the parameter `backend: State<'_, Arc<crate::remote::backend::BackendState>>,` after `daemon: State<'_, Arc<PersistentDaemon>>,` in `chat_turn`;
- change `Execution::Background => run_background(app_handle, input, daemon).await,` to `Execution::Background => run_background(app_handle, input, backend).await,`;
- in `run_background`, change the parameter to `backend: State<'_, Arc<crate::remote::backend::BackendState>>` and pass `backend` to `submit_orchestrator_task`.

- [ ] **Step 6: Suppress local streams while remote.**
  - In `orchestrator.rs`, add `backend: Arc<crate::remote::backend::BackendState>` as the last parameter of `spawn_orchestrator_status_stream` and `spawn_agent_event_stream`.
  - Make the first statement inside each `while let Some(…) = rx.recv().await {` loop:

    ```rust
                if backend.is_remote() {
                    continue;
                }
    ```

    The local subscription stays alive, so switching back to local resumes instantly. Remote agent events arrive through the Task 8 relay.

- [ ] **Step 7: Wire `main.rs`.**
  1. After the `mod remote;` line, add the macro:

     ```rust
     /// `tauri::generate_handler!` plus the (root, module, command) table the remote-mode gate needs.
     macro_rules! axis_invoke_handler {
         ($($root:ident :: $module:ident :: $cmd:ident),* $(,)?) => {{
             const COMMAND_MODULES: &remote::gate::CommandTable =
                 &[$((stringify!($root), stringify!($module), stringify!($cmd))),*];
             let inner = remote::gate::typed_handler(tauri::generate_handler![$($root::$module::$cmd),*]);
             move |invoke: tauri::ipc::Invoke<tauri::Wry>| {
                 remote::gate::gate_invoke(COMMAND_MODULES, invoke, &inner)
             }
         }};
     }
     ```

  2. Before `tauri::Builder::default()`, add `let persistent_daemon = std::sync::Arc::new(commands::daemon::PersistentDaemon::default());`. Replace the `.manage(std::sync::Arc::new(commands::daemon::PersistentDaemon::default(),))` call with:

     ```rust
             .manage(persistent_daemon.clone())
             .manage(std::sync::Arc::new(remote::backend::BackendState::new(
                 persistent_daemon,
             )))
     ```

  3. In `.setup`, after `let daemon = app.state::<…PersistentDaemon…>().inner().clone();` (the first one), add `let backend = app.state::<std::sync::Arc<remote::backend::BackendState>>().inner().clone();`. Pass `backend.clone()` as the new last argument to `spawn_orchestrator_status_stream` and `spawn_agent_event_stream`.
  4. Replace `.invoke_handler(tauri::generate_handler![` with `.invoke_handler(axis_invoke_handler![`. The list body and closing `])` are unchanged.

- [ ] **Step 8: Verify.**

```bash
cargo check -p vox-gui
cargo nextest run -p vox-gui
cargo clippy -p vox-gui --all-targets -- -D warnings
```

Expected: all pass, including the existing `mcp`, `control_plane`, `vcs_isolation`, `chat_turn` and `daemon` tests. That is the local back-compat evidence for the refactor. If `generate_handler!` rejects the forwarded paths, keep `tauri::generate_handler![…]` with the literal list and build `COMMAND_MODULES` from the same list with the macro's `stringify!` arm only. Do not hand-maintain a second list.

- [ ] **Step 9: Mutation check (security guard).** In `local_only_denial`, replace `Some(c @ (ModuleClass::Db | ModuleClass::Host)) => c.as_str(),` with `Some(ModuleClass::Db | ModuleClass::Host) => return None,`. Expected: `remote_mode_denies_db_and_host_commands` FAILS. Restore, then confirm with `rg -c 'Some\(c @ \(ModuleClass::Db \| ModuleClass::Host\)\) => c.as_str\(\)' crates/vox-gui/src/remote/gate.rs` (expected `1`) and rerun to green.

- [ ] **Step 10: Commit.**

```bash
vox run scripts/fmt.vox
git add crates/vox-gui/src/remote/ crates/vox-gui/src/main.rs crates/vox-gui/src/commands/control_plane.rs crates/vox-gui/src/commands/mission_control.rs crates/vox-gui/src/commands/vcs_isolation.rs crates/vox-gui/src/commands/mcp.rs crates/vox-gui/src/commands/daemon.rs crates/vox-gui/src/commands/chat_turn.rs crates/vox-gui/src/commands/orchestrator.rs
git commit -m "feat(gui): Backend enum, remote-mode invoke gate, daemon commands via BackendState"
```

---

### Task 8: Credentials, `axis_*` Tauri commands, remote agent-event relay

**Files:**
- Create: `crates/vox-gui/src/remote/credentials.rs`
- Replace stub: `crates/vox-gui/src/commands/axis_remote.rs`
- Modify: `crates/vox-gui/src/remote/mod.rs`, `crates/vox-gui/src/main.rs`

**Interfaces:**

```rust
// credentials.rs
pub const URL_KEY: &str = "axis_remote_url";
pub const PIN_KEY: &str = "axis_remote_tls_sha3_256";
pub fn token_registry_name(url: &Url) -> String;                                   // "axis-remote:<origin>"
pub fn pin_for(saved_url: Option<&str>, saved_pin: Option<&str>, url: &Url) -> Option<String>;
pub fn load_token(url: &Url) -> Option<String>; pub fn store_token(url: &Url, token: &str) -> Result<(), AxisError>;
pub fn saved_pin(url: &Url) -> Option<String>; pub fn save_server(url: &Url, pin: Option<&str>) -> Result<(), AxisError>;
// commands/axis_remote.rs
#[tauri::command] pub async fn axis_connect_remote<R: Runtime>(app: AppHandle<R>, url: String, token: Option<String>, accept_fingerprint: Option<String>, backend: State<'_, Arc<BackendState>>) -> Result<BackendStatusDto, String>;
#[tauri::command] pub async fn axis_disconnect_remote(backend: State<'_, Arc<BackendState>>) -> Result<BackendStatusDto, String>;
#[tauri::command] pub async fn axis_backend_status(backend: State<'_, Arc<BackendState>>) -> Result<BackendStatusDto, String>;
```

**Secret handling:**
- The bearer token is a per-server, dynamically keyed secret. It goes through the existing Clavis keyed store `vox_secrets::{set_registry_token, get_registry_token}`: the OS keyring with round-trip verification, falling back to `~/.vox/auth.json` with a warning when the keyring is unavailable.
- No `SecretId` is added: a fixed `SecretId` cannot name "the token for server X", and nothing reads an env var.
- The fingerprint and URL are not secret. They are stored as flat keys in `~/.vox/config.toml` through `vox_config::toml_config`.

**Parallel-safe with:** Tasks 9, 13.

- [ ] **Step 1: Write the failing tests.** `credentials.rs`:

```rust
//! Persistence for the one remembered Remote Axis server: bearer token in the OS keyring (Clavis
//! registry token keyed by origin), URL + pinned certificate fingerprint in `~/.vox/config.toml`.

use reqwest::Url;

use super::error::AxisError;

pub const URL_KEY: &str = "axis_remote_url";
pub const PIN_KEY: &str = "axis_remote_tls_sha3_256";

pub fn token_registry_name(url: &Url) -> String {
    let _ = url;
    todo!()
}

pub fn pin_for(saved_url: Option<&str>, saved_pin: Option<&str>, url: &Url) -> Option<String> {
    let _ = (saved_url, saved_pin, url);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn token_is_keyed_by_origin() {
        assert_eq!(token_registry_name(&url("https://vox.lan:3921/")), "axis-remote:https://vox.lan:3921");
        assert_eq!(token_registry_name(&url("https://vox.lan/")), "axis-remote:https://vox.lan");
    }

    #[test]
    fn pin_applies_only_to_the_saved_server() {
        let u = url("https://vox.lan:3921/");
        assert_eq!(pin_for(Some("https://vox.lan:3921/"), Some("ab"), &u), Some("ab".into()));
        assert_eq!(pin_for(Some("https://other.lan:3921/"), Some("ab"), &u), None);
        assert_eq!(pin_for(None, Some("ab"), &u), None);
        assert_eq!(pin_for(Some("https://vox.lan:3921/"), None, &u), None);
        assert_eq!(pin_for(Some("not a url"), Some("ab"), &u), None);
    }
}
```

`axis_remote.rs` (replaces the stub):

```rust
//! Remote Axis connect / disconnect / status commands and the remote agent-event relay.

use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::broadcast;

use crate::remote::backend::{BackendState, BackendStatusDto};
use crate::remote::credentials;
use crate::remote::error::AxisError;
use crate::remote::gateway_client::{ConnectRequest, GatewayClient, parse_server_url};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::daemon::PersistentDaemon;
    use tauri::Manager;

    fn app_with_backend() -> tauri::App<tauri::test::MockRuntime> {
        let app = tauri::test::mock_app();
        app.manage(Arc::new(BackendState::new(Arc::new(PersistentDaemon::default()))));
        app
    }

    #[tokio::test]
    async fn backend_status_starts_local() {
        let app = app_with_backend();
        let status = axis_backend_status(app.state()).await.expect("status");
        assert_eq!(status.mode, "local");
        assert!(status.surfaces.iter().all(|s| s.available));
    }

    #[tokio::test]
    async fn connect_rejects_plain_http_to_a_lan_host_before_any_io() {
        let app = app_with_backend();
        let err = axis_connect_remote(app.handle().clone(), "http://vox.lan:3921".into(), Some("t".into()), None, app.state())
            .await
            .unwrap_err();
        let v: Value = serde_json::from_str(&err).expect("json envelope");
        assert_eq!(v["error"]["code"], "insecure_url");
        assert!(!app.state::<Arc<BackendState>>().is_remote());
    }

    #[tokio::test]
    async fn disconnect_returns_to_local() {
        let app = app_with_backend();
        let status = axis_disconnect_remote(app.state()).await.expect("status");
        assert_eq!(status.mode, "local");
    }
}
```

Add `pub mod credentials;` to `remote/mod.rs`.

- [ ] **Step 2: Run the tests and watch them fail.**

```bash
cargo nextest run -p vox-gui remote::credentials commands::axis_remote
```

Expected: compile errors for the missing commands, or `not yet implemented`.

- [ ] **Step 3: Implement `credentials.rs`** (replace the stubs, keep the tests):

```rust
pub fn token_registry_name(url: &Url) -> String {
    format!("axis-remote:{}", url.origin().ascii_serialization())
}

pub fn pin_for(saved_url: Option<&str>, saved_pin: Option<&str>, url: &Url) -> Option<String> {
    let saved = Url::parse(saved_url?).ok()?;
    (saved.origin() == url.origin())
        .then(|| saved_pin.map(str::to_string))
        .flatten()
}

// toestub-ignore(skeleton/untested-pub-api) — thin wrapper over vox_secrets registry tokens (keyring round-trip covered by vox-secrets tests)
pub fn load_token(url: &Url) -> Option<String> {
    vox_secrets::get_registry_token(&token_registry_name(url))
}

// toestub-ignore(skeleton/untested-pub-api) — thin wrapper over vox_secrets registry tokens (keyring round-trip covered by vox-secrets tests)
pub fn store_token(url: &Url, token: &str) -> Result<(), AxisError> {
    vox_secrets::set_registry_token(&token_registry_name(url), token, None)
        .map(|_| ())
        .map_err(|e| AxisError::Remote { code: "secret_store".into(), message: e.to_string() })
}

// toestub-ignore(skeleton/untested-pub-api) — reads vox_config user config; decision logic is `pin_for` (tested)
pub fn saved_pin(url: &Url) -> Option<String> {
    let cfg = vox_config::toml_config::load_user_config();
    let get = |k: &str| cfg.values.get(k).and_then(|v| v.as_str()).map(str::to_string);
    pin_for(get(URL_KEY).as_deref(), get(PIN_KEY).as_deref(), url)
}

// toestub-ignore(skeleton/untested-pub-api) — writes vox_config user config (covered by vox-config tests)
pub fn save_server(url: &Url, pin: Option<&str>) -> Result<(), AxisError> {
    let io = |e: String| AxisError::Remote { code: "config_write".into(), message: e };
    vox_config::toml_config::set_user_config_value(URL_KEY, url.as_str()).map_err(io)?;
    match pin {
        Some(p) => vox_config::toml_config::set_user_config_value(PIN_KEY, p).map_err(io)?,
        None => {
            vox_config::toml_config::unset_user_config_value(PIN_KEY).map_err(io)?;
        }
    }
    Ok(())
}
```

**Implement `axis_remote.rs`** (above the tests):

```rust
#[tauri::command]
pub async fn axis_connect_remote<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    token: Option<String>,
    accept_fingerprint: Option<String>,
    backend: State<'_, Arc<BackendState>>,
) -> Result<BackendStatusDto, String> {
    let base = parse_server_url(&url)?;
    let token = token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .or_else(|| credentials::load_token(&base))
        .ok_or_else(|| AxisError::Unauthorized("enter the server's bearer token".into()))?;
    let expected_pin = accept_fingerprint
        .map(|f| f.trim().to_ascii_lowercase())
        .filter(|f| !f.is_empty())
        .or_else(|| credentials::saved_pin(&base));
    let (client, pin) = GatewayClient::connect(ConnectRequest {
        url: base.clone(),
        token: token.clone(),
        expected_pin,
    })
    .await?;
    credentials::store_token(&base, &token)?;
    credentials::save_server(&base, pin.as_deref())?;
    let client = Arc::new(client);
    let relay = spawn_agent_event_relay(app, client.subscribe_events());
    backend.set_remote(client, Some(relay)).await;
    Ok(backend.status().await)
}

#[tauri::command]
pub async fn axis_disconnect_remote(backend: State<'_, Arc<BackendState>>) -> Result<BackendStatusDto, String> {
    backend.set_local().await;
    Ok(backend.status().await)
}

#[tauri::command]
pub async fn axis_backend_status(backend: State<'_, Arc<BackendState>>) -> Result<BackendStatusDto, String> {
    Ok(backend.status().await)
}

fn spawn_agent_event_relay<R: Runtime>(
    app: AppHandle<R>,
    mut events: broadcast::Receiver<Value>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(mut frame) => {
                    let loggable = crate::commands::event_annotate::annotate_agent_event(&mut frame);
                    let _ = app.emit(crate::commands::orchestrator::AGENT_EVENTS_EVENT, frame);
                    if loggable {
                        let _ = app.emit(crate::commands::event_annotate::ACTIVITY_APPENDED_EVENT, ());
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}
```

In `main.rs`'s `axis_invoke_handler![…]` list, add after `commands::daemon::orchestrator_version_mismatch,`:

```rust
            commands::axis_remote::axis_connect_remote,
            commands::axis_remote::axis_disconnect_remote,
            commands::axis_remote::axis_backend_status,
```

- [ ] **Step 4: Run the tests again.** Same command. Expected: 5 PASS.

- [ ] **Step 5: Secret and storage guards.**

```bash
cargo run -q -p vox-cli -- ci secret-env-guard
cargo run -q -p vox-cli -- ci secrets-parity
cargo run -q -p vox-cli -- ci data-storage-guard
cargo run -q -p vox-cli -- ci gui-catalog-parity
```

Expected: all exit 0.

- [ ] **Step 6: Lint and commit.**

```bash
cargo clippy -p vox-gui --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-gui/src/remote/credentials.rs crates/vox-gui/src/remote/mod.rs crates/vox-gui/src/commands/axis_remote.rs crates/vox-gui/src/main.rs
git commit -m "feat(gui): axis_connect_remote / disconnect / backend_status with keyring token"
```

---

### Task 9: UI types, transport, error decoding, `BackendStatusProvider`

**Files:**
- Modify: `crates/vox-gui/ui/src/types/tauri.ts`, `crates/vox-gui/ui/src/transport.ts`, `crates/vox-gui/ui/src/main.tsx`
- Create: `crates/vox-gui/ui/src/lib/axisError.ts`, `crates/vox-gui/ui/src/lib/axisError.test.ts`
- Create: `crates/vox-gui/ui/src/hooks/useBackendStatus.tsx`, `crates/vox-gui/ui/src/hooks/useBackendStatus.test.tsx`

**Interfaces (TS):**

```ts
export type BackendMode = 'local' | 'remote';
export type SurfaceUnavailableReason = 'not_in_profile' | 'needs_local_machine' | 'not_remote_capable';
export interface SurfaceAvailability { surface: string; available: boolean; reason: SurfaceUnavailableReason | null }
export interface ServerProfile { profiles: string[]; max_exposure: 'public' | 'operator' | 'internal'; subsystems: { booted: string[]; failed: { id: string; error: string }[] }; tools: string[]; axis_surfaces: string[]; server_version: string }
export interface BackendStatus { mode: BackendMode; url: string | null; profile: ServerProfile | null; surfaces: SurfaceAvailability[] }
export interface AxisErrorPayload { code: string; message: string; fingerprint?: string; pinned?: string; server?: string; gui?: string; command?: string; module?: string; class?: string; method?: string }
// transport.ts
export function axisBackendStatus(): Promise<BackendStatus>;
export function axisConnectRemote(url: string, token: string | null, acceptFingerprint: string | null): Promise<BackendStatus>;
export function axisDisconnectRemote(): Promise<BackendStatus>;
// lib/axisError.ts
export function parseAxisError(err: unknown): AxisErrorPayload | null;
// hooks/useBackendStatus.tsx
export const LOCAL_BACKEND_STATUS: BackendStatus;
export function BackendStatusProvider(props: { children: React.ReactNode }): JSX.Element;
export function StaticBackendStatusProvider(props: { status: BackendStatus; children: React.ReactNode }): JSX.Element;
export function useBackendStatus(): { status: BackendStatus; setStatus: (s: BackendStatus) => void; refresh: () => Promise<void> };
export function surfaceAvailability(status: BackendStatus, viewKey: string): SurfaceAvailability;
```

**Parallel-safe with:** Tasks 3–8. The DTO shapes are fixed by this plan.

- [ ] **Step 1: Types.** Append to `crates/vox-gui/ui/src/types/tauri.ts`:

```ts
// ── Remote Axis (plan 06) — mirrors crates/vox-gui/src/remote/{backend,manifest,gateway_client,error}.rs ──

export type BackendMode = 'local' | 'remote';

export type SurfaceUnavailableReason = 'not_in_profile' | 'needs_local_machine' | 'not_remote_capable';

export interface SurfaceAvailability {
  surface: string;
  available: boolean;
  reason: SurfaceUnavailableReason | null;
}

/** `GET /v1/profile` as served by the gateway (00-contracts §Wire shapes). */
export interface ServerProfile {
  profiles: string[];
  max_exposure: 'public' | 'operator' | 'internal';
  subsystems: { booted: string[]; failed: { id: string; error: string }[] };
  tools: string[];
  axis_surfaces: string[];
  server_version: string;
}

export interface BackendStatus {
  mode: BackendMode;
  url: string | null;
  profile: ServerProfile | null;
  surfaces: SurfaceAvailability[];
}

/** `{"error":{...}}` envelope carried by rejected Axis remote commands. */
export interface AxisErrorPayload {
  code: string;
  message: string;
  fingerprint?: string;
  pinned?: string;
  server?: string;
  gui?: string;
  command?: string;
  module?: string;
  class?: string;
  method?: string;
}
```

- [ ] **Step 2: Write the failing tests.** `crates/vox-gui/ui/src/lib/axisError.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { parseAxisError } from './axisError';

describe('parseAxisError', () => {
  const envelope = JSON.stringify({ error: { code: 'tls_pin_required', message: 'first', fingerprint: 'ab' } });

  it('decodes a rejected string envelope', () => {
    expect(parseAxisError(envelope)).toEqual({ code: 'tls_pin_required', message: 'first', fingerprint: 'ab' });
  });

  it('decodes an Error whose message is the envelope', () => {
    expect(parseAxisError(new Error(envelope))?.code).toBe('tls_pin_required');
  });

  it('returns null for plain strings and non-envelope JSON', () => {
    expect(parseAxisError('daemon unreachable')).toBeNull();
    expect(parseAxisError(JSON.stringify({ ok: true }))).toBeNull();
    expect(parseAxisError(undefined)).toBeNull();
  });
});
```

`crates/vox-gui/ui/src/hooks/useBackendStatus.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, waitFor, cleanup } from '@testing-library/react';
import React from 'react';
import type { BackendStatus } from '../types/tauri';

const axisBackendStatus = vi.fn();
vi.mock('../transport', () => ({ axisBackendStatus: () => axisBackendStatus() }));

import { BackendStatusProvider, LOCAL_BACKEND_STATUS, surfaceAvailability, useBackendStatus } from './useBackendStatus';

const REMOTE: BackendStatus = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: null,
  surfaces: [
    { surface: 'mesh', available: true, reason: null },
    { surface: 'console', available: false, reason: 'needs_local_machine' },
  ],
};

function Probe() {
  const { status } = useBackendStatus();
  return <span data-testid="mode">{status.mode}</span>;
}

afterEach(() => {
  cleanup();
  axisBackendStatus.mockReset();
});

describe('useBackendStatus', () => {
  it('defaults to local without a provider', () => {
    render(<Probe />);
    expect(screen.getByTestId('mode').textContent).toBe('local');
  });

  it('loads the backend status on mount', async () => {
    axisBackendStatus.mockResolvedValue(REMOTE);
    render(<BackendStatusProvider><Probe /></BackendStatusProvider>);
    await waitFor(() => expect(screen.getByTestId('mode').textContent).toBe('remote'));
  });

  it('falls back to local when the command is missing or returns garbage', async () => {
    axisBackendStatus.mockResolvedValue(null);
    render(<BackendStatusProvider><Probe /></BackendStatusProvider>);
    await waitFor(() => expect(axisBackendStatus).toHaveBeenCalled());
    expect(screen.getByTestId('mode').textContent).toBe('local');
    cleanup();
    axisBackendStatus.mockRejectedValue(new Error('unknown command'));
    render(<BackendStatusProvider><Probe /></BackendStatusProvider>);
    await waitFor(() => expect(axisBackendStatus).toHaveBeenCalledTimes(2));
    expect(screen.getByTestId('mode').textContent).toBe('local');
  });
});

describe('surfaceAvailability', () => {
  it('is always available in local mode', () => {
    expect(surfaceAvailability(LOCAL_BACKEND_STATUS, 'console')).toEqual({ surface: 'console', available: true, reason: null });
  });

  it('uses the server-computed entry in remote mode', () => {
    expect(surfaceAvailability(REMOTE, 'console').reason).toBe('needs_local_machine');
    expect(surfaceAvailability(REMOTE, 'mesh').available).toBe(true);
  });

  it('treats an unknown view as not in profile when remote', () => {
    expect(surfaceAvailability(REMOTE, 'mystery').reason).toBe('not_in_profile');
  });
});
```

- [ ] **Step 3: Run the tests and watch them fail.**

```bash
pnpm --dir crates/vox-gui/ui exec vitest run src/lib/axisError.test.ts src/hooks/useBackendStatus.test.tsx
```

Expected: FAIL (modules not found).

- [ ] **Step 4: Implement.** `crates/vox-gui/ui/src/lib/axisError.ts`:

```ts
import type { AxisErrorPayload } from '../types/tauri';

/** Decode the `{"error":{code,message,...}}` envelope Axis remote commands reject with; null otherwise. */
export function parseAxisError(err: unknown): AxisErrorPayload | null {
  const raw = typeof err === 'string' ? err : err instanceof Error ? err.message : null;
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as { error?: AxisErrorPayload };
    return parsed?.error && typeof parsed.error.code === 'string' ? parsed.error : null;
  } catch {
    return null;
  }
}
```

`crates/vox-gui/ui/src/hooks/useBackendStatus.tsx`:

```tsx
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { axisBackendStatus } from '../transport';
import type { BackendStatus, SurfaceAvailability } from '../types/tauri';

export const LOCAL_BACKEND_STATUS: BackendStatus = { mode: 'local', url: null, profile: null, surfaces: [] };

interface BackendStatusContextValue {
  status: BackendStatus;
  setStatus: (status: BackendStatus) => void;
  refresh: () => Promise<void>;
}

const BackendStatusContext = createContext<BackendStatusContextValue>({
  status: LOCAL_BACKEND_STATUS,
  setStatus: () => {},
  refresh: async () => {},
});

function isBackendStatus(value: unknown): value is BackendStatus {
  const v = value as BackendStatus | null;
  return !!v && (v.mode === 'local' || v.mode === 'remote') && Array.isArray(v.surfaces);
}

export function BackendStatusProvider({ children }: { children: React.ReactNode }) {
  const [status, setStatus] = useState<BackendStatus>(LOCAL_BACKEND_STATUS);
  const refresh = useCallback(async () => {
    try {
      const next = await axisBackendStatus();
      setStatus(isBackendStatus(next) ? next : LOCAL_BACKEND_STATUS);
    } catch {
      setStatus(LOCAL_BACKEND_STATUS);
    }
  }, []);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  const value = useMemo(() => ({ status, setStatus, refresh }), [status, refresh]);
  return <BackendStatusContext.Provider value={value}>{children}</BackendStatusContext.Provider>;
}

/** Fixed status, for tests and previews. */
export function StaticBackendStatusProvider({ status, children }: { status: BackendStatus; children: React.ReactNode }) {
  const value = useMemo(() => ({ status, setStatus: () => {}, refresh: async () => {} }), [status]);
  return <BackendStatusContext.Provider value={value}>{children}</BackendStatusContext.Provider>;
}

export function useBackendStatus(): BackendStatusContextValue {
  return useContext(BackendStatusContext);
}

export function surfaceAvailability(status: BackendStatus, viewKey: string): SurfaceAvailability {
  if (status.mode === 'local') return { surface: viewKey, available: true, reason: null };
  return (
    status.surfaces.find((s) => s.surface === viewKey) ?? { surface: viewKey, available: false, reason: 'not_in_profile' }
  );
}
```

`transport.ts` — add `import type { BackendStatus } from './types/tauri';` next to the existing `./types/tauri` re-export, and append after `getAutoModelRecommendation`:

```ts
/** Remote Axis: current backend (local daemon or remote gateway) and per-surface availability. */
export function axisBackendStatus(): Promise<BackendStatus> {
  return safeInvoke<BackendStatus>('axis_backend_status');
}

/** Connect to a `vox serve` gateway. Rejects with an Axis error envelope (`tls_pin_required`, …). */
export function axisConnectRemote(
  url: string,
  token: string | null,
  acceptFingerprint: string | null,
): Promise<BackendStatus> {
  return safeInvoke<BackendStatus>('axis_connect_remote', { url, token, acceptFingerprint });
}

export function axisDisconnectRemote(): Promise<BackendStatus> {
  return safeInvoke<BackendStatus>('axis_disconnect_remote');
}
```

`main.tsx` — add `import { BackendStatusProvider } from './hooks/useBackendStatus'` and wrap `<App />`:

```tsx
        <QueryClientProvider client={queryClient}>
          <BackendStatusProvider>
            <App />
          </BackendStatusProvider>
        </QueryClientProvider>
```

- [ ] **Step 5: Run the tests and typecheck.**

```bash
pnpm --dir crates/vox-gui/ui exec vitest run src/lib/axisError.test.ts src/hooks/useBackendStatus.test.tsx src/guards
pnpm --dir crates/vox-gui/ui typecheck
```

Expected: all PASS, including `ipcBoundaries` and `transportIpcGuard`. Typecheck exits 0.

- [ ] **Step 6: Commit.**

```bash
git add crates/vox-gui/ui/src/types/tauri.ts crates/vox-gui/ui/src/transport.ts crates/vox-gui/ui/src/main.tsx crates/vox-gui/ui/src/lib/axisError.ts crates/vox-gui/ui/src/lib/axisError.test.ts crates/vox-gui/ui/src/hooks/useBackendStatus.tsx crates/vox-gui/ui/src/hooks/useBackendStatus.test.tsx
git commit -m "feat(gui-ui): backend status context and Remote Axis transport calls"
```

---

### Task 10: Surface gating — `SurfaceGate`, Sidebar, BreadcrumbBar, status pill

**Files:**
- Create: `crates/vox-gui/ui/src/components/layout/SurfaceUnavailable.tsx` (+ `.test.tsx`)
- Create: `crates/vox-gui/ui/src/components/layout/BackendStatusPill.tsx` (+ `.test.tsx`)
- Modify: `crates/vox-gui/ui/src/components/layout/{surfaceComponents.tsx,Sidebar.tsx,BreadcrumbBar.tsx}`

**Interfaces:**

```ts
export const UNAVAILABLE_COPY: Record<SurfaceUnavailableReason, { title: string; description: string }>;
export const UNAVAILABLE_REASON_LABEL: Record<SurfaceUnavailableReason, string>;
export function SurfaceUnavailable(props: { viewKey: string; reason: SurfaceUnavailableReason }): JSX.Element;
export function SurfaceGate(props: { viewKey: string; children: React.ReactNode }): JSX.Element;
export function BackendStatusPill(props: { collapsed?: boolean }): JSX.Element | null;
```

**Parallel-safe with:** Task 13.

- [ ] **Step 1: Write the failing tests.** `SurfaceUnavailable.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { SurfaceGate } from './SurfaceUnavailable';
import { LOCAL_BACKEND_STATUS, StaticBackendStatusProvider } from '../../hooks/useBackendStatus';
import type { BackendStatus } from '../../types/tauri';

const REMOTE: BackendStatus = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: null,
  surfaces: [
    { surface: 'mesh', available: true, reason: null },
    { surface: 'console', available: false, reason: 'needs_local_machine' },
    { surface: 'models', available: false, reason: 'not_remote_capable' },
    { surface: 'tasks', available: false, reason: 'not_in_profile' },
  ],
};

afterEach(cleanup);

function gate(status: BackendStatus, viewKey: string) {
  return render(
    <StaticBackendStatusProvider status={status}>
      <SurfaceGate viewKey={viewKey}><div data-testid="surface-body">body</div></SurfaceGate>
    </StaticBackendStatusProvider>,
  );
}

describe('SurfaceGate', () => {
  it('renders the surface in local mode', () => {
    gate(LOCAL_BACKEND_STATUS, 'console');
    expect(screen.getByTestId('surface-body')).toBeTruthy();
  });

  it('renders an available remote surface', () => {
    gate(REMOTE, 'mesh');
    expect(screen.getByTestId('surface-body')).toBeTruthy();
  });

  it.each([
    ['console', 'needs_local_machine', /files, processes, or settings on this machine/i],
    ['models', 'not_remote_capable', /cannot run against a remote server yet/i],
    ['tasks', 'not_in_profile', /profile does not include/i],
  ])('names the reason for %s', (viewKey, reason, text) => {
    gate(REMOTE, viewKey);
    const node = screen.getByTestId('surface-unavailable');
    expect(node.getAttribute('data-reason')).toBe(reason);
    expect(screen.getByText(/not available on this server/i)).toBeTruthy();
    expect(screen.getByText(text)).toBeTruthy();
    expect(screen.queryByTestId('surface-body')).toBeNull();
  });
});
```

`BackendStatusPill.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import React from 'react';
import { BackendStatusPill } from './BackendStatusPill';
import { LOCAL_BACKEND_STATUS, StaticBackendStatusProvider } from '../../hooks/useBackendStatus';

afterEach(cleanup);

describe('BackendStatusPill', () => {
  it('renders nothing in local mode', () => {
    const { container } = render(
      <StaticBackendStatusProvider status={LOCAL_BACKEND_STATUS}><BackendStatusPill /></StaticBackendStatusProvider>,
    );
    expect(container.innerHTML).toBe('');
  });

  it('names the host and profile when remote', () => {
    render(
      <StaticBackendStatusProvider
        status={{
          mode: 'remote',
          url: 'https://vox.lan:3921/',
          profile: { profiles: ['mesh'], max_exposure: 'operator', subsystems: { booted: [], failed: [] }, tools: [], axis_surfaces: ['mesh'], server_version: '0.6.0' },
          surfaces: [],
        }}
      >
        <BackendStatusPill />
      </StaticBackendStatusProvider>,
    );
    const pill = screen.getByTestId('backend-status-pill');
    expect(pill.getAttribute('aria-label')).toBe('Connected to vox.lan:3921, profile mesh');
    expect(pill.textContent).toContain('vox.lan:3921');
  });
});
```

Append to the existing `Sidebar.test.tsx` (it already mocks `invoke` and the surface registry, and imports `LanguageProvider` and `baseProps`):

```tsx
import { StaticBackendStatusProvider } from '../../hooks/useBackendStatus';

describe('Sidebar remote gating', () => {
  it('marks unavailable child surfaces with their reason', () => {
    render(
      <LanguageProvider>
        <StaticBackendStatusProvider
          status={{ mode: 'remote', url: 'https://vox.lan:3921/', profile: null, surfaces: [{ surface: 'dashboard', available: true, reason: null }] }}
        >
          <Sidebar {...baseProps} mode="wide" view="dashboard" />
        </StaticBackendStatusProvider>
      </LanguageProvider>,
    );
    const flow = screen.getAllByRole('button').find((b) => b.getAttribute('data-unavailable') === 'not_in_profile');
    expect(flow).toBeTruthy();
    expect(flow?.getAttribute('title')).toMatch(/not available on this server/i);
  });
});
```

Append to `BreadcrumbBar.test.tsx`:

```tsx
import { StaticBackendStatusProvider } from '../../hooks/useBackendStatus';

describe('BreadcrumbBar remote gating', () => {
  it('appends the unavailability reason for the current view', () => {
    render(
      <StaticBackendStatusProvider
        status={{ mode: 'remote', url: 'https://vox.lan:3921/', profile: null, surfaces: [{ surface: 'models', available: false, reason: 'not_remote_capable' }] }}
      >
        <BreadcrumbBar viewKey="models" />
      </StaticBackendStatusProvider>,
    );
    expect(screen.getByTestId('breadcrumb-unavailable').textContent).toContain('not remote-capable yet');
  });
});
```

If `BreadcrumbBar.test.tsx` wraps renders in `LanguageProvider`, wrap the new render the same way.

- [ ] **Step 2: Run the tests and watch them fail.**

```bash
pnpm --dir crates/vox-gui/ui exec vitest run src/components/layout/SurfaceUnavailable.test.tsx src/components/layout/BackendStatusPill.test.tsx src/components/layout/Sidebar.test.tsx src/components/layout/BreadcrumbBar.test.tsx
```

Expected: the new tests FAIL; the existing Sidebar/BreadcrumbBar tests still PASS.

- [ ] **Step 3: Implement.** `SurfaceUnavailable.tsx`:

```tsx
import React from 'react';
import { EmptyState } from '../ui/EmptyState';
import { surfaceAvailability, useBackendStatus } from '../../hooks/useBackendStatus';
import type { SurfaceUnavailableReason } from '../../types/tauri';

export const UNAVAILABLE_COPY: Record<SurfaceUnavailableReason, { title: string; description: string }> = {
  not_in_profile: {
    title: 'Not available on this server',
    description: "This server's profile does not include this surface.",
  },
  needs_local_machine: {
    title: 'Not available on this server',
    description: 'This surface works on files, processes, or settings on this machine. Disconnect from the server to use it.',
  },
  not_remote_capable: {
    title: 'Not available on this server',
    description: 'This surface cannot run against a remote server yet.',
  },
};

export const UNAVAILABLE_REASON_LABEL: Record<SurfaceUnavailableReason, string> = {
  not_in_profile: 'not in server profile',
  needs_local_machine: 'needs this machine',
  not_remote_capable: 'not remote-capable yet',
};

export function SurfaceUnavailable({ viewKey, reason }: { viewKey: string; reason: SurfaceUnavailableReason }) {
  const copy = UNAVAILABLE_COPY[reason];
  return (
    <div data-testid="surface-unavailable" data-reason={reason} data-view={viewKey} className="p-6">
      <EmptyState variant="no-connection" title={copy.title} description={copy.description} headingLevel={2} />
    </div>
  );
}

export function SurfaceGate({ viewKey, children }: { viewKey: string; children: React.ReactNode }) {
  const { status } = useBackendStatus();
  const availability = surfaceAvailability(status, viewKey);
  if (availability.available || !availability.reason) return <>{children}</>;
  return <SurfaceUnavailable viewKey={viewKey} reason={availability.reason} />;
}
```

`BackendStatusPill.tsx`:

```tsx
import React from 'react';
import { useBackendStatus } from '../../hooks/useBackendStatus';

export function BackendStatusPill({ collapsed = false }: { collapsed?: boolean }) {
  const { status } = useBackendStatus();
  if (status.mode === 'local') return null;
  let host = 'server';
  try {
    if (status.url) host = new URL(status.url).host;
  } catch {
    host = status.url ?? 'server';
  }
  const profiles = status.profile?.profiles.join(', ') ?? '';
  const label = `Connected to ${host}${profiles ? `, profile ${profiles}` : ''}`;
  return (
    <div
      data-testid="backend-status-pill"
      role="status"
      aria-label={label}
      title={label}
      className={`flex items-center ${collapsed ? 'justify-center' : 'gap-1.5 px-2'} rounded-md border border-accent-secondary/30 bg-accent-secondary/8 py-1 font-mono text-[10px] text-text-secondary`}
    >
      <span className="size-1.5 shrink-0 rounded-full bg-accent-secondary" aria-hidden="true" />
      {!collapsed && <span className="truncate">{host}</span>}
      {!collapsed && profiles && <span className="truncate text-text-muted">· {profiles}</span>}
    </div>
  );
}
```

`surfaceComponents.tsx` — add `import { SurfaceGate } from './SurfaceUnavailable';` and replace the body of `renderSurfaceContent`:

```tsx
export function renderSurfaceContent(viewKey: string, props: SurfaceProps): React.ReactNode {
  return <SurfaceGate viewKey={viewKey}>{childRenderer(props, viewKey)}</SurfaceGate>;
}
```

`Sidebar.tsx`:
1. Add imports:

   ```tsx
   import { surfaceAvailability, useBackendStatus } from '../../hooks/useBackendStatus';
   import { BackendStatusPill } from './BackendStatusPill';
   import { UNAVAILABLE_COPY } from './SurfaceUnavailable';
   ```

2. Inside `Sidebar`, after `const { lang } = useLang();`, add `const { status: backendStatus } = useBackendStatus();`.
3. Replace the child button block inside `children.map(childKey => (` with:

   ```tsx
                    {children.map(childKey => {
                      const availability = surfaceAvailability(backendStatus, childKey);
                      return (
                        <button
                          key={childKey}
                          type="button"
                          onClick={() => onOpenTab(childKey)}
                          aria-current={view === childKey ? 'page' : undefined}
                          data-unavailable={availability.available ? undefined : availability.reason ?? undefined}
                          title={availability.available || !availability.reason ? undefined : UNAVAILABLE_COPY[availability.reason].title}
                          className={`w-full rounded-lg px-2 py-1.5 text-left font-display text-[11px] tracking-widest uppercase transition ${
                            view === childKey
                              ? 'bg-brass/10 text-brass'
                              : 'text-text-muted hover:bg-overlay-hover hover:text-text-secondary'
                          } ${availability.available ? '' : 'opacity-50'}`}
                        >
                          {childLabelFor(childKey, lang)}
                        </button>
                      );
                    })}
   ```

4. Directly inside `<div className="flex flex-col gap-2 pt-3 shrink-0">` (the footer), add `<BackendStatusPill collapsed={collapsed} />` as the first child.

`BreadcrumbBar.tsx`:
1. Add imports:

   ```tsx
   import { surfaceAvailability, useBackendStatus } from '../../hooks/useBackendStatus';
   import { UNAVAILABLE_REASON_LABEL } from './SurfaceUnavailable';
   ```

2. Before `if (segments.length === 0 || viewKey === 'chat') return null;`, add:

   ```tsx
   const { status } = useBackendStatus();
   const availability = surfaceAvailability(status, viewKey);
   ```

3. Inside `<nav aria-label="Breadcrumb" …>`, after the `segments.map(…)` block, add:

   ```tsx
         {!availability.available && availability.reason && (
           <span data-testid="breadcrumb-unavailable" className="font-display text-[10px] uppercase tracking-[0.14em] text-amber-300/80">
             · {UNAVAILABLE_REASON_LABEL[availability.reason]}
           </span>
         )}
   ```

- [ ] **Step 4: Run the tests and typecheck.**

```bash
pnpm --dir crates/vox-gui/ui exec vitest run src/components/layout
pnpm --dir crates/vox-gui/ui typecheck
```

Expected: all `layout` tests PASS (existing + new). Typecheck exits 0.

- [ ] **Step 5: Commit.**

```bash
git add crates/vox-gui/ui/src/components/layout/
git commit -m "feat(gui-ui): gate surfaces by server profile with typed unavailable states"
```

---

### Task 11: Settings "Server" section (connect, TOFU confirm, disconnect) + local-machine sections locked when remote

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Settings/ServerConnectionSection.tsx` (+ `.test.tsx`)
- Create: `crates/vox-gui/ui/src/components/surfaces/Settings/settingsBackends.ts` (+ `.test.ts`)
- Modify: `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`, `settingsIndex.ts`

**Interfaces:**

```ts
export function formatFingerprint(fp: string): string;
export function ServerConnectionSection(props: { pushToast: (t: Toast) => void }): JSX.Element;
export const LOCAL_MACHINE_SECTIONS: ReadonlySet<string>;
export const AXIS_LOCAL_SECTIONS: ReadonlySet<string>;
export function sectionLockedRemotely(sectionId: string, mode: BackendMode): boolean;
```

**Parallel-safe with:** Task 13.

- [ ] **Step 1: Write the failing tests.** `settingsBackends.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { AXIS_LOCAL_SECTIONS, LOCAL_MACHINE_SECTIONS, sectionLockedRemotely } from './settingsBackends';
import { SECTIONS } from './SettingsView';

describe('settingsBackends', () => {
  it('classifies every Settings section exactly once', () => {
    for (const s of SECTIONS) {
      const inLocal = LOCAL_MACHINE_SECTIONS.has(s.id);
      const inAxis = AXIS_LOCAL_SECTIONS.has(s.id);
      expect(inLocal !== inAxis, `section ${s.id}`).toBe(true);
    }
  });

  it('locks local-machine sections only when remote', () => {
    expect(sectionLockedRemotely('secrets', 'remote')).toBe(true);
    expect(sectionLockedRemotely('secrets', 'local')).toBe(false);
    expect(sectionLockedRemotely('server', 'remote')).toBe(false);
    expect(sectionLockedRemotely('theme', 'remote')).toBe(false);
  });
});
```

`ServerConnectionSection.test.tsx`:

```tsx
// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import React from 'react';
import type { BackendStatus } from '../../../types/tauri';

const axisConnectRemote = vi.fn();
const axisDisconnectRemote = vi.fn();
vi.mock('../../../transport', () => ({
  axisConnectRemote: (...a: unknown[]) => axisConnectRemote(...a),
  axisDisconnectRemote: () => axisDisconnectRemote(),
  axisBackendStatus: () => Promise.resolve({ mode: 'local', url: null, profile: null, surfaces: [] }),
}));

import { ServerConnectionSection, formatFingerprint } from './ServerConnectionSection';
import { BackendStatusProvider } from '../../../hooks/useBackendStatus';

const FP = 'ab'.repeat(32);
const REMOTE: BackendStatus = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: { profiles: ['mesh'], max_exposure: 'operator', subsystems: { booted: ['mesh'], failed: [] }, tools: [], axis_surfaces: ['mesh'], server_version: '0.6.0' },
  surfaces: [],
};

afterEach(() => {
  cleanup();
  axisConnectRemote.mockReset();
  axisDisconnectRemote.mockReset();
});

function renderSection() {
  const pushToast = vi.fn();
  render(<BackendStatusProvider><ServerConnectionSection pushToast={pushToast} /></BackendStatusProvider>);
  fireEvent.change(screen.getByLabelText(/server url/i), { target: { value: 'https://vox.lan:3921' } });
  fireEvent.change(screen.getByLabelText(/bearer token/i), { target: { value: 'tok' } });
  return pushToast;
}

describe('ServerConnectionSection', () => {
  it('groups fingerprints in fours', () => {
    expect(formatFingerprint('abcdef012345')).toBe('abcd ef01 2345');
  });

  it('asks to confirm the fingerprint on first use, then connects with it', async () => {
    axisConnectRemote
      .mockRejectedValueOnce(JSON.stringify({ error: { code: 'tls_pin_required', message: 'first', fingerprint: FP } }))
      .mockResolvedValueOnce(REMOTE);
    const pushToast = renderSection();
    fireEvent.click(screen.getByRole('button', { name: /^connect$/i }));
    const fp = await screen.findByTestId('server-fingerprint');
    expect(fp.textContent).toContain('abab abab');
    expect(screen.getByText(/first connection to this server/i)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /trust and connect/i }));
    await waitFor(() => expect(screen.getByTestId('server-connected')).toBeTruthy());
    expect(axisConnectRemote).toHaveBeenLastCalledWith('https://vox.lan:3921', 'tok', FP);
    expect(pushToast).toHaveBeenCalledWith(expect.objectContaining({ tone: 'ok', cause: 'backend-ok' }));
  });

  it('warns when the pinned certificate changed', async () => {
    axisConnectRemote.mockRejectedValueOnce(
      JSON.stringify({ error: { code: 'tls_pin_mismatch', message: 'changed', pinned: 'cd'.repeat(32), fingerprint: FP } }),
    );
    renderSection();
    fireEvent.click(screen.getByRole('button', { name: /^connect$/i }));
    expect(await screen.findByText(/certificate changed since you last connected/i)).toBeTruthy();
    expect(screen.getByText(/previously pinned/i).textContent).toContain('cdcd cdcd');
  });

  it('shows the version mismatch message', async () => {
    axisConnectRemote.mockRejectedValueOnce(
      JSON.stringify({ error: { code: 'version_mismatch', message: 'server runs Vox 0.7.0, Axis is 0.6.0; major.minor must match' } }),
    );
    renderSection();
    fireEvent.click(screen.getByRole('button', { name: /^connect$/i }));
    expect((await screen.findByRole('alert')).textContent).toContain('major.minor must match');
  });
});
```

- [ ] **Step 2: Run the tests and watch them fail.**

```bash
pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Settings/settingsBackends.test.ts src/components/surfaces/Settings/ServerConnectionSection.test.tsx
```

Expected: FAIL (modules missing; `SECTIONS` not exported).

- [ ] **Step 3: Implement.** `settingsBackends.ts`:

```ts
import type { BackendMode } from '../../../types/tauri';

/** Sections backed by host/db-class modules: they configure this machine's local Vox. */
export const LOCAL_MACHINE_SECTIONS: ReadonlySet<string> = new Set([
  'orchestrator', 'scaling', 'llm', 'routing', 'runtime', 'voice', 'mesh', 'signing', 'secrets', 'onboarding', 'gamify',
]);

/** Sections that configure this Axis install itself (shell-class); usable in both modes. */
export const AXIS_LOCAL_SECTIONS: ReadonlySet<string> = new Set(['server', 'telemetry', 'keybinds', 'theme', 'display']);

export function sectionLockedRemotely(sectionId: string, mode: BackendMode): boolean {
  return mode === 'remote' && LOCAL_MACHINE_SECTIONS.has(sectionId);
}
```

`ServerConnectionSection.tsx`:

```tsx
import React, { useState } from 'react';
import { axisConnectRemote, axisDisconnectRemote } from '../../../transport';
import { parseAxisError } from '../../../lib/axisError';
import { useBackendStatus } from '../../../hooks/useBackendStatus';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../../ui/Dialog';
import type { Toast } from '../../../types/tauri';

interface PendingPin {
  fingerprint: string;
  pinned: string | null;
}

export function formatFingerprint(fp: string): string {
  return (fp.match(/.{1,4}/g) ?? []).join(' ');
}

const INPUT_CLASS =
  'rounded-md border border-border-subtle bg-black/20 px-2 py-1.5 font-mono text-[12px] text-text-primary focus:outline-hidden focus:ring-1 focus:ring-brass/50';
const BUTTON_CLASS =
  'self-start rounded-md border border-brass/40 bg-brass/10 px-3 py-1.5 font-display text-[11px] uppercase tracking-[0.12em] text-brass hover:bg-brass/20 disabled:opacity-40';

export function ServerConnectionSection({ pushToast }: { pushToast: (t: Toast) => void }) {
  const { status, setStatus } = useBackendStatus();
  const [url, setUrl] = useState(status.url ?? '');
  const [token, setToken] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingPin | null>(null);

  const connect = async (acceptFingerprint: string | null) => {
    setBusy(true);
    setError(null);
    try {
      const next = await axisConnectRemote(url.trim(), token.trim() || null, acceptFingerprint);
      setPending(null);
      setToken('');
      setStatus(next);
      pushToast({ tone: 'ok', title: 'Connected to server', body: next.url ?? undefined, cause: 'backend-ok' });
    } catch (e) {
      const err = parseAxisError(e);
      if (err?.code === 'tls_pin_required' && err.fingerprint) {
        setPending({ fingerprint: err.fingerprint, pinned: null });
      } else if (err?.code === 'tls_pin_mismatch' && err.fingerprint) {
        setPending({ fingerprint: err.fingerprint, pinned: err.pinned ?? null });
      } else {
        setPending(null);
        setError(err?.message ?? String(e));
      }
    } finally {
      setBusy(false);
    }
  };

  const disconnect = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await axisDisconnectRemote());
      pushToast({ tone: 'info', title: 'Disconnected — using this machine', cause: 'backend-ok' });
    } catch (e) {
      setError(parseAxisError(e)?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-labelledby="server-connection-heading" data-testid="server-connection">
      <h2 id="server-connection-heading" className="font-display text-[18px] font-semibold tracking-tight text-text-primary">
        Connect to server
      </h2>
      <p className="mt-0.5 text-[11px] text-text-muted">
        Attach Axis to a Vox server on your network. Surfaces the server does not serve stay hidden.
      </p>

      {status.mode === 'remote' ? (
        <div className="mt-4 flex flex-col gap-2" data-testid="server-connected">
          <div className="text-[12px] text-text-secondary">
            Connected to <span className="font-mono">{status.url}</span>
          </div>
          <div className="text-[11px] text-text-muted">
            Profile: {status.profile?.profiles.join(', ') || 'unknown'} · server v{status.profile?.server_version ?? '?'}
          </div>
          <button type="button" className={BUTTON_CLASS} onClick={() => void disconnect()} disabled={busy}>
            Disconnect
          </button>
        </div>
      ) : (
        <form
          className="mt-4 flex max-w-md flex-col gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            void connect(null);
          }}
        >
          <label className="flex flex-col gap-1 text-[11px] text-text-muted">
            Server URL
            <input className={INPUT_CLASS} value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://vox.lan:3921" />
          </label>
          <label className="flex flex-col gap-1 text-[11px] text-text-muted">
            Bearer token
            <input
              className={INPUT_CLASS}
              type="password"
              autoComplete="off"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="leave empty to reuse the saved token"
            />
          </label>
          <button type="submit" className={BUTTON_CLASS} disabled={busy || url.trim() === ''}>
            {busy ? 'Connecting…' : 'Connect'}
          </button>
        </form>
      )}

      {error && (
        <p role="alert" data-testid="server-connection-error" className="mt-3 text-[11px] text-red-400">
          {error}
        </p>
      )}

      <Dialog open={pending !== null} onOpenChange={(open) => { if (!open) setPending(null); }}>
        <DialogContent>
          <DialogTitle>Confirm server certificate</DialogTitle>
          <DialogDescription>
            {pending?.pinned
              ? 'This server certificate changed since you last connected. Continue only if you expected it, for example after rotating the certificate.'
              : 'First connection to this server. Compare this fingerprint with the one shown on the server before trusting it.'}
          </DialogDescription>
          {pending?.pinned && (
            <p className="mt-3 font-mono text-[11px] text-text-muted">Previously pinned: {formatFingerprint(pending.pinned)}</p>
          )}
          <p data-testid="server-fingerprint" className="mt-3 break-all font-mono text-[12px] text-text-primary">
            SHA3-256 {formatFingerprint(pending?.fingerprint ?? '')}
          </p>
          <div className="mt-4 flex justify-end gap-2">
            <button type="button" className={BUTTON_CLASS} onClick={() => setPending(null)}>
              Cancel
            </button>
            <button type="button" className={BUTTON_CLASS} disabled={busy} onClick={() => pending && void connect(pending.fingerprint)}>
              Trust and connect
            </button>
          </div>
        </DialogContent>
      </Dialog>
    </section>
  );
}
```

`SettingsView.tsx`:
1. Change `const SECTIONS = [` to `export const SECTIONS = [`. Insert `{ id: 'server', icon: 'link', label: 'Server' },` as its first entry.
2. Add imports:

   ```tsx
   import { useBackendStatus } from '../../../hooks/useBackendStatus';
   import { SurfaceUnavailable } from '../../layout/SurfaceUnavailable';
   import { ServerConnectionSection } from './ServerConnectionSection';
   import { sectionLockedRemotely } from './settingsBackends';
   ```

3. After `const [section, setSection] = useState('orchestrator');`, add:

   ```tsx
     const { status: backendStatus } = useBackendStatus();
     const sectionLocked = sectionLockedRemotely(section, backendStatus.mode);
     const shownSection = sectionLocked ? '' : section;
   ```

4. Replace every `{section === '` with `{shownSection === '` (StrReplace `replace_all`; `rg -c "\{section === '" SettingsView.tsx` goes 15 → 0). The nav's `const on = section === s.id;` is not matched and stays.
5. Directly after `{/* Content */}` / `<Glass className="col-span-12 md:col-span-9 p-5">`, insert:

   ```tsx
           {sectionLocked && <SurfaceUnavailable viewKey={`settings:${section}`} reason="needs_local_machine" />}
           {shownSection === 'server' && <ServerConnectionSection pushToast={pushToast} />}
   ```

`settingsIndex.ts` — add before the `// Registry-derived entries` comment:

```ts
  { id: 'server-connection', section: 'server', label: 'Connect to server', hint: 'Attach Axis to a remote Vox server', keywords: ['remote', 'server', 'lan', 'profile', 'connect', 'gateway'] },
```

- [ ] **Step 4: Run the tests and typecheck.**

```bash
pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Settings
pnpm --dir crates/vox-gui/ui typecheck
```

Expected: all Settings tests PASS (existing `SettingsView.test.tsx`, `settingsIndex.test.ts` + new). Typecheck exits 0.

- [ ] **Step 5: Commit.**

```bash
git add crates/vox-gui/ui/src/components/surfaces/Settings/
git commit -m "feat(gui-ui): Settings Server section with TOFU confirm; lock local-machine sections remotely"
```

---

### Task 12: Playwright — remote gating, connect flow, local back-compat (screenshots)

**Files:**
- Create: `crates/vox-gui/ui/e2e/axis-remote.spec.ts`

**Interfaces:** none (test only). The remote status fixture mirrors what `BackendState::status()` computes for `axis_surfaces: [mesh, repository, console, models]`. That computation is pinned by `real_inventory_gates_known_surfaces` in Task 5.

- [ ] **Step 1: Write the spec:**

```ts
/**
 * Remote Axis (plan 06): surface gating names its reason, the Settings connect flow walks the TOFU
 * fingerprint confirm, and local mode is unchanged. installTauriMock in remote mode via an
 * axis_backend_status override. Screenshots land in review-bundle/latest/.
 *
 * Run: pnpm --dir crates/vox-gui/ui exec playwright test e2e/axis-remote.spec.ts --project=chromium
 */
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';
import { addInvokeOverrides } from './lib/invokeOverrides';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, '..', 'review-bundle', 'latest');
mkdirSync(OUT_DIR, { recursive: true });

const FP = '3f9a'.repeat(16);
const PROFILE = {
  profiles: ['mesh'],
  max_exposure: 'operator',
  subsystems: { booted: ['db', 'mesh'], failed: [] },
  tools: ['vox_mesh_status'],
  axis_surfaces: ['mesh', 'repository', 'console', 'models'],
  server_version: '0.6.0',
};
const REMOTE_STATUS = {
  mode: 'remote',
  url: 'https://vox.lan:3921/',
  profile: PROFILE,
  surfaces: [
    { surface: 'mesh', available: true, reason: null },
    { surface: 'repository', available: true, reason: null },
    { surface: 'settings', available: true, reason: null },
    { surface: 'console', available: false, reason: 'needs_local_machine' },
    { surface: 'models', available: false, reason: 'not_remote_capable' },
    { surface: 'tasks', available: false, reason: 'not_in_profile' },
  ],
};

async function openRemote(page: Page, view: string) {
  await page.setViewportSize({ width: 1400, height: 900 });
  await addMockInitScript(page, installTauriMock, view);
  await addInvokeOverrides(page, { responses: { axis_backend_status: REMOTE_STATUS } });
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
}

/** Self-contained (serialized into the page): stateful connect flow with a first-use pin prompt. */
function installConnectFlow(arg: { fp: string; remote: unknown }): void {
  const internals = (window as any).__TAURI_INTERNALS__;
  const base = internals.invoke;
  const local = { mode: 'local', url: null, profile: null, surfaces: [] };
  let status: unknown = local;
  (window as any).__AXIS_CONNECT_CALLS__ = [];
  internals.invoke = async (cmd: string, args?: any) => {
    if (cmd === 'axis_backend_status') return status;
    if (cmd === 'axis_connect_remote') {
      (window as any).__AXIS_CONNECT_CALLS__.push(args);
      if (!args?.acceptFingerprint) {
        throw JSON.stringify({ error: { code: 'tls_pin_required', message: 'first connection', fingerprint: arg.fp } });
      }
      status = arg.remote;
      return status;
    }
    if (cmd === 'axis_disconnect_remote') {
      status = local;
      return status;
    }
    return base(cmd, args);
  };
}

test.describe('Remote Axis', () => {
  for (const [view, reason, file] of [
    ['console', 'needs_local_machine', 'axis-remote-console-needs-local.png'],
    ['models', 'not_remote_capable', 'axis-remote-models-not-remote-capable.png'],
    ['tasks', 'not_in_profile', 'axis-remote-tasks-not-in-profile.png'],
  ] as const) {
    test(`${view} shows "not available on this server" (${reason})`, async ({ page }) => {
      await openRemote(page, view);
      const gate = page.getByTestId('surface-unavailable');
      await expect(gate).toBeVisible();
      await expect(gate).toHaveAttribute('data-reason', reason);
      await expect(page.getByText(/not available on this server/i).first()).toBeVisible();
      await expect(page.getByTestId('backend-status-pill')).toBeVisible();
      await page.screenshot({ path: join(OUT_DIR, file), fullPage: true });
    });
  }

  test('mesh renders remotely and the status pill names the server', async ({ page }) => {
    await openRemote(page, 'mesh');
    await expect(page.getByTestId('surface-unavailable')).toHaveCount(0);
    await expect(page.getByTestId('backend-status-pill')).toHaveAttribute('aria-label', 'Connected to vox.lan:3921, profile mesh');
    await page.screenshot({ path: join(OUT_DIR, 'axis-remote-mesh-available.png'), fullPage: true });
  });

  test('connect flow confirms the certificate fingerprint, then connects', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await addMockInitScript(page, installTauriMock, 'settings');
    await page.addInitScript(installConnectFlow, { fp: FP, remote: REMOTE_STATUS });
    await page.goto('/');
    await page.getByRole('button', { name: /^server$/i }).click();
    await page.getByLabel(/server url/i).fill('https://vox.lan:3921');
    await page.getByLabel(/bearer token/i).fill('t0ken');
    await page.getByRole('button', { name: /^connect$/i }).click();
    const dialog = page.getByRole('dialog', { name: /confirm server certificate/i });
    await expect(dialog).toBeVisible();
    await expect(page.getByTestId('server-fingerprint')).toContainText('3f9a 3f9a');
    await page.screenshot({ path: join(OUT_DIR, 'axis-remote-connect-tofu.png'), fullPage: true });
    await page.getByRole('button', { name: /trust and connect/i }).click();
    await expect(page.getByTestId('server-connected')).toBeVisible();
    await expect(page.getByTestId('backend-status-pill')).toBeVisible();
    const calls = await page.evaluate(() => (window as any).__AXIS_CONNECT_CALLS__);
    expect(calls).toEqual([
      { url: 'https://vox.lan:3921', token: 't0ken', acceptFingerprint: null },
      { url: 'https://vox.lan:3921', token: 't0ken', acceptFingerprint: FP },
    ]);
    await page.screenshot({ path: join(OUT_DIR, 'axis-remote-connected.png'), fullPage: true });
  });

  test('local mode is unchanged: no pill, no gate', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await addMockInitScript(page, installTauriMock, 'console');
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.getByTestId('surface-unavailable')).toHaveCount(0);
    await expect(page.getByTestId('backend-status-pill')).toHaveCount(0);
    await page.screenshot({ path: join(OUT_DIR, 'axis-local-unchanged.png'), fullPage: true });
  });
});
```

- [ ] **Step 2: Run the spec.** Fix product code, not assertions, until it is green.

```bash
pnpm --dir crates/vox-gui/ui exec playwright test e2e/axis-remote.spec.ts --project=chromium
```

Expected: 6 passed, plus 7 PNGs in `crates/vox-gui/ui/review-bundle/latest/`. Open each screenshot and check that:
- the unavailable state is centered and the reason text is readable in the dark theme;
- the pill does not overlap the sidebar footer;
- the dialog shows the grouped fingerprint.

- [ ] **Step 3: Run the full UI gates.**

```bash
pnpm --dir crates/vox-gui/ui typecheck
pnpm --dir crates/vox-gui/ui test
pnpm --dir crates/vox-gui/ui test:e2e
```

Expected: all green. The full e2e suite passing unchanged is the local-mode back-compat evidence: none of the existing mocks answer `axis_backend_status`, so they exercise the local fallback.

- [ ] **Step 4: Commit.** Commit the spec. Check whether `review-bundle/latest/` is tracked with `git check-ignore crates/vox-gui/ui/review-bundle/latest/axis-local-unchanged.png`, and commit the PNGs only if they are not ignored.

```bash
git add crates/vox-gui/ui/e2e/axis-remote.spec.ts
git commit -m "test(gui-ui): Playwright coverage for Remote Axis gating and connect flow"
```

---

### Task 13: Live end-to-end verification against `vox serve` (manual, recorded)

**Files:** none committed.

**Interfaces:** exercises plan 04's `vox serve --profile … --bind … --tls-cert … --tls-key …` and `GET /v1/profile`.

**Parallel-safe with:** Tasks 10, 11, 14.

- [ ] **Step 1: Create a throwaway self-signed certificate** outside the repo, and note its SHA3-256 fingerprint:

```bash
mkdir -p ~/dev/_scratch/axis-remote && cd ~/dev/_scratch/axis-remote
openssl req -x509 -newkey ed25519 -nodes -keyout key.pem -out cert.pem -days 2 -subj "/CN=vox.lan" -addext "subjectAltName=DNS:vox.lan,IP:127.0.0.1"
openssl x509 -in cert.pem -outform DER | openssl dgst -sha3-256
```

- [ ] **Step 2: Start a mesh-profile server.** Set the read/write bearer tokens through `vox secrets set` as plan 04 documents:

```bash
cd /Users/brbrainerd/dev/vox/.worktrees/capability-profiles
cargo run -p vox-cli -- serve --profile mesh --bind 127.0.0.1:3921 --tls-cert ~/dev/_scratch/axis-remote/cert.pem --tls-key ~/dev/_scratch/axis-remote/key.pem
```

- [ ] **Step 3: Connect from a debug Axis** (`cargo run -p vox-gui`). Go to Settings → Server, enter `https://127.0.0.1:3921` and the write token, then press Connect. Expected:
  - the dialog shows exactly the Step 1 digest (grouped);
  - Trust → connected, and the pill shows `127.0.0.1:3921 · mesh`;
  - Mesh renders live nodes; Console/Models/Tasks show their reasons;
  - Disconnect → local surfaces return.

- [ ] **Step 4: Rotate the certificate** (rerun Step 1 and restart the server), then reconnect. Expected: the "certificate changed" dialog with both fingerprints. Restart the server as `0.0.0-test`, or otherwise change major.minor if plan 04 allows a version override. If that cannot be done, record that `check_version` is covered by unit tests only. Expected: `version_mismatch` alert.

- [ ] **Step 5: Record the results** (pass/fail per step, plus screenshots) in the PR description. Do not commit scratch files.

---

### Task 14: Docs — Remote Axis how-to, profile reference, where-things-live

**Files:**
- Modify: `docs/src/how-to/how-to-run-vox-as-a-server.md` (created by plan 04 Task 11, which this plan depends on; append only)
- Modify: `docs/src/reference/capability-profiles.md` (created by plan 02; same rule)
- Modify: `docs/src/architecture/where-things-live.md`

**Interfaces:** none.

**Parallel-safe with:** Tasks 10–13.

- [ ] **Step 1: Write the how-to section.** The file exists (plan 04 Task 11). Append to it:

````markdown
## Remote Axis

Axis can attach to a `vox serve` gateway instead of the daemon on your own machine.

1. Start the server with TLS (or `--behind-tunnel`) and bearer tokens, for example:
   `vox serve --profile mesh --bind 0.0.0.0:3921 --tls-cert cert.pem --tls-key key.pem`.
2. On the server, print the certificate fingerprint Axis will show:

   ```bash
   openssl x509 -in cert.pem -outform DER | openssl dgst -sha3-256
   ```

3. In Axis, open **Settings → Server**, enter `https://<host>:<port>` and the read or write token,
   and press **Connect**. Compare the SHA3-256 fingerprint in the dialog with step 2, then
   **Trust and connect**. Axis pins that certificate; if it ever changes, Axis asks again and shows
   both fingerprints.
4. **Disconnect** returns Axis to the local daemon.

Plain `http://` is accepted only for loopback addresses. Axis and the server must share the same
`major.minor` version, otherwise the connection is refused with `version_mismatch`.

**Where things are stored.** The token is kept in the OS keyring through Clavis (registry token
`axis-remote:<origin>`); if the keyring is unavailable Clavis falls back to `~/.vox/auth.json` and
prints a warning. The server URL and pinned fingerprint are the `axis_remote_url` and
`axis_remote_tls_sha3_256` keys in `~/.vox/config.toml`.

**What you see.** A surface is shown only if the server's profile lists it in `axis_surfaces` and
every command module it uses can run remotely (`contracts/gui/command-backends.v1.yaml`). Otherwise
it shows *Not available on this server* with one of three reasons:

| Reason | Meaning |
|---|---|
| not in server profile | The profile does not include the surface. |
| needs this machine | The surface uses files, processes, or settings on the local machine. |
| not remote-capable yet | The surface reads the workspace database directly; it moves behind the gateway per profile (plan 07). |

Remotely usable today: Mesh, Repository (VCS isolation), Search (Vox graph), plus Settings and
Coverage, which configure Axis itself. In Settings, sections that configure the local Vox install
are locked while connected.

**Known limits.** The orchestrator status stream is not relayed from the server (agent events are);
the GUI permission mode is not forwarded, so dangerous tools follow the server's approval default.
````

- [ ] **Step 2: Write the reference section.** In `capability-profiles.md` (if created here, use frontmatter `title: "Capability profiles reference"`, `description: "Profile manifests, exposure classes, subsystems, error codes, and how profiles drive Axis surfaces."`, `category: "reference"`, `status: "current"`), append:

```markdown
## Axis surfaces

`axis_surfaces` in a profile manifest lists `view_key`s from `contracts/gui/surface-registry.v1.yaml`.
When Axis is attached remotely, a surface renders iff it is in `axis_surfaces` and every module that
`contracts/gui/command-backends.v1.yaml` lists for it is `daemon` (with a remote route) or `shell`.

| Class | Meaning | Remote behaviour |
|---|---|---|
| `daemon` | Orchestrator daemon RPC only | Routed through the gateway (`/v1/ws`, `/api/v2`) where a route exists, else `not_remote_capable` |
| `db` | Opens the workspace database directly | `local_only`; migrated per profile by plan 07 |
| `host` | Local files, processes, VCS, CLI sidecar, OS | `local_only`; hidden remotely |
| `shell` | Axis-local (chrome, UI preferences, connect) | Always works |

`vox ci gui-catalog-parity` fails when a command module is unclassified or a row names a missing module.

Axis client-side error codes: `local_only`, `not_remote_capable`, `version_mismatch`,
`tls_pin_required`, `tls_pin_mismatch`, `unauthorized`, `unreachable`, `invalid_url`, `insecure_url`.
```

- [ ] **Step 3: Add the where-things-live row** after the "Axis Drive (`vox gui drive`)" row:

```markdown
| Remote Axis (GUI ↔ `vox serve`) | `Backend`/`BackendState`, `GatewayClient`, TOFU pin, invoke gate in `crates/vox-gui/src/remote/`; commands `crates/vox-gui/src/commands/axis_remote.rs`; manifest [`contracts/gui/command-backends.v1.yaml`](../../../contracts/gui/command-backends.v1.yaml) (CI: `vox ci gui-catalog-parity`); UI `hooks/useBackendStatus.tsx`, `components/layout/SurfaceUnavailable.tsx`, Settings → Server. |
```

- [ ] **Step 4: Lint.**

```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths how-to/how-to-run-vox-as-a-server.md reference/capability-profiles.md architecture/where-things-live.md
```

Expected: exit 0.

- [ ] **Step 5: Commit.**

```bash
git add docs/src/how-to/how-to-run-vox-as-a-server.md docs/src/reference/capability-profiles.md docs/src/architecture/where-things-live.md
git commit -m "docs: Remote Axis how-to, Axis surfaces reference, where-things-live row"
```

---

### Task 15: Final gates

**Files:** none new.

**Interfaces:** none.

- [ ] **Step 1: Run the Rust gates.**

```bash
cargo check -p vox-gui -p vox-cli -p vox-orchestrator-mcp
cargo clippy -p vox-gui -p vox-cli -p vox-orchestrator-mcp --all-targets -- -D warnings
cargo nextest run -p vox-gui
cargo nextest run -p vox-cli gui_command_backends
cargo nextest run -p vox-orchestrator-mcp origin_guard
cargo run -q -p vox-arch-check
cargo run -q -p vox-cli -- ci crate-edges
```

Expected: all green. `crate-edges` reports no new workspace edge.

- [ ] **Step 2: Run the repo gates.**

```bash
cargo run -q -p vox-cli -- ci ssot-drift
cargo run -q -p vox-cli -- ci secret-env-guard
cargo run -q -p vox-cli -- ci secrets-parity
cargo run -q -p vox-cli -- ci data-storage-guard
cargo run -q -p vox-cli -- ci line-endings
VOX_FMT_CHECK=1 vox run scripts/fmt.vox -- --all
vox ci pre-push --complete
```

Expected: all green.

- [ ] **Step 3: Run the UI gates.**

```bash
pnpm --dir crates/vox-gui/ui typecheck
pnpm --dir crates/vox-gui/ui test
pnpm --dir crates/vox-gui/ui test:e2e
```

Expected: all green.

- [ ] **Step 4: Self-review the branch diff.** Run `git diff origin/main...HEAD -- . ':(exclude)Cargo.lock'`, read every hunk, and confirm:
  - each `#[tauri::command]` touched in Task 7 still compiles to the same JS argument names (`tabId`-style camelCase is unchanged);
  - `Cargo.lock` gained no `[[package]]`.

---

## Self-Review

- **Spec §7 coverage:**
  - Backend abstraction (Tasks 4–8);
  - manifest + CI gate on unclassified modules (Tasks 1–2);
  - surface visibility rule with the three named reasons (Tasks 5, 10);
  - connect flow with keyring token, TOFU pin + re-prompt on change, and `version_mismatch` (Tasks 6, 8, 11);
  - `transport.ts` stays the IPC hub (Task 9);
  - migration order deferred to plan 07 with the db list above.
- **Spec §8 / §9:**
  - the new client codes are typed and tested (Task 4);
  - Axis Playwright covers remote gating, the unavailable states, the connect flow and screenshots (Task 12);
  - back-compat is covered by the existing e2e/vitest suites with no `axis_backend_status` handler, `starts_local_with_every_surface_available`, `local_mode_never_denies`, and the unchanged daemon-module tests (Tasks 7, 9, 12).
- **Security guards, mutation-verified:**
  - certificate pin acceptance (Task 4 Step 7);
  - remote `local_only` gate (Task 7 Step 9);
  - origin-guard relaxation in both directions (Task 3 Step 5);
  - manifest completeness (Task 2 Step 7).
- **Contracts used verbatim:** `/v1/profile` field names (`ServerProfile`); the error envelope shape; `version_mismatch`; `VOX_PROFILE` untouched; the `command-backends.v1.yaml` path. The deviations (`shell` class, `notes`, `schema_version`, new client codes, `accept_fingerprint`) are written into `00-contracts.md` in Task 1, as that file requires.
- **No new workspace crate edges:** `vox-gui` already depends on `vox-secrets`, `vox-config`, `vox-crypto`, `vox-http-client`, `vox-foundation`, `vox-orchestrator`. The new dependencies are third-party crates already in the lockfile (Task 4 STOP checkpoint).
- **Placeholder scan:** every code step contains the full code. The only conditional instructions are explicit fallbacks with their trigger condition (Task 3 if plan 04 already relaxed the guard; Task 6 `http` major mismatch; Task 7 `generate_handler!` forwarding).
- **Type consistency:**
  - `BackendStatusDto` (Rust) ↔ `BackendStatus` (TS): `mode`/`url`/`profile`/`surfaces`.
  - `SurfaceAvailability.reason` is `snake_case` on both sides.
  - `axis_connect_remote(url, token, accept_fingerprint)` ↔ `{ url, token, acceptFingerprint }`.

## Risks

- **Thin remote surface set at first.** The most-restrictive classification leaves only `mesh`, `repository` and `vox-search` usable remotely, and Chat, Models, Tasks and Dashboard show a reason. This is deliberate and honest. Plan 07 grows the set by migrating the 15 `db` modules and splitting the six db-dominant `host` modules.
- **Unrouted daemon methods.**
  - `control_plane` (`orch.submit_task`, `orch.list_tasks`, …) and `mission_control` have no gateway equivalent.
  - MCP tools exist only for some of them (`vox_submit_task`, `vox_cancel_task`, `vox_pause_agent`, `vox_resume_agent`, `vox_doubt_task`, `vox_reorder_task`, `vox_pending_approvals`, `vox_task_status`), and their argument schemas were not verified against the daemon params.
  - Mapping them is plan 07 work. `every_daemon_module_has_a_routing_decision` forces a decision for any new daemon module.
- **Gateway origin guard (Task 3).** This plan changes a security guard owned by plan 04's area. If plan 04 rewrote `origin_guard.rs`, re-derive Task 3 against the new code. The invariant is unchanged: browsers are still blocked by `Origin`, and tokens are still checked by `enforce_auth`.
- **Token at rest.** Clavis falls back to plaintext `~/.vox/auth.json` (with a warning) when the OS keyring cannot round-trip. That matches existing registry-token behaviour, but it is weaker than "OS keyring only". Follow-up: refuse to store when the secure store is unavailable, which needs a Clavis API that reports where the token landed.
- **TOFU on tunnel endpoints.** Behind a CA-signed, auto-rotating tunnel certificate (`--behind-tunnel`), every rotation re-prompts. Pinning the leaf is still correct; offering "trust the CA chain" for tunnels is a follow-up.
- **Remote streams.** Only agent events are relayed. `vox://orch-status` is muted while remote (Dashboard/Flow are hidden anyway). A gateway status topic would be needed to un-hide them.
- **`permission_mode` not forwarded.** Remote `invoke_mcp_tool` follows the server's HITL default (`ask`). That fails safe, but differs from local "accept edits" behaviour.
- **Hand-written client DTO for `/v1/profile`.** The data-storage policy prefers schema generation for contract shapes. Task 8 runs `vox ci data-storage-guard`. If it flags `ServerProfile`, generate it from `contracts/mcp/http-gateway.openapi.yaml` (plan 04) instead.
- **No saved-session auto-reconnect.** Axis starts local on every launch; the saved URL and token make reconnecting one click. Auto-reconnect is a follow-up.
- **`include_str!` across crate boundaries.** `vox-gui` embeds two files from `contracts/gui/`. That is fine because `vox-gui` is never published, but a moved contract breaks the build, which is a loud failure, not a silent one.
