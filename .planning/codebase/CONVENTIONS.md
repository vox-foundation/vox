---
last_mapped_commit: 9a79f708de1ce8f8b3944dc99afb51a2877fb960
last_mapped_at: 2026-09-22
---
# Coding Conventions

**Analysis Date:** 2026-09-22

Vox is a large Rust workspace (138 crates under `crates/*`) with a TypeScript/React frontend at `crates/vox-gui/ui/`. Rust is the dominant surface by volume; conventions below cover both, plus `.vox` language source (governed separately by `AGENTS.md` §Vox Language Enforcement Rules — not duplicated here).

## Naming Patterns

**Rust files:**

- `snake_case.rs`, one primary concept per file (`bootstrap.rs`, `circuit_breaker.rs`, `attachment_manifest.rs` — see `crates/vox-orchestrator/src/`).
- Error types live in a dedicated `error.rs` or `store/types/error.rs` next to the module they serve (e.g. `crates/vox-db/src/store/types/error.rs`).
- Large modules split into subdirectories with `mod.rs`-free re-exports (`config/`, `context/`, `budget/` under `crates/vox-orchestrator/src/`) rather than one file growing unbounded — see the AGENTS.md structural-limits/TOESTUB governance.

**Rust crates:**

- All first-party crates are prefixed `vox-` (`vox-db`, `vox-search`, `vox-orchestrator-mcp`). Companion `-types`/`-core` crates split shared surfaces to keep dependency edges narrow (see root `AGENTS.md` §Dependency Discipline).

**Functions:** `snake_case`, verb-led (`remote_creds`, `atomic_write_secure`, `memory_config_for_state`). Boolean predicates read as questions in effect (`try_consume`, `is_err()` call sites), not prefixed `is_`/`has_` by hard rule but common where it reads naturally.

**Variables:** `snake_case`; short-lived locals (`cfg`, `db`, `dir`) are fine in tight scopes, descriptive names required once a function exceeds a few lines.

**Types:** `PascalCase` for structs/enums (`StoreError`, `MemoryStoreParams`, `TavilySessionBudget`). Params structs for MCP/tool boundaries are suffixed `Params` (`MemoryStoreParams`, `KnowledgeQueryParams`, `ResearchRunParams` — `crates/vox-orchestrator-mcp/src/memory_tools/params.rs`). Error enums are suffixed `Error` (`StoreError`, `CircuitBreakerError`).

**TypeScript/React files (`crates/vox-gui/ui/src/`):**

- Components: `PascalCase.tsx` with a co-located `PascalCase.test.tsx` (`Button.tsx` + `Button.test.tsx`, `Dialog.tsx` + `Dialog.test.tsx` — `crates/vox-gui/ui/src/components/ui/`).
- Non-component modules (transport, stores, utils): `camelCase.ts` (`transport.console.test.ts`).
- Playwright e2e specs: `kebab-case.spec.ts` under `crates/vox-gui/ui/e2e/` (`chat-composer-dock.spec.ts`, `dashboard-pilot.spec.ts`).

## Code Style

**Formatting:**

- `rustfmt` via workspace root `rustfmt.toml` — deliberately minimal (`edition = "2024"` only); the comment in the file explains defaults are made explicit so an edition bump can't silently reformat the tree, and that changing import grouping/brace style needs its own bulk-reformat commit. Do not add stylistic options without that discussion.
- Never run `cargo fmt --all` directly (Windows `CreateProcess` command-line overflow on this large workspace). Use `vox run scripts/fmt.vox` (dirty-file default) or `vox run scripts/fmt.vox -- --all` for a full sweep. See root `AGENTS.md` §VoxScript-First Glue Code.
- `lefthook.yml` `pre-commit.fmt-fix` auto-formats and re-stages staged `.rs` files on commit — a format slip should never reach a commit as a separate follow-up.
- No dedicated `.prettierrc`/eslint config in `crates/vox-gui/ui/` — the frontend `lint` script is `tsc --noEmit && vite build` (`crates/vox-gui/ui/package.json`); TypeScript's own formatting is left to editor defaults.

**Linting:**

- `clippy.toml` (workspace root) raises `too-many-arguments-threshold = 12` and `type-complexity-threshold = 600` — store/RPC-facade layers legitimately need wider signatures; don't treat the default clippy thresholds as the ceiling.
- `[workspace.lints.clippy] all = "warn"` and `[workspace.lints.rust] unsafe_code = "warn"` in root `Cargo.toml`; crates opt in with `[lints] workspace = true` in their own `Cargo.toml` (see `crates/vox-search/Cargo.toml`).
- Pre-push clippy is **not** run by the default/fast `vox ci pre-push` tier — only `--complete` and above. Run `cargo clippy -p <crate> -- -D warnings` before pushing Rust changes (see root `AGENTS.md` §Perennial Bug Patterns — this gap alone caused ~45 historical fixes).
- `vox-code-audit` runs additional structural detectors beyond clippy: `skeleton/untested-pub-api`, `skeleton/no-test-for-pub-fn` (Test-First Policy), `vox/crypto/banned-crate-import`, `llm_provider_call`, etc. — see root `AGENTS.md` for the full detector catalog.

## Import Organization

