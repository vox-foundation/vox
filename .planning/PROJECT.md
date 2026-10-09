# Vox

## What This Is

Vox is a mature, actively-developed cross-platform Rust workspace (138 crates) that implements its own programming language — the `.vox` frontend, compiler, and interpreter — plus the surfaces built on top of it: `vox-cli` (the `vox` binary), `vox-gui` (a Tauri 2 desktop application), a multi-agent orchestrator with MCP tool exposure, a local-first mesh (iroh QUIC) for distributed agent/model work, and a native-Rust ML training stack (Candle/QLoRA) for its own fine-tuned model ("Mens"). It is developed and stewarded as a FOSS project under an in-formation Vox Foundation, not a monetized product.

v1.0 (Architectural Housekeeping, shipped 2026-10-01) disposed of 20 audited dead/misplaced crates and closed the open architecture decisions a full-corpus ADR/SPEC ingest surfaced. The current milestone, v1.1 Research Trial Flywheel, turns Vox's existing research, telemetry, evaluation, model-routing, and retrieval infrastructure into a governed measure–compare–promote–replay loop for its research agents.

## Core Value

The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout this cleanup — no crate disposition or architecture-decision closure is worth a broken workspace.

## Current Milestone: v1.1 Research Trial Flywheel

**Goal:** Race research-agent conditions under preregistered, budgeted campaigns; rate answer quality, speed, and efficiency; accumulate verified, searchable knowledge; and propose — never silently apply — the next improvement.

**Milestone value:** Each research run produces reproducible evidence, comparable quality/latency/cost metrics, searchable verified knowledge, and a safe proposal for the next experiment.

**Target features:**
- One signed campaign/run identity (campaign, run, replicate, attempt, retry lineage) propagated through the research pipeline, eval runner, telemetry, and Tier A rows.
- `vox research eval` as a bounded-concurrency matrix runner with spend reservations, arm isolation, hard-gate + Pareto scorecards, group-sequential stopping, and offline replay.
- Verified findings persisted authoritatively with provenance, projected into memory and search, frozen per arm, and measured via memory ablations.
- Shadow challengers and dual-control, scope-bound champion designation with safety suspension — no automatic runtime mutation.

Reuses existing infrastructure; adds one trial identity instead of a parallel stack. No GUI/dashboard work.

## Requirements

### Validated

<!-- Shipped and confirmed valuable; inferred from the existing, working codebase. -->

- ✓ `.vox` compiler pipeline (lex → parse → HIR → typeck → codegen) and a separate `--interp` execution path — `crates/vox-compiler/`
- ✓ `vox` CLI with full subcommand surface (`crates/vox-cli/`)
- ✓ Multi-agent orchestrator with D1-D10 policy modules and MCP tool/HTTP exposure — `crates/vox-orchestrator/`, `crates/vox-orchestrator-mcp/`
- ✓ Tauri 2 desktop GUI (`vox-gui`) as the orchestration surface, replacing the retired Axum dashboard (`vox-dashboard` confirmed absent from the tree)
- ✓ iroh QUIC mesh transport for Populi (ADR-047, locked) replacing the earlier bespoke HTTP/JWT transport
- ✓ Durable functions runtime (`workflow`/`activity`/`actor`/`@scheduled`) with journal-backed replay (ADR-019/021/041, locked)
- ✓ Native Rust ML training (Candle + qlora-rs) replacing the earlier Python/Unsloth path (ADR-003/006, locked)
- ✓ Codex/Arca/Turso as the sole database stack (ADR-004, locked)
- ✓ Secrets resolution exclusively through `vox_secrets::resolve_secret(...)` (Clavis/`vox-secrets`)
- ✓ 20 audited dead/misplaced crates disposed (delete, wire-up, plugin extraction, reclassification, frozen confirmation) — v1.0
- ✓ ADR-045 ratified; `vox-gui` command surface sourced from `CommandCatalog`; Vox-native vs React interop boundary enforced — v1.0
- ✓ `ResourceLockManager` multi-agent resource coordination (ADR-025) — v1.0
- ✓ HMAC tool-call receipts under two-tier formal-intent verification (ADR-029) — v1.0
- ✓ Reporting-only Pareto frontier for model scoreboards (ADR-046) — v1.0
- ✓ Candle/peft-rs/qlora-rs unified and compile-validated by a fail-closed hosted CUDA lane (ADR-034) — v1.0

### Active

<!-- Current scope for v1.1 Research Trial Flywheel. Detailed, phase-mapped requirements live in REQUIREMENTS.md. -->

