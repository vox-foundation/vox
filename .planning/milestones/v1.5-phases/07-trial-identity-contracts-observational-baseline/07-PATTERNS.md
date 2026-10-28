# Phase 7: Trial Identity, Contracts & Observational Baseline - Pattern Map

**Mapped:** 2026-10-12
**Files analyzed:** 41 new/modified (grouped by plan below)
**Analogs found:** 38 / 41 (3 with no analog or only partial: exclusive-tx helper + failpoint, baseline-stamped guard arm, `results.v2` JSON-schema validation)

All analog paths below were checked with `git ls-files --error-unmatch` (tracked source, no gitignored mirrors). `07-RESEARCH.md` already carries the file:line problem statements; this file adds the copyable excerpts.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/vox-crypto/src/canonical.rs` (NEW) + `lib.rs` `pub mod` | utility | transform | `crates/vox-orchestrator/src/preregistration/trusty_uri.rs` L18-72 (`CanonicalError`, `canonical_json`, `sort_keys`) | exact (promote, do not rewrite) |
| `crates/vox-crypto/Cargo.toml` (+serde, serde_json) | config | - | `crates/vox-eval/Cargo.toml` (serde/serde_json workspace deps) | role-match |
| `crates/vox-orchestrator/src/preregistration/trusty_uri.rs` (MOD: 3-line wrapper) | utility | transform | itself | exact |
| `crates/vox-research-trial-core/Cargo.toml` (NEW) | config | - | `crates/vox-oauth-pkce/Cargo.toml`, `crates/vox-eval/Cargo.toml` | exact |
| root `Cargo.toml` `[workspace.dependencies]` row | config | - | `Cargo.toml:101` (`vox-eval`), `:104` (`vox-crypto`) | exact |
| `docs/src/architecture/layers.toml` `[crates]` row | config | - | `layers.toml:185` (`vox-eval = { layer = 2 }`) | exact |
| `docs/src/architecture/where-things-live.md` row (+ fix stale `vox-prereg` row ~L307) | config/doc | - | row at `where-things-live.md:100` (`vox-eval`), commit `2ef9442ba` diff | exact |
| `crates/vox-research-trial-core/src/{campaign,sign,allowlist,gate}.rs` | model + service | request-response (pure sign/verify) | `preregistration/signing.rs` + `gate.rs` + `research-events/src/preregistration.rs` | role-match (typestate is new) |
| `crates/vox-research-trial-core/src/lifecycle.rs` | model | state machine | `crates/vox-eval/src/report.rs` `FailureClass`/`classify_failure` (pure enum+fn) | partial |
| `crates/vox-research-trial-core/src/identity.rs` | model | transform | `crates/vox-eval/src/provenance.rs` `RunIdentity` L16-30 | role-match |
| `contracts/ci/crate-edges.allow.v1.json` (PROPOSAL ONLY, human-action) | config | - | existing `exceptions` entries (L2141+) | exact |
| `crates/vox-secrets/src/spec/ids.rs` (+2 SecretIds, +metadata arm) | config | - | `VoxNanopubSigningKeyHex` (L99, L640), `VoxUserRsaNanopubPrivateKeyB64` (L672-685) | exact |
| `crates/vox-secrets/src/spec/registry/<new or scholarly>.rs` (+2 `SecretSpec`) | config | - | `registry/scholarly.rs:281-291` (allowlist CSV), `:336-346` (signing key hex) | exact |
| `contracts/secrets/managed-env-names.{md,v1.json}`, `secret-capabilities.v1.json` | config (generated) | - | `VoxNanopubSigningKeyHex` entries | exact (regenerate) |
| `docs/src/reference/secrets-ssot.md` (inventory row) | doc | - | row at `secrets-ssot.md:43` | exact |
| `crates/vox-cli-research/src/trial/allowlist.rs` (Clavis loader) | utility | request-response | `crates/vox-publisher/src/scholarly/flags.rs:80-90` `zenodo_upload_allowlist` | exact |
| `docs/src/architecture/research-trial-tier-a-schema-2026.md` (STORE-01 contract) | doc | - | `docs/src/architecture/model-routing-handoff-2026-10-11.md` (frontmatter) | role-match |
| `crates/vox-db/src/schema/domains/trials.rs` + `domains/mod.rs` | model (DDL) | CRUD | `schema/domains/harness_eval.rs` | exact |
| `crates/vox-db/src/schema/manifest.rs` (fragment, `// 96:` comment, `BASELINE_VERSION`) | config | - | commit `139bdd20e` diff (95), `5ff0c75dd` diff (fragment) | exact |
| `contracts/db/baseline-version-policy.yaml` | config | - | `139bdd20e` diff | exact |
| `crates/vox-db/src/codex_legacy.rs` `LEGACY_EXPORT_TABLES` | config | - | `139bdd20e` (`"model_benchmark_results"` line) | exact |
| `crates/vox-db-types/src/store_types/{mod,trials}.rs` | model | CRUD | `store_types/harness_eval.rs` | exact |
| `crates/vox-db/src/trials.rs` + `lib.rs` module/re-export | service | CRUD | `crates/vox-db/src/harness_eval.rs` | exact (single-row writes) |
| `crates/vox-db/src/lib.rs` `GuardedConnection::exclusive` | utility | transaction | `GuardedConnection` (lib.rs:472-540) + `knowledge.rs:335-365` | partial (new primitive) |
| `crates/vox-db/src/trials.rs` `persist_trial_batch` + `PersistFailpoint` | service | transaction/CRUD | `knowledge.rs:335-365` `delete_knowledge_node` (BEGIN IMMEDIATE/ROLLBACK) | role-match; no failpoint analog |
| `crates/vox-db/src/store/open.rs` loud guard arm + `StoreError::BaselineStampedWithoutTables` | middleware/guard | request-response | `open.rs:112-127`, `store/types/error.rs:47-60`, `codex_schema.rs:26` | role-match |
| `crates/vox-db` tests (`trials::guard_`, `forced_mid_write_failure`, CAS) | test | CRUD | `local_tests.rs:507-522`, `research_pipeline.rs:1256-1281`, `migration.rs:327` | exact |
| `crates/vox-research-shim/.../pipeline.rs` + `config.rs` (`bypass_result_cache`, label seam, `verdict_cache_hits`) | service | request-response | `pipeline.rs:52-62`, `:128-137`; `metering.rs:85-127` | exact (in-file) |
| `crates/vox-cli-research/src/trial/{mod,runner,executor,baseline}.rs` | service | batch / event-driven | `crates/vox-cli-research/src/eval.rs` (L26-120, L122-215) | role-match |
| `crates/vox-cli-research/src/eval.rs` (SCORE-04 residuals, `score_result` split) | service | transform | itself L260, L354-373 | exact |
| `crates/vox-cli-research/src/lib.rs` (`--sign/--campaign/--baseline/--replicates`) | controller (clap) | request-response | `lib.rs:88-110`, `:205-209` | exact |
| `contracts/reports/research-eval/results.v2.schema.json` | config (schema) | transform | `results.v1.schema.json` + `crates/vox-eval/src/report.rs` (typed schema-2 report) | role-match |
| `crates/vox-cli-research/tests/eval_e2e_test.rs` (`#[ignore]`) + egress guard | test | request-response | `crates/vox-search/src/lib.rs:95-120` | exact |
| `crates/vox-cli-research/src/trial/*` tests (fake executor, canary, mutation proofs) | test | event-driven | `gate.rs` tests L140-260; `research_pipeline.rs:1256` canary shape | role-match |
| `research_trial.*` producers in runner | service | event-driven | `vox-telemetry/src/research_trial.rs` + `VoxDb::record_research_metric` | exact |
| `crates/vox-eval/src/*` new stat primitives (if needed) | utility | transform | `corpus_stats.rs` | exact |