**Rust:** `rustfmt`'s default grouping (std → external crates → local `crate::`/`super::`), one `use` block per file, no manual re-ordering beyond what rustfmt enforces. Test modules commonly `use super::*;` to pull the parent module's items (`crates/vox-search/src/policy.rs` `mod tests`).

**Path re-exports:** workspace-internal crates are referenced by their `vox-*` name via `[workspace.dependencies]` in the root `Cargo.toml`, never by relative path outside the owning crate.

**TypeScript:** no enforced import-order tool; observed order in components is external packages (`react`, `vitest`, `@testing-library/react`) before relative imports (`./Button`).

## Error Handling

**Patterns:**

- Library/domain crates define a dedicated error enum with `thiserror::Error` (116 files use `thiserror::Error` derive across the workspace). Example: `crates/vox-db/src/store/types/error.rs` `StoreError` — one variant per failure class, `#[error("...")]` messages written as actionable sentences (often including the exact remediation command, e.g. `LegacySchemaChain` tells the caller to run `vox codex export-legacy`).
- Wrap underlying errors with `#[from]` + `#[error(transparent)]` for pass-through causes (`Turso(#[from] turso::Error)`, `CircuitBreaker(#[from] CircuitBreakerError)`); use a `String`-carrying variant (`Db(String)`, `Internal(String)`) only when there's no structured source error to wrap.
- Application/binary code (CLI commands, orchestration glue) uses `anyhow::Result` liberally (844 files reference `anyhow::`) — reserve `thiserror` enums for library boundaries that callers need to `match` on, use `anyhow` where the error is only ever surfaced to a human/log.
- `.expect("reason")` is standard in tests and in code paths proven unreachable by construction — always pass a message describing *why* it should hold, never a bare `.unwrap()` in new non-test code.

## Logging

**Framework:** `tracing` ecosystem is standard across orchestrator/runtime crates (see `vox-telemetry`, `vox-telemetry-otlp` in `[workspace.dependencies]`); do not introduce `println!`/`eprintln!` for anything beyond a CLI's user-facing output.

**Patterns:**

- Telemetry is a first-class cross-cutting concern — `vox.script.*` events, `vox_actor_runtime` process primitives, and `vox-audit` findings are all structured, not ad hoc log lines. See root `AGENTS.md` §VoxScript-First Glue Code and §Code Intelligence.

## Comments

**When to comment:**

- Module-level `//!` doc comments explain *why*, not just what, especially around retired/legacy surfaces and migration rationale (`crates/vox-config/src/bootstrap_inference.rs` documents why a fallback model list exists, when it was last verified, and how to re-verify it with a `curl` one-liner).
- Inline comments call out non-obvious invariants, dates, and links to the doc that has the full rationale (`docs/src/architecture/...`) rather than re-explaining in place.
- `// ponytail:` / `// vox-deprecated-since=... retire-by=... reason=... canonical=...` style structured markers are used for deliberate simplifications and tracked deprecations — grep for the marker rather than re-deriving intent.

**Doc comments (`///`):**

- Every `pub` item in library crates gets a `///` doc comment, one sentence minimum, often cross-referencing the owning subsystem or SSOT doc (`/// [`crate::DbCircuitBreaker`] is open...`). Public structs' fields are individually documented (see `MemoryStoreParams` in `crates/vox-orchestrator-mcp/src/memory_tools/params.rs` — every field has a `///` line, useful because these structs double as the JSON-Schema surface via `schemars::JsonSchema`).

## Function Design

**Size:** Small, single-purpose. Files stay `snake_case`-scoped to one concept; when a module grows past a comfortable size it's split into a subdirectory rather than growing the file (see `crates/vox-orchestrator/src/` subdirs). Structural limits are enforced by `vox-arch-check` (`docs/src/architecture/layers.toml` — fan-in/LoC budgets) — consult `docs/src/architecture/where-things-live.md` before adding a new file.

**Parameters:** Prefer a handful of positional args; when a function legitimately needs more (store/RPC facades), the workspace clippy threshold is intentionally raised to 12 rather than forcing an artificial builder — don't invent a builder pattern purely to dodge the default clippy lint.

**Return values:** `Result<T, E>` with the crate's own error enum for fallible library functions; `Option<T>` for "may legitimately be absent" (`Option<String>` fields throughout MCP params structs, using `#[serde(default)]` so callers can omit them).

## Module Design

**Exports:** Crates expose a curated `pub` surface from `lib.rs`; internal helpers stay private to the module unless another crate genuinely needs them (§Dependency Discipline in root `AGENTS.md` governs when a new cross-crate edge is justified — prefer a narrower `-types`/`-core` crate or a documented "defactor" duplication under ~50 lines over widening a crate's public surface).

**Barrel files:** Not used in Rust (each module's `mod.rs`/`lib.rs` re-exports selectively, no blanket `pub use *` barrels). Not used in the TS frontend either — components are imported directly from their file, not through an `index.ts` aggregator.

**Serialization boundary types:** MCP/tool-facing params and results derive `Debug, Deserialize, JsonSchema` (params) and are documented field-by-field — this doc comment doubles as the schema description surfaced to LLM tool callers, so keep it accurate and specific, not generic.

---

*Convention analysis: 2026-09-22*
