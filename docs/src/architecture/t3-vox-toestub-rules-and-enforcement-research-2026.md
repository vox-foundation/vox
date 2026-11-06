---
title: "T3 Ecosystem Engineering Best Practices for Vox: TOESTUB Detectors, Wire/SQL Budgets & Agent Rules (2026)"
description: "Source- and web-verified audit of T3 Code and T3 Chat static analysis rules (oxlint-plugin-t3code, Macroscope check-run agents, transferBudgetV2, SqlStatementCounter, AGENTS.md), critically filtered and adapted into concrete Vox TOESTUB detectors, CI budgets, and agent rules."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Ecosystem Engineering Best Practices for Vox: TOESTUB Detectors, Wire/SQL Budgets & Agent Rules (2026)

<!-- kb:summary -->
This document audits the static-analysis rules (`oxlint-plugin-t3code`), AI check-run review agents (`.macroscope/check-run-agents/`), wire and SQLite performance regression harnesses (`transferBudgetV2.integration.test.ts`, `SqlStatementCounter.integration.ts`), UI/V8 memory practices (`docs/internals/web-ui.md`, `performance-regressions.md`, `consistency-action-names.md`), and operational guardrails (`AGENTS.md`, `PLAYBOOK.md`) across `pingdotgg/t3code` and the 9 T3 ecosystem repositories. It critically separates T3-specific conventions from universal engineering best practices and specifies concrete new detectors for `vox-code-audit` (TOESTUB), CI transfer/SQL budgets, and `AGENTS.md` rules (`Phase 226`, `T3-TOESTUB-RULES-01..04`).
<!-- /kb:summary -->

## 1. Philosophy: Take the Best to Improve Vox — Do Not Become T3

Our objective is **selective, high-leverage extraction**: adopting the static enforcement, performance budgets, and operational guardrails where `t3code` and the T3 ecosystem excel, while rejecting rules that conflict with Vox's Rust/VoxLang architecture or where Vox's existing 55 `vox-code-audit` (TOESTUB) detectors are already stronger.

### 1.1 Critical Critique: What to Adopt, What to Adapt, and What to Reject

