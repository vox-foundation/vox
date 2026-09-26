# Vox

## What This Is

Vox is a mature, actively-developed cross-platform Rust workspace (138 crates) that implements its own programming language — the `.vox` frontend, compiler, and interpreter — plus the surfaces built on top of it: `vox-cli` (the `vox` binary), `vox-gui` (a Tauri 2 desktop application), a multi-agent orchestrator with MCP tool exposure, a local-first mesh (iroh QUIC) for distributed agent/model work, and a native-Rust ML training stack (Candle/QLoRA) for its own fine-tuned model ("Mens"). It is developed and stewarded as a FOSS project under an in-formation Vox Foundation, not a monetized product.

This milestone is not a new-feature push — it is architectural housekeeping on an existing, working system: disposing of 20 audited dead/misplaced crates per a standing PRD, and closing out a handful of "current but not formally locked" architecture decisions that a full-corpus ADR/SPEC ingest surfaced as open.

## Core Value

The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout this cleanup — no crate disposition or architecture-decision closure is worth a broken workspace.

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

### Active

<!-- Current scope for this milestone. -->

- [ ] Dispose of the 20 crates audited in the dead-crate-fate plan: delete zero-consumer crates, wire up functionally-complete-but-unadopted crates, extract plugin-appropriate crates out of CORE, fix misplaced-tier crates, confirm frozen crates need no action, and verify catalog hygiene
- [x] Formally ratify ADR-045 (Tauri GUI replaces Axum dashboard) and verify `vox-gui`'s command surface is fully sourced from `vox-cli`'s `CommandCatalog` SSOT
- [x] Establish and enforce the documented boundary between Vox-native and React/TanStack interop UI primitives (external-frontend-interop-plan-2026; ADR-027 is superseded), and confirm the Tauri desktop-convergence clause of ADR-037 as complete
- [ ] Extend the locks subsystem with `ResourceLockManager` for multi-agent resource coordination (ADR-025)
- [ ] Ship HMAC tool-call receipts under the two-tier formal-intent verification system (ADR-029)
- [ ] Ship Pareto-frontier reporting for model scoreboards (ADR-046, reporting-only)
- [ ] Unify Candle/peft-rs/qlora-rs dependency versions via a GPU-CI-verified upgrade train (ADR-034)

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
| ADR-025: Multi-Agent Lock Coherence | Extends locks subsystem with `ResourceLockManager` for multi-agent contention | — Pending — this milestone implements it |
| ADR-029: Formal Intent and Tool Receipt Auditing | Two-tier verification with HMAC receipts for agent tool calls | — Pending — this milestone implements it |
| ADR-046: Pareto-Frontier Model Reporting | Reporting-only Pareto view over reliability/cost/latency; not a routing change | — Pending — this milestone implements it |
| ADR-034: Candle/QLoRA stack upgrades | Defers version unification to a dedicated GPU-CI-verified upgrade train | — Pending — this milestone runs that upgrade train |

---
*Last updated: 2026-09-22 after initial roadmap creation (full-corpus ADR/SPEC/PRD ingest)*
