# Agents, secrets, and where architecture lives

This file is the **required** SSOT for **secret management (Clavis)**. It does **not** replace language/compiler architecture docs.

## Architecture & compiler pipeline (pointers)

- **End-to-end pipeline (lex → parse → AST → HIR → typecheck → codegen):** [`docs/src/explanation/expl-architecture.md`](docs/src/explanation/expl-architecture.md)
- **Internal web IR strategy (compiler frontend boundary):** [`docs/src/adr/012-internal-web-ir-strategy.md`](docs/src/adr/012-internal-web-ir-strategy.md) — Phase 0 schema lives in **`crates/vox-compiler/src/web_ir/`** (`mod.rs`, `validate.rs`).
- **Bell-curve app scope / product lanes / ranking model:** [`docs/src/architecture/vox-bell-curve-strategy.md`](docs/src/architecture/vox-bell-curve-strategy.md)
- **Where new app capability should land first:** [`docs/src/architecture/feature-growth-boundaries.md`](docs/src/architecture/feature-growth-boundaries.md)
- **Interop tiers / approved wrappers / escape-hatch policy:** [`docs/src/architecture/interop-tier-policy.md`](docs/src/architecture/interop-tier-policy.md)
- **Lowering / HIR:** [`docs/src/explanation/expl-compiler-lowering.md`](docs/src/explanation/expl-compiler-lowering.md)
- **Runtime / execution context:** [`docs/src/explanation/expl-runtime.md`](docs/src/explanation/expl-runtime.md)
- **CLI surface:** [`docs/src/reference/cli.md`](docs/src/reference/cli.md)
- **Cross-platform CI & runners:** [`docs/src/ci/runner-contract.md`](docs/src/ci/runner-contract.md)
- **Python / shell scripts vs `vox` (migration SSOT):** [`docs/src/architecture/script-surface-audit.md`](docs/src/architecture/script-surface-audit.md)
- **Contributor governance / TOESTUB:** [`docs/agents/governance.md`](docs/agents/governance.md) — includes **scratch vs TOESTUB** (`.gitignore` for artifacts; TOESTUB for tracked source / CRLF).
- **Doc ↔ code acceptance:** [`docs/src/architecture/doc-to-code-acceptance-checklist.md`](docs/src/architecture/doc-to-code-acceptance-checklist.md)
- **Diagnostic categories (parse / type / HIR / lint):** [`docs/src/reference/diagnostic-taxonomy.md`](docs/src/reference/diagnostic-taxonomy.md)
- **MENS long runs (train, CUDA build, `mens-gate`):** prefer detached processes + log tails; see **IDE / Cursor timeouts** in [`docs/src/reference/mens-training.md`](docs/src/reference/mens-training.md) and [`scripts/populi/mens_gate_safe.ps1`](scripts/populi/mens_gate_safe.ps1) **`-Detach`**.

---

# Secret Management: Use Clavis (Required)

For API keys, tokens, and credentials, use the Clavis system instead of hard-coded `std::env::var(...)` callsites.

- **Why**: Clavis is the SSOT for secret names, aliases, precedence, policy, remediation, and CI guardrails.
- **Define metadata in one place**: add/update secret IDs and env aliases in `crates/vox-clavis/src/spec.rs`.
- **Use Clavis resolution in code**: read secrets with `vox_clavis::resolve_secret(...)` from consumers; avoid new direct secret env reads.
- **CLI lifecycle surface**: `crates/vox-cli/src/commands/clavis.rs` is the canonical UX for `doctor`, `set/get`, backend status, and auth-store migration.
- **Source/precedence logic**: keep resolver/source behavior in `crates/vox-clavis/src/resolver.rs` and `crates/vox-clavis/src/sources/*`.
- **Guardrails and parity**: update and run `vox ci secret-env-guard` and `vox ci clavis-parity` after secret-surface changes.
- **Naming rule**:
  - use `VOX_*` for Vox-owned platform boundaries
  - keep provider-native keys as canonicals for upstream compatibility
  - if adding migration aliases, mark old names as deprecated and surface via doctor warnings
- **Required vs optional**: model blocking requirements as workflow/profile requirement groups (`AnyOf`/`AllOf`), not as a flat global key list.

## When adding a new API key

1. Add `SecretId` + `SecretSpec` entry in `crates/vox-clavis/src/spec.rs`.
2. Migrate consumer callsites to `vox_clavis::resolve_secret(...)`.
3. Add/update `vox clavis doctor` workflow/profile expectations if needed.
4. Ensure docs parity in `docs/src/reference/clavis-ssot.md`.