| T3 Practice / Rule Source | Verdict for Vox | Critical Rationale & Web/Code Verification |
|---|---|---|
| **`require-suppression-reason.ts` & `.macroscope/approvability.md`** (Every lint/type/LSP suppression must include `-- <reason>` or an adjacent explanation; PRs adding suppressions require human review) | **ADOPT & EXPAND (`vox/lint/unreasoned-suppression`)** | Aligns directly with Rust 1.81+ `clippy::allow_attributes_without_reason` and `#[expect(..., reason = "...")]`. Currently `crates/vox-code-audit/src/detectors/scaling.rs:115-128` honours bare `// toestub-ignore(...)` without requiring a reason. Enforcing mandatory reasons across Rust `#[allow]`/`#[expect]`, `toestub-ignore`, and TS `@ts-ignore`/`eslint-disable` prevents agents from silently lowering the quality bar. |
| **`no-test-timeout-or-unreasoned-skip.ts`, `no-real-time-wait-in-tests.ts`, `no-polling-loop-in-tests.ts`, `no-wall-clock-in-tests.ts`, `no-frame-waits-in-browser-tests.ts`** | **ADOPT (`vox/test/flaky-wait-and-unreasoned-skip`)** | Complements Vox's existing `weak_test.rs` detector (`crates/vox-code-audit/src/detectors/weak_test.rs`). Whereas `weak_test.rs` catches assertion-free or shallow `.is_ok()` tests, T3's test rules catch **flaky timing anti-patterns** (`sleep` polling loops, wall-clock delta assertions, `requestAnimationFrame` waits in browser tests, and unquarantined `#[ignore]` / `it.skip` without an issue/reason). |
| **`no-raw-mcp-registration.ts` & `effect-service-conventions.md` ("Thin Transports / Service-First Placement" & "Bounded Safe Error Attributes")** | **ADOPT & ADAPT (`vox/mcp/raw-registration-and-unbounded-error`)** | In `t3code`, RPC/HTTP/MCP handlers only decode input, call one domain service method, and map typed errors, ensuring every capability is reachable by CLI, GUI, MCP agents, and scheduled tasks alike. Adapting this to `vox-orchestrator-mcp` prevents tool handlers from inlining subprocess/SQL/Git logic or leaking unbounded raw payloads/secrets in error strings. |
| **`no-unscoped-has.ts`, `require-centered-scroll-gutter.ts`, `no-outset-state-indicators.ts`, `no-raw-resize-observer.ts`** | **ADOPT (`vox/ui/layout-and-perf-hazards`)** | Verified against Chromium/WebView2 rendering engines: (1) unanchored `:has(...)` forces full-DOM style recalculation on every mutation; (2) centered `overflow-y-auto` containers shift horizontally (CLS) when scrollbars appear unless `scrollbar-gutter: stable both-edges` is set; (3) outset focus rings (`focus:ring` without `ring-inset`) clip inside scroll/dock panels; (4) raw `ResizeObserver` callbacks fire before paint and trigger loop errors or 1-frame-late React state commits. |
| **`transferBudgetV2.integration.test.ts` & `SqlStatementCounter.integration.ts`** (`120 KiB` / `75`-row cold-open cap, `7 KiB` wire cap, per-turn SQLite statement counter) | **ADOPT (`crates/vox-orchestrator-mcp/tests/transfer_budget.rs`)** | Pinning wire bytes, decoded JSON bytes, WebSocket frame counts, and SQLite statement counts in a deterministic integration test catches N+1 SQLite regressions and accidental payload bloat before merge. |
| **`docs/internals/web-ui.md` §Handlers in large components (V8 Shared-Closure Scope Retention)** | **ADOPT (`vox/ui/closure-scope-retention` & UI guidelines)** | V8 allocates a single shared `Context` for all closures created in the same function invocation. In long-lived components (`ChatView`, `UrbsCanvas`), multiple `useCallback`/`useMemo` hooks with different dependency arrays chain old render scopes together in memory. Using plain functions internally and `useStableCallback` at prop boundaries eliminates this leak. |
| **`AGENTS.md` §The three ways to hurt yourself & §Hit every surface ("Reverse states: no one-way doors")** | **ADOPT into `AGENTS.md` & `.agents/rules/00-core.md`** | Banning pattern-based `pkill -f`/`pgrep` (which kills sibling agent worktrees), banning read-write test attachments to live `~/.vox/` user state, and requiring reverse states (`snooze` ↔ `unsnooze`, `archive` ↔ `unarchive`) directly harden multi-agent development in Vox. |
| **`no-low-value-comments.ts` (`MAX_COMMENT_LINES = 4`, "Default to no comment")** | **PARTIAL ADAPT ONLY (Reject 4-line cap; adopt commented-out code & unlinked `TODO` checks)** | **Critique:** Blanket-adopting T3's `MAX_COMMENT_LINES = 4` and "Default to no comment" would **violate** Vox's core guideline (*"Maintain documentation integrity. Preserve all existing comments and docstrings"*) and conflict with `crates/vox-code-audit/src/detectors/adr_citation.rs`. However, two narrow sub-rules inside `no-low-value-comments.ts`—flagging **commented-out code blocks** and **unlinked `TODO`/`FIXME`/`HACK` markers**—are high-signal additions to TOESTUB's `stub.rs` and `ai_laziness.rs`. |
| **`prefer-effect-fn.ts`, `prefer-catch-tags.ts`, `no-manual-effect-runtime-in-tests.ts`** | **REJECT (Effect-TS specific)** | These rules govern `Effect-TS` generator syntax (`Effect.fn` vs `Effect.gen`) and do not apply to Vox's Rust (`tokio` / `vox-actor-runtime`) or React/Vite stack. Note that the *underlying observability principle* of `prefer-effect-fn` (named tracing spans on service operations, untraced on tight per-chunk loops) is already idiomatic in Rust via `#[tracing::instrument]`. |
| **`CONTRIBUTING.md` §Models we do not accept** | **REJECT (Repo-specific social policy)** | Vox's `AGENTS.md` explicitly mandates a **Model-Agnostic LLM Boundary** (`vox_actor_runtime::llm`) where every frontier and local MENS model is evaluated empirically on scoreboard evidence (`discovery_pipeline.rs`), never via hardcoded model bans. |

