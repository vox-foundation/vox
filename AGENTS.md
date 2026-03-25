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
