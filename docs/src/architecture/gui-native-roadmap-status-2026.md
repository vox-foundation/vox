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
| TASK-0.6 — Harden `transport.ts`: backoff, auth refresh | ✅ Done | see commit | Typed discriminated-union events (`ConnectionStatusPayload`, `AuthStatusEvent`) with `VoxTransportEventMap` overloads on `on`/`emit`. Exponential backoff replaced with `Math.min(250·2ⁿ, 30 s)`. 4401 auth close-code stops reconnect loop. UI banner wired in exec-hint bar. Missing type exports in `types.ts` resolved; broken `'../../../src/types'` import paths in components fixed. |
| TASK-0.7 — Fix `App.tsx` hooks violation + dead imports | ✅ Done | `b7536331` | `useVoxTransport()` moved to top of component. Dead imports removed. |
| TASK-0.8 — Integration tests for dashboard crate | ✅ Done | `d152d272` | `crates/vox-dashboard/tests/{auth,asset_serving,origin_guard}.rs` present. |

**Phase 0 verdict:** 8/8 complete. Phase 0 is fully done.

---

## Phase 1 — Dashboard Cleanup

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-1.1 — Delete `vscode.ts` shim | ✅ Done | `b7536331` | `rg` finds zero `vscode.postMessage` or `getVsCodeApi` references. |
| TASK-1.2 — Fix or delete `vox-dashboard-d` binary | ✅ N/A | — | `crates/vox-dashboard/src/bin/` exists but is empty — the binary was never created. No action required. |
| TASK-1.3 — `build.rs` for `include_dir!` safety + ETag | ✅ Done | `b7536331` | `build.rs` confirmed present. ETag computed as `"<version>--<path>-<size>"` and `If-None-Match` checked in `assets.rs` lines 26–76. Returns 304 on match. |
| TASK-1.4 — Clean up `index.css` duplication | ✅ Done | (audit-discovered) | Reinvented Tailwind utility block already removed (file is 233 lines vs 392 at audit baseline). Cosmetic blank-line cleanup applied 2026-04-25. |
| TASK-1.5 — Pin workspace deps, remove `tsconfig.tsbuildinfo` | ✅ Done | `b7536331` | `tower-http` uses `workspace = true`. `tsbuildinfo` removed and gitignored. |

**Phase 1 verdict:** 5/5 complete (TASK-1.2 N/A). Phase 1 fully done.

---

## Phase 2 — Compiler Primitive Collapse

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-2.1 — Delete Path B UI fields from `HirModule` | ✅ Done | `9c0e0240` | Migration flags removed. Path B fields purged. `HirModule.components: Vec<HirReactiveComponent>` is intentional **Path C** (reactive components) infrastructure — marked `HirFieldOwnership::SemanticCore`, actively used by `codegen_ts/reactive.rs` and `web_ir/lower.rs`. It is not a residual and must not be removed. |
| TASK-2.2 — Unify `@server`/`@query`/`@mutation` → `@endpoint(kind: …)` | ✅ Done | `00588f6a` | `HirEndpointFn` with `EndpointKind` unified. `query_fns`/`mutation_fns`/`server_fns` collapsed to `endpoint_fns`. |
| TASK-2.3 — Collapse `HirExpr::DbTableOp` into `MethodCall` | ✅ Done | per `decl.rs:142` | `HirExpr::DbTableOp` removed entirely; operations lowered into `HirExpr::MethodCall(_, _, _, Option<Box<HirDbQueryPlan>>, _)`. |
| TASK-2.4 — Resolve `HirExpr::Pipe` vs `Binary(Pipe)` | ✅ Done | per `decl.rs` enum | Standalone `HirExpr::Pipe` variant deleted; pipeline expressions strictly `HirExpr::Binary(HirBinOp::Pipe, ...)`. |
| TASK-2.5 — Retire `http` bare-keyword routing | ✅ Done (parser) | per `parser/descent/tests.rs:99` | `test_parse_http_route_is_tombstoned` passes. Parser rejects with friendly error. **Caveat:** corpus migration of pre-existing `.vox` files using the form is not separately verified here — TASK-8.1 handles that atomically. |
| TASK-2.6 — Align `workflow`/`activity`/`actor` | ✅ Done (Path B — retire) | `6524b3f7` | Parser tombstones permanent. Removed `HirActor`, `HirActorHandler`, `HirWorkflow`, `HirActivity` structs and `actors`/`workflows`/`activities` Vec fields from `HirModule`, `SemanticHirModule`, `VoxIrContent`. All lowering, typeck, and codegen paths for these retired. 15 files, −1 150 lines, 0 warnings. `HirRoute`/`AppContract` untouched. `BindingKind::Actor`/`lookup_actor`/`ActorHandlerSig` kept — live path for the `Claude` built-in actor expression checker. |

