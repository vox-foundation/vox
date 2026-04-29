---
title: "GUI-Native Language Roadmap — Execution Status"
description: "Live tracking of task completion for the Vox GUI-native language roadmap (April 2026)."
category: "architecture"
status: "current"
last_updated: "2026-04-25"
training_eligible: false
---

# GUI-Native Language Roadmap — Execution Status

> **Provenance.** Derived from the April 2026 roadmap authored by Bertrand
> Reyna-Brainerd. This file tracks what has actually been implemented versus
> what is planned. Updated 2026-04-25 from a Cowork-sandbox audit against
> commit `fa350de8` (HEAD on `main`).
>
> **Canonical roadmap source.** The full task specifications live in
> `VOX_GUI_NATIVE_ROADMAP_2026.md` at the repository root. This file is the
> status overlay.

---

## Phase 0 — Dashboard Safety

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-0.1 — ADR 024: Dashboard as Axum SPA | ✅ Done | `b7536331` | `docs/src/adr/024-dashboard-axum-spa.md` created. Index updated. |
| TASK-0.2 — Replace loopback-auto-unauth with token auth | ✅ Done | `bb72c7e3` | `token.rs` created. `DashboardToken::generate_or_load()` implemented. Meta-tag injection in `assets.rs`. |
| TASK-0.3 — Strict Origin/Host allowlist middleware | ✅ Done | `327bf460` | Extracted origin check into `origin_guard.rs` with JSON error body, strict WS-upgrade check, and comprehensive unit tests. |
| TASK-0.4 — CSP, X-Frame-Options, Referrer-Policy, CORS | ✅ Done | `d152d272` | `X-Frame-Options` and `Content-Security-Policy` headers present in `assets.rs`. `CorsLayer` installed in gateway router. |
| TASK-0.5 — Fix `vox dashboard` CLI detachment + readiness polling | ✅ Done | `b7536331` | `DashboardLauncher` struct with `setsid()`/Windows `DETACHED_PROCESS` flags implemented. |
| TASK-0.6 — Harden `transport.ts`: backoff, auth refresh | 🟡 Partial | `b7536331` | Bearer attached, basic reconnect present. Still need: explicit `connectionStatus`/`authStatus` event union, exponential backoff with cap, 4401 close-code handling, UI banner wiring. |
| TASK-0.7 — Fix `App.tsx` hooks violation + dead imports | ✅ Done | `b7536331` | `useVoxTransport()` moved to top of component. Dead imports removed. |
| TASK-0.8 — Integration tests for dashboard crate | ✅ Done | `d152d272` | `crates/vox-dashboard/tests/{auth,asset_serving,origin_guard}.rs` present. |

**Phase 0 verdict:** 7 complete, 1 partial. Only TASK-0.6 polish remains.

---

## Phase 1 — Dashboard Cleanup

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-1.1 — Delete `vscode.ts` shim | ✅ Done | `b7536331` | `rg` finds zero `vscode.postMessage` or `getVsCodeApi` references. |
| TASK-1.2 — Fix or delete `vox-dashboard-d` binary | 🔲 Needs decision | — | Operator must choose Option A (delete) or Option B (make it work). |
| TASK-1.3 — `build.rs` for `include_dir!` safety + ETag | 🟡 Partial | `b7536331` | `build.rs` exists; ETag/`If-None-Match` handling not yet confirmed in `assets.rs`. |
| TASK-1.4 — Clean up `index.css` duplication | ✅ Done | (audit-discovered) | Reinvented Tailwind utility block already removed (file is 233 lines vs 392 at audit baseline). Cosmetic blank-line cleanup applied 2026-04-25. |
| TASK-1.5 — Pin workspace deps, remove `tsconfig.tsbuildinfo` | ✅ Done | `b7536331` | `tower-http` uses `workspace = true`. `tsbuildinfo` removed and gitignored. |

**Phase 1 verdict:** 3 complete, 2 partial/decision-pending, 0 not started.

---

## Phase 2 — Compiler Primitive Collapse

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-2.1 — Delete Path B UI fields from `HirModule` | 🟡 Mostly done | `9c0e0240` | Migration flags removed. Path B fields purged. **Residual:** `HirModule.components: Vec<HirReactiveComponent>` still present at HEAD (line 74) and on `SemanticHirModule` (line 106). To finish: either remove (Path C lowering uses `HirIsland` and `legacy_ast_nodes` only) or rename to `reactive_components` with intent docs. |
| TASK-2.2 — Unify `@server`/`@query`/`@mutation` → `@endpoint(kind: …)` | ✅ Done | `00588f6a` | `HirEndpointFn` with `EndpointKind` unified. `query_fns`/`mutation_fns`/`server_fns` collapsed to `endpoint_fns`. |
| TASK-2.3 — Collapse `HirExpr::DbTableOp` into `MethodCall` | ✅ Done | per `decl.rs:142` | `HirExpr::DbTableOp` removed entirely; operations lowered into `HirExpr::MethodCall(_, _, _, Option<Box<HirDbQueryPlan>>, _)`. |
| TASK-2.4 — Resolve `HirExpr::Pipe` vs `Binary(Pipe)` | ✅ Done | per `decl.rs` enum | Standalone `HirExpr::Pipe` variant deleted; pipeline expressions strictly `HirExpr::Binary(HirBinOp::Pipe, ...)`. |
| TASK-2.5 — Retire `http` bare-keyword routing | ✅ Done (parser) | per `parser/descent/tests.rs:99` | `test_parse_http_route_is_tombstoned` passes. Parser rejects with friendly error. **Caveat:** corpus migration of pre-existing `.vox` files using the form is not separately verified here — TASK-8.1 handles that atomically. |
| TASK-2.6 — Align `workflow`/`activity`/`actor` | 🟡 Half-done (negative half) | `fa350de8` and earlier | **Parser side:** `actor`/`workflow`/`activity` are tombstoned per `test_parse_actor_is_tombstoned`/`test_parse_workflow_is_tombstoned`/`test_parse_activity_is_tombstoned`. **HIR side:** `HirActor`/`HirActorHandler`/`HirWorkflow`/`HirActivity`/`HirRoute`/`HirHttpMethod` were over-purged in TASK-2.1 then restored in `fa350de8`. The original collapse goal (sugar `workflow foo()` to `@durable fn foo()`, unifying into `FnDecl + Option<DurabilityKind>`) was **not** achieved. Re-plan needed: either keep tombstone permanent and remove the orphan HIR structs, or restore the source surface AND collapse to decorator sugar. See "TASK-2.6 retrospective" below. |

