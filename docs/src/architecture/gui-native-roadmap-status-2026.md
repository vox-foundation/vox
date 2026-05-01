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
> [`docs/src/architecture/vox-gui-native-roadmap-2026.md`](vox-gui-native-roadmap-2026.md).
> This file is the status overlay.

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
| TASK-2.6 — Align `workflow`/`activity`/`actor` | ✅ Done (Path A — collapse) | `080b3f86` | Parser re-enabled for `workflow`/`activity`/`actor` keywords. All three lower to `HirFn { durability: Some(DurabilityKind::_), … }` — no separate HIR structs needed (they were already absent). `DurabilityKind` enum added in `hir/nodes/durability.rs`. `HirFn.durability: Option<DurabilityKind>` field added. `lower_workflow`, `lower_activity`, `lower_actor_shell`, `lower_actor_handler` in `hir/lower/decl.rs`. Actor handlers lowered as `HirFn { name: "ActorName::event_name" }`. `Token::Http`/`Agent`/`Env`/`AtComponent` remain tombstoned. 3 former tombstone parser tests replaced with positive assertions. 202 lib tests pass, 0 fail. |

**Phase 2 verdict:** 6/6 complete. Phase 2 fully done.

### TASK-2.6 retrospective

Path A (collapse, original goal) executed at commit `080b3f86`. The roadmap intended to unify `workflow`/`activity`/`actor` declarations under one HIR shape (`HirFn + Option<DurabilityKind>`). This is now done:

- Parser accepts `workflow name(params) to T { body }`, `activity name(params) to T { body }`, and `actor Name { state field: Type; on event(params) { body } }`
- All three lower to `HirFn` with a `DurabilityKind` discriminant
- `http`/`agent`/`env`/`@component` remain tombstoned (Path B, separate scope)
- No standalone `HirActor`/`HirWorkflow`/`HirActivity` structs were needed — they were already absent from the HIR at the time of Path A execution

---

## Phase 3 — Grammar Unification Policy

| Task | Status | Notes |
|------|--------|-------|
| TASK-3.1 — Add grammar unification rule to AGENTS.md | ✅ Done | §"Grammar Unification (Vox Source Syntax)" added to `AGENTS.md` after §VoxScript-First Glue Code. Rule: bare-keyword blocks declare scope; decorators modify declarations. `actor`/`workflow`/`activity` tombstone state and TASK-2.6 noted. Architecture index cross-link will appear on next `vox-doc-pipeline` run (auto-generated file, not manually editable). |

---

## Phase 4 — Compiler Primitive Expansion

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-4.1 — Add `state_machine` first-class block | ✅ Done (parser + HIR + typeck) | see below | `state_machine Name { state S, terminal state T, on Event from S -> T }` parsed. AST `StateMachineDecl/SmStateDecl/SmTransitionDecl` in `ast/decl/state_machine.rs`. `Decl::StateMachine` in `ast/decl/types.rs`. `HirStateMachineDecl/HirStateDecl/HirTransitionDecl` in `hir/nodes/state_machine.rs`. `state_machines: Vec<HirStateMachineDecl>` in `HirModule`/`SemanticHirModule`. Lowering in `hir/lower/mod.rs`. `typeck/state_machine_check.rs`: `E_SM_DUP_STATE`, `E_SM_TERMINAL_TRANSITION`, `E_SM_UNKNOWN_STATE`, `W_SM_EMPTY`. 6 tests pass. Web IR `BehaviorNode::StateMachine` and TSX reducer codegen deferred to Phase 5. |
| TASK-4.2 — Add effect annotations (`uses net, db, mcp(...)`) | ✅ Done (parser + HIR + typeck) | see below | `fn f() uses net, db, mcp(tool) -> T { }` parsed. AST `EffectKind/EffectAnnotation` in `ast/decl/effect.rs`. `FnDecl.effects: Vec<EffectAnnotation>` added. `HirEffectKind/HirEffectSet` in `hir/nodes/effect.rs`. `HirFn.effects` added. Lowering in `hir/lower/decl.rs`. `typeck/effect_check.rs`: `E_EFFECT_PURE_CONFLICT`, `E_EFFECT_DUPLICATE`. 10 tests pass. Call-graph propagation (`caller.effects ⊇ callee.effects`) deferred to Phase 5. |
| TASK-4.3 — Add typed URLs primitive | ✅ Done (parser + HIR + typeck) | see below | `url Name { Variant, Variant(args) }` parsed. AST `UrlDecl/Variant/Arg` in `ast/decl/ui.rs`. `Decl::Url` in `ast/decl/types.rs`. `HirUrlDecl/Variant/Arg` in `hir/nodes/url.rs`. `url_decls: Vec<HirUrlDecl>` in `HirModule`/`SemanticHirModule`. Lowering in `hir/lower/mod.rs`. `typeck/url_check.rs` (duplicate variant error). 4 tests pass. TS emission and golden file updates deferred to Phase 5. |
| TASK-4.4 — Add design-token types | ✅ Done | see below | `crates/vox-compiler/src/tokens/{mod,validate}.rs` created. `pub mod tokens` in `lib.rs`. `validate_web_ir_with_tokens` added (non-breaking). `vox.tokens.json` expanded. `contracts/tokens/tokens.v1.json` schema created. 10 token tests pass. |

