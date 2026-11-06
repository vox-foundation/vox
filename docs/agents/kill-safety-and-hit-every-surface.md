---
title: "Kill-safety triad & hit-every-surface checklist"
description: "Agent-ops rules adapted from t3code: never pattern-kill, never write live userdata, never bake localhost into bundles; multi-surface completion checklist for harness/GUI changes."
category: "contributor"
status: "current"
---

# Kill-safety triad & hit-every-surface checklist

Most Vox contributions will come from coding agents (often remote). Be careful about killing processes, writing live userdata, and baking localhost origins into GUI bundles.

## Kill-safety triad (Vox paths)

1. **Never pattern-kill.** Do not `pkill -f`, `pgrep | kill`, or `kill` a PID found by matching a name, path, or worktree string. Your agent process has this worktree's path in argv, and the machine may run several `vox` / Vite / Tauri processes. Kill only a PID you captured at spawn, or confirm ownership (cwd / port) first. Prefer `vox ci free-binary` / `vox ci kill-stuck-tests` for scoped reaps.
2. **Never write the live userdata/home.** Do not open the developer's primary Vox home read-write from an agent worktree. Resolve homes through `vox_config::paths` / `VOX_*` dirs. Reading and copying fixtures is fine; starting a daemon against the live home is not. Worktree isolation: stamp `.vox/worktree-setup.marker` and use a worktree-local config/home (see [worktree-isolation.md](worktree-isolation.md)).
3. **Never bake localhost origins into bundles.** Do not hard-code `http://127.0.0.1` / `localhost` Vite/Tauri proxy URLs into shipped UI bundles. Dev stays single-origin; Vite proxies `/api` and related paths. Remote browsers break when localhost is baked in.

## Hit every surface (harness / GUI changes)

When a PR changes the harness, orchestrator MCP, or GUI chat/control plane, complete **all** applicable rows before claiming done:

| # | Surface | Check |
|---|---|---|
| 1 | Entry points | CLI (`vox …`), Tauri commands, MCP tools registered |
| 2 | Clients | GUI hooks / IPC clients updated |
| 3 | Providers / adapters | Model registry, LLM facade, plugin adapters |
| 4 | MCP / agent reachability | Tool listed + dispatched; agent can invoke |
| 5 | Contracts | `contracts/**` + schema regen if shapes changed |
| 6 | Reverse / error states | Empty, offline, denied, timeout paths |
| 7 | Connection modes | Local Tauri vs remote daemon vs MCP-only |
| 8 | Docs | AGENTS / architecture / runner-contract pointer |

Advisory: `vox ci hit-every-surface` prints this checklist when the changed-path set touches multiple surface classes without an acknowledgement file (`.github/pull_request_template.md` checkbox or `HIT_EVERY_SURFACE=1`).

## Suppression human review

PRs that only add `toestub-ignore` / `#[allow]` / eslint-disable without linked reasons require the `needs-human-review` label (or CODEOWNERS approval on `contracts/toestub/**`). See [suppression-review.md](suppression-review.md).