**Phase 2 verdict:** 4 complete, 2 partial. Phase 2 is not done; TASK-2.1 has a residual field, and TASK-2.6 needs a re-plan decision from the operator.

### TASK-2.6 retrospective

The roadmap intended to *unify* four declaration kinds (`fn`, `workflow`, `activity`, `actor`) under one HIR shape (`FnDecl + Option<DurabilityKind>`) while keeping source ergonomics. What actually happened:

1. TASK-2.1 over-purged the AST and HIR types for these constructs.
2. Parser tombstoning was added as a band-aid (rejecting the source forms).
3. `fa350de8` restored the HIR types so the workspace would compile, but did not restore source-level acceptance.

Net state: source forms are rejected, but the HIR can still represent durability primitives. That is a non-goal halfway point. To finish properly, choose ONE:

- **Path A (collapse, original goal):** Re-enable parser acceptance of `workflow`/`activity`/`actor` keywords, lower them as sugar to `FnDecl { durability: Some(_), … }`, delete the standalone `HirActor`/`HirWorkflow`/`HirActivity` structs.
- **Path B (retire, simpler):** Keep parser tombstones permanent. Delete the orphan HIR types. Migrate any callers expecting them to use the unified `FnDecl + decorator` form. Mark durability as a future feature.

Recommend Path A: matches the roadmap, preserves expressivity, and consolidates four primitives into one. Estimated effort: 1 day after a clear decision.

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

`FORGE_TOKEN` is stored in `~/.vox/auth.json` (local Clavis vault, **not
committed to the repo**). `vox ci watch-run` reads it automatically. No
more `$env:FORGE_TOKEN=...` prefix required for CI polling.

The `gho_*` token is a GitHub OAuth token scoped to your existing `gh` session.
It is **safe to store in Clavis** for local use — Clavis writes to
`~/.vox/auth.json` on your machine, never to the repository. You do NOT need
to generate a new PAT. The existing OAuth token is sufficient for the
`workflow` and `repo` scopes needed by `watch-run`.

---

## Repository hygiene flags (must address before next phase)

- **AGENTS.md §VoxScript-First Glue Code violation.** Repo root contains 9
  Python glue scripts left over from the TASK-2.1/2.3 migrations:
  `aggressive_purge.py`, `fix_all.py`, `fix_all_clean.py`, `fix_codegen_ts.py`,
  `fix_methodcall.py`, `migrate.py`, `purge_all.py`, `purge_dbtableop.py`,
  `update_legacy.py`. These were one-shot tools and must be removed. The
  Cowork sandbox cannot delete files; operator should run on Windows:
  `Remove-Item *.py -Force` from the repo root, then commit as
  `chore: remove one-shot .py glue scripts (AGENTS.md P1 compliance)`.
- **Stale WIP in working tree at audit time** (8 modified files): a
  partial attempt to finish removing `components` and reorganize
  `hir/nodes/decl.rs`, plus a buggy addition of a `HirExpr::Pipe(a, b, _)`
  match arm in `crates/vox-workflow-runtime/src/workflow/plan.rs` that
  references a variant deleted by TASK-2.4 and would fail to compile. Plus
  a status-doc revert that undoes correctly-claimed wins. **Recommend
  discard:** `git checkout -- crates/ docs/src/architecture/gui-native-roadmap-status-2026.md`
  on Windows after closing whatever editor/process is holding
  `.git/index.lock`.

---

## Immediate Next Tasks (in dependency order)

1. **HYGIENE** — Discard stale WIP. Delete the 9 .py glue scripts.
2. **TASK-2.1 finish** — Decide and implement: remove `HirModule.components`
   and `SemanticHirModule.components` fields, OR rename to
   `reactive_components` with documented intent. If removed, audit
   `crates/vox-compiler/src/codegen_ts/` and `web_ir/lower.rs` for any
   remaining readers and migrate them to use `legacy_ast_nodes` projection
   or `HirIsland` directly.
3. **TASK-2.6 decision + finish** — Operator picks Path A (collapse) or
   Path B (retire). Implement accordingly.
4. **TASK-0.6 finish** — Discriminated-union event types,
   exponential-backoff cap, UI banner.
6. **TASK-3.1** — Add the grammar unification rule to AGENTS.md (now that
   Phase 2 is functionally complete after items 2 + 3 above).

After items 2 + 3 land, Phase 4 (state machines, effect annotations, typed
URLs, design-token types) is unblocked.

---

## Audit log

- 2026-04-24 — Initial status tracker created (commit `08c8ad87`).
- 2026-04-25 — Audit refresh against HEAD `fa350de8`. TASK-0.4, TASK-0.8
  promoted to ✅ (commit `d152d272`). TASK-2.1 demoted to 🟡 due to
  residual `components` field. TASK-2.6 reclassified as half-done with
  retrospective + re-plan note. Hygiene flags surfaced. (Cowork session.)