---

## Pattern Assignments

### 1. `crates/vox-crypto/src/canonical.rs` (utility, transform) - D-10

**Analog:** `/Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trusty_uri.rs` L18-72. Promote this, make it generic; the orchestrator file becomes a wrapper.

**Error type to copy verbatim** (L18-26):
```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonicalError {
    #[error("value does not survive a JSON round trip (non-finite number?)")]
    Lossy,
    #[error("serialization failed: {0}")]
    Serialization(String),
}
```

**Core pattern to generalise** (L44-72). Currently typed to `PreregistrationV1`; make it `<T: Serialize + DeserializeOwned + PartialEq>`:
```rust
let value = serde_json::to_value(&with_empty_id)
    .map_err(|e| CanonicalError::Serialization(e.to_string()))?;
match serde_json::from_value::<PreregistrationV1>(value.clone()) {
    Ok(back) if back == with_empty_id => {}
    _ => return Err(CanonicalError::Lossy),
}
serde_json::to_string(&sort_keys(value))
    .map_err(|e| CanonicalError::Serialization(e.to_string()))

// serde_json keeps insertion order workspace-wide (`preserve_order`), so sort explicitly.
fn sort_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> =
                map.into_iter().map(|(k, v)| (k, sort_keys(v))).collect();
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(items) =>
            serde_json::Value::Array(items.into_iter().map(sort_keys).collect()),
        other => other,
    }
}
```
Generic signature to add: `pub fn canonical_json_bytes<T: Serialize + DeserializeOwned + PartialEq>(v: &T) -> Result<Vec<u8>, CanonicalError>`. Orchestrator keeps the "blank `id`" step then calls it (wrapper stays `pub(crate) fn canonical_json(&PreregistrationV1) -> Result<String, _>`).

**Crate setup:** `vox-crypto/src/lib.rs` is just `#![forbid(unsafe_code)] pub mod facades; pub use facades::*;` - add `pub mod canonical;`. `Cargo.toml` deps style is `name.workspace = true` / `{ workspace = true }`; add `serde = { workspace = true, features = ["derive"] }`, `serde_json = { workspace = true }`, `thiserror = { workspace = true }`.

**Tests to port first** (test-first rule): `nested_key_order_independent`, `nan_rejected`, golden-bytes. The current tests live at the bottom of `trusty_uri.rs` (`mod tests`); move/duplicate the NaN and nested-order cases rather than inventing new fixtures.

---

### 2. `crates/vox-research-trial-core/` new crate - D-02

**Cargo.toml analog:** `/Users/brbrainerd/dev/vox/crates/vox-eval/Cargo.toml` (L2 peer, smallest) and `crates/vox-oauth-pkce/Cargo.toml` (scaffold commit `2ef9442ba`).
```toml
[package]
name = "vox-research-trial-core"
description = "…"            # layers.toml rule: description = "error"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true, features = ["v4"] }     # as in vox-cli-research/Cargo.toml
hex = { workspace = true }
vox-crypto = { workspace = true }
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[lints]
workspace = true
```
**Registration touchpoints (copy from commit `2ef9442ba`, 6 files):** root `Cargo.toml` row `vox-research-trial-core = { path = "crates/vox-research-trial-core" }` (next to `vox-eval` L101); `where-things-live.md` table row (format: ``| [`vox-eval`](../../../crates/vox-eval/) | description. |``, L100); `layers.toml` `[crates]` row `vox-research-trial-core = { layer = 2 }` next to L185 (do NOT add `max_loc`); `Cargo.lock`. Per RESEARCH A10 also refresh `workspace-hack` (hakari). Orphan rule is "error": `vox-cli-research` must depend on it in the same PR series.

