---
phase: 07-trial-identity-contracts-observational-baseline
plan: 02
subsystem: secrets
tags: [clavis, secret-id, ed25519, trial-signing, d-11]
requires: []
provides:
  - "SecretId::VoxResearchTrialSigningKeyHex (VOX_RESEARCH_TRIAL_SIGNING_KEY_HEX): vault-only in strict mode, not shareable"
  - "SecretId::VoxResearchTrialSignerAllowlist (VOX_RESEARCH_TRIAL_SIGNER_ALLOWLIST): optional CSV of hex Ed25519 public keys; unset trusts nobody"
affects: [07-05, 07-09, 12-09]
status: complete
actuals:
  tasks: 2
  commits: 2
plan_head_before: 2a63d8ab1a3be4cb80bde27303041888dcd008f4
key-files:
  created: []
  modified:
    - crates/vox-secrets/src/spec/ids.rs
    - crates/vox-secrets/src/spec/registry/scholarly.rs
    - contracts/secrets/managed-env-names.md
    - contracts/secrets/managed-env-names.v1.json
    - contracts/secrets/secret-capabilities.v1.json
    - docs/src/reference/secrets-ssot.md
key-decisions:
  - "D-11: separate SecretIds from the nanopub key; the private key gets its own metadata arm (allow_env_in_strict=false, allow_compat_sources_in_strict=false, shareable=false), copied from VoxUserRsaNanopubPrivateKeyB64"
  - "The allowlist falls through to the default metadata arm, as VoxZenodoUploadAllowlist does (Operator class, env allowed); it holds only public keys"
requirements-completed: []
---

# Phase 7 Plan 02: Trial signer Clavis key source. Summary

Two new Clavis SecretIds give the trial signer one allowlisted trust root: a vault-only Ed25519 private key and an optional public-key allowlist CSV. The runner in 07-09 resolves both through `vox_secrets::resolve_secret`.

## Accomplishments

- **Task 1 (8aaaca4f7), tracer:** added both `SecretId` variants and `SecretSpec` entries, plus a dedicated `metadata()` arm for the signing key. Three tests (`research_trial_*`) pass. Regenerated the three `contracts/secrets/*` files with `vox ci secrets-contracts`. The only drift was the two new entries and the `generated_at_ms` field. `secrets-parity` and `secret-env-guard` both pass.
- **Task 2 (9a62b1e2f):** added one row to the Secret Inventory table in `docs/src/reference/secrets-ssot.md`. `secret-env-guard` passes, and the scoped doc lint reports no hard errors.

## Mutation proof (T-07-02-01)

Flipped the signing key arm's `allow_env_in_strict` from `false` to `true`, with a before-count of 1 in the arm. `research_trial_signing_key_is_vault_only_in_strict` failed on all 3 tries, and the other two tests passed. I then restored the line and confirmed it reads `false` again. The committed tree has the restored value.

## Deviations from Plan

- **[Rule 3 - Blocking] Doc lint path.** The plan's `--paths docs/src/reference/secrets-ssot.md` fails with "did not match any file or directory", because the tool resolves `--paths` relative to `docs/src`. I used `--paths reference/secrets-ssot.md` instead, and it passed.
- **Test-first was not shown as a separate red run.** The tests and the implementation were written in one edit, then run green. The mutation proof above is the evidence that the key test fails when the guard is broken.
- **The commit hook ran rustfmt.** `fmt-fix` reformatted one assert in my new test, and the reformatted file landed in the same commit (8aaaca4f7). No extra commit was needed.

## Skipped checks (lean verification)

- No full `vox-secrets` suite, no workspace build and no clippy. I ran only the 3 new tests, plus `secrets-parity`, `secret-env-guard`, `secrets-contracts` and the scoped doc lint. CI covers the rest.

## Known Stubs

None.

## Threat Flags

None. Nothing new beyond the plan's threat model.

## Next

07-05 and 07-09 can resolve `VoxResearchTrialSigningKeyHex` and `VoxResearchTrialSignerAllowlist`. 07-09 must make an unset allowlist trust nobody. TRIAL-05 is not marked complete, because 07-01, 07-05 and 07-09 also deliver it.

## Self-Check: PASSED

- Files exist: `ids.rs`, `scholarly.rs`, the three `contracts/secrets/*` files and `secrets-ssot.md`.
- Commits exist: 8aaaca4f7 and 9a62b1e2f. `git rev-list --count 2a63d8ab1..HEAD` was 2 before the SUMMARY commit.