- [ ] Trial identity, signed campaign preregistration, structural telemetry, and atomic Tier A persistence with an observational baseline
- [ ] Bounded parallel condition matrix with spend reservations, arm isolation, hard-gate + Pareto scorecards, and replayable reports
- [ ] Verified, provenance-rich, searchable knowledge accumulation with frozen per-arm manifests and memory ablations
- [ ] Shadow challengers, two-campaign confirmation, and dual-control scoped champion designation
- [ ] Adversarial/stress trials, mutation testing of gates, and operational hardening

### Out of Scope

<!-- Explicit boundaries, with reasoning, surfaced by the full-corpus ADR/SPEC ingest. -->

- Hosted Mens / BaaS managed service (ADR-009) — explicitly scoped as "future design" in its own source doc; self-hosted remains the default. Revisit when there's a concrete hosting driver.
- SWC parser migration (ADR-035) — evaluation-only; the ADR itself requires a fresh sign-off ADR before any migration starts. No committed direction to build toward yet.
- Internal Web IR strategy as a standalone initiative (ADR-012) — its direction is already subsumed by the locked ADR-036 (HIR core + WebIR projections); treated as ongoing compiler-track work rather than a discrete milestone deliverable here.
- Re-litigating the ADR-030/ADR-031 "vox-dashboard" naming — both ADRs' core decisions (state_machine as reactive-state SSoT; vox-vscode deprecation) remain valid and locked; only their prose still names the retired `vox-dashboard` surface. No source edit needed per operator review; downstream readers should treat `vox-gui` as current.
- Transport crypto provider collapse (`reqwest` 0.12→0.13, `ring`/`aws-lc-rs` dedup) — real, documented, and unresolved (see `contracts/crypto/transport-providers.v1.json`), but it's a codebase-audit finding, not an ADR/SPEC-sourced decision this milestone's ingest scope covers. Tracked for a future milestone.

## Context

- **Scale**: 138 first-party crates under a strict L0→L5 layered Cargo workspace (`docs/src/architecture/layers.toml`, enforced by `vox-arch-check`); TypeScript/React only in `crates/vox-gui/ui/` (Tauri frontend) and `docs-astro/` (docs site).
- **Provenance of this milestone's scope**: a full-corpus ingest of 430 classified documents (49 ADRs, 96 SPECs, 1 PRD, 284 DOCs) from `docs/src/adr/` and `docs/src/architecture/`. Full synthesis at `.planning/intel/SYNTHESIS.md`; per-type detail in `.planning/intel/{decisions,requirements,constraints,context}.md`; conflict/warning detail in `.planning/INGEST-CONFLICTS.md`.
- **The one PRD in the corpus** (`docs/src/architecture/dead-crate-fate-plan-2026-05-08.md`) is a per-crate disposition plan from a 2026-05-08 workspace audit, covering 20 crates across six fates (DELETE, WIRE-UP-AS-IS, EXTRACT-TO-PLUGIN, KEEP-FROZEN, MISPLACED, catalog-cleanup).
- **Two ADR supersessions were resolved in the source tree before this ingest ran**: ADR-024 (Axum dashboard) → superseded by ADR-045 (Tauri GUI); ADR-037 (Tauri Convergence)'s mobile-target clause → partially superseded by locked `adr-NNN` (React Native + Expo + uniffi for mobile). Both are reflected in Validated/Active above.
- **ADR-045 is the current, non-contradicted dashboard/GUI decision but isn't itself locked/Accepted** (frontmatter `status: "current"`, no explicit Accepted line) — flagged as a WARNING during ingest, already reviewed by the operator, tracked here as an Active item to close rather than a blocker.
- **Known tech debt not in this milestone's scope** (from `.planning/codebase/CONCERNS.md`): 549 God-object findings from a stale (2026-06-06) `vox-arch-check` scan; duplicated dispatch logic between `vox-populi` and `vox-plugin-populi-mesh`; three deprecation markers (`retire-by="0.7.0"`) that will start failing CI the moment the workspace version bumps past 0.6.0; the `reqwest` 0.12/0.13 split noted in Out of Scope above.

## Constraints