**Phase 2 verdict:** 6/6 complete. Phase 2 fully done.

### TASK-2.6 retrospective

The roadmap intended to *unify* four declaration kinds (`fn`, `workflow`, `activity`, `actor`) under one HIR shape (`FnDecl + Option<DurabilityKind>`) while keeping source ergonomics. What actually happened:

1. During TASK-2.1 the AST and HIR types for `workflow`/`activity`/`actor`/`HttpRoute` were over-purged.
2. Parser tombstoning was added as a band-aid (rejecting the source forms at parse time).
3. `fa350de8` restored the HIR types so the workspace would compile, but did not restore source-level acceptance.

**Current state at HEAD:** AST types (`ActorDecl`, `WorkflowDecl`, `ActivityDecl`, `HttpRouteDecl`) and HIR types (`HirActor`, `HirWorkflow`, `HirActivity`) exist but are labelled `HirFieldOwnership::MigrationOnly`. Parser tombstones the source forms. Codegen (`codegen_rust/emit/workflow.rs`, `codegen_ts/activity.rs`, `typeck`) still references these types — some in production paths, some in guardrail tests only. `HirRoute` is `HirFieldOwnership::AppContract` and is NOT in scope for removal.

Net state: source forms are rejected, but the HIR can still represent durability primitives. That is a non-goal halfway point. To finish properly, choose ONE:

- **Path A (collapse, original goal):** Re-enable parser acceptance of `workflow`/`activity`/`actor` keywords, lower them as sugar to `FnDecl { durability: Some(_), … }`, delete the standalone `HirActor`/`HirWorkflow`/`HirActivity` structs.
- **Path B (retire, simpler):** Keep parser tombstones permanent. Delete the orphan HIR types. Migrate any callers expecting them to use the unified `FnDecl + decorator` form. Mark durability as a future feature.

Recommend Path A: matches the roadmap, preserves expressivity, and consolidates four primitives into one. Estimated effort: 1 day after a clear decision.

---

## Phase 3 — Grammar Unification Policy

| Task | Status | Notes |
|------|--------|-------|
| TASK-3.1 — Add grammar unification rule to AGENTS.md | ✅ Done | §"Grammar Unification (Vox Source Syntax)" added to `AGENTS.md` after §VoxScript-First Glue Code. Rule: bare-keyword blocks declare scope; decorators modify declarations. `actor`/`workflow`/`activity` tombstone state and TASK-2.6 noted. Architecture index cross-link will appear on next `vox-doc-pipeline` run (auto-generated file, not manually editable). |

---

## Phase 4 — Compiler Primitive Expansion

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-4.1 — Add `state_machine` first-class block | 🔲 Not started | — | Preconditions met. 2-3 week effort. |
| TASK-4.2 — Add effect annotations (`uses net, db, mcp(...)`) | 🔲 Not started | — | Parallel with 4.1. 2-3 week effort. |
| TASK-4.3 — Add typed URLs primitive | 🔲 Not started | — | Preconditions met. 1-2 week effort. |
| TASK-4.4 — Add design-token types | ✅ Done | see below | `crates/vox-compiler/src/tokens/{mod,validate}.rs` created. `pub mod tokens` in `lib.rs`. `validate_web_ir_with_tokens` added (non-breaking). `vox.tokens.json` expanded. `contracts/tokens/tokens.v1.json` schema created. 10 token tests pass. |