---

## 2. Complete Audit of `t3code` Static Enforcement (`oxlint-plugin-t3code` — 29 Rules)

For completeness and traceability against `~/dev/t3code/oxlint-plugin-t3code/rules/`, the table below catalogs all 29 custom static-analysis rules in `t3code` and maps each to its disposition in Vox:

| # | `t3code` Rule (`oxlint-plugin-t3code/rules/`) | Enforced Invariant in `t3code` | Vox Disposition & Target |
|---|---|---|---|
| 1 | `require-suppression-reason.ts` | Every `oxlint-disable`, `eslint-disable`, `@ts-expect-error`, `@ts-ignore`, or `@effect-diagnostics` must include `-- reason` or a preceding comment | **Add to TOESTUB (`unreasoned_suppression.rs`)** covering Rust `#[allow]`/`#[expect]`, `toestub-ignore`, and TS suppressions |
| 2 | `no-test-timeout-or-unreasoned-skip.ts` | Bans per-test `timeout`/`retry` overrides and requires `// quarantine: #<issue> <why>` above any `.skip`/`.todo`/`.only` | **Add to TOESTUB (`test_clock_and_skip.rs`)** covering Rust `#[ignore]` and TS `.skip`/`.only` |
| 3 | `no-real-time-wait-in-tests.ts` | Bans wall-clock `setTimeout`/`setInterval` waits in tests and `Effect.sleep` without `TestClock.adjust` | **Add to TOESTUB (`test_clock_and_skip.rs`)** flagging `std::thread::sleep` / unpaused `tokio::time::sleep` in unit tests |
| 4 | `no-polling-loop-in-tests.ts` | Bans `for`/`while`/`do` loops in tests that spin on `sleep`/`yieldNow`/timer ticks until a predicate holds | **Add to TOESTUB (`test_clock_and_skip.rs`)** flagging sleep-polling loops in Rust and TS tests |
| 5 | `no-wall-clock-in-tests.ts` | Bans unmocked `Date.now()`, `new Date()`, `performance.now()`, and `process.hrtime()` in test files | **Add to TOESTUB (`test_clock_and_skip.rs`)** flagging unseeded wall-clock delta assertions in unit tests |
| 6 | `no-frame-waits-in-browser-tests.ts` | Bans `requestAnimationFrame`, `transitionend`, `animationend`, and `new Promise(setTimeout)` in `.browser.test.tsx` | **Add to TOESTUB (`test_clock_and_skip.rs`)** for `crates/vox-gui/ui` Playwright/Vitest tests |
| 7 | `no-test-in-loop.ts` | Bans declaring `it(...)` / `test(...)` inside `for` loops; requires `it.each(cases)` | **Add to TOESTUB (`test_clock_and_skip.rs`)** for TS test files |
| 8 | `require-disposable-render.ts` | Requires `await using screen = await render(...)` so mounted trees always unmount on failure | **Adopt pattern** in `crates/vox-gui/ui` test helpers |
| 9 | `no-dom-test-environment.ts` | Bans `jsdom`/`happy-dom` comments; requires real headless browser or pure node tests | **Informational guideline** (Vox uses Playwright + Vitest) |
| 10 | `no-raw-mcp-registration.ts` | Bans registering tools on `McpServer` without `McpToolAccess` capability wrappers | **Add to TOESTUB (`mcp_access_and_error_safety.rs`)** enforcing `McpToolAccess` in `vox-orchestrator-mcp` |
| 11 | `no-rpc-permission-bypass.ts` | Bans raw `.client` RPC access or `RpcPermissionGuard` outside the shared command boundary | **Add to TOESTUB (`mcp_access_and_error_safety.rs`)** enforcing permission-guarded dispatch |
| 12 | `no-unscoped-has.ts` | Bans Tailwind/CSS `:has(...)` selectors not anchored to a class, attribute, id, or non-root tag | **Add to TOESTUB (`ui_perf_hazards.rs`)** for `crates/vox-gui/ui` |
| 13 | `require-centered-scroll-gutter.ts` | Requires `scrollbar-gutter-both` / `scrollbar-gutter-stable` on `overflow-y-auto` containers that center children | **Add to TOESTUB (`ui_perf_hazards.rs`)** for `crates/vox-gui/ui` |
| 14 | `no-outset-state-indicators.ts` | Requires `ring-inset` (or negative outline offset) on `focus*`/`selected`/`checked` rings so scroll containers never clip them | **Add to TOESTUB (`ui_perf_hazards.rs`)** for `crates/vox-gui/ui` |
| 15 | `no-raw-resize-observer.ts` | Bans direct `new ResizeObserver` in web components; requires shared frame-synchronized `observeResize` | **Add to TOESTUB (`ui_perf_hazards.rs`)** for `crates/vox-gui/ui` |
| 16 | `no-native-title-tooltip.ts` | Bans native `title="..."` attribute on intrinsic JSX elements (except `iframe`/`embed`/`object`/`math`) | **Add to TOESTUB (`ui_perf_hazards.rs`)** for accessible tooltips in `crates/vox-gui/ui` |
| 17 | `no-inline-schema-compile.ts` | Bans compiling `Schema.decode*`/`encode*` validators inside function bodies (rebuilt on every call); requires hoisting to module scope | **Extend TOESTUB `scaling.rs` (`scaling/regex-new-hot`)** to also flag inline `serde_json` schema compilation or `Regex::new` inside functions in Rust & TS |
| 18 | `no-global-process-runtime.ts` | Bans direct `process.platform`/`process.arch`/`os.platform()` reads outside injected host process config | **Adapt** for cross-platform path/shell checks in Vox |
| 19 | `no-low-value-comments.ts` | Rejects divider lines, test step labels (`// Arrange`), unlinked `TODO`s, bare PR refs, commented-out code, and `>4` line comments | **Selective adoption:** Add commented-out-code and unlinked-`TODO` checks to `stub.rs`; **reject** `>4` line comment cap |
| 20 | `no-hookless-use-function.ts` | Bans `use*` functions in React that call no React hooks (misleads React Compiler) | **Add to `ui_perf_hazards.rs`** for `crates/vox-gui/ui` |
| 21 | `no-manual-memo.ts` | Bans manual `useMemo`/`useCallback` when React Compiler is enabled | **Defer** until React Compiler is enabled in `crates/vox-gui/ui`; adopt `useStableCallback` closure-retention guidance now |
| 22–29 | `namespace-node-imports`, `no-em-dash-copy`, `no-hermes-unsupported-apis`, `no-manual-effect-runtime-in-tests`, `no-mobile-pull-request-glyph-literals`, `no-mobile-uniwind-theme-escape-hatches`, `prefer-catch-tags`, `prefer-effect-fn` | Effect-TS, React Native Hermes/Uniwind, or stylistic copy rules specific to `t3code` | **Skip (YAGNI / Stack-Specific)** |

