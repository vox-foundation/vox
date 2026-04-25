---
title: "GUI-Native Language Roadmap — Execution Status"
description: "Live tracking of task completion for the Vox GUI-native language roadmap (April 2026)."
category: "architecture"
status: "current"
last_updated: "2026-04-24"
training_eligible: false
---

# GUI-Native Language Roadmap — Execution Status

> **Provenance.** Derived from the April 2026 roadmap authored by Bertrand
> Reyna-Brainerd. This file tracks what has actually been implemented versus
> what is planned. Updated by AI agent on 2026-04-24.
>
> **Canonical roadmap source.** The full task specifications live in the
> roadmap document provided by the operator. This file is the status overlay.

---

## Phase 0 — Dashboard Safety

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-0.1 — ADR 024: Dashboard as Axum SPA | ✅ Done | `b7536331` | `docs/src/adr/024-dashboard-axum-spa.md` created. Index updated. |
| TASK-0.2 — Replace loopback-auto-unauth with token auth | ✅ Done | `bb72c7e3` | `token.rs` created. `DashboardToken::generate_or_load()` implemented. Meta-tag injection in `assets.rs`. |
| TASK-0.3 — Strict Origin/Host allowlist middleware | ✅ Done | `327bf460` | Extracted origin check into `origin_guard.rs` with JSON error body, strict WS-upgrade check, and comprehensive unit tests. |
| TASK-0.4 — CSP, X-Frame-Options, Referrer-Policy, CORS | 🟡 Partial | `b7536331` | `X-Frame-Options` and `Content-Security-Policy` headers present in `assets.rs`. CorsLayer not yet installed in gateway router. |
| TASK-0.5 — Fix `vox dashboard` CLI detachment + readiness polling | ✅ Done | `b7536331` | `DashboardLauncher` struct with `setsid()`/Windows `DETACHED_PROCESS` flags implemented. |
| TASK-0.6 — Harden `transport.ts`: backoff, auth refresh | 🟡 Partial | `b7536331` | Needs verification of backoff caps and `authStatus` event emission. |
| TASK-0.7 — Fix `App.tsx` hooks violation + dead imports | ✅ Done | `b7536331` | `useVoxTransport()` moved to top of component. Dead imports removed. |
| TASK-0.8 — Integration tests for dashboard crate | ❌ Not started | — | No test files under `crates/vox-dashboard/tests/`. |

**Phase 0 verdict:** 4 complete, 2 partial, 2 not started. TASK-0.3 is the critical blocker for security hardening.

---

## Phase 1 — Dashboard Cleanup

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-1.1 — Delete `vscode.ts` shim | ✅ Done | `b7536331` | `rg` finds zero `vscode.postMessage` or `getVsCodeApi` references. |
| TASK-1.2 — Fix or delete `vox-dashboard-d` binary | 🔲 Needs decision | — | Operator must choose Option A (delete) or Option B (make it work). |
| TASK-1.3 — `build.rs` for `include_dir!` safety + ETag | 🟡 Partial | `b7536331` | ETag support not yet confirmed. `build.rs` presence needs verification. |
| TASK-1.4 — Clean up `index.css` duplication | ❌ Not started | — | Stale hand-rolled Tailwind utilities still likely present. |
| TASK-1.5 — Pin workspace deps, remove `tsconfig.tsbuildinfo` | ✅ Done | `b7536331` | `tower-http` uses `workspace = true`. `tsbuildinfo` removed and gitignored. |

**Phase 1 verdict:** 2 complete, 2 partial/decision-pending, 1 not started.

---

## Phase 2 — Compiler Primitive Collapse

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-2.1 — Delete Path B UI fields from `HirModule` | ✅ Done | `HEAD` | `HirLoweringMigrationFlags` removed. `endpoint_fns` unified. `hir.components` fully purged from `codegen_ts`, `web_ir`, and `typeck`. |
| TASK-2.2 — Unify `@server`/`@query`/`@mutation` → `@endpoint(kind: …)` | ✅ Done | `00588f6a` | `HirEndpointFn` with `EndpointKind` unified. `query_fns`/`mutation_fns`/`server_fns` collapsed to `endpoint_fns`. |
| TASK-2.3 — Collapse `HirExpr::DbTableOp` into `MethodCall` | ✅ Done | `HEAD` | `HirExpr::DbTableOp` removed entirely; operations lowered into `MethodCall` with `HirDbQueryPlan`. Obsolete comment removed. |
| TASK-2.4 — Resolve `HirExpr::Pipe` vs `Binary(Pipe)` | ✅ Done | `HEAD` | Removed redundant `HirExpr::Pipe` variant; pipeline expressions now strictly use `HirExpr::Binary(HirBinOp::Pipe, ...)`. All matches updated safely. |
| TASK-2.5 — Retire `http` bare-keyword routing | ❌ Not started | — |  |
| TASK-2.6 — Align `workflow`/`activity`/`actor` | ❌ Not started | — |  |

**Phase 2 verdict:** 4 complete, 0 partial, 2 not started.

---

## Phase 3 — Grammar Unification Policy

| Task | Status | Notes |
|------|--------|-------|
| TASK-3.1 — Add grammar unification rule to AGENTS.md | ❌ Not started | Depends on Phase 2. |

---

## Phases 4–8

All not started. Dependencies on Phases 2–3.

---

## Token / Clavis Status

`FORGE_TOKEN` is now stored in `~/.vox/auth.json` (local Clavis vault, **not
committed to the repo**). `vox ci watch-run` reads it automatically. No
more `$env:FORGE_TOKEN=...` prefix required for CI polling.

The `gho_*` token is a GitHub OAuth token scoped to your existing `gh` session.
It is **safe to store in Clavis** for local use — Clavis writes to
`~/.vox/auth.json` on your machine, never to the repository. You do NOT need
to generate a new PAT. The existing OAuth token is sufficient for the
`workflow` and `repo` scopes needed by `watch-run`.

---

## Immediate Next Tasks (in order)

1. **TASK-0.8** — Write integration tests for dashboard auth + asset serving.
2. **TASK-0.4 (finish)** — Install `CorsLayer` in gateway router.
3. **TASK-2.5** — Retire `http` bare-keyword routing.