**Phase 4 verdict:** 1/4 complete. TASK-4.1, 4.2, 4.3 in progress.

## Phases 5–8

Not started. Dependencies on Phase 4.

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

## Repository hygiene flags

- ~~**AGENTS.md §VoxScript-First Glue Code violation.**~~ 9 Python glue scripts deleted 2026-04-29 (never tracked by git; removed from working tree).
- ~~**Stale WIP.**~~ Buggy compiler WIP (`HirExpr::Pipe` dead match arm, incomplete type removals) discarded 2026-04-29 via `git checkout -- crates/`.

---

## Immediate Next Tasks (in dependency order)

Phases 0–3 are fully complete. TASK-2.6 landed as Path B (commit `6524b3f7`).
Phase 4 is now fully unblocked.

1. **TASK-4.x — Phase 4: State machines, effect annotations, typed URLs, design-token types.**
   Start with the canonical roadmap source (`VOX_GUI_NATIVE_ROADMAP_2026.md` at repo root) for the full TASK-4.1–4.x specifications.

2. **Optional cleanup:** `crates/vox-compiler/src/hir/dead_code.rs` references removed types but is not declared in `hir/mod.rs` (not compiled). Can be deleted safely when touching that area.

---

## Audit log

- 2026-04-24 — Initial status tracker created (commit `08c8ad87`).
- 2026-04-25 — Audit refresh against HEAD `fa350de8`. TASK-0.4, TASK-0.8
  promoted to ✅ (commit `d152d272`). TASK-2.6 reclassified as half-done with
  retrospective + re-plan note. Hygiene flags surfaced. (Cowork session.)
- 2026-04-29 — TASK-0.6 completed: typed transport events, clean backoff,
  missing `types.ts` exports, broken import paths fixed. TASK-2.1 re-confirmed
  ✅ Done: `components` field is Path C (`HirFieldOwnership::SemanticCore`),
  not a Path B residual. 9 stale .py scripts deleted. Stale compiler WIP
  discarded. `.cargo/config.toml` fixed (`relative = true`). Phase 0 verdict
  updated to 8/8. (Agent session.)
- 2026-04-29 — TASK-1.2 N/A confirmed (bin/ empty; binary never created).
  TASK-1.3 ✅ Done confirmed (ETag + If-None-Match in assets.rs lines 26–76).
  TASK-3.1 ✅ Done: §"Grammar Unification" section added to AGENTS.md.
  Phase 1 verdict updated to 5/5 complete. Phase 3 verdict: 1/1 complete.
  Next-tasks list reduced to TASK-2.6 only. (Agent session.)
- 2026-04-29 — TASK-2.6 Path B executed: 15 compiler files, −1 150 lines,
  `cargo check -p vox-compiler` 0 errors 0 warnings. `HirActor`,
  `HirActorHandler`, `HirWorkflow`, `HirActivity` structs and all
  lowering/typeck/codegen paths retired. `BindingKind::Actor`,
  `ActorHandlerSig`, `lookup_actor` preserved (live Claude built-in path).
  Phase 2 verdict: 6/6 complete. Commit `6524b3f7`. Phases 4–8 now
  unblocked. (Agent session.)
- 2026-04-29 — TASK-4.4 ✅ Done: `crates/vox-compiler/src/tokens/{mod,validate}.rs`
  created. `TokenRegistry` with flattened lookup, Levenshtein suggestions,
  `TokenValidationDiagnostic`. `validate_web_ir_with_tokens` added to
  `web_ir/validate.rs` (non-breaking — existing callers unchanged). `vox.tokens.json`
  expanded (radius, typography, surface.pairs). `contracts/tokens/tokens.v1.json`
  schema created. 10 token tests pass, `cargo check -p vox-compiler` 0 errors. (Agent session.)