- **Build toolchain**: pinned Rust `1.98.1` (`rust-toolchain.toml`); a clean clone must build with only the pinned toolchain, a platform C compiler, and Node+pnpm — no cmake, nasm, Go, perl, or libclang via any dependency (AGENTS.md Build-toolchain invariant).
- **Crate dependency edges**: CI-gated by `vox ci crate-edges` against `contracts/ci/crate-edges.allow.v1.json`; new edges need a narrower `-types`/`-core` crate or the <50-line defactor policy; `exceptions` ledger entries are user-authorized-only, never self-added — directly binds the EXTRACT-TO-PLUGIN and WIRE-UP dead-crate work.
- **Plugin isolation**: L0-L3 crates must never take a compile-time dependency on any `vox-plugin-*` (L4) crate; plugins depend only on `vox-plugin-api`/`vox-plugin-sdk` and are cdylib-loaded at runtime — binds every EXTRACT-TO-PLUGIN requirement.
- **LLM provider boundary**: all LLM calls go through `vox_actor_runtime::llm`; no direct vendor SDK/hostname in workspace code, enforced by the `llm_provider_call` `vox-code-audit` detector.
- **Cryptography**: application crypto exclusively through `vox-crypto`; MD5/SHA-1/AEGIS/DES/RC4/ECB permanently banned; transport TLS/QUIC crypto is allowlisted and ledgered separately (`contracts/crypto/transport-providers.v1.json`), never routed through `vox-crypto`.
- **Secrets**: all API keys/tokens resolve via `vox_secrets::resolve_secret(...)`; no new direct `env::var` reads for secrets.
- **Build coordination**: every `cargo` invocation routes through the machine-wide build broker (a `cargo`-named shim); never call the resolved cargo binary directly.
- **Test-first**: every new `pub fn` in `crates/*/src/**` needs an adjacent test (`skeleton/untested-pub-api` detector, pre-commit `tdd-guard` hook) — binds all six phases below, especially the wire-up and extraction work which touches call sites directly.
- **VoxScript-first glue**: new automation is `.vox` run via `vox run`, not `.sh`/`.ps1`/`.py` (two retained bootstrap launchers excepted).

## Key Decisions

<!-- Locked ADRs load-bearing for this milestone, plus the non-locked/open decisions this milestone acts on. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| ADR-045: Tauri GUI replaces Axum dashboard | `vox-dashboard` decommissioned and confirmed absent from the tree; `vox-gui` is the sole orchestration surface | ⚠️ Revisit — current & non-contradicted but not itself locked/Accepted; this milestone ratifies it |
| ADR-047: iroh QUIC replaces bespoke populi mesh transport | Retires hand-rolled HTTP/JWT control plane for a vetted P2P transport with NAT traversal | ✓ Good — locked, already the operative transport |
| ADR-041: Durable functions completion | Closes the parse-only stub gap with a working runtime, codegen, journal-backed replay | ✓ Good — locked, supersedes ADR-028's removal proposal |
| ADR-026: Third-party code provenance policy | AGPL guardrails and attribution for vendored code | ✓ Good — locked; directly governs how EXTRACT-TO-PLUGIN work vendors/attributes code |
| ADR-037: Tauri Convergence (desktop clause) | Desktop shell convergence on Tauri 2 remains Accepted; only the mobile clause was superseded | ⚠️ Revisit — this milestone confirms the desktop clause complete |
| adr-NNN: Scope Tauri to desktop only; RN+Expo+uniffi for mobile | Supersedes ADR-037's mobile clause specifically | ✓ Good — locked |
| ADR-027: Dual-Track UI Surfaces | Split UI primitives into Vox-native vs React/TanStack interop tracks. Superseded 2026-05-03 by external-frontend-interop-plan-2026 | Boundary established and enforced in Phase 4 under the superseding plan (`authoring_track` in contracts/frontend/surface-ownership.v1.yaml) |
| ADR-025: Multi-Agent Lock Coherence | Extends locks subsystem with `ResourceLockManager` for multi-agent contention | ✓ Good — implemented in v1.0 Phase 5 |
| ADR-029: Formal Intent and Tool Receipt Auditing | Two-tier verification with HMAC receipts for agent tool calls | ✓ Good — implemented in v1.0 Phase 5 |
| ADR-046: Pareto-Frontier Model Reporting | Reporting-only Pareto view over reliability/cost/latency; not a routing change | ✓ Good — closed in v1.0 Phase 6 |
| ADR-034: Candle/QLoRA stack upgrades | Versions verified unified; compile-validated by fail-closed hosted CUDA lane | ✓ Good — accepted 2026-10-01 (compile-only, no physical-GPU claim) |
| v1.1 trial governance (grill G1–G25, R1–R9) | Signed immutable campaigns, atomic spend reservations, group-sequential stats, two-campaign promotion, dual-control designation, structural-only telemetry, linked retries instead of resume | — Pending — v1.1 implements it |

## Workstreams
- `docs-freshness` — v1.3 Self-Maintaining Public Docs: the public site (voxlang.org, Starlight over `docs/src/`) stays accurate to the code without constant manual upkeep. Brief: `.planning/workstreams/docs-freshness/CONTEXT.md`. Phase 20 (deploy unblock + public-surface honesty) complete 2026-10-09: deploys live again, failures escalate to one edit-in-place issue, research/roadmap pages labelled Internals; next Phase 20.1.

---
*Last updated: 2026-10-09 after docs-freshness Phase 20*