---

## 3. Specification of New TOESTUB (`vox-code-audit`) Detectors (`Phase 226`)

All four new detectors implement `crate::rules::DetectionRule` in `crates/vox-code-audit/src/detectors/` and emit structured `Finding` JSONL with `diagnostic_id` (`vox/<category>/<name>`).

### 3.1 Detector 1: `unreasoned_suppression.rs` (`vox/lint/unreasoned-suppression`)
- **Supported Languages:** `[Language::Rust, Language::TypeScript, Language::Vox]`
- **Severity:** `Severity::Warning` (upgraded to `Severity::Error` on changed lines in CI)
- **Detection Rules:**
  1. **Rust `#[allow(...)]` / `#[expect(...)]`:**
     - Scans non-comment Rust lines for `#[allow(` or `#![allow(` or `#[expect(`.
     - Flags `#[allow(...)]` when neither `reason = "..."` is present inside the attribute nor an explanatory `//` comment exists on the immediately preceding line.
     - Emits a suggestion to prefer `#[expect(<lint>, reason = "<why>")]` (Rust 1.81+ standard) so the compiler automatically warns when the underlying suppression becomes obsolete.
  2. **TOESTUB `toestub-ignore(...)`:**
     - Scans for `toestub-ignore(`. Requires a non-empty `-- <reason>` or `: <reason>` after the closing `)` (e.g., `// toestub-ignore(scaling/blocking-in-async) -- CLI bootstrap runs before Tokio pool`).
  3. **TypeScript `@ts-ignore` / `@ts-expect-error` / `eslint-disable` / `oxlint-disable`:**
     - Requires `-- <reason>` on the directive or an adjacent comment line above it (`require-suppression-reason.ts` parity).

