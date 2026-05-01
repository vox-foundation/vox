---
title: "AGENTS.md"
description: "Documentation for AGENTS.md."
category: "architecture"
status: "current"
training_eligible: true
training_rationale: "Project architecture context."
---
# Agents Policy (Cross-Tool, Session-Critical)

This file is the always-loaded policy surface for contributors and coding agents.
Keep it short, stable, and implementation-oriented.

## Scope

- Use this file for non-negotiable project policy that should apply in every session.
- Do not turn this file into a full repository table of contents.
- Put detailed rationale and long reference maps in contributor docs, then link from here.

Primary navigation:

- Contributor entry point: [`docs/src/contributors/contributor-hub.md`](docs/src/contributors/contributor-hub.md)
- Documentation authority map: [`docs/src/contributors/documentation-governance.md`](docs/src/contributors/documentation-governance.md)
- Architecture map: [`docs/src/architecture/architecture-index.md`](docs/src/architecture/architecture-index.md)
- Classification SSOT: [`docs/src/architecture/classification-ssot-2026.md`](docs/src/architecture/classification-ssot-2026.md)
- Agent discovery index: [`docs/src/.well-known/llms.txt`](docs/src/.well-known/llms.txt)

## Research and Documentation Storage (IDE Agent Directive)

When working under the Vox repository, ALL research findings, architecture documents, and knowledge artifacts MUST be written to `docs/src/architecture/` (or the appropriate `docs/src/` subdirectory) in this repository — **not** to any IDE-private knowledge base (e.g., Antigravity's `~/.gemini/antigravity/knowledge/`). The Vox `docs/` tree is the single source of truth for all project knowledge.

- Research docs follow the naming pattern: `*-research-2026.md`, `*-findings-2026.md`
- Architecture SSoT docs: `*-ssot.md` or descriptive names in `docs/src/architecture/`
- After writing to `docs/`, update [`docs/src/architecture/research-index.md`](docs/src/architecture/research-index.md)
- Do not store Vox-specific research in IDE knowledge bases that are only accessible to one tool

## AI Context Exclusion (SSOT)

`.voxignore` is the **single source of truth** for what files and directories should be excluded from AI context.

- Edit `.voxignore`; derive `.cursorignore`, `.aiignore`, `.aiexclude` via `vox ci sync-ignore-files`
- Do **not** edit derived ignore files directly — they are regenerated and tracked for drift
- GitHub Copilot exclusions must be configured separately in GitHub Settings → Copilot → Content exclusion; see [`docs/agents/copilot-exclusions.md`](docs/agents/copilot-exclusions.md)
- Research: [`docs/src/architecture/multi-repo-context-isolation-research-2026.md`](docs/src/architecture/multi-repo-context-isolation-research-2026.md)

## Telemetry trust (SSOT)

- Map and boundaries: [`docs/src/architecture/telemetry-trust-ssot.md`](docs/src/architecture/telemetry-trust-ssot.md); optional explicit remote upload: [`docs/src/adr/023-optional-telemetry-remote-upload.md`](docs/src/adr/023-optional-telemetry-remote-upload.md), [`docs/src/architecture/telemetry-remote-sink-spec.md`](docs/src/architecture/telemetry-remote-sink-spec.md), **`vox telemetry`**
- Research: [`docs/src/architecture/telemetry-unification-research-findings-2026.md`](docs/src/architecture/telemetry-unification-research-findings-2026.md)
- Implementation plan + checklist: [`docs/src/architecture/telemetry-implementation-blueprint-2026.md`](docs/src/architecture/telemetry-implementation-blueprint-2026.md), [`docs/src/architecture/telemetry-implementation-backlog-2026.md`](docs/src/architecture/telemetry-implementation-backlog-2026.md)

## Secret Management (Required, SSOT)

Use Clavis for API keys, tokens, and credentials. Do not introduce new direct secret reads from environment variables in consumers.

- Define and maintain secret metadata in `crates/vox-clavis/src/spec.rs`.
- Resolve secrets with `vox_clavis::resolve_secret(...)`.
- Keep resolver/source behavior in `crates/vox-clavis/src/resolver.rs` and `crates/vox-clavis/src/sources/*`.
- After secret-surface changes, run `vox ci secret-env-guard` and `vox ci clavis-parity`.

Naming policy:

- Use `VOX_*` for Vox-owned platform boundaries.
- Keep provider-native keys as canonical when upstream compatibility matters.
- Mark migration aliases as deprecated and expose them through doctor warnings.

API key lifecycle checklist:

1. Add `SecretId` and `SecretSpec` entries in `crates/vox-clavis/src/spec.rs`.
2. Migrate callsites to `vox_clavis::resolve_secret(...)`.
3. Update `vox clavis doctor` workflow/profile expectations when requirements change.
4. Keep docs in sync at `docs/src/reference/clavis-ssot.md`.

## Cryptography Policy (SSOT)

All cryptographic logic MUST use the `vox-crypto` crate. We explicitly ban AEGIS, `ring`, `zig`-chains, and any wrapper dragging in `cmake` or `nasm` for C-assembly optimization on Windows. Pure-Rust `chacha20poly1305` is standard for AEAD. See:
- Policy: [`docs/src/architecture/cryptography-ssot-2026.md`](docs/src/architecture/cryptography-ssot-2026.md)

## VoxScript-First Glue Code (Required)

All project automation — CI prep, corpus transforms, training pipelines, install helpers, data migrations — MUST be written as `.vox` files and executed via `vox run`. Do **not** introduce new `.ps1`, `.sh`, or `.py` glue scripts.

**Why:** A single `vox run scripts/foo.vox` command shape is:
- Type-checked by the Vox compiler before execution (`vox check scripts/foo.vox`)
- Cross-platform (same file, same command on Windows/Linux/macOS)
- Auditable via `vox.script.*` telemetry events
- Free from shell-specific approval friction in IDE allowlists
- A natural addition to the MENS training corpus

**Execution tiers** (choose based on need):

| Need | Command | Notes |
|---|---|---|
| Pure computation, fast startup | `vox run --interp scripts/foo.vox` | No compile step; ~50ms cold start |
| File I/O, subprocess | `vox run scripts/foo.vox` | Native tier; content-hash cached |
| Untrusted / sandboxed | `vox run --isolation wasm scripts/foo.vox` | Wasmtime WASI; explicit `--wasi-dir` |

**Bootstrap exception:** `scripts/windows/vox-dev.ps1` and `scripts/vox-dev.sh` are **retained as thin launchers only** (≤10 lines of primary logic where possible). They forward to `cargo run -p vox-cli -- run <args>` to solve the chicken-and-egg problem of needing `vox` to run `.vox` before `vox` is built.

**Security invariants:**
- Scripts that modify the Vox repository MUST be committed to VCS before an agent executes them
- No `.vox` script may use `shell_exec` to bypass the compiler sandbox
- Subprocess calls go through `vox-runtime` process primitives (telemetry-observable)
- Use Clavis (`clavis.resolve(...)`) for secrets — never `env.get("MY_KEY")` for sensitive values

**Do NOT use Python or shell for glue.** Vox is the glue language. Python and shell are retired glue surfaces in this repository.

Full rationale, execution tier map, security model, and migration plan: [`docs/src/architecture/vox-as-glue-research-2026.md`](docs/src/architecture/vox-as-glue-research-2026.md)

## Grammar Unification (Vox Source Syntax)

Vox source follows one rule for top-level declarations:

> **Bare-keyword blocks declare scope. Decorators modify declarations.**

**Bare-keyword blocks** (each opens a scope with its own rules):
`type`, `fn`, `component`, `state_machine`, `routes`, `module`, `actor`,
`workflow`, `activity`.

**Decorators** (modifiers composed on top of a declaration):
`@table`, `@endpoint`, `@pure`, `@deprecated`, `@require`, `@mcp.tool`,
`@durable`, `@v0`, `@test`, `@scheduled`.

Decorators compose with bare-keyword blocks:

```vox
@table type Task { … }                        // decorator on a type declaration
@endpoint(kind: query) fn list_tasks() { … }  // decorator on a function
@durable fn process_order(id: OrderId) { … }  // durability via decorator, not keyword
```

**Rule for new features:** Do NOT introduce a new bare keyword for behavior
that can be expressed as a decorator. New execution semantics (durability,
tracing, sandboxing, rate-limiting) belong as decorators on `fn`.

**Implementation status (Phase 2):** `actor`, `workflow`, and `activity` are
tombstoned at the parser level — source files cannot use these forms.
TASK-2.6 has landed (commit `e7f3e884`): `ActorDecl`/`WorkflowDecl`/`ActivityDecl`
AST types, all corresponding HIR types, codegen emit paths, and typeck/walker
registrations were removed. The compiler now rejects these keywords with a
tombstone error. The decorator-equivalent forms (`@durable fn`, `@actor fn`) are
**not yet implemented** and are future work beyond TASK-2.6.

See: [`docs/src/architecture/gui-native-roadmap-status-2026.md`](docs/src/architecture/gui-native-roadmap-status-2026.md) §Phase 2.

## Cross-Platform Shell Discipline (Stable Rules)

> **Scope of the PowerShell preference.** The PS-first stance below is a **host-side allowlisting and output-parsing** preference, not a claim that agents produce better code in PowerShell than Bash. Project automation is **Vox** (see §VoxScript-First Glue Code). See [`docs/src/architecture/terminal-exec-policy-ssot.md`](docs/src/architecture/terminal-exec-policy-ssot.md) for what this policy does and does not claim.

- **PowerShell 7 (`pwsh`) when available:** On any host where `pwsh` is installed, prefer it for the **two retained launcher files** and for interactive terminal work, so behavior matches [`contracts/terminal/exec-policy.v1.yaml`](contracts/terminal/exec-policy.v1.yaml) and [`vox shell check`](docs/src/reference/cli.md). On Windows, PowerShell is the default expectation even when only Windows PowerShell 5.1 (`powershell.exe`) is present.
- **CI vs local:** Repository CI jobs often run under **bash** on Linux self-hosted runners ([`docs/src/ci/runner-contract.md`](docs/src/ci/runner-contract.md)); that does not override the **local/agent** preference for `pwsh` when you have it.
- Prefer structured tooling and project CLIs (`vox`, `cargo`, `pnpm`, `rg`) over ad hoc shell pipelines. **`uv` and Python are no longer preferred** for project automation — use `vox run` instead.
- **Dev launcher when `vox` is missing from `PATH`:** [`scripts/windows/vox-dev.ps1`](scripts/windows/vox-dev.ps1) / [`scripts/vox-dev.sh`](scripts/vox-dev.sh) — forwards argv to `vox` via `cargo run -p vox-cli` from the workspace root (optional env: `VOX_REPO_ROOT`, `VOX_USE_PATH=1`, `VOX_DEV_FEATURES`, `VOX_DEV_QUIET=1`). See [`docs/src/reference/cli.md`](docs/src/reference/cli.md) (heading **Bootstrap / dev launcher (missing `vox` on `PATH`)**).
- Do not rely on shell-specific one-liners as policy boundaries; approvals and allowlists vary across IDEs.
- Keep commands decomposed into clear steps when safety or portability is at risk.

Environment-specific overlays (for example Antigravity on Windows) add stricter command-shape rules on top of this base; see [`GEMINI.md`](GEMINI.md). If Claude Code is in use, also see [`CLAUDE.md`](CLAUDE.md) for Claude-specific additions.
For Cursor-specific rules see [`.cursor/rules/`](.cursor/rules/) — four `.mdc` rule files control build environment, CI runner conventions, CLI registry, and source hygiene.

Live SSOT (scoped claim + evidence): [`docs/src/architecture/terminal-exec-policy-ssot.md`](docs/src/architecture/terminal-exec-policy-ssot.md). Optional A/B eval design (not run, not required): [`docs/src/architecture/agent-shell-fluency-eval-design-2026.md`](docs/src/architecture/agent-shell-fluency-eval-design-2026.md). Archived 2026-Q1 background research is under `docs/src/archive/research-2026-q1/` — do not ingest autonomously per §Archival Protocol.

## Markdown Hygiene and Code Snippets (Doctest Policy)

- All ````vox``` blocks in documentation must compile cleanly via `vox-doc-pipeline`'s dynamic doctest runner.
- Always write inline ````vox``` blocks, do NOT use mdBook `{{#include}}` directives for new code.
- If rendering invalid code for illustrative reasons, disable validation explicitly by using `// vox:skip` inside the snippet. 
- You MUST enforce syntax correctness programmatically over legacy include files. 

## Retired Surfaces (LLM Guard)

Do **NOT** use the following retired symbols, crates, or env vars. Using them will result in hallucinations and broken integration:

| Retired / Deprecated | Canonical Replacement (Use Instead) |
|---|---|
| `vox-dei` (old large orchestrator crate) | `vox-orchestrator` |
| `vox-ars` (crate) | `vox-skills` |
| `vox-gamify` | `vox-ludus` |
| `vox-lexer`, `vox-parser`, `vox-hir`, `vox-typeck` | `vox-compiler` (monolith) |
| `@component fn Name()` | `component Name() {}` |
| `TURSO_URL` / `VOX_TURSO_URL` / `VOX_TURSO_TOKEN` | `VOX_DB_URL` / `VOX_DB_TOKEN` |
| `recall()` (synchronous memory read) | `recall_async(query_spec)` |
| `persist_fact()` | `sync_to_db()` |

## Structural Limits & Code Quality

Agents and contributors must strictly adhere to architectural invariants. Ensure you verify against skeleton code limits (TOESTUB), God Object constraints, and maximum sprawl limits.

**Full details:** See [`docs/agents/governance.md`](docs/agents/governance.md) for the complete policy, line limits, and module freeze rules.

## Related Operational Surfaces

- CI and runner behavior: [`docs/src/ci/runner-contract.md`](docs/src/ci/runner-contract.md)
- Workspace artifact hygiene and governance policy: [`docs/agents/governance.md`](docs/agents/governance.md)
- Agent instruction architecture: [`docs/src/contributors/agent-instruction-architecture.md`](docs/src/contributors/agent-instruction-architecture.md)
- Continuation prompt strategy: [`docs/src/contributors/continuation-prompt-engineering.md`](docs/src/contributors/continuation-prompt-engineering.md)
- Machine-readable feature matrix: [`docs/agents/ai-ide-feature-matrix-2026.json`](docs/agents/ai-ide-feature-matrix-2026.json); full doc inventory: [`docs/agents/doc-inventory.json`](docs/agents/doc-inventory.json)

## Archival Protocol (LLM Guard)

Do **NOT** read, ingest, or attempt to modify files in the `archive/` or `docs/src/archive/` directories when planning new features or writing new code. These directories are tombstoned. They exist for manual human reference only. If an LLM includes an archived pattern in new code, it is considered a severe system hallucination. If you need historical context about an archived pattern, ask the human operator to retrieve it; do not ingest the archive autonomously. See [`docs/agents/governance.md`](docs/agents/governance.md) §Nomenclature for canonical migration paths.
