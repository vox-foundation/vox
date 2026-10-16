# Milestone Context — Clavis Next: Key Management That Saves Developer Time

**Captured:** 2026-10-06 (queued — start with `/gsd-new-milestone` after Phase 6 and 06.1 close)
**Status:** Queued. Do NOT archive current phases for this until the in-flight milestone completes.

## Goal

Turn Clavis from "one big registry of ~500 secret IDs" into a system that removes
developer toil across the whole key lifecycle — acquire, wire, verify, rotate,
distribute — while keeping raw key material out of agents' hands.

## Who it serves

Both:
- **Vox contributors** managing Vox's own ~500 keys (orchestrator, mesh, ML, publishing).
- **`.vox` app authors** whose apps call third-party APIs — a `.vox` app should be able to
  declare the APIs it needs and have Clavis handle the keys.

## Current state (grounded 2026-10-06)

- `crates/vox-secrets`: `SecretId` enum (~500 variants, `spec/ids.rs`), registry under
  `spec/registry/` (llm, mesh, scholarly, social, platform, identity, config…),
  `TaxonomyClass` derived from `Capability`, `free_tier.rs`, backends `infisical`,
  `vault`, `vox_vault`; sources `env`, `auth_json`, `populi_env`.
- CLI `vox secrets`: `login` (incl. `--oauth` — OpenRouter only), `status` per
  workflow/profile, `set` (stdin), `get` (redacted), `backend-status`.
- `LifecycleMeta { rotation_cadence_days, expiry_warning_days, track_stale_rotation }`
  is **declared but read nowhere outside `spec/`** — dead metadata today.

## Target pain points (selected)

1. **Acquire & wire** — per-provider acquisition recipes (deep link to key page,
   required scopes, free-tier notes), OAuth/device flow for more providers than
   OpenRouter, and need-driven resolution: compute the exact missing key set for a
   workflow / `.vox` app before runtime, prompt just-in-time when first needed.
2. **Health & rotation** — live validation probes per provider (cheap auth check),
   expiry warnings, rotation reminders that actually fire (wire up `LifecycleMeta`),
   leaked-key response runbook (revoke → rotate → redistribute).
3. **Agents never see keys** — credential-injecting proxy/broker so agents and LLM
   tools make authenticated calls without holding raw values; per-agent key scoping.
   Highest-leverage item for an agent-heavy system. Must respect the LLM boundary
   (`vox_actor_runtime::llm`) and `vox-crypto` policy.
4. **Sync across envs/team** — one source of truth reaching laptop, CI, mesh peers,
   teammates (profiles `dev/ci/mobile/prod` already exist).

## Interop / library organization (selected)

- **Provider manifests as data** — move from a hand-maintained Rust enum to per-provider
  data files (canonical env name, aliases/deprecated names, scopes, acquisition recipe,
  validation probe, lifecycle). Community-contributable; Rust types generated or loaded.
- **Import from existing managers** — `.env`, 1Password, Doppler, Bitwarden, Infisical.
- **Standard reference syntax** — portable `clavis://<provider>/<key>` refs usable in
  config files and `.vox` source, in the spirit of `op://`.

## Open questions for milestone start

- "Sync across envs/team" was selected but "export/sync to targets (GitHub Actions,
  deploy targets)" was not. Clarify: is sync pull-based (CI/mesh/teammates fetch
  from a Clavis-backed vault) rather than push-to-third-party? Default assumption: pull.
- Manifest format & location (TOML under `contracts/`? per-provider files?) and whether
  `SecretId` stays a generated enum for compile-time safety.
- Proxy placement: in-process in `vox-actor-runtime`, a local daemon, or the orchestrator.
- Which providers get live validation probes first (LLM providers are the obvious start).

## Constraints that bind this work

- AGENTS.md Secret Management SSOT: everything via `vox_secrets::resolve_secret(...)`;
  run `vox ci secret-env-guard` and `vox ci secrets-parity` after surface changes;
  keep `docs/src/reference/secrets-ssot.md` in sync.
- Crypto via `vox-crypto` only; no new crate edges without authorization.
- Test-first for every new `pub fn`; VoxScript-first for any automation.

## Recommended at milestone start

Research first (4 parallel researchers) — prior art: 1Password CLI / `op://`, Doppler,
Infisical, SOPS, direnv, Teller, Vault agent injection, GitHub/GitGuardian secret
scanning partner programs (leak detection), credential-proxy patterns for AI agents.
