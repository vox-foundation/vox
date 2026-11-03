---
last_mapped_commit: 9a79f708de1ce8f8b3944dc99afb51a2877fb960
last_mapped_at: 2026-09-22
---
<!-- refreshed: 2026-09-22 -->

# Architecture

**Analysis Date:** 2026-09-22

## System Overview

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ L5 — Surfaces (binaries)                                                  │
├──────────────┬──────────────┬───────────────┬────────────┬───────────────┤
│  vox-cli     │  vox-gui     │ vox-orchestr- │ vox-ml-cli │  vox-term     │
│ (CLI/`vox`)  │ (Tauri desk- │ ator-d (daemon│ (ML/Mens   │ (ratatui TUI) │
│`crates/      │  top shell)  │ binary)       │  CLI)      │`crates/       │
│ vox-cli/`    │`crates/      │`crates/       │`crates/    │ vox-term/`    │
│              │ vox-gui/`    │ vox-orchestr- │ vox-ml-cli/│               │
│              │ +ui/ (React) │ ator-d/`      │`           │               │
└──────┬───────┴──────┬───────┴───────┬───────┴─────┬──────┴───────┬───────┘
       │               │               │             │              │
       ▼               ▼               ▼             ▼              ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ L3 — Heavy domain runtimes                                                │
│  vox-compiler (lex→parse→HIR→typeck→codegen)  `crates/vox-compiler/`     │
│  vox-orchestrator (multi-agent router, D1-D10 policies) `crates/         │
│  vox-orchestrator/`                                                      │
│  vox-orchestrator-mcp (MCP tool/HTTP layer)  `crates/vox-orchestrator-mcp/`│
│  vox-actor-runtime (actors/mailboxes/LLM activities) `crates/            │
│  vox-actor-runtime/`                                                     │
│  vox-db (VoxDb facade over Turso/libSQL)  `crates/vox-db/`               │
│  vox-populi (mesh worker registry, MENS ML) `crates/vox-populi/`         │
│  vox-search, vox-scientia, vox-terminal-core, vox-plugin-host, ...       │
└──────┬─────────────────────────────────────────────────────────┬─────────┘
       │                                                          │
       ▼                                                          ▼
┌───────────────────────────────┐                    ┌────────────────────┐
│ L2 — Domain libraries          │                    │ L4 — Plugins        │
│  vox-config, vox-eval,         │                    │ (cdylib, loaded not │
│  vox-journal, vox-llm-egress,  │                    │  compiled-in)       │
│  vox-repository, ...           │                    │`crates/vox-plugin-*/│
│ `crates/vox-<domain>/`         │                    └────────────────────┘
└──────┬─────────────────────────┘
       ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ L1/L0 — Primitives & pure types                                           │
│  vox-foundation, vox-crypto, vox-telemetry, vox-ast, vox-db-types,        │
│  vox-orchestrator-types, vox-mesh-types, workspace-hack                   │
│ `crates/vox-<name>/`                                                     │
└──────────────────────────────────────────────────────────────────────────┘
```

Layering is enforced mechanically by `cargo run -p vox-arch-check`, driven by
`docs/src/architecture/layers.toml` (per-crate `layer` 0-5, `max_loc`,
`max_dependents`, `[[known_inversions]]`). Lower layers may not depend on
higher ones; violations are CI errors unless explicitly allow-listed. The
concept→crate lookup table is `docs/src/architecture/where-things-live.md` —
treat it as the canonical index and this document as the narrative summary.

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| `vox-compiler` | Lexer, parser, AST→HIR, typechecker, interpreter (`--interp`), the `.vox` frontend pipeline | `crates/vox-compiler/src/pipeline.rs` |
| `vox-codegen` / `vox-codegen-ts` | HIR → WebIR/RuntimeProjection/ShellProjection; TypeScript emission | `crates/vox-codegen/src/projection_bundle.rs`, `crates/vox-codegen-ts/src/` |
| `vox-orchestrator` | Multi-agent file-affinity router; D1-D10 policy modules (tier cascade, circuit breaker, budget gate, compaction, etc.) | `crates/vox-orchestrator/src/orchestrator_policy.rs` |
| `vox-orchestrator-mcp` | MCP tool surface + HTTP gateway; dispatches tool calls into `vox-orchestrator` | `crates/vox-orchestrator-mcp/src/dispatch.rs` |
| `vox-orchestrator-queue` | Locks, oplog, affinity tracking backing the orchestrator | `crates/vox-orchestrator-queue/src/` |
| `vox-actor-runtime` | Actor/mailbox/supervision runtime; LLM/Mens activity primitives; shell stdlib native codegen targets | `crates/vox-actor-runtime/src/builtins/mod.rs` |
| `vox-db` | Schema migrations + store ops over Turso/libSQL (`VoxDb` facade) | `crates/vox-db/src/` |
| `vox-cli` | `vox` binary: argv parsing (clap) + subcommand dispatch table | `crates/vox-cli/src/main.rs` |
| `vox-gui` | Tauri 2 desktop shell (Rust commands) + React frontend (`ui/`) | `crates/vox-gui/src/main.rs`, `crates/vox-gui/ui/src/App.tsx` |
| `vox-orchestrator-d` | Standalone orchestrator daemon binary | `crates/vox-orchestrator-d/src/bin/` |
| `vox-populi` | Multi-node mesh worker registry, HTTP control plane, MENS Candle QLoRA training/inference | `crates/vox-populi/src/` |
| `vox-search` | Local-first retrieval: hybrid memory search, repo inventory, Codex chunks | `crates/vox-search/src/` |
| `vox-scientia` | SCIENTIA research pipeline cluster (producers, replay, manuscript, critic-gate, dashboard) | `crates/vox-scientia/src/` |
| `vox-arch-check` | CI guard binary enforcing `layers.toml` | `crates/vox-arch-check/src/main.rs` |
| `vox-vcs` / `vox-git` | VCS backend abstraction (jj-lib) / pure-Rust Git bridge (gix) | `crates/vox-vcs/`, `crates/vox-git/` |

## Pattern Overview

**Overall:** Layered Rust workspace monorepo (Cargo workspace, 138 crates)
organized as a strict dependency DAG (L0 pure types → L5 binaries), with a
plugin system (L4, cdylib, loaded not linked) for concrete sandboxes/ML
backends. Domain logic is split into many small single-responsibility
crates rather than a few large ones — extraction crates like
`vox-orchestrator-queue`, `vox-orchestrator-driver`, `vox-cli-ci`,
`vox-cli-research` exist purely to keep god-crates under LoC/fan-in budgets.

**Key Characteristics:**

- Compiler pipeline is a classic lex→parse→AST→HIR→typeck→codegen chain
  (`crates/vox-compiler/src/pipeline.rs`), with a second interpreter
  execution path (`crates/vox-compiler/src/eval/`) for `--interp` mode.
- Agent orchestration is policy-module composition: a dozen independently
  testable `D1..D10` policy files (`crates/vox-orchestrator/src/*.rs`, e.g.
  `tier_cascade.rs`, `circuit_breaker.rs`, `budget_gate.rs`) are composed by
  `orchestrator_policy.rs` rather than one monolithic scheduler.
- Every capability the agent exposes (CLI subcommand, MCP tool, Tauri
  command) is a thin adapter over the same underlying L2/L3 domain crate —
  `vox-cli`, `vox-orchestrator-mcp`, and `vox-gui` all call into
  `vox-orchestrator`/`vox-db`/`vox-compiler` rather than reimplementing logic.
- Plugins are architecturally firewalled: `vox-plugin-*` crates depend on
  `vox-plugin-api` only and are excluded from the root workspace's L0-L3
  compile graph — they are discovered/loaded at runtime, never linked in.
- SSOT-driven codegen is pervasive: many crates parse a `contracts/**/*.yaml`
  or `.json` file as the single source of truth and generate Rust/docs/CLI
  surfaces from it (drift is CI-gated via `vox ci ssot-drift`).

## Layers

**L0 — Pure types:**

- Purpose: data-only structs/enums, zero async, zero DB, serde-only.
- Location: `crates/vox-ast/`, `crates/vox-db-types/`, `crates/vox-orchestrator-types/`, `crates/vox-mesh-types/`, `crates/workspace-hack/`
- Contains: AST node types, row/ID newtypes, agent/task ID types, mesh wire types.
- Depends on: other L0 crates only.
- Used by: everything above it.

**L1 — Primitives & utilities:**

- Purpose: cross-cutting infrastructure with no domain knowledge.
- Location: `crates/vox-foundation/`, `crates/vox-crypto/`, `crates/vox-telemetry/`, `crates/vox-bounded-fs/`, `crates/vox-http-client/`, `crates/vox-secrets/`
- Contains: trace IDs, backoff, crypto primitives (sole crypto SSOT), telemetry facade, secret resolution.
- Depends on: L0.
- Used by: L2+.

**L2 — Domain libraries:**

- Purpose: pure-data domain logic without heavy runtimes.
- Location: `crates/vox-config/`, `crates/vox-eval/`, `crates/vox-journal/`, `crates/vox-llm-egress/`, `crates/vox-repository/`, `crates/vox-capability-registry/`
- Depends on: L0-L1.
- Used by: L3+.

**L3 — Heavy domain runtimes:**

- Purpose: crates with DBs, compilers, orchestration, or heavy async runtimes.
- Location: `crates/vox-compiler/`, `crates/vox-orchestrator/`, `crates/vox-orchestrator-mcp/`, `crates/vox-actor-runtime/`, `crates/vox-db/`, `crates/vox-populi/`, `crates/vox-search/`, `crates/vox-scientia/`, `crates/vox-plugin-host/`
- Depends on: L0-L2 (and other L3 crates within-layer).
- Used by: L5 surfaces (and L4 plugins via `vox-plugin-api` only).

**L4 — Concrete plugins:**

- Purpose: cdylib consumers loaded at runtime — ML backends, sandboxes, GPU probes, browser automation.
- Location: `crates/vox-plugin-*/` (e.g. `vox-plugin-mens-candle-cuda`, `vox-plugin-browser`, `vox-plugin-runtime-wasm`)
- Depends on: `vox-plugin-api` / `vox-plugin-sdk` only — never `vox-orchestrator` or `vox-cli`.
- Used by: nothing at compile time; discovered via `vox-plugin-host` at runtime.

**L5 — Surfaces:**

- Purpose: product-shipped binaries and end-to-end test harnesses.
- Location: `crates/vox-cli/`, `crates/vox-gui/`, `crates/vox-orchestrator-d/`, `crates/vox-langtool/`, `crates/voxup/`, `crates/vox-audit/`, `crates/vox-cli-tests/`
- Depends on: everything below.
- Used by: end users / CI.

## Data Flow

### Primary Compile Path (`vox build` / `vox check`)

1. CLI parses argv, dispatches to `commands::build` (`crates/vox-cli/src/main.rs`)
2. Frontend runs lex → parse → resolve intra-project imports → typecheck, producing `FrontendResult` (`crates/vox-compiler/src/pipeline.rs:run_frontend_str_with_options`)
3. AST is lowered to HIR (`crates/vox-compiler/src/hir/lower/`)
4. HIR is assembled into a projection bundle (WebIR + AppContract + RuntimeProjection + ShellProjection + RequiredRuntimeCapabilities) (`crates/vox-codegen/src/projection_bundle.rs:project_bundle_from_hir`)
5. Target emitters consume the bundle: TypeScript (`crates/vox-codegen-ts/src/`), Rust native (`crates/vox-codegen/src/codegen_rust/`)
6. Diagnostics are formatted for terminal or `--json` (`crates/vox-compiler/src/pipeline.rs:format_diagnostics_json`)

### Interpreter Path (`vox run --interp`)

1. Same frontend (lex/parse/typeck) as above.
2. Execution goes through `Interpreter` in `crates/vox-compiler/src/eval/mod.rs` instead of codegen — `VoxValue` uses `Rc` + copy-on-write, no GC, intentionally `!Send` (single-threaded).
3. Capabilities are enforced per-script via `CapabilitySet` (`crates/vox-compiler/src/eval/caps.rs`); every embedder (CLI `run`/`repl`, `vox-langtool`, `vox-terminal-core`, `vox-orchestrator-mcp` workspace dispatch) assigns `Interpreter::caps` explicitly.
4. Shell-tier `std.*` builtins are mirrored between the interpreter (`crates/vox-compiler/src/eval/shell_stdlib.rs`) and native codegen (`crates/vox-actor-runtime/src/builtins/mod.rs`) via shared types in `vox-shell-stdlib-types` (no direct crate dep — Cargo cycle avoidance).

### Agent/MCP Request Path

1. A client (Claude Code, GUI, CLI) sends an MCP tool call, received by `crates/vox-orchestrator-mcp/src/dispatch.rs`.
2. Dispatch resolves the tool name to a handler in one of the `*_tools.rs` files (e.g. `git_tools.rs`, `agent_tools.rs`, `chat_tools/`) and applies timeouts/error-class wrapping (`dispatch_timeout.rs`).
3. Handlers call into `vox-orchestrator` (`Orchestrator`, built repo-scoped via `crates/vox-orchestrator/src/bootstrap.rs::build_repo_scoped_orchestrator`), which applies D1-D10 policy modules (tier cascade, budget gate, risk matrix, etc.) before dispatching to an LLM provider via `vox_actor_runtime::llm`.
4. Task/queue state is persisted through `vox-orchestrator-queue` (locks/oplog/affinity) and `vox-db`.
5. Results return as `ToolResult` JSON envelopes back through MCP.

### GUI Path

1. React frontend (`crates/vox-gui/ui/src/App.tsx`) invokes Tauri commands via `transport.ts`.
2. Rust command handlers in `crates/vox-gui/src/commands/` (e.g. `control_plane.rs`, `pty.rs`, `browser.rs`, `scientia_review.rs`) call into `vox-orchestrator`, `vox-db`, or spawn PTY/subprocess work.
3. Long-lived state lives in `GuiState` (`crates/vox-gui/src/commands/app_state.rs`), a `Mutex`-guarded struct set up in `main.rs` and injected via `tauri::Builder::manage`.
4. Streaming/event data (task updates, terminal blocks) flows back to the frontend via Tauri events, not polling.

**State Management:**

- Orchestrator/task state is DB-backed (`vox-db`, SQLite/libSQL via Turso) plus an in-memory oplog/lock layer (`vox-orchestrator-queue`) for fast-path coordination.
- Interpreter state is `Rc`-based in-process value graph, not persisted.
- GUI state is a single `Mutex<GuiState>` Tauri-managed singleton per process.
- Durable workflows persist via `vox-journal` (append-only JSONL, crash-safe via per-record sync) — see ADR-019/021/041.

## Key Abstractions

**`Orchestrator` (repo-scoped agent router):**

- Purpose: routes tasks to models/agents, applies policy gates, tracks affinity.
- Examples: `crates/vox-orchestrator/src/lib.rs`, `crates/vox-orchestrator/src/bootstrap.rs`
- Pattern: constructed once per repository via `build_repo_scoped_orchestrator[_for_repository]`, shared identically across CLI/MCP/daemon/test surfaces so config and memory paths never drift between entry points.

**`VoxValue` (interpreter runtime value):**

- Purpose: represents all interpreted `.vox` values with copy-on-write semantics.
- Examples: `crates/vox-compiler/src/eval/value.rs`
- Pattern: `Rc<Vec<..>>`/`Rc<HashMap>` scope frames + `Rc::make_mut` CoW; no garbage collector, no `Send` (interpreter is single-threaded by design).

**`CapabilitySet` (interpreter sandboxing):**

- Purpose: receiver-imposed capability gating (`--caps` CLI flag / `// vox:caps` pragma) restricting fs/net/etc access per script.
- Examples: `crates/vox-compiler/src/eval/caps.rs`
- Pattern: every production embedder assigns `Interpreter::caps` explicitly right after construction — never left as an implicit default.

**Policy modules (D1-D10, orchestrator decision points):**

- Purpose: each file is one independently-testable orchestration decision (model tier, plan-mode trigger, risk escalation, budget gate, compaction strategy, calibration).
- Examples: `crates/vox-orchestrator/src/tier_cascade.rs`, `circuit_breaker.rs`, `confidence_fusion.rs`, `budget_gate.rs`, `risk_matrix.rs`, `calibration.rs`
- Pattern: composed by `orchestrator_policy.rs` as a façade; new policy = new file + registration, never inline logic added to the façade itself.

**Projection bundle (compiler → runtime surface contract):**

- Purpose: one HIR-derived struct (`project_bundle_from_hir`) assembles everything downstream emitters need (WebIR, AppContract, RuntimeProjection, ShellProjection, RequiredRuntimeCapabilities) so TypeScript/Rust/Tauri packaging never diverge.
- Examples: `crates/vox-codegen/src/projection_bundle.rs`
- Pattern: single SSOT assembly function; emitters consume the bundle, never re-derive projections from HIR directly.

**SSOT contracts + generated artifacts:**

- Purpose: a `contracts/**/*.{yaml,json}` file is the source of truth; Rust types/docs/CLI output are generated and drift-checked, never hand-edited.
- Examples: `contracts/policy/policy-registry.v1.yaml` → `vox-config::policy::registry`; `layers.toml` → `vox-arch-check`; `contracts/scientia/*.schema.json` → `vox-scientia-jsonschema-codegen`.
- Pattern: generator crate/script + `vox ci ssot-drift` gate; never hand-regenerate after merge (auto-regen bot handles it).

## Entry Points

**`vox` CLI (`crates/vox-cli/src/main.rs`):**

- Location: `crates/vox-cli/src/main.rs`, dispatch table documented at the top of that file.
- Triggers: user/agent invokes `vox <subcommand>` in a terminal or scripted CI step.
- Responsibilities: argv parsing (clap), ML-command interception/delegation to `vox-ml-cli`, subcommand dispatch into `crates/vox-cli/src/commands/`.

**Vox GUI (`crates/vox-gui/src/main.rs`):**

- Location: `crates/vox-gui/src/main.rs` (Rust/Tauri) + `crates/vox-gui/ui/src/main.tsx` (React).
- Triggers: desktop app launch; supports `--drive-headless` and `--print-action-manifest-json` CLI modes for automation/testing.
- Responsibilities: Tauri builder setup, plugin registration (shell, dialog), `GuiState` construction, command handler registration via `generate_handler!`.

**Orchestrator daemon (`crates/vox-orchestrator-d/src/bin/`):**

- Location: `crates/vox-orchestrator-d/src/bin/`
- Triggers: long-running background process managing agent tasks outside CLI/GUI process lifetime.
- Responsibilities: hosts `orch.*` RPCs (e.g. `list_tasks`, `edit_task`) consumed by GUI Tauri wrappers and CLI.

**MCP tool server (`crates/vox-orchestrator-mcp/src/dispatch.rs`):**

- Location: `crates/vox-orchestrator-mcp/src/dispatch.rs` (+ `src/bin/gui-visual-review.rs` for the standalone visual-review binary)
- Triggers: MCP client (Claude Code, other agent harnesses) issuing a tool call over stdio/HTTP.
- Responsibilities: tool-name → handler resolution, timeout wrapping, error-class normalization, JSON envelope responses.

**Vox LSP (`crates/vox-lsp/`):**

- Location: `crates/vox-lsp/src/`
- Triggers: editor connects over stdio JSON-RPC.
- Responsibilities: language server capabilities (see `docs/src/architecture/vox-lsp-capabilities-ssot-2026.md`).

## Architectural Constraints

- **Threading:** The `.vox` interpreter (`vox-compiler/src/eval/`) is intentionally single-threaded — `VoxValue` is `!Send`. Async/multi-threaded work (orchestrator, MCP, DB) runs on Tokio (`#[tokio::main]` in `vox-cli`, `vox-gui`, `vox-orchestrator-d`).
- **Global state:** `GuiState` in `crates/vox-gui/src/commands/app_state.rs` is a single process-wide `Mutex`-guarded singleton managed by Tauri. `vox-telemetry::config::TelemetryConfig` reads a process-wide master switch (org policy → env var → default).
- **Circular imports:** `vox-compiler` and `vox-actor-runtime` cannot depend on each other directly (would cycle), so shell-tier stdlib types are factored out into the zero-dep `vox-shell-stdlib-types` L0 crate and each side implements its own mirror of the builtins — see `docs/src/architecture/vox-shell-stdlib-ssot-2026.md`. Similarly `vox-language-surface` was extracted to avoid a `vox-compiler` ↔ `vox-grammar-export` cycle.
- **Plugin isolation:** L0-L3 crates must never take a compile-time dependency on any `vox-plugin-*` (L4) crate — plugins are cdylib-loaded at runtime only, enforced by `vox-arch-check`.
- **Layer enforcement is mechanical, not conventional:** `cargo run -p vox-arch-check` fails CI on any lower→higher dependency edge not explicitly listed in `layers.toml`'s `[[known_inversions]]`.

## Anti-Patterns

### Adding a new crate-to-crate edge without updating `layers.toml`

**What happens:** A crate takes a new workspace dependency edge without a corresponding row/edge update.
**Why it's wrong:** `vox ci crate-edges` and `vox-arch-check` both fail — the edge-set is ratcheted against `contracts/ci/crate-edges.allow.v1.json`, and `exceptions` entries are user-authorized-only (an agent must never self-approve one).
**Do this instead:** Prefer a narrower `-types`/`-core` crate, or apply the defactor policy (duplicate a <50-line helper with a `// vox:defactored-from` comment) instead of taking the edge. See root `AGENTS.md` §Dependency Discipline.

### Hand-regenerating a generated/SSOT file after a merge

**What happens:** A generated artifact (e.g. `docs/src/reference/cli-command-surface.generated.md`, `Cargo.lock`) is edited by hand to "fix" drift after merging.
**Why it's wrong:** The `ssot-autoregen` CI job already regenerates and commits these mechanically; a manual `fix(#N): regenerate … after merge` pattern historically produced 60+ redundant commits.
**Do this instead:** Fix the generator or its input at the source; let CI regenerate.

### A plugin crate depending on `vox-orchestrator` or `vox-cli`

**What happens:** An L4 plugin crate (`crates/vox-plugin-*/`) adds a compile-time dependency on an L3/L5 host crate.
**Why it's wrong:** Breaks the plugin isolation invariant — plugins must be loadable/replaceable without recompiling the host; a compile-time dep defeats that and inverts the layer graph.
**Do this instead:** Depend only on `vox-plugin-api` / `vox-plugin-sdk`; communicate with the host through the plugin ABI.

## Error Handling

**Strategy:** `anyhow::Result` at binary/CLI boundaries (`main.rs` returns `anyhow::Result<()>`); typed errors (`thiserror`-style enums) within domain crates for cases callers need to match on (e.g. MCP `error_class_from_err` in `crates/vox-orchestrator-mcp/src/dispatch.rs`).

**Patterns:**

- MCP tool results are wrapped in a `ToolResult<T>` JSON envelope with `.err(e)` / `.to_json()` rather than raising raw exceptions across the tool boundary (`crates/vox-orchestrator-mcp/src/dispatch.rs`).
- Compiler diagnostics are collected into a `FrontendResult` (errors + warnings) rather than short-circuiting on first error, then formatted for terminal or `--json` output (`crates/vox-compiler/src/pipeline.rs`).
- Interpreter capability violations are gated before execution via `CapabilitySet`, not caught after the fact.

## Cross-Cutting Concerns

**Logging:** `tracing` + `tracing_subscriber` everywhere; GUI backend routes logs to stderr with `RUST_LOG` (default `info,vox_gui=debug`) in `crates/vox-gui/src/main.rs`. Structured telemetry (distinct from debug logging) goes through `vox-telemetry`'s `record_event!` macro and `TelemetryRecorder` trait, with OTLP egress gated behind `vox-telemetry-otlp` and a master on/off switch.

**Validation:** JSON Schema validation is centralized in `vox-jsonschema-util` for contracts/tooling; compiler-side validation is the typechecker (`crates/vox-compiler/src/typeck/`) plus interpreter capability checks (`crates/vox-compiler/src/eval/caps.rs`).

**Authentication:** Secrets/credentials resolve exclusively through `vox_secrets::resolve_secret(...)` (`crates/vox-secrets/`) — no direct `env::var` reads for sensitive values anywhere in the workspace (enforced convention, see root `AGENTS.md` §Secret Management). Mesh/agent identity uses `vox-identity` (signing keys, trust ledger) and `vox-mesh-transport` (iroh QUIC endpoint trust allowlist).

---

*Architecture analysis: 2026-09-22*