**Signing analog (typestate source):** `/Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/signing.rs`
Imports (L15-21):
```rust
use thiserror::Error;
use vox_crypto::facades::{
    SigningKey, sign, to_verifying_key, verify, verifying_key_from_bytes, verifying_key_to_bytes,
};
```
Sign (L55-76) - note the defects to NOT copy: `SystemTime::now()` inside (take `now_ms` as a parameter) and mutate-in-place (return a new `SignedCampaign`):
```rust
let vk = to_verifying_key(signing_key);
prereg.signing_key = hex::encode(verifying_key_to_bytes(&vk));
prereg.id = compute_trusty_uri(prereg)?;
let canonical = canonical_json(prereg)?;
let sig_bytes = sign(signing_key, canonical.as_bytes());
Ok(Signature(hex::encode(sig_bytes)))
```
Verify (L79-107) decodes hex -> `[u8;64]`, key hex -> `[u8;32]` -> `verifying_key_from_bytes`, then `verify(&vk, canonical.as_bytes(), &sig_arr)`. **Gap to close (Pitfall 1):** it never compares `prereg.id` to `compute_trusty_uri(...)`. `verify_campaign` must. Prefer the one-call `vox_crypto::verify_signature_hex(pubkey_hex, msg, sig_hex) -> Result<bool,String>` (facades.rs:259) used in RESEARCH's code example.

**Gate analog:** `/Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/gate.rs` L22-100.
```rust
#[derive(Debug, Default, Clone)]
pub struct PreregGate { trusted_signers: std::collections::BTreeSet<String> }
impl PreregGate {
    pub fn new<I, S>(trusted_signers: I) -> Self where I: IntoIterator<Item = S>, S: AsRef<str> {
        Self { trusted_signers: trusted_signers.into_iter()
            .map(|key| key.as_ref().to_ascii_lowercase()).collect() }
    }
    // …
    if !self.trusted_signers.contains(&prereg.signing_key.to_ascii_lowercase()) {
        return GateResult::Refused { reason: format!("signing key {} is not a trusted preregistration signer", prereg.signing_key) };
    }
```
Copy into `SignerAllowlist` (`BTreeSet<String>`, lowercase on insert AND on lookup, `Default` = trust nobody). Differences to apply: check allowlist BEFORE crypto (RESEARCH example), return a typed `GateRefusal` enum (not a `String` reason), and produce `VerifiedCampaign` (private ctor).

**Test names/shape to mirror** (gate.rs L143-260): helper `trusted_gate(sk)`, tests `refused_when_signer_is_not_trusted`, `trusted_signer_matches_case_insensitively`, `approved_with_valid_signed_prereg`, `refused_without_prereg`, `refused_without_signature`, `refused_with_bad_signature`. Add `tampered_campaign_id_fails`, `nested_key_order_independent`, `foreign_signer_refused` (names fixed by RESEARCH mutation proofs). Key generation in tests: `vox_crypto::facades::generate_signing_keypair()`.