### 3.2 Detector 2: `test_clock_and_skip.rs` (`vox/test/flaky-wait-and-unreasoned-skip`)
- **Supported Languages:** `[Language::Rust, Language::TypeScript]`
- **Severity:** `Severity::Warning`
- **Detection Rules:**
  1. **Unquarantined Ignored / Skipped Tests (`vox/test/unreasoned-skip`):**
     - In Rust: flags bare `#[ignore]` without `#[ignore = "<issue or reason>"]` or a preceding `// quarantine:` comment.
     - In TypeScript (`.test.ts`, `.test.tsx`, `.spec.ts`): flags `.skip(`, `.todo(`, `.only(`, `xit(`, `xdescribe(`, `fit(`, `fdescribe(` unless preceded by `// quarantine: #<issue> <why>`.
  2. **Sleep-Polling Loops in Tests (`vox/test/polling-loop`):**
     - Inside `#[test]` / `#[tokio::test]` (Rust) or `.test.ts*` / `.spec.ts*` (TS), flags any `for` / `while` / `loop` body that calls `sleep` (`std::thread::sleep`, `tokio::time::sleep`, `setTimeout`, `yield_now`) combined with a conditional `break` / `return` / `assert`, instructing the author to await a deterministic `Notify`, `oneshot`, `watch`, `Barrier`, or `vi.waitFor` milestone instead.
  3. **Browser Frame / Timer Races (`vox/test/browser-frame-wait`):**
     - In `.spec.ts` / `.test.tsx`, flags `requestAnimationFrame`, `new Promise(setTimeout)`, and `addEventListener("transitionend" | "animationend")`.

### 3.3 Detector 3: `mcp_access_and_error_safety.rs` (`vox/mcp/access-and-error-safety`)
- **Supported Languages:** `[Language::Rust]`
- **Severity:** `Severity::Error` for missing `McpToolAccess`; `Severity::Warning` for inline handler I/O and unbounded error strings.
- **Detection Rules:**
  1. **Mandatory `McpToolAccess` Capability Registration (`vox/mcp/raw-registration`):**
     - In `crates/vox-orchestrator-mcp/src/`, flags any tool dispatch arm that is not wrapped in `McpToolAccess::{reads, writes, writes_with_capability, starts_threads}` (once Phase 222 lands).
  2. **Thin Transport Handler Boundary (`vox/mcp/thick-transport-handler`):**
     - Flags MCP tool entrypoints and HTTP/WS route handlers that directly construct `std::process::Command`, `tokio::process::Command`, or raw SQL queries (`SELECT `, `INSERT `, `UPDATE `, `DELETE `) inline rather than calling a `vox-orchestrator` service method.
  3. **Safe Bounded Error Attributes (`vox/mcp/unbounded-error-payload`):**
     - Flags `ToolResult::err` / `Err(format!(...))` calls in transport boundaries that interpolate raw `stdout`, `stderr`, `body`, or `payload` variables without passing through `safe_truncate_for_prompt` or redaction.

### 3.4 Detector 4: `ui_perf_hazards.rs` (`vox/ui/layout-and-perf-hazards`)
- **Supported Languages:** `[Language::TypeScript]`
- **Severity:** `Severity::Warning`
- **Detection Rules (`crates/vox-gui/ui/src/**`):**
  1. **`vox/ui/unscoped-has`:** Flags `:has(...)` in Tailwind class strings or CSS that is not anchored to a class (`.foo`), attribute (`[data-*]`), ID (`#id`), or non-root element tag (`t3code` `no-unscoped-has.ts` algorithm).
  2. **`vox/ui/centered-scroll-gutter`:** Flags JSX elements combining `overflow-y-auto` / `overflow-auto` with centering utilities (`items-center`, `justify-center`, `mx-auto`) unless `scrollbar-gutter-stable` or `scrollbar-gutter-both` is present.
  3. **`vox/ui/outset-focus-ring`:** Flags `focus:ring*` / `focus-visible:ring*` / `selected:ring*` without `ring-inset` (or positive `outline-offset-*`) where parent scroll/dock containers can clip the indicator.
  4. **`vox/ui/raw-resize-observer`:** Flags `new ResizeObserver(` outside the shared frame-throttled `observeResize` utility.
  5. **`vox/ui/native-title-tooltip`:** Flags `title=` on intrinsic JSX HTML elements (`<button>`, `<div>`, `<span>`, `<a>`) in favor of accessible `Tooltip` primitives.

