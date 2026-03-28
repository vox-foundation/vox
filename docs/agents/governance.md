# Architectural Governance (TOESTUB)

The Vox codebase enforces architectural health automatically using the TOESTUB engine.

## Running TOESTUB

**CI / agents (canonical)** — no `vox` feature gate; calls the `toestub` binary directly:

```bash
bash scripts/quality/toestub_scoped.sh                    # default root: crates/vox-repository
cargo run -p vox-toestub --bin toestub -- <PATH>         # explicit scan root
```

**Minimal `vox` binary** — subcommand is behind **`--features stub-check`** (see [`cli.md`](../reference/cli.md#vox-stub-check-feature-stub-check)):

```bash
cargo build -p vox-cli --features stub-check
vox stub-check --path .                    # or positional PATH / `-p`
vox stub-check --severity error            # only errors and critical
# Fix suggestions: `--suggest-fixes` (default true); there is no `--fix` flag
```

GitHub CI runs the **scoped** TOESTUB pass above (`toestub_scoped.sh`). When you run **`vox stub-check`**, it exits non-zero on error/critical findings for the configured scan (see CLI flags in `ref-cli.md`).

## Enforced Rules

TOESTUB rule IDs are emitted as shown below (see `crates/vox-toestub/src/detectors/`). Policy names in prose map to these IDs.

| Rule ID (TOESTUB) | Description | Default severity |
|---|---|---|
| `arch/stub` | Registry id; emitted findings use `stub/todo`, `stub/unimplemented`, … | Error / Warning |
| `arch/empty_body` | Empty or trivial function bodies | Warning |
| `magic-value/*` | Suspicious literals (ports, long strings, large ints); some sub-ids are Error | Warning |
| `victory-claim` | “Done / solved / fixed” style comments | Warning (`victory-claim/hack` is Info) |
| `arch/unwired` | Declared modules never wired (`unwired/module` findings) | Warning |
| `dry-violation` | Near-duplicate blocks (heuristic) | Warning |
| `unresolved-ref` | Likely undefined symbols in-file (heuristic) | Info |
| `deprecated-usage` | `@deprecated` in Vox sources | Warning |
| `security/hardcoded-secret` | High-entropy / credential-shaped literals | Error |
| `arch/god_object` | Oversized files / high method count | Error |
| `arch/sprawl` | Forbidden generic filenames + directory file-count sprawl | Error (forbidden names); Warning (directory sprawl) |
| `arch/schema_compliance` | Paths vs `vox-schema.json` | Error (when schema path configured) |
| `arch/organization` | Bloated `lib.rs` / type-dump organization | Warning |
| `stringly-typed-enum` | String fields with enum-like comment lists | Warning |
| `rust/unwrap-call` | Heuristic `.unwrap()` nudge (skips common test paths) | Info |
| `cross-platform/line-endings` | CR / CRLF in scanned sources vs LF policy (finding id `cross-platform/crlf`) | Warning |

**CI parity:** hard gate is **`vox ci line-endings`** (forward-only diff); see [runner contract](../src/ci/runner-contract.md#line-endings-cross-platform).

## Local scratch, logs, and side `target/` trees

- **`.gitignore` (first line of defense):** keep broad, *root-anchored* rules where possible (see root `.gitignore` — `target-*/`, `/target*.stale-*/`, `/*.txt`, …) so ad-hoc logs, overflow Cargo target dirs, and rename leftovers stay untracked without listing every filename variant.
- **TOESTUB:** use for **tracked source** shape (stubs, sprawl, god-object, **CR/LF** via `cross-platform/line-endings`). It does **not** replace `.gitignore` for build trees or one-off command output — those never enter the scan set if ignored.
- **When adding a new scratch pattern:** prefer one **general** rule (e.g. `/check_err*.log` at repo root) over many exact names; avoid ignoring paths that could be real product folders (if unsure, root-anchor with `/`).
- **Optional cleanup:** `cargo clean` (honors `.cargo/config.toml` `CARGO_TARGET_DIR`); remove stale `target*.stale-*` dirs after closing handles on `target/**/vox.exe`.

## God Object Lock

Files exceeding **500 lines** or structs with **> 12 methods** are locked for new features.
Before adding to them, refactor into traits and sub-modules first.

Affected files as of March 2026:
- `crates/vox-orchestrator/src/orchestrator.rs` (70 KB) → See ORCH-01 in plan
- `crates/vox-orchestrator/src/memory.rs` (31 KB) → tracked

## Sprawl Guard

No directory may contain more than **20 files**. When exceeded, sub-slice into
feature modules. Example:
```
# From:
crates/vox-mcp/src/tool_a.rs
crates/vox-mcp/src/tool_b.rs  (20+ files)

# To:
crates/vox-mcp/src/tools/
  tool_a.rs
  tool_b.rs
  mod.rs
```

## Nomenclature (English-first + CLI aliases)

- **SSOT:** [Nomenclature migration map](../src/architecture/nomenclature-migration-map.md) — canonical English terms, Latin CLI aliases, and retired identifiers (`vox-mens`, phantom crate names, broken doc links).
- **Mesh vs model:** use **mesh** / **Populi** for coordination; **model** / **Mens** for the ML stack. Do not call the mesh control plane “mens” in new docs.
- **CLI:** prefer documented English command names; Latin routes (`fabrica`, `clavis`, `oratio`, …) remain discoverability aliases.

## Naming Enforcement

Generic names are **strictly forbidden**:
- `utils.rs`, `helpers.rs`, `misc.rs`, `common.rs`
- `utils.ts`, `helpers.ts`, `types.ts` (unless it is the canonical types file)

All files must have a specific, meaningful name tied to their domain.

## Schema Compliance

All new crate definitions and path conventions must be registered in `vox-schema.json`
at the workspace root before the file is created. The `arch/schema_compliance` TOESTUB rule
enforces this.

## Vox Quality Rules (Code Review Checklist)

Before marking any PR or task complete, verify:

- [ ] No `.unwrap()` or `.expect("TODO")` in production codepaths
- [ ] No `todo!()` macros outside tests
- [ ] All `match` arms are exhaustive (no wildcard `_ => panic!()` unless explicitly justified)
- [ ] New public APIs have doc comments
- [ ] `cargo check --workspace` passes with zero errors
- [ ] `vox stub-check` finds no Error/Critical severity issues
- [ ] `cargo clippy` (or equivalent) shows no denies
- [ ] Changed LF-policy files have no CR/CRLF (`vox ci line-endings`; `*.ps1` exempt)

## Agent Scope Rules

- **File Affinity**: An agent must hold a lock via `vox_claim_file` before editing.
  Overlapping edits are blocked by the Orchestrator's `scope.rs` guard.
- **Scope Violation**: Writing outside assigned scope emits a `ScopeViolation` event
  which is logged and surfaced in the VS Code extension status bar.

## Build Environment Notes (Windows)

```powershell
# Always use full path in agent shell sessions where PATH may not be set:
& "$env:USERPROFILE\.cargo\bin\cargo.exe" check --workspace

# Prefer check over build for agent sessions (faster, no linker lock):
cargo check -p vox-cli

# Transient Windows linker errors (LNK1104) → retry or use check:
$env:CARGO_TARGET_DIR = "target_alt"; cargo check -p vox-orchestrator
```