**Prereg struct analog:** `/Users/brbrainerd/dev/vox/crates/vox-research-events/src/preregistration.rs` L69-90 (plain `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`, no `deny_unknown_fields`, `f64 cost_cap_usd` - both are things `CampaignPreregV1` must differ on: add `#[serde(deny_unknown_fields)]`, `i64` micros, `PartialEq` on the whole type tree for the canonicalizer's round-trip bound).

**Lifecycle / identity analogs (partial):** pure data + pure fn, no I/O: `/Users/brbrainerd/dev/vox/crates/vox-eval/src/report.rs` L23-90 (`FailureClass` enum with `Serialize/Deserialize`, `classify_failure(&VerifyFacts)`); identity record shape: `crates/vox-eval/src/provenance.rs:16` `RunIdentity` (doc: "filled by 12-09"). Define `RunIdentity {campaign_id, run_id, replicate_id, attempt_index, retry_of_run_id}` to match IDENTITY_KEYS in `vox-telemetry/src/research_trial.rs:17-23`. No existing campaign state machine to copy; transition table is in RESEARCH Q2.

**Ledger proposals:** entry shape at `/Users/brbrainerd/dev/vox/contracts/ci/crate-edges.allow.v1.json` L2141+:
```json
{ "from": "vox-research-shim", "to": "vox-compiler",
  "reason": "…", "date": "2026-09-27",
  "authorized_by": "user (explicit chat approval, 2026-09-27)" }
```
Plan writes the 3 proposals (trial-core->vox-crypto, vox-cli-research->trial-core, vox-cli-research->vox-eval) into the PLAN text only; the task that writes the JSON is `checkpoint:human-action`. Never run `--tighten` or regenerate the baseline to admit edges.

---

### 3. Clavis SecretIds (D-11)

**Analogs:** `VoxNanopubSigningKeyHex` (private ed25519 hex) and `VoxZenodoUploadAllowlist` (CSV). Touch list is the 4 places below; there is no per-id Rust code anywhere else (`git grep VoxZenodoUploadAllowlist` hits exactly these).

**(a) enum variant** `/Users/brbrainerd/dev/vox/crates/vox-secrets/src/spec/ids.rs` near L99 / L134:
```rust
    VoxNanopubSigningKeyHex,
    …
    VoxZenodoUploadAllowlist,
```
Add `VoxResearchTrialSigningKeyHex` and `VoxResearchTrialSignerAllowlist` (names are proposals; keep `Vox*` prefix and env `VOX_RESEARCH_TRIAL_*`).

**(b) metadata arm** `ids.rs:604+ metadata()`. Private key must be vault-only in strict mode - copy the arm at L670-685, NOT the generic `AuxTooling` arm at L640 that `VoxNanopubSigningKeyHex` sits in:
```rust
SecretId::VoxUserRsaNanopubPrivateKeyB64 => SecretMetadata {
    class: SecretClass::Integration,
    material_kind: SecretMaterialKind::ApiKey,
    persistable_account_secret: true,
    shareable: false,
    device_local_only: false,
    // A private signing key must be vault-only in strict mode: never
    // sourced from the environment or compat fallbacks.
    allow_env_in_strict: false,
    allow_compat_sources_in_strict: false,
    rotation_policy: RotationPolicy::Manual,
    taxonomy_class: TaxonomyClass::ScholarlyPublication,
    lifecycle: LifecycleMeta::MANUAL,
},
```
The allowlist (public keys, non-secret in content) can join the `L625-657` `Integration/ApiKey/shareable` group or fall through to the `_ =>` default at L705; verify with the `metadata_and_spec_resolve_for_known_id` test (L743).

**(c) registry spec** `/Users/brbrainerd/dev/vox/crates/vox-secrets/src/spec/registry/scholarly.rs` (or `config.rs`; keep with the same domain file):
```rust
SecretSpec {
    id: SecretId::VoxZenodoUploadAllowlist,
    canonical_env: "VOX_ZENODO_UPLOAD_ALLOWLIST",
    aliases: &[],
    deprecated_aliases: &[],
    backend_key: None,
    auth_registry: None,
    policy: SecretPolicy::optional_skip(),
    remediation: "Allowlist for Zenodo file uploads.",
    scope_description: "Zenodo upload allowlist.",
},
```
Signing key spec: copy `VoxNanopubSigningKeyHex` (scholarly.rs:336-346) changing id/env/remediation text.

**(d) generated contracts** `contracts/secrets/{managed-env-names.md,managed-env-names.v1.json,secret-capabilities.v1.json}` - regenerate; do not hand-edit. Entry shape (managed-env-names.v1.json L2334): `{id, canonical_env, aliases, deprecated_aliases, class, material_kind, capabilities:["Orchestration"]}`. Then `vox ci secret-env-guard` and `vox ci secrets-parity`.

**(e) docs** `/Users/brbrainerd/dev/vox/docs/src/reference/secrets-ssot.md`: the "Managed Secret Env Names" section is `{{#include ../../../contracts/secrets/managed-env-names.md}}` (auto-updates); add one hand-written row to the "Secret Inventory" table (L32-58, row format at L43: `` | `ENV_NAMES` | purpose | Optional/Required | notes | ``).

**Loader analog (CSV allowlist -> set)** `/Users/brbrainerd/dev/vox/crates/vox-publisher/src/scholarly/flags.rs:80-90`:
```rust
vox_secrets::resolve_secret(vox_secrets::SecretId::VoxZenodoUploadAllowlist)
    .expose()
    .map(|s| s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect())
    .unwrap_or_default()
```
Copy into `vox-cli-research/src/trial/allowlist.rs`, then lowercase + validate 64-hex into `SignerAllowlist`. Unset/empty = empty set = trust nobody. `vox-cli-research` already depends on `vox-secrets` (no new edge). Never `std::env::var` for the key (AGENTS.md).

---

### 4. vox-db `trials` domain (D-01, D-05, STORE-01/02)

Analog commit for the domain-adding touch list: `5ff0c75dd` (harness_eval); analog for the baseline-bump touch list: `139bdd20e` (95). Combine: **8 files for the domain + 3 for the bump**.

**DDL file** `/Users/brbrainerd/dev/vox/crates/vox-db/src/schema/domains/harness_eval.rs` L1-35 - module doc pointing to the VoxDb impl + record types, then one `pub const`:
```rust
pub const SCHEMA_HARNESS_EVAL: &str = r#"
CREATE TABLE IF NOT EXISTS harness_eval_run (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id              TEXT    NOT NULL UNIQUE,
    …
    started_at_ms       INTEGER NOT NULL,
    finished_at_ms      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_harness_eval_run_time ON harness_eval_run(started_at_ms);
CREATE TABLE IF NOT EXISTS harness_eval_task_result (
    …
    FOREIGN KEY(run_id) REFERENCES harness_eval_run(run_id)
);
"#;
```
Conventions to copy: `CREATE TABLE/INDEX IF NOT EXISTS` only (idempotent: baseline_sql re-runs on DBs at 95), `*_ms INTEGER` timestamps, `FOREIGN KEY(...) REFERENCES …` clauses, no CHECK, no TRIGGER (Turso rejects both). Use text PKs for `trial_*` (RESEARCH DDL) so no `last_insert_rowid` is needed. **Table-name order:** `LEGACY_EXPORT_TABLES` import order is alphabetical parent-before-child; rename `trial_campaign_events` (sorts before `trial_campaigns`).

**Registration** (`5ff0c75dd` diff):
- `crates/vox-db/src/schema/domains/mod.rs`: `pub mod trials;` (alphabetical among `pub mod harness_eval; pub mod history;`)
- `crates/vox-db/src/schema/manifest.rs`:
```rust
    SchemaFragment {
        name: "harness_eval",
        sql: domains::harness_eval::SCHEMA_HARNESS_EVAL,
    },
```
Append `SchemaFragment { name: "trials", sql: domains::trials::SCHEMA_TRIALS }`.
- `crates/vox-db/src/lib.rs` (L95-99 region):
```rust
mod harness_eval;
pub use harness_eval::*;
```
add `mod trials; pub use trials::*;` Downstream imports records via `vox_db::...` (see `eval.rs:6`).
- `crates/vox-db-types/src/store_types/mod.rs`: `pub mod trials;` + `pub use trials::*;` next to `harness_eval`.

**Baseline bump** (`139bdd20e`; next free = **96**, re-check `origin/main` + open PRs just before the commit):
`manifest.rs` L35-36:
```rust
// 95: feat(vox-db): add model_benchmark_results (graded per-case model benchmarks, `vox models bench`)
pub const BASELINE_VERSION: i64 = 95;
```
-> add `// 96: feat(vox-db): add trial_* tables (research/code benchmark campaign identity, Tier A)` and `= 96`.
`/Users/brbrainerd/dev/vox/contracts/db/baseline-version-policy.yaml`:
```yaml
  repository_baseline_integer: 95
  …
  # re-updated for 95: model_benchmark_results table (scientia domain; `vox models bench`).
  repository_baseline_digest_hex: "0x896929bf…"
```
-> new integer, add a `# re-updated for 96: …` comment line, new digest. The digest is printed by the failing test `baseline_policy_matches_compiled_schema` (`manifest.rs:240-267`: "set repository_baseline_digest_hex to {digest}"); run it, copy the digest, re-run.
`/Users/brbrainerd/dev/vox/crates/vox-db/src/codex_legacy.rs` `LEGACY_EXPORT_TABLES` (sorted; `"model_benchmark_results",` was inserted between `"metadata"` and `"model_pricing_catalog"`): insert the `trial_*` names in alphabetical position. Test: `local_tests.rs:507-522 legacy_export_covers_all_baseline_tables`.

**Record types** `/Users/brbrainerd/dev/vox/crates/vox-db-types/src/store_types/harness_eval.rs` L1-41:
```rust
use serde::{Deserialize, Serialize};
/// One row per … 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessEvalRunRecord {
    pub run_id: String,
    pub changed_files: Vec<String>,   // serialised to *_json by the VoxDb method
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
}
```
Plain structs, `i64`/`Option<_>` fields, no dependency on trial-core (keeps the `vox-db -> trial-core` edge avoided). Money as `*_micros: i64`.

**VoxDb methods** `/Users/brbrainerd/dev/vox/crates/vox-db/src/harness_eval.rs` L1-51 (single-statement writes via the circuit breaker):
```rust
use crate::VoxDb;
use crate::store::StoreError;
use turso::params;
pub use vox_db_types::store_types::harness_eval::*;

impl VoxDb {
    pub async fn record_harness_eval_run(&self, rec: &HarnessEvalRunRecord) -> Result<i64, StoreError> {
        let rec = rec.clone();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute("INSERT INTO … VALUES (?1, ?2, …)", params![rec.run_id, …]).await?;
                Ok::<(), StoreError>(())
            })
            .await?;
        Ok(self.conn.last_insert_rowid())      // <- DO NOT copy; racy (lib.rs:455-471). Return nothing / use RETURNING.
    }
}
```
Copy the `breaker.call(|| async move {...})` + `params![]` + `StoreError` shape; drop the `last_insert_rowid` tail. Conditional-UPDATE CAS (terminal immutability): `UPDATE trial_runs SET state=?1 … WHERE run_id=?2 AND state='running'` and check the `u64` returned by `GuardedConnection::execute` equals 1 (`execute` returns rows changed, lib.rs:518-525).

**Exclusive transaction (STORE-02): NO analog for the primitive.** Existing pattern to learn from and fix: `/Users/brbrainerd/dev/vox/crates/vox-db/src/store/ops_memory/knowledge.rs:335-365`:
```rust
breaker.call(|| async move {
    conn.execute("BEGIN IMMEDIATE", ()).await.map_err(StoreError::from)?;
    let result = async {
        conn.execute("DELETE FROM knowledge_edges WHERE src_id = ?1 OR dst_id = ?1", params![id_str.as_str()]).await?;
        conn.execute("DELETE FROM knowledge_nodes WHERE id = ?1", params![id_str.as_str()]).await?;
        conn.execute("COMMIT", ()).await?;
        Ok::<(), StoreError>(())
    }.await;
    if result.is_err() { let _ = conn.execute("ROLLBACK", ()).await; }
    result
}).await
```
Reuse the BEGIN IMMEDIATE / inner async block / ROLLBACK-on-error skeleton. **Why it is not enough:** `GuardedConnection` (lib.rs:472-532) takes `self.lock` (a `tokio::Mutex<()>` behind an `Arc`) per statement, so another task's statement can run inside your open tx. Add a method beside `query/execute/execute_batch` (lib.rs:499-532, same `#[inline] pub async fn` + `let _guard = self.lock.lock().await;` shape) that holds the guard across a closure and passes `&self.inner`. `inner` is a private field, so the helper must live in `lib.rs` next to the struct. The failpoint is a plain `pub enum PersistFailpoint { None, AfterRunRow, AfterSamples(usize) }` parameter (not `cfg(test)`: 12-09 tests live in another crate); no repo precedent.

**Loud baseline guard arm (STORE-01):** insert after the `if current_version < BASELINE_VERSION { ... }` block in `/Users/brbrainerd/dev/vox/crates/vox-db/src/store/open.rs` (existing guard immediately above it, L112-124, is the style to match: `tracing::warn!` with `target: "vox_db::legacy_schema"`, then `return Err(StoreError::LegacySchemaChain {..})`).
Error variant style `/Users/brbrainerd/dev/vox/crates/vox-db/src/store/types/error.rs:47-60` (thiserror message embeds remediation + `crate::schema::BASELINE_VERSION`):
```rust
    #[error("quarantine drop aborted: non-empty quarantined table(s) found: {}. … Remediation: …", tables.join(", "), crate::schema::BASELINE_VERSION)]
    QuarantineDropAborted { tables: Vec<String> },
```
Table probe: `codex_schema.rs:26` `table_present` uses `SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1` through `db.query_all`; inside `migrate` you only have a `&turso::Connection`, so copy the SQL, not the fn. Test harness: `mem_conn()` at `migration.rs:327`:
```rust
async fn mem_conn() -> turso::Connection {
    let db = turso::Builder::new_local(":memory:").build().await.expect("build mem db");
    db.connect().expect("connect")
}
```
Seed `schema_version` = 96 with no trial tables, call `migrate`, assert `BaselineStampedWithoutTables`. Mutation proof 6: remove the arm.

**DB test conventions:** `VoxDb::connect(DbConfig::Memory).await.expect("open db")` (`research_pipeline.rs:1258-1260`); `#[tokio::test]` multi_thread for the concurrent-writer failpoint test.

---

### 5. Telemetry producers and canary (TELEM-01/02)

**Allowlist (landed):** `/Users/brbrainerd/dev/vox/crates/vox-telemetry/src/research_trial.rs` L1-80 - `IDENTITY_KEYS`, `RESEARCH_TRIAL_ALLOWLIST: &[(&str, &[&str])]`. Producer rows must use exactly these keys (e.g. `research_trial.run`: `config_hash, query_hash, build_version, status, duration_ms, served_from_cache, tokens_in, tokens_out, cost_usd_micros, llm_calls, tool_calls, retry_of_run_id` plus identity keys).

**Write path / choke point to call from the runner:** `/Users/brbrainerd/dev/vox/crates/vox-db/src/research_pipeline.rs:142-172`
```rust
pub async fn record_research_metric(&self, session_id: i64, metric_type: &str, value: f64,
    metadata_json: Option<&str>) -> Result<(), StoreError> {
    let sid = session_id.to_string();
    vox_telemetry::validate_research_metric_row(&sid, metric_type, metadata_json)
        .map_err(|e| StoreError::Db(e.to_string()))?;
    …
```
**Canary test shape to copy** (`research_pipeline.rs:1256-1281`):
```rust
let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
let sid = db.create_research_session("k:trial", "q").await.unwrap();
let leaked = db.record_research_metric(sid, "research_trial.run", 1.0,
    Some(r#"{"query":"what did the user ask"}"#)).await;
assert!(leaked.is_err(), "content keys must not reach research_metrics");
db.record_research_metric(sid, "research_trial.run", 1.0,
    Some(r#"{"run_id":"r-1","status":"ok"}"#)).await.expect("structural trial row is accepted");
```
Runner canary (`trial::canary_*`): run the fake-executor campaign with a distinctive query string, then `SELECT metadata_json FROM research_metrics` and assert the canary never appears. Mutation proof 8: bypass `validate_research_metric_row` in the runner's emitter.

**Session creation and identity seam:** session key `trial:{run_id}` via `VoxDb::create_research_session(key, q)` (errors on a repeated key; test `create_research_session_rejects_a_repeated_key`). Pipeline label seam to modify: `/Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/orchestrator/pipeline.rs:128-137`
```rust
super::super::metering::set_run_session(if session_id > 0 {
    format!("research_session:{session_id}")
} else {
    session_key
});
```
and `metering.rs:85-127` (`task_local! RUN_STATE`, `scope`, `set_run_session`, `tag_candidates`): add a pre-seeded label parameter to `scope` (or a `scope_labelled`) and make `set_run_session` not overwrite a seeded label. Test shape: existing `metering::` unit tests in that file.

**Legacy bridge off (D-12):** `crates/vox-research-shim/src/research/research_event_metrics_bridge.rs:~134-150` writes `scientia.finding_candidate_proposed` content JSON. Gate it on "trial identity set" in the config the runner builds; test: run a trial and assert no `scientia.*` content rows land in `research_metrics`. Non-trial path keeps current behavior (add a regression test for that too).

---

### 6. `crates/vox-cli-research/src/trial/{runner,executor,baseline}.rs` (service, batch)

**Analog:** `/Users/brbrainerd/dev/vox/crates/vox-cli-research/src/eval.rs`.
- Imports (L1-7): `use owo_colors::OwoColorize; use serde::{Deserialize, Serialize}; use uuid::Uuid; use vox_db::{DbConfig, VoxDb, now_unix_ms};`.
- DB connect (L133): `VoxDb::connect(DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?).await?`.
- Per-item execution loop with failure -> worst-score sample, not dropped (L148-172):
```rust
match evaluate_single_query_pipeline(&run_id, item, &ctx, Some(&db), &config).await {
    Ok(sample) => results.push(sample),
    Err(e) => { failed += 1; … results.push(failed_query_sample(&run_id, item, elapsed)); }
}
```
- Pipeline entry: `vox_research_shim::research::run_research_with_context(query_req, Some(ctx), db, config).await?` (L58-60) -> in the runner call `run_research_with_context_and_session(..., Some(session_id))`.
- Fixes to apply while copying: `unwrap_or(0.0)` on missing gold (L260), `query: item.query.clone()` stored in the sample (L99; G24 says ids/hashes only in Tier A - use `query_hash`), `Uuid::new_v4()` run id (L134) stays the run-id mint (`uuid` v4 is already a dep).
- Split `evaluate_single_query_pipeline` into live-produce + pure `score_result(&ResearchResult, &GoldenQueryItem)`; reuse `effective_citation_precision`, `answer_abstained`, `legacy_quality_score` (all in this file).
- Trait seam for TEST-01: `TrialExecutor` (real = pipeline, fake = canned `ResearchResult` JSON; types round-trip already tested in `scientia_phase_0a_types_round_trip.rs`).
- Typestate entry (first statement; mutation proof 1 counts this call): `let v = verify_campaign(&signed, allow, now)?;`.

**Statistics:** call `vox_eval::corpus_stats::{bootstrap_ci(&[f64], reps, seed) -> (f64,f64), wilson_interval(successes, trials, z) -> ConfidenceInterval, min_detectable_difference(n) -> f64}` (signatures at corpus_stats.rs L114, L143, L178). `vox-cli-research/Cargo.toml` needs `vox-eval = { workspace = true }` and `vox-research-trial-core = { workspace = true }` (ledger proposals 2, 3). Alphabetical dep order as in the existing file.

**Usage plumbing (METER-01):** `metering::RunUsage` (metering.rs:33-47, `#[serde(default)]`, `Ttft::{NotApplicable, Measured}`) read from `result.research_metadata.usage`; unknown cost = `calls_with_unknown_cost > 0` -> Missing flag, never 0. Persist as `cost_usd_micros: i64`.

**Clap wiring** `/Users/brbrainerd/dev/vox/crates/vox-cli-research/src/lib.rs:88-110` and `:205-209`: extend the existing variant, do not add a subcommand:
```rust
Eval {
    #[arg(long)] queries: Option<std::path::PathBuf>,
    #[arg(long)] output: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = 4)] concurrency: usize,
    // + --sign <draft>, --campaign <signed.json>, --baseline, --replicates N
},
…
ResearchCmd::Eval { queries, output, concurrency } => eval::run_eval(queries, output, concurrency).await,
```
Arg-order regression test already exists: `crates/vox-cli-research/tests/clap_arg_order_test.rs`; extend it.

---

### 7. `results.v2` report and metrics (BASE-01, SCORE-04 R7)

**v1 analog (what NOT to be):** `/Users/brbrainerd/dev/vox/contracts/reports/research-eval/results.v1.schema.json` (5 required metrics, `"schema_version": {"const": 1}`, `additionalProperties:false`) and `build_eval_report` (`eval.rs:~228-262`, hand-built `serde_json::json!`). Its test (`eval.rs:~661-679 eval_report_matches_results_schema`) compares key sets only - R7 asks for real validation.

**v2 analog (typed, versioned, pure):** `/Users/brbrainerd/dev/vox/crates/vox-eval/src/report.rs`:
```rust
pub const REPORT_SCHEMA_VERSION: u32 = 2;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCorpusReport { pub schema_version: u32, …, pub pass_at_1: Option<ArmStats>, … }
// ArmStats (L129-147) + arm_stats(per_problem, reps, seed) -> Option<ArmStats> (L149): CI via bootstrap_ci, None when nothing scorable.
```
Copy the pattern: schema-version const, `Option<…>` for "nothing scorable" (explicit Missing/NotApplicable, no `unwrap_or(0.0)`), assemble-once constructor. Put the typed BASE-01 report in `vox-eval` (D-06) or `vox-cli-research`; serialise it, and validate the output against `results.v2.schema.json` in the test (no JSON-schema validator in vox-cli-research today; RESEARCH suggests `vox-jsonschema-util`, an L1 option - confirm before adding an edge, otherwise assert via round-trip through the typed struct with `deny_unknown_fields`). Schema file style: copy v1's `$schema` (draft 2020-12), `$id` (`https://vox.local/contracts/reports/research-eval/results.v2.schema.json`), `required`, `additionalProperties:false`, `{"const": 2}`. Keep v1 file and keep writing v1 when no campaign is given. Schema-drift pattern for telemetry (reference): `contracts/telemetry/research-trial.v1.schema.json` + test `contract_schema_matches_allowlist`.

**SCORE-04 residual locations:** R5 `eval.rs:260` (`s.recall_at_5.unwrap_or(0.0)`) and `vox-search/src/evaluation.rs:56-58` (empty gold = 1.0); R6 `pipeline.rs:911` + `config.rs:173 fallback_quality_score`; R8 `eval.rs:369-373` (`query.contains("then")`, duplicated in test helper `:388-392`). Existing tests to extend in place: `eval.rs` `mod tests` (`failed_query_counts_as_worst_score`, `legacy_quality_score_does_not_impute_missing_gold`, `multi_hop_queries_require_backend_and_source_coverage`).

---

### 8. Hermetic test guard (TEST-01)

**Analog:** `/Users/brbrainerd/dev/vox/crates/vox-search/src/lib.rs:95-120` (copy verbatim into `vox-cli-research/src/lib.rs` under `#[cfg(test)] mod test_egress_guard` and, for integration binaries, `tests/common/mod.rs` pattern):
```rust
#[cfg(test)]
mod test_egress_guard {
    #[ctor::ctor(unsafe)]
    #[allow(unsafe_code)] // SAFETY: ctors run pre-main, single-threaded, before any test thread.
    fn block_non_loopback_egress() {
        unsafe {
            for key in ["HTTP_PROXY","HTTPS_PROXY","ALL_PROXY","http_proxy","https_proxy","all_proxy"] {
                std::env::set_var(key, "http://127.0.0.1:9");
            }
            for key in ["NO_PROXY", "no_proxy"] { std::env::set_var(key, "127.0.0.1,localhost,::1"); }
        }
    }
}
```
Add `ctor` as a `[dev-dependencies]` workspace dep (already a workspace dep; no new package). Live-lane marker style: `#[ignore = "live ..."]` as in `crates/vox-research-shim/tests/scientia_phase_0a_pipeline_smoke.rs:249`; apply to `crates/vox-cli-research/tests/eval_e2e_test.rs`.

---

### 9. STORE-01 contract document (07-06)

**Frontmatter analog:** `/Users/brbrainerd/dev/vox/docs/src/architecture/model-routing-handoff-2026-10-11.md` L1-7:
```yaml
---
title: "…"
description: "One specific sentence."
category: "Architecture SSOTs"
status: "research"
training_eligible: false
---
```
(AGENTS.md minimum: `title`, `description`, `category`.) Verify with `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/research-trial-tier-a-schema-2026.md`. Fenced `sql`/`text` blocks only; mark any ```vox block `// vox:skip`.

---

## Shared Patterns

### Crypto only through `vox-crypto`
**Source:** `/Users/brbrainerd/dev/vox/crates/vox-crypto/src/facades.rs` (`sign` L221, `verify` L226, `verify_signature_hex` L259, `interop_sha256` L98, `generate_signing_keypair` L209, `signing_key_from_bytes` L232, `to_verifying_key` L242, `verifying_key_to_bytes/from_bytes` L248/L252).
**Apply to:** trial-core, canonicalizer, runner, tests. Never import `sha2`/`ed25519_dalek`/`blake3` outside `vox-crypto` (detector `vox/crypto/banned-crate-import`). To avoid a `vox-cli-research -> vox-crypto` edge, expose `TrialSigner::from_hex` from trial-core.

### Allowlist-first, typestate gate
**Source:** `preregistration/gate.rs:22-100`. Empty set = trust nobody; compare lowercase hex; check before crypto; downstream APIs accept only `VerifiedCampaign`.
**Apply to:** `verify_campaign`, runner entry, Phase 12 12-09 bundle signer (pass the allowlist explicitly; do not read ambient config inside the verifier).

### Tier A DB writes only through `vox-db`
**Source:** `crates/vox-db/src/harness_eval.rs` (method shape) + `contracts/db/data-storage-policy.v1.yaml` (`allow_direct_access`). The runner never writes raw SQL.

### Idempotent DDL, no CHECK/TRIGGER, Rust-validated vocabularies
**Source:** `schema/domains/harness_eval.rs`; `scientia.rs:283`, `:379-381` (Turso limitations). Terminal immutability = conditional `UPDATE ... WHERE state='running'` + changed-rows == 1.

### Structural-only telemetry
**Source:** `crates/vox-telemetry/src/research_trial.rs` + `VoxDb::record_research_metric` choke point (`research_pipeline.rs:142-150`). Ids/hashes/counters only; strings <=128 chars `[A-Za-z0-9_.:+-]`.

### Error handling style
- Library crates: `thiserror` enums (`signing.rs:25-45` `SignError`/`VerifyError`; `trusty_uri.rs:18` `CanonicalError`). Refusals carry reason codes, never key material or prompt text.
- CLI surface: `anyhow::Result` (`eval.rs:26`, `lib.rs`), failures in a batch scored worst/Missing and counted, then `anyhow::bail!` only when every item failed (`eval.rs:172-174`).
- DB: `StoreError` with remediation text in the `#[error(...)]` string.

### Test-first + mutation proofs
**Source:** AGENTS.md; RESEARCH "Required mutation proofs" 1-9. For each guard: record `grep -c` of the guarded call before, break it, show the named test FAILS, restore, show count + green. Named tests (fixed): `foreign_signer_refused`, `runner_refuses_unsigned_campaign`, `runner_refuses_foreign_signer`, `tampered_campaign_id_fails`, `nested_key_order_independent`, `nan_rejected`, `forced_mid_write_failure_leaves_no_partial_run`, `stamped_version_without_tables_errors`, `terminal_run_is_immutable`, `judge_error_is_missing`.

### Commit hygiene (shared tree)
Pathspec-scoped commits only; cargo commands in background (project memory "never block on cargo"); `cargo check` + new tests only; `vox run scripts/fmt.vox` for dirty `.rs` (never `cargo fmt --all`).

---

## No Analog Found

| File / Concern | Role | Data Flow | Reason |
|---|---|---|---|
| `GuardedConnection::exclusive` (hold lock across a closure) | utility | transaction | Every existing `BEGIN`/`COMMIT` site (`knowledge.rs:340`, `ops_a2a.rs:129-214`) is non-isolated on the shared connection; the primitive does not exist. Build in `lib.rs` next to `GuardedConnection::execute` (L518-525), same lock field. |
| `PersistFailpoint` | utility | test seam | `grep` for failpoint/fail_after over vox-db, vox-db-types, research-shim, orchestrator found none. Plain public enum parameter. |
| Baseline-stamped-without-tables guard (`open.rs` arm) | guard | request-response | No "version stamped but tables missing" check exists; only `current_version > BASELINE_VERSION` (open.rs:112-124). Use that block's style plus `QuarantineDropAborted`'s error style. |
| Campaign/run state machine with CAS | model | state machine | No persisted lifecycle with conditional-UPDATE CAS exists; use the RESEARCH Q2 transition table. Nearest shape: `FailureClass`/`classify_failure` for the pure enum/fn part only. |
| JSON-schema validation of the report in tests | test | transform | v1 test compares key sets only (`eval.rs:661-679`); no validator is used in `vox-cli-research`. Decide between `vox-jsonschema-util` (needs an edge/ledger check) and typed-struct round-trip. |
| Campaign Clavis loader for a *public-key* set | utility | request-response | Closest is the CSV allowlist loader (flags.rs:80-90); no existing one validates 64-hex keys. |

## Pitfalls observed while mapping (analog-specific)

- `signing.rs:60-64` calls `SystemTime::now()` and mutates the prereg in place; do not copy either into `sign_campaign`.
- `eval.rs:99` stores raw `query` text in `ResearchEvalSampleRecord` (legacy `research_eval_samples`). Legacy path stays unchanged; campaign path must store `query_hash` only (G24).
- `harness_eval.rs` VoxDb methods return `self.conn.last_insert_rowid()`; racy under the shared connection (lib.rs:455-471). Do not copy that line.
- Commit `5ff0c75dd` changed the policy digest without bumping `repository_baseline_integer`, but `139bdd20e` bumps both; Phase 7 bumps both (96) and the guard keys off the integer.
- `where-things-live.md` row ~307 still says `crates/vox-prereg/`; fix in the same PR that adds the new-crate row (arch-check parity rule).

## Metadata

**Analog search scope:** `crates/vox-orchestrator/src/preregistration/`, `crates/vox-crypto`, `crates/vox-db` (schema, store, lib, harness_eval, research_pipeline, migration, codex_legacy), `crates/vox-db-types`, `crates/vox-secrets/src/spec`, `crates/vox-publisher/src/scholarly`, `crates/vox-telemetry`, `crates/vox-research-shim/src/research`, `crates/vox-cli-research`, `crates/vox-eval`, `crates/vox-search/src/lib.rs`, `contracts/{db,secrets,ci,reports,telemetry}`, `docs/src/architecture/{layers.toml,where-things-live.md}`, git commits `5ff0c75dd`, `139bdd20e`, `2ef9442ba`, `1565f1606`. `docs/src/archive/` not read.
**Files scanned:** ~45
**Pattern extraction date:** 2026-10-12
