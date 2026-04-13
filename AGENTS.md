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
- Research index: [`docs/src/architecture/research-index.md`](docs/src/architecture/research-index.md)

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

## Cross-Platform Shell Discipline (Stable Rules)

- **PowerShell 7 (`pwsh`) when available:** On any host where `pwsh` is installed, prefer it for interactive terminal work and agent-driven shell steps so behavior matches [`contracts/terminal/exec-policy.v1.yaml`](contracts/terminal/exec-policy.v1.yaml) and [`vox shell check`](docs/src/reference/cli.md). On Windows, PowerShell is the default expectation even when only Windows PowerShell 5.1 (`powershell.exe`) is present.
- **CI vs local:** Repository CI jobs often run under **bash** on Linux self-hosted runners ([`docs/src/ci/runner-contract.md`](docs/src/ci/runner-contract.md)); that does not override the **local/agent** preference for `pwsh` when you have it.
- Prefer structured tooling and project CLIs (`vox`, `cargo`, `pnpm`, `uv`, `rg`) over ad hoc shell pipelines.
- **Dev launcher when `vox` is missing from `PATH`:** [`scripts/windows/vox-dev.ps1`](scripts/windows/vox-dev.ps1) / [`scripts/vox-dev.sh`](scripts/vox-dev.sh) — forwards argv to `vox` via `cargo run -p vox-cli` from the workspace root (optional env: `VOX_REPO_ROOT`, `VOX_USE_PATH=1`, `VOX_DEV_FEATURES`, `VOX_DEV_QUIET=1`). See [`docs/src/reference/cli.md`](docs/src/reference/cli.md) (heading **Bootstrap / dev launcher (missing `vox` on `PATH`)**).
- Do not rely on shell-specific one-liners as policy boundaries; approvals and allowlists vary across IDEs.
- Keep commands decomposed into clear steps when safety or portability is at risk.

Environment-specific overlays (for example Antigravity on Windows) add stricter command-shape rules on top of this base; see [`GEMINI.md`](GEMINI.md).

Research synthesis (IDE matchers, PowerShell-first, SSOT terminal policy): [`docs/src/architecture/terminal-exec-policy-research-findings-2026.md`](docs/src/architecture/terminal-exec-policy-research-findings-2026.md). Machine-checked policy entrypoint: [`docs/src/architecture/terminal-ast-validation-research-2026.md`](docs/src/architecture/terminal-ast-validation-research-2026.md).

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
| `recall()` (synchronous memory read) | `recall_async()` |
| `persist_fact()` | `sync_to_db()` |

## Structural Limits & Code Quality

Agents and contributors must strictly adhere to these invariants. These take precedence over general coding guidelines.

- **TOESTUB / Skeleton Code:** Structural quality is enforced via TOESTUB. Finding IDs (`stub/todo`, `stub/unimplemented`, `empty-body`, etc.) in non-test code are CI blockers.
- **Verification Ritual:** Before completing work, mentally (or physically) run `vox stub-check --path <changed-dirs>` to ensure no skeleton code leaked.
- **God Object Limit:** Maximum 500 lines or 12 methods per struct/class. Refactor into domains before adding logic.
- **Sprawl Limit:** Maximum 20 files per directory. Create sub-modules if you exceed this.
- **Frozen Modules:** Do not expose new `pub` items in modules marked as FROZEN.
- **Scripting Restraint:** Do not write new `.py` files in the `scripts/` directory; prefer Rust tooling.
- **Documentation Hygiene:** All `.vox` or `.tsx` code blocks in `docs/src/` must use the `{{#include}}` directive (pulling from `examples/golden/`) or be marked with `// vox:skip` to ensure examples are machine-verified.
- **Completion Policy:** Understand `contracts/operations/completion-policy.v1.yaml` (Tier A, Tier B, Tier C detectors).

## Related Operational Surfaces

- CI and runner behavior: [`docs/src/ci/runner-contract.md`](docs/src/ci/runner-contract.md)
- Workspace artifact hygiene (Cargo target sprawl, `mens/runs`, scratch): [`docs/agents/governance.md`](docs/agents/governance.md) — `vox ci artifact-audit` / `artifact-prune`, retention SSOT [`contracts/operations/workspace-artifact-retention.v1.yaml`](contracts/operations/workspace-artifact-retention.v1.yaml)
- Continuation prompt strategy: [`docs/src/contributors/continuation-prompt-engineering.md`](docs/src/contributors/continuation-prompt-engineering.md)
- Governance and TOESTUB policy: [`docs/agents/governance.md`](docs/agents/governance.md)