---

## 4. Wire Transfer & SQL Statement Regression Gate (`crates/vox-orchestrator-mcp/tests/transfer_budget.rs`)

Inspired by `t3code`'s `apps/server/integration/transferBudgetV2.integration.test.ts` and `SqlStatementCounter.integration.ts`:
- **Scenario Fixture:** 10 historical turns (each with 5 command tool executions and 1 large `900 KiB` MCP tool result) followed by 1 measured turn (20 command tools + `1.1 MiB` MCP result) and 1 long turn (`900` work rows of reasoning, commands, file diffs, checkpoints, and subagents).
- **Hard CI Ceilings Enforced by `transfer_budget.rs`:**
  1. **Bounded Cold-Open Snapshot:** Decoded JSON `<= 120_000` bytes (`120 KiB`) and `<= 75` timeline rows for the 900-row turn (proving inline command/tool/diff bodies are omitted at the wire boundary while remaining inspectable on demand).
  2. **Measured-Turn Streamed Egress:** Total wire bytes `<= 7_000` bytes (`7 KiB` compressed / ID-interned / body-deduped) and `<= 8` batched frames.
  3. **`CommandOutputHub` Isolation:** `0` raw command stdout/stderr delta events persisted to the orchestration event store or emitted via `turn_item.updated` (all live terminal chunks flow exclusively through `CommandOutputHub`).
  4. **`SqlStatementCounter` Ceiling:** Counts SQLite prepared-statement executions per turn and per reconnect via tracing spans (`name == "sqlite"`), failing the test if a refactor introduces an N+1 query loop across turns or subagents.

---

## 5. Additions to `AGENTS.md` and `.agents/rules/00-core.md`

The following four operational rules distilled from `t3code/AGENTS.md` and `docs/internals/` are codified for Vox agents:

1. **Process & Sandbox Hygiene ("Three Ways to Hurt Yourself"):**
   - **Never kill by pattern:** Never run `pkill -f`, `pgrep`, or `Stop-Process` matching a binary name, repo path, or worktree substring—sibling agents and the parent harness share the repository path in their `argv`. Only terminate a PID captured at spawn or verified by port ownership + working directory.
   - **Never point tests at live `~/.vox/` userdata:** Copy a read-only fixture or use an in-memory/temp-dir SQLite store with scheduled tasks and live auth sessions stripped.
   - **Never bake `localhost` URLs into client bundles:** Keep dev single-origin behind the Vite/server proxy so LAN, Tailscale, and remote tunnel sessions work identically.
2. **Service-First Placement (Thin Transports):**
   - Any capability a user can trigger in `vox-gui` must be implemented as a domain service method in `vox-orchestrator` (not inline in a GUI/HTTP/MCP handler) so CLI (`vox-cli`), MCP (`vox-orchestrator-mcp`), and scheduled tasks execute the exact same code path.
3. **Reverse States ("No One-Way Doors") & Action Name Agreement:**
   - Every state mutation (`snooze`, `archive`, `pin`, `mute`, `lock`, `quarantine`) must ship with its reverse action and a visible state indicator.
   - A command's visible label, tooltip, context-menu wording, and `aria-label` must identify the same action and target object (`"Copy message"` vs. `"Copy code block"` vs. `"Copy link"`).
4. **V8 Shared-Closure Scope Retention in Large React Surfaces:**
   - In long-lived components (`ChatView`, `Composer`, `UrbsCanvas`), do not chain multiple `useCallback`/`useMemo` closures with staggered dependencies (which pin older V8 shared `Context` scopes and retain stale megabyte-scale thread transcripts in heap memory). Write internal handlers as plain functions and wrap with `useStableCallback` only where callbacks cross prop or effect boundaries.