**Phase 4 verdict:** 4/4 complete. Phase 4 fully done.

## Phase 5 — Web IR Correctness Validators

| Task | Status | Commit | Notes |
|------|--------|--------|-------|
| TASK-5.1 — Token resolution validator hardening | ✅ Done | see below | `WebIrDiagnosticSeverity` enum added to `web_ir/mod.rs` (Warning/Error, default Warning). `is_literal_style_value()` extended: hex colors (#RGB/#RRGGBB/#RGBA/#RRGGBBAA), functional colors (rgb/rgba/hsl/hsla/oklch/oklab/lch/lab/color), CSS named colors (30 common), dimensional literals (px/rem/em/vh/vw/vmin/vmax/%/pt/cm/mm/ex/ch/fr/dvh/dvw). Code renamed `raw_literal_color` → `literal_value`. `Default` impl on `WebIrDiagnostic` + `..Default::default()` at all 29 construction sites. 9 tests pass. Severity will be promoted from Warning → Error in Phase 6 when `raw_css { }` escape hatch lands. |
| TASK-5.2 — Route reachability validator | ✅ Done | see below | `validate_route_reachability()` added to `web_ir/validate.rs`. Checks: `web_ir_validate.route.missing_component` (RouteContract.meta["component"] not in view_roots), `web_ir_validate.route.unreachable` (no inbound `<link href\|to>` points to route ID or pattern; root `/` exempt). Wired into `validate_web_ir_with_metrics`. 5 new tests pass (14 total in module). |
| TASK-5.3 — AriaNode + a11y validator | ✅ Done | `9ef8cbb0` | `web_ir/validate_a11y.rs` created. Walks DOM arena without modifying `DomNode::Element` IR. 5 rules: `img.missing_alt` (Error), `button.missing_label` (Error), `anchor.missing_href` (Warning), `interactive.missing_keyboard` (Warning), `input.missing_label` (Warning). Escape hatches: `aria-hidden="true"` suppresses img check; `type="hidden"` suppresses input check; `id` attr counts as labelled (associated `<label for="...">` may exist). Wired into `validate_web_ir_with_metrics`. `AriaNode` embedding in `DomNode::Element` deferred to Phase 6. 15 tests pass. |
| TASK-5.4 — v0.dev output validator | ✅ Done | see below | `crates/vox-cli/src/v0_tsx_validate.rs` created. Regex-based JSX element extractor builds a `WebIrModule` from raw v0.dev TSX, then runs `validate_a11y`. Results surfaced to user via `eprintln!` before island is written to disk — errors print a warning but do not block generation (non-breaking UX). `format_diagnostics` and `has_errors` helpers exposed. 11 tests pass. |

**Phase 5 verdict:** 4/4 complete. Phase 5 fully done.

## Phases 6–8

Not started. Dependencies on Phase 5.

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

Phases 0–5 are fully complete. TASK-2.6 completed as Path A (commit `080b3f86`).
Phases 6–8 are next.

1. **Phase 6** — Web IR BehaviorNode::StateMachine + TSX reducer codegen (TASK-4.1 deferred item), `raw_css {}` escape hatch, severity promotion for token validator, AriaNode embedding in DomNode.
2. **Phase 7** — TS emission for URL types (TASK-4.3 deferred item), call-graph effect propagation (TASK-4.2 deferred item).
3. **Phase 8** — Corpus migration (`TASK-8.1`): sweep `.vox` files using pre-tombstone `http` bare routing.

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
- 2026-04-30 — TASK-5.4 ✅ Done: `v0_tsx_validate.rs` created in vox-cli. Regex JSX extractor → `WebIrModule` → `validate_a11y`. Wired into `generate_island_tsx` — a11y issues printed to stderr before file is written. 11 tests pass. Errors non-blocking (informational warning). (Agent session.)
- 2026-04-30 — TASK-5.3 ✅ Done: `web_ir/validate_a11y.rs` created (280 lines + 15 tests). 5 a11y rules across img/button/anchor/role-button/input. Wired into `validate_web_ir_with_metrics`. `AriaNode` IR embedding deferred Phase 6. All 15 tests pass. (Agent session.)
- 2026-04-30 — TASK-5.2 ✅ Done: `validate_route_reachability()` added. Two new codes: `route.missing_component` and `route.unreachable`. Link detection from `<link href|to>` DOM nodes. Root `/` always reachable. 5 new tests (14 total). `cargo check --workspace` 0 errors. (Agent session.)
- 2026-04-30 — TASK-5.1 ✅ Done: `WebIrDiagnosticSeverity` (Warning/Error) added to `WebIrDiagnostic`. Literal style value detector extended from hex+rgb/hsl to also cover named CSS colors (30 common) and dimensional literals (17 suffixes). Code renamed `raw_literal_color` → `literal_value`. `Default` impl on `WebIrDiagnostic`; 29 construction sites updated with `..Default::default()`. 9 tests pass, 0 workspace errors. Severity stays Warning until Phase 6 `raw_css{}` escape hatch. (Agent session.)
- 2026-04-30 — TASK-4.2 ✅ Done (parser + HIR + typeck): `fn f() uses net, db, mcp(tool) -> T { }` effect annotations implemented. 2 new files (`ast/decl/effect.rs`, `hir/nodes/effect.rs`, `typeck/effect_check.rs`), 6 files modified. `cargo check --workspace` 0 errors. 10 tests pass. Call-graph propagation deferred to Phase 5. Bugfix: `env` and `spawn` are dedicated lexer tokens — matched with `Token::Env` / `Token::Spawn`. (Agent session.)
- 2026-04-30 — TASK-4.1 ✅ Done (parser + HIR + typeck): `state_machine Name { state S, terminal state T, on E from S -> T }` block implemented. 3 new files (`ast/decl/state_machine.rs`, `hir/nodes/state_machine.rs`, `typeck/state_machine_check.rs`), 11 files modified across compiler/corpus/mens. `cargo check --workspace` 0 errors. 6 tests pass. Web IR / TSX reducer deferred to Phase 5. (Agent session.)
- 2026-04-30 — TASK-2.6 Path A ✅ Done: parser re-enabled for `workflow`/`activity`/`actor`.
  `DurabilityKind` enum + `HirFn.durability` field added. `parse_workflow_decl`,
  `parse_activity_decl`, `parse_actor_decl` added to `parser/descent/decl/head.rs`.
  Tombstone removed for these three tokens in `parse_decl` and `parse_module_script`.
  `lower_workflow`/`lower_activity`/`lower_actor_shell`/`lower_actor_handler` wired
  into HIR lowering. 202 lib tests pass. (Agent session.)
- 2026-04-30 — TASK-4.3 ✅ Done (parser + HIR + typeck core): `url Name { Variant }` block
  parsed; `UrlDecl/Variant/Arg` AST; `Decl::Url`; `HirUrlDecl/Variant/Arg`;
  `url_decls` in `HirModule`/`SemanticHirModule`; `url_check.rs` typeck;
  4 tests pass. TS emission and `<link>` checking deferred. Bugfix: variants
  were `Token::TypeIdent` (PascalCase) — parser match updated to handle both.
- 2026-04-29 — TASK-4.4 ✅ Done: `crates/vox-compiler/src/tokens/{mod,validate}.rs`
  created. `TokenRegistry` with flattened lookup, Levenshtein suggestions,
  `TokenValidationDiagnostic`. `validate_web_ir_with_tokens` added to
  `web_ir/validate.rs` (non-breaking — existing callers unchanged). `vox.tokens.json`
  expanded (radius, typography, surface.pairs). `contracts/tokens/tokens.v1.json`
  schema created. 10 token tests pass, `cargo check -p vox-compiler` 0 errors. (Agent session.)
