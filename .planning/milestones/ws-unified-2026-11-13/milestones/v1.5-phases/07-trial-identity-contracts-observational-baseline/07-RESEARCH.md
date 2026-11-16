# Phase 7: Trial Identity, Contracts & Observational Baseline - Research

**Researched:** 2026-10-12
**Domain:** Rust workspace (vox): signed campaign identity, vox-db Tier A persistence, structural telemetry, research-pipeline metering, evaluator correctness
**Confidence:** MEDIUM-HIGH. Every file:line below was read this session. Nothing was compiled or run (cargo is backgrounded by project rule), so "test present" never means "test passes".

<user_constraints>
## User Constraints (from 07-CONTEXT.md)

### Locked Decisions
- **D-01 Tier A home is a new active domain.** Campaign, run and sample tables go in a dedicated vox-db domain (working name `trials`; ruling G22), not a scientia extension. Quarantined `scientia_prereg` is not touched. Phase 12 code benchmarks and research campaigns share this identity.
- **D-02 New crate is authorized** for the trial engine, because `vox-research-shim` is ~11.4k LoC against an 8k budget.
  - The recommended shape is `vox-research-trial-core` at L2. It holds pure types, recursive canonicalization reuse, identity, lifecycle state machine and ledger math, with no I/O.
  - The runner lives in existing `vox-cli-research` (L5), or in a new L3 crate if the planner shows L5 is wrong.
  - The crate needs a `layers.toml` `[crates]` row at creation.
  - Any `exceptions` ledger entries it needs are written by the planner as **proposals**. Their execution step is a `checkpoint:human-action` so the user approves and writes them. AGENTS.md makes ledger entries USER-AUTHORIZED-ONLY, so an executor must never write one itself.
- **D-03 Crate edge `vox-cli-research → vox-eval` is authorized.** It lets research reuse the `vox-eval` statistics and gate template; do not duplicate them. The user approves the ledger entry, as in D-02.
- **D-04 The STORE-01 human gate stands.** The canonical campaign/run/sample schema contract is presented for human approval before any migration. Plans must put a `checkpoint:decision` before the migration task.
- **D-05 Baseline number.** Phase 7 takes the **next free DB baseline after 95**. 95 is already committed on this branch by `139bdd20e`. Add a guard that fails loudly if a shared DB already carries that version without these tables (per STORE-01).
- **D-06 Statistics.** Reuse `vox-eval` `corpus_stats` (bootstrap, Wilson, McNemar, Holm, MDD) wherever it fits. New primitives are added to `vox-eval`, not to the trial crate. For example, if BASE-01 needs a paired sign-flip test or Lan–DeMets spending, add it there. Phase 7 is **observational only**: the race/stopping machinery is Phase 8.
- **D-07 The signer allowlist is the Phase 12 bundle signer** (Phase 12 D-10). TRIAL-05's allowlisted key set is the one trust root for both research campaigns and code-benchmark bundles. It is configured through Clavis (`vox-secrets` `SecretId` plus a public-key allowlist config). Signing keys are never read from env directly.
- **D-08 Phase 7 delivers this interface to Phase 12** (consumed by plan 12-09; its names are assumptions to confirm):
  1. a run-identity API (`campaign_id`, `run_id`, `replicate_id`, `attempt_index`, `retry_of_run_id`);
  2. the campaign preregistration id plus the allowlisted signer;
  3. a generic recursive canonicalizer that rejects non-finite numbers (promote the one now in `trusty_uri.rs` rather than writing another);
  4. an allowlist check function;
  5. a one-transaction run persist with a test failpoint (STORE-02).
- **D-09 Phase 12's schema ask goes to the STORE-01 gate.** The ask is a run-level `provenance_json TEXT NOT NULL` column plus indexed scalars (`syntax_fingerprint`, `model_id`, `condition`, `problem_set_hash`, `run_complete`). The human gate must also decide how the already-committed scientia `model_benchmark_results` (baseline 95, written by `vox models bench`) relates to the new run tables. The options are a projection/view over runs, FK columns to `run_id`, or leaving it as a separate writer with a documented boundary. Phase 12 D-09's original "fold into the Phase 7 baseline" is no longer possible as stated, because the table already shipped.

### Claude's Discretion
CONTEXT.md declares none explicitly. Treated as discretion (within D-01..D-09): run/sample grain, DDL column details, state vocabularies, the canonicalizer's home crate, how the runner is wired, baseline metric definitions, plan decomposition. Anything touching D-04's gate is a recommendation to the human, not a decision.

### Deferred Ideas (OUT OF SCOPE)
- Race harness, budgets, stopping (Phase 8).
- Tier C evidence storage (Phase 9, STORE-03); until then campaigns are observational, never replay- or promotion-eligible.
- Promotion (Phase 10).
- Any policy change: BASE-01 runs the current pipeline as is.
- Hermetic default tests (TEST-01): no model or HTTP access; live coverage only in an ignored/live lane.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TRIAL-01 | one signed `campaign_id`; unique `run_id` + replicate/attempt/retry link, propagated everywhere | §Q2 propagation points; session-key convention `trial:{run_id}`; `metering` label seam; identity keys already in telemetry allowlist |
| TRIAL-02 | versioned immutable expanded prereg, sign+verify freezes, tamper rejected, supersession linked | §Q2 `CampaignPreregV1` shape; **verify must recompute the id** (existing `verify_prereg` does not) |
| TRIAL-03 | lifecycle Draft→Signed→Running→Completed/Failed/Stopped enforced | §Q2 transition table; no SQL triggers/CHECK in Turso, so enforce in Rust + conditional UPDATE |
| TRIAL-04 | interrupted attempt terminalized, retried under new linked run_id | §Q2 run machine; recovery needs an owner/lease column |
| TRIAL-05 | gate called by production runner (mutation-proven), allowlisted signers, canonicalizer at every depth + rejects NaN, hashing/signing via vox-crypto | §Q1: allowlist + canonicalizer already landed in `0cde42c9b`; remaining = typestate caller, Clavis key source, generic canonicalizer home |
| STORE-01 | active Tier A fragment, approved contract, next free baseline, loud guard | §Q3 mechanics, candidate DDL, gate options |
| STORE-02 | atomic campaign/run/sample batch, forced failure rolls back | §Q3: existing BEGIN/COMMIT pattern is NOT isolated on the shared `GuardedConnection`; needs an exclusive-transaction primitive + failpoint |
| TELEM-01 | structural-only allowlist, lands before producers | §Q5: allowlist + validators already landed (`b03cc6491`..`6f5d7ddb3`); no producer yet; canary test missing |
| TELEM-02 | one trace per run; real session/run id; unique per attempt; no `INSERT OR IGNORE`+rowid | §Q5: mostly landed (`edd0ab725`, `24b1aa5a8`); remaining = bind to Phase 7 `run_id` |
| METER-01 | tokens/cost/latency/TTFT/tool calls from `LlmResponse` into run results; duration covers all stages | §Q6: `RunUsage` landed (`2ff39c01c`); remaining = eval/runner consumption + persistence |
| SCORE-04 | evaluator correctness | §Q7: R1-R5,R7(partial),R8(p50) landed; residual defects listed |
| TEST-01 | hermetic default tests | §Q8: no fake seam in the pipeline; `eval_e2e_test.rs` hits live providers in the default lane |
| BASE-01 | observational baseline, every metric, no policy change | §Q8 metric table + command shape |
</phase_requirements>

## Summary

**The 2026-10-01 research and 07-CONTEXT.md are staler than the context says.** Nine commits dated 2026-10-01 (`b03cc6491`, `409e5711c`, `b9caf4108`, `d0b799378`, `24b1aa5a8`, `6f5d7ddb3`, `6ca8d70fc`, `2ff39c01c`, `edd0ab725`; first one's message says "Phase 7 first slice") already shipped most of SCORE-04, the TELEM-01 row allowlist, the TELEM-02 trace/session work and the METER-01 accumulator. 07-CONTEXT.md's "Changed since research" lists only `0cde42c9b`, `139bdd20e` and Phase 12, so planning from it alone would re-plan finished work. The real Phase 7 residue is: (1) the identity/contract/persistence spine that does not exist at all (no campaign type, no `trial_*` tables, no runner, no producer of `research_trial.*` rows); (2) residual defects in the already-landed slices; (3) plumbing metered usage into run results; (4) the hermetic seam; (5) the baseline command.

Four findings change the plan shape. **(a) `verify_prereg` never checks the Trusty-URI `id`** (the canonical form blanks it), so a tampered `campaign_id` verifies today; the new verifier must recompute it. **(b) Turso rejects `CREATE TRIGGER` and `CHECK`**, so TRIAL-DESIGN's "trigger or app check" is app-check only (conditional `UPDATE ... WHERE state IN (...)` + `changes()==1`). **(c) `model_benchmark_results` feeds live routing** (`model_benchmark_summary` aggregates every row per model), so a "projection over runs" would silently change routing and violate the no-policy-change fence; recommend leaving it a separate writer. **(d) The "existing" BEGIN/COMMIT pattern is not isolated** on the shared `GuardedConnection` (its mutex is per statement), so STORE-02 needs a new exclusive-transaction helper, not a copy of `delete_knowledge_node`.

The ledger burden is smaller than feared if the generic canonicalizer goes into `vox-crypto` (every consumer already depends on it): the minimum is **3 new exceptions** (trial-core→vox-crypto, vox-cli-research→trial-core, vox-cli-research→vox-eval) plus an optional 4th (vox-cli→trial-core for 12-09).

**Primary recommendation:** Wave 1 builds the pure `vox-research-trial-core` tracer (sign→verify→allowlist→lifecycle→identity) with the generic canonicalizer in `vox-crypto`, in parallel with the residual SCORE-04/hermetic fixes; Wave 2 writes the Tier A contract document and stops at the `checkpoint:decision`; only then migrate at baseline **96**, build the runner on the typestate gate, and finish with the BASE-01 command.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Campaign/run types, lifecycle, sign/verify typestate, allowlist type | L2 `vox-research-trial-core` | L1 `vox-crypto` (primitives, canonical bytes) | D-02: pure, no I/O |
| Canonical JSON bytes (generic) | L1 `vox-crypto` | — | Every signer/verifier already depends on it: zero new edges; "do not hand-roll a signature envelope" (AGENTS.md) |
| Tier A tables, atomic persist, baseline guard | L3 `vox-db` (+ L0 `vox-db-types` records) | — | Only `vox-db`/`vox-secrets`/`vox-sql` may touch Tier A (`data-storage-policy.v1.yaml` `allow_direct_access`) |
| Clavis SecretIds (signer key, allowlist CSV) | L1 `vox-secrets` | — | AGENTS.md Secret Management |
| Allowlist loader (Clavis → `SignerAllowlist`) | L5 `vox-cli-research` | consumed by `vox-cli` via existing edge | avoids a trial-core→vox-secrets edge |
| Trial runner (gate call, run loop, retry, telemetry producers) | L5 `vox-cli-research` | — | D-02 default |
| Research pipeline seam (cache bypass, run label, usage) | L3 `vox-research-shim` | — | pipeline lives there |
| Structural telemetry allowlist | L1 `vox-telemetry` (landed) | `vox-db` write choke point | `validate_research_metric_row` |
| Statistics for BASE-01 | L2 `vox-eval` | — | D-03/D-06 |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vox-crypto` (workspace) | 0.6.0 | ed25519 sign/verify, BLAKE3, `interop_sha256` for Trusty URIs, new canonical-JSON module | sole crypto SSOT; `facades::sign/verify/verify_signature_hex/interop_sha256` exist [VERIFIED: crates/vox-crypto/src/facades.rs:98,221,226,259] |
| `serde` / `serde_json` (workspace) | 1.x | canonical bytes; `preserve_order` is on via workspace-hack so key sorting must be explicit | [VERIFIED: crates/workspace-hack/Cargo.toml:145 `serde_json = { version = "1", features = ["alloc", "preserve_order", "raw_value", "unbounded_depth"] }`] |
| `vox-db` + `vox-db-types` (workspace) | 0.6.0 | `trials` fragment, record types, atomic persist | harness_eval precedent (`5ff0c75dd`) |
| `vox-eval` (workspace, L2) | 0.6.0 | `bootstrap_ci`, `wilson_interval`, `mcnemar_exact_p`, `holm_reject`, `min_detectable_difference` | [VERIFIED: crates/vox-eval/src/corpus_stats.rs:46,88,114,143,178] |
| `uuid` v4, `thiserror` 2, `hex` 0.4.3 (workspace) | — | run ids, errors, key hex | already workspace deps [VERIFIED: Cargo.toml:272,238,269] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `wiremock`, `serial_test` | workspace | loopback HTTP, env mutation in tests | only for live-shaped tests; hermetic runner tests use a fake executor [CITED: 12-VALIDATION.md] |
| `ctor` | workspace | pre-main egress guard (proxy env to 127.0.0.1:9) | copy the `vox-search` pattern into `vox-cli-research` tests [VERIFIED: crates/vox-search/src/lib.rs:100-120] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| canonicalizer in `vox-crypto` | `vox-research-events` (L1, already has serde_json) | also zero edges for orchestrator/cli, but trial-core/cli-research would need a new edge to it; crypto is the better "envelope" owner |
| hand-written canonicalizer | RFC 8785 JCS crate (`serde_json_canonicalizer`) | standard for third-party verification, but a new external package: must pass `package-legitimacy check`; tagged `[ASSUMED]`, not recommended without a user call |
| new SecretIds | reuse `VoxNanopubSigningKeyHex` for the private key | exists but unused; conflates the publishing key with the research trust root |

**Installation:** no new third-party packages. Workspace additions only: root `Cargo.toml` `[workspace.dependencies]` row for the new crate, crate `Cargo.toml` with `workspace-hack = { version = "0.6", path = "../workspace-hack" }` (every crate has it), `layers.toml` + `where-things-live.md` rows.

**Version verification:** `uuid 1` (`v4`,`serde`), `thiserror 2`, `hex 0.4.3`, `serde_json 1` read from root `Cargo.toml` this session. No registry lookups were needed because nothing new is installed.

## Package Legitimacy Audit

No external package is added by this phase (new crate is first-party; `uuid`, `thiserror`, `hex`, `serde*` are existing workspace dependencies).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none new) | — | — | — | — | — | n/a |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none. If the planner chooses the JCS crate alternative, tag it `[ASSUMED]`, run `gsd_run query package-legitimacy check --ecosystem crates <name>`, and add a `checkpoint:human-verify` before install.

## Architecture Patterns

### System Architecture Diagram

```text
 operator CLI                       Clavis (vox-secrets)
 `vox research eval`                  signer key (private) | signer allowlist (CSV pubkeys)
   --sign / --campaign / --baseline          |                    |
        |                                    v                    v
        |         +-------------- vox-cli-research (L5 runner) -----------------+
        |         | load_signer_allowlist()   sign_campaign(draft,key,clock)    |
        v         |        |                          |                         |
  draft campaign ----------+------------> SignedCampaign --verify_campaign()--> VerifiedCampaign   <- TYPESTATE: only way
  (CampaignPreregV1)                          (trial-core, L2)                      |               to obtain runner input
                                                                                    v
                         refuse: unsigned | tampered id/field | foreign signer | NaN
                                                                                    |
              +-----------------------------------------------------------------+   |
              | vox-db (L3)  trial_campaigns -> trial_campaign_events           |<--+ create_campaign (1 tx)
              |              trial_runs (unique identity index)  <-- open_run   |<--- Signed->Running CAS
              |              trial_samples                                      |
              +------------------------------^----------------------------------+
                                             | finish_run: ONE exclusive tx
   per item x arm x replicate x attempt:     |   (conditional terminal UPDATE + sample batch)
   RunIdentity{campaign,run,replicate,attempt,retry_of}
        |
        v   precreate session key "trial:{run_id}" -> run_research_with_context_and_session
   TrialExecutor (trait)  --real--> vox-research-shim pipeline (cache bypass, label=run_id, one TRACE_CTX)
                          --fake--> canned ResearchResult fixtures (default tests)
        |
        v  ResearchResult.research_metadata.usage (RunUsage) + duration_ms + diagnostics
   pure scorer score_result(result,item)->metrics ---> vox-eval stats (bootstrap/Wilson) ---> baseline report JSON
        |
        v  research_trial.campaign / research_trial.run rows via record_research_metric (allowlist-validated)
```

### Recommended Project Structure
```text
crates/vox-crypto/src/canonical.rs            # canonical_json_bytes<T>, CanonicalError (+ sign/verify helpers)
crates/vox-research-trial-core/src/
  lib.rs  campaign.rs  sign.rs  allowlist.rs  lifecycle.rs  identity.rs  error.rs
crates/vox-db-types/src/store_types/trials.rs  # plain records (no trial-core dep)
crates/vox-db/src/schema/domains/trials.rs     # SCHEMA_TRIALS
crates/vox-db/src/trials.rs                    # VoxDb methods + persist + failpoint
crates/vox-cli-research/src/trial/{mod,runner,executor,baseline,allowlist}.rs
docs/src/architecture/research-trial-tier-a-schema-2026.md   # STORE-01 contract (frontmatter required)
```

### Findings by planning question

#### Q1 TRIAL-05 remaining work

**Landed in `0cde42c9b`** [VERIFIED: git show --stat 0cde42c9b]: `PreregGate::new(trusted_signers)` (gate.rs:32), refusal when `trusted_signers` lacks `prereg.signing_key` (gate.rs:~75-86), recursive `sort_keys` (trusty_uri.rs:58), non-finite rejection via serde round trip (trusty_uri.rs:43-55), `interop_sha256` facade (facades.rs:98) used by `compute_trusty_uri` (trusty_uri.rs:30-35). Signing/verification use `vox_crypto::facades` only (signing.rs imports).

**Still missing**
1. **No production caller.** `check_campaign`/`check_campaign_prereg` are referenced only from `preregistration/{gate,mod,reply_window}.rs`, `research_gate.rs` and `tests/research_gate.rs` [VERIFIED: grep -l over crates]. The legacy gate takes `PreregistrationV1`, which cannot express arms; TRIAL-02 makes it a conversion input only.
2. **Where the caller lives:** put the verifier in trial-core as a typestate: `verify_campaign(&SignedCampaign, &SignerAllowlist, now) -> Result<VerifiedCampaign, GateRefusal>`; the runner's only entry point takes `SignedCampaign`, calls `verify_campaign` itself, and downstream fns accept only `VerifiedCampaign`. Leave `PreregGate` in vox-orchestrator untouched (no new code in an 85k-LoC crate); feed both from the same `SignerAllowlist` (D-07 single trust root).
3. **`id` is not covered by verification.** `canonical_json` always blanks `id` (trusty_uri.rs:43-46: `with_empty_id.id = String::new();`), and `verify_prereg` (signing.rs:78-107) only checks the signature over that blanked form. Tests present: `sign_and_verify_round_trip`, `tamper_detection_fails_verify`, `wrong_signature_hex_fails_verify` - none flips `id`. A tampered `campaign_id` therefore verifies. `verify_campaign` must recompute the trusty id and require equality; add `tampered_campaign_id_fails`.
4. **Allowlist key source.** No SecretId carries a public-key allowlist. Existing relevant ids: `VoxNanopubSigningKeyHex` (private ed25519 hex, `canonical_env: "VOX_NANOPUB_SIGNING_KEY_HEX"`, no consumer in `crates/`) [VERIFIED: crates/vox-secrets/src/spec/registry/scholarly.rs:336-346; grep found no other use]; CSV allowlist precedents `VoxRoutingProviderAllowlist` (llm.rs:182-192) and `VoxZenodoUploadAllowlist`, parsed in `vox-publisher/src/scholarly/flags.rs:80-90` (split ',', trim, drop empties). Recommend two new ids: a private-key id (vault-preferred, like the "private signing key must be vault-only in strict mode" arm in `ids.rs:~678`) and `VoxResearchTrialSignerAllowlist` (CSV of 64-hex verifying keys, `SecretPolicy::optional_skip()`). Empty/unset allowlist = trust nobody (matches `PreregGate::default`). Adding a SecretId touches `ids.rs`, a `registry/*.rs` entry, `contracts/secrets/{managed-env-names.md,managed-env-names.v1.json,secret-capabilities.v1.json}` (seen in the last SecretId-adding merge) and needs `vox ci secret-env-guard` + `vox ci secrets-parity`.
5. **Generic canonicalizer.** Current shape is prereg-specific (`canonical_json(&PreregistrationV1)`), generic only in `sort_keys(Value)` (private). Promote as `vox_crypto::canonical::canonical_json_bytes<T: Serialize + DeserializeOwned + PartialEq>(&T) -> Result<Vec<u8>, CanonicalError>` (serialize → `Value`, round-trip equality to detect NaN/inf, sort keys at every depth, `to_vec`); keep `CanonicalError::{Lossy, Serialization}`. **Why the extra bounds:** `serde_json` maps NaN/inf to `null` in both `to_value` and `to_string`, so non-finite floats are undetectable from the output; round-trip equality is the only cheap generic detector (NaN != NaN, and `null` fails to deserialize into `f64`). **Consequence for 12-09:** its assumed `canonical_json_bytes<T: Serialize>` is not achievable cheaply; `EvalCorpusReport` derives only `Debug, Clone, Serialize, Deserialize` (report.rs:216) - add `PartialEq` through its type tree, or accept a custom finite-checking `Serializer` (~100 lines, no bounds). Recommend the derive route. `orchestrator::preregistration::trusty_uri::canonical_json` becomes a 3-line wrapper (blank `id`, call generic). Zero new workspace edges: orchestrator (`["vox-orchestrator","vox-crypto"]`), cli (`["vox-cli","vox-crypto"]`) already depend on vox-crypto [VERIFIED: crate-edges.allow.v1.json edge list].
6. **Format caveat.** Output is serde_json's compact form of a byte-ordered key sort, not RFC 8785 (UTF-16 key order, ECMAScript number formatting). Signed bytes stay stable only while serde_json's float formatting does; pin with a golden-bytes test and say so in the contract if third parties will verify offline.
7. **Clock.** `sign_prereg` calls `SystemTime::now()` internally (signing.rs:60-64); the new `sign_campaign` must take `now_ms` as a parameter (deterministic tests, and Phase 12 can pass its own clock).
8. **Mutation proofs** (AGENTS.md; guard each with a before/after presence count): (i) delete the allowlist membership check in `verify_campaign` → `foreign_signer_refused` fails; (ii) delete the `verify_campaign` call in the runner entry → `runner_refuses_unsigned_campaign` and `runner_refuses_foreign_signer` fail; (iii) delete the id-recompute → `tampered_campaign_id_fails` fails; (iv) revert `sort_keys` recursion → `nested_key_order_independent` fails.

#### Q2 TRIAL-01..04: shapes, lifecycle, propagation

**`CampaignPreregV1`** - start from TRIAL-DESIGN §2, with these deltas (verified against REQUIREMENTS TRIAL-02 wording, which lists more than §2 does):
- Fields: `schema_version`, `campaign_id` (= Trusty URI, derived), `kind: Observational | Exploratory | HoldoutConfirmation` (Observational is new: BASE-01), `supersedes_campaign_id`, `converted_from` (a `PreregistrationV1` id; **no converter code in Phase 7**, it would force a trial-core→vox-research-events edge; the field plus "runner accepts only `SignedCampaign`" satisfies "conversion input only"), `substrate {repo_commit, eval_set_digest, item_ids_hash}`, `hypotheses[]` (**missing in §2**), `arms[] {arm_id, label, is_control, condition}` (condition = full expanded `ResearchConfig` snapshot + knobs, never a reference), `replicates_per_cell`, `randomization {seed, rng_id}`, `metrics[] {name, direction, role, unit, scorer_version}`, `hard_gates[]` (**missing in §2**; seed from QUALITY-EVAL §2 thresholds), `validity_rules` (**missing**: infra errors, Missing>10%), `analysis {test, ci, alpha, spending, looks, multiplicity, futility}` (Phase 7 may carry `None`/Observational), `spend_caps`, `retry_policy {max_attempts, retry_on}`, `decision_rules` (**missing**), `signer_pubkey_hex`, `signed_at_ms`, `signature_hex`.
- `#[serde(deny_unknown_fields)]`, `schema_version` const, money as `i64` micros (TRIAL-DESIGN pitfall), no `f64` anywhere except in `analysis.alpha`-style fields that the canonicalizer already guards.
- Signer id: use the verifying-key hex itself (64 chars ≤ the 128-char structural telemetry bound) [ASSUMED A2].
- Phase 8 will consume the schema: define it complete now; a later field means a new signed `schema_version`.

**Campaign lifecycle** (persisted from `Signed`; `Draft` exists only in memory because the id is a content hash computed at signing):

| From | To | Guard |
|---|---|---|
| (new) Signed | — | `verify_campaign` ok; row + event in one tx |
| Signed | Running | CAS `UPDATE ... SET state='running' WHERE campaign_id=? AND state='signed'`, `changes()==1` |
| Running | Completed / Failed / Stopped{reason} | CAS from `running` |
| Signed | Stopped{Operator\|Superseded} | allowed (cancel before start) |
| any terminal | anything | rejected |
Supersession: new row with `supersedes_campaign_id`; whether the old row moves to `Stopped{Superseded}` is Open Question 9.

**Run lifecycle** (rows inserted *before* the call; fixes eval.rs end-only persistence): `running → {succeeded | failed{class} | interrupted}`; Phase 8 adds `reserved` (string vocabulary, no migration). Terminal rows are never updated: `UPDATE trial_runs SET state=?, ... WHERE run_id=? AND state='running'` + `changes()==1`. Recovery: a restarted runner marks stale `running` rows `interrupted` and creates `attempt_index+1`, `retry_of_run_id=old`. Without an owner column a second live runner cannot be told from a dead one: store `owner_instance_id` + `last_heartbeat_ms` on the run (or lease on the campaign) and only terminalize rows past the lease; Phase 7 is single-runner so require an explicit `--recover` flag [ASSUMED A3].

**Every place a session/run id is created today**
| # | Site | Id | Notes |
|---|---|---|---|
| 1 | `vox-research-shim/.../helpers.rs:86` `attempt_session_key` → `pipeline.rs:121-129` → `VoxDb::create_research_session` | `research:{fnv(query\|scope)}:{nanos}-{pid}-{seq}`; DB `i64` | per attempt; failure → `unwrap_or(0)` then label falls back to the key |
| 2 | `vox-orchestrator-mcp/.../memory_tools/handlers_memory.rs:383` | `research_async:{uuid v4}` precreated | passes `precreated_session_id` |
| 3 | `vox-cli-research/src/eval.rs:134` | `Uuid::new_v4()` eval `run_id` | never reaches the pipeline; written to `research_eval_runs` |
| 4 | `vox-cli/src/commands/model/bench/mod.rs:624` | `format!("bench-{}", now_ms())` | not unique under concurrent starts; `model_benchmark_results` |
| 5 | `vox-cli/.../eval_corpus.rs` | none yet | identity is `None` until 12-09 (`RunIdentity` doc: "filled by 12-09", provenance.rs:14) |
| 6 | `vox-actor-runtime/src/llm/chat.rs:291` | `"anon-session"` default | still the default for any call without `telemetry_session_id` |
| 7 | `metering::scope` → `run_trace_scope` (run_trace.rs) | one `trace_id` per run (or reuse outer) | landed |
Other callers of the pipeline that mint nothing themselves: CLI `vox research run` (lib.rs:575), MCP chat `research_turn.rs:623`, MCP sync `handlers_memory.rs:323`.

**Propagation recipe (TRIAL-01):** runner mints `run_id` (uuid v4); precreates the session with key `trial:{run_id}` (unique by construction; `create_research_session` errors on a repeated key, test `create_research_session_rejects_a_repeated_key`); calls `run_research_with_context_and_session(..., Some(session_id))`; the pipeline's `metering::set_run_session` label must become the `run_id` when the runner supplied one (today it is unconditionally `research_session:{id}`, pipeline.rs:131-135) - add a pre-seeded label to `metering::scope`; Tier A rows carry the identity; `research_trial.*` rows carry the five identity keys (already in `IDENTITY_KEYS`, research_trial.rs:17-23); join session↔run through the `session_key` convention (no ALTER on `scientia_research_sessions`).

**Grain (the biggest STORE-01 question, see Q3):** research wants run = one (arm, item, replicate, attempt) so Phase 8 can reserve spend per run row; Phase 12 wants run = one whole benchmark invocation with many samples and one provenance blob. Recommendation "item-batch grain": a run is an arm attempt over an *item batch*; `item_set_hash` + `item_count` live on the run, `item_id` + `sample_index` on samples. Research uses batch size 1, code benchmarks use all fixtures; one table set serves both.

#### Q3 STORE-01/02: vox-db mechanics

**Adding a domain (precedent: `5ff0c75dd`, files touched):** `schema/domains/<name>.rs` (a `pub const SCHEMA_X: &str`), `schema/domains/mod.rs` (`pub mod`), `schema/manifest.rs` (`SchemaFragment { name, sql }` appended to `SCHEMA_FRAGMENTS`, a `// 96: ...` comment line, bump `BASELINE_VERSION`), `contracts/db/baseline-version-policy.yaml` (`repository_baseline_integer` + `repository_baseline_digest_hex`; the test `baseline_policy_matches_compiled_schema` prints the new digest in its assert message, manifest.rs:240-267), `crates/vox-db/src/lib.rs` (module + re-exports; downstream crates import records as `vox_db::...`, e.g. eval.rs:6, so no vox-db-types edge is needed), `crates/vox-db-types/src/store_types/{mod,trials}.rs`, and `codex_legacy.rs` `LEGACY_EXPORT_TABLES` (sorted; test `legacy_export_covers_all_baseline_tables`, local_tests.rs:~514-520 compares it with `sqlite_master`). Import iterates that list in order and clears in reverse (codex_legacy.rs:379,445): choose table names whose alphabetical order is parent-before-child. With the draft names, `trial_campaign_events` sorts *before* `trial_campaigns` (`_` < `s`), so the event table would import before its FK parent; rename it (e.g. `trial_campaigns_log`) or drop its FK. `trial_runs` < `trial_samples` and `trial_campaigns` < `trial_runs` are already parent-first.

**Baseline number [VERIFIED: git show]:** `main` and `origin/main` are `BASELINE_VERSION: i64 = 94`; HEAD is `pub const BASELINE_VERSION: i64 = 95;` (manifest.rs:36, set by `139bdd20e`). Next free = **96**. I could not enumerate all 332 refs (shell loop returned nothing); no `.planning/workstreams/{autonomy-ux,code-atlas,docs-freshness}` plan mentions a baseline bump [A7]. Re-check `origin/main` and open PRs immediately before the migration commit. Phase 12's `12-09` text "Phase 7 must not reuse baseline 95" is still correct, but its "fold the uncommitted `model_benchmark_results`" is obsolete (committed).

**Migration mechanics:** `VoxDb::migrate` runs `baseline_sql()` only when `current_version < BASELINE_VERSION` (store/open.rs:126-129); fully idempotent `CREATE TABLE IF NOT EXISTS`, so DBs at 95 pick up the new tables on open; no ALTER is needed unless an existing table gains a column (precedents `hopper_inbox.resource_id`, open.rs:187-203). `current_version > BASELINE_VERSION` → `StoreError::LegacySchemaChain` (open.rs:114-124).

**The "shared DB carries the version without the tables" guard** does not exist. Failure mode: another branch/worktree binary stamps `schema_version=96` on the shared store; this branch then sees `current_version == BASELINE_VERSION`, skips `baseline_sql()`, and the first trial query hits "no such table". Add an `else` arm after the `current_version < BASELINE_VERSION` block: query `sqlite_master` for the `TRIAL_REQUIRED_TABLES` const and return a new `StoreError::BaselineStampedWithoutTables { version, missing }` with remediation text (pattern: `QuarantineDropAborted`, error.rs:~53; table probe: `codex_schema.rs::table_present`). Test: build a connection whose `schema_version` row is 96 and no trial tables (`mem_conn()` pattern in migration.rs tests) → `migrate` returns that error. Mutation: remove the arm → test fails.

**No CHECK, no triggers [VERIFIED]:** `scientia.rs:283` "Turso does not support CHECK constraints." and `scientia.rs:379-381` "DB-level triggers are NOT used because Turso/ libSQL rejects `CREATE TRIGGER`". `PRAGMA foreign_keys=ON` and `busy_timeout=5000` are applied (open.rs:19-23). So: vocabularies validated in Rust; terminal immutability and Signed→Running CAS by conditional UPDATE; no append-only trigger (append-only = no UPDATE/DELETE ops exist).

**Transactions - the existing pattern is not isolated.** `delete_knowledge_node` (knowledge.rs:340-362) and the a2a claim (ops_a2a.rs:129-214) do `BEGIN IMMEDIATE` … `COMMIT`/`ROLLBACK` through `self.conn.clone()`, but `GuardedConnection` takes its `tokio::Mutex` per statement (lib.rs:516-532). Another task on the same `VoxDb` can run a statement inside your open transaction (and be rolled back with it), and `last_insert_rowid()` is documented racy (lib.rs:455-471). The actor-runtime's LLM-turn recorder uses a separate global handle (`actor-runtime/src/db.rs:25 get_db()`), so cross-handle contention is SQLite-level (WAL, busy_timeout). Recommendation: add a `GuardedConnection::exclusive(|raw| ...)` helper that holds the mutex across the whole closure and hands out the raw `turso::Connection`; implement `persist_trial_batch` on it. **No failpoint pattern exists anywhere** (grep for failpoint/fail_after over vox-db, vox-db-types, research-shim, orchestrator: no hits). Introduce a plain public enum parameter (e.g. `PersistFailpoint::{None, AfterRunRow, AfterSamples(n)}`), not `cfg(test)`, because 12-09 tests live in another crate. Also use `INSERT ... RETURNING` or no rowid at all (text PKs); `create_research_session` already uses `RETURNING id`.

**Candidate DDL** (for the contract document; no CHECK/trigger; money as integer micros; no content columns, per G24):
```sql
CREATE TABLE IF NOT EXISTS trial_campaigns (
  campaign_id TEXT PRIMARY KEY,              -- Trusty URI of canonical prereg (id blanked)
  schema_version INTEGER NOT NULL,
  kind TEXT NOT NULL,                        -- observational|exploratory|holdout_confirmation (Rust-validated)
  state TEXT NOT NULL,                       -- signed|running|completed|failed|stopped
  stop_reason TEXT,
  prereg_json TEXT NOT NULL,                 -- canonical bytes exactly as signed; never updated
  signer_pubkey_hex TEXT NOT NULL,
  signature_hex TEXT NOT NULL,
  signed_at_ms INTEGER NOT NULL,
  supersedes_campaign_id TEXT REFERENCES trial_campaigns(campaign_id),
  converted_from TEXT,
  owner_instance_id TEXT, last_heartbeat_ms INTEGER,
  created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS trial_campaign_events (   -- append-only transition log
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  campaign_id TEXT NOT NULL REFERENCES trial_campaigns(campaign_id),
  from_state TEXT, to_state TEXT NOT NULL, reason_code TEXT, at_ms INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS trial_runs (
  run_id TEXT PRIMARY KEY,
  campaign_id TEXT NOT NULL REFERENCES trial_campaigns(campaign_id),
  arm_id TEXT NOT NULL, replicate_id INTEGER NOT NULL, attempt_index INTEGER NOT NULL,
  retry_of_run_id TEXT REFERENCES trial_runs(run_id),
  item_set_hash TEXT NOT NULL, item_count INTEGER NOT NULL,
  state TEXT NOT NULL,                       -- running|succeeded|failed|interrupted (Phase 8 adds reserved)
  error_class TEXT,
  research_session_id INTEGER,               -- scientia_research_sessions.id when research
  provenance_json TEXT NOT NULL,             -- D-09: canonical run provenance
  syntax_fingerprint TEXT, model_id TEXT, condition TEXT, problem_set_hash TEXT,  -- D-09 indexed scalars
  run_complete INTEGER NOT NULL,             -- 0/1
  tokens_in INTEGER, tokens_out INTEGER, cost_usd_micros INTEGER,
  llm_calls INTEGER, tool_calls INTEGER, duration_ms INTEGER, served_from_cache INTEGER,
  owner_instance_id TEXT, started_at_ms INTEGER NOT NULL, finished_at_ms INTEGER);
CREATE UNIQUE INDEX IF NOT EXISTS uq_trial_runs_identity
  ON trial_runs(campaign_id, arm_id, item_set_hash, replicate_id, attempt_index);
CREATE INDEX IF NOT EXISTS idx_trial_runs_state ON trial_runs(campaign_id, state);
CREATE INDEX IF NOT EXISTS idx_trial_runs_model ON trial_runs(model_id, condition, run_complete);
CREATE TABLE IF NOT EXISTS trial_samples (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT NOT NULL REFERENCES trial_runs(run_id),
  item_id TEXT NOT NULL,                     -- query_hash / fixture id, never query text
  sample_index INTEGER NOT NULL,
  outcome TEXT NOT NULL,                     -- ok|failed|abstained|infra_error (Rust-validated)
  failure_class TEXT,
  scores_json TEXT NOT NULL,                 -- versioned typed metric blob; NotApplicable/Missing explicit
  evidence_ref TEXT,                         -- reserved for Phase 9 Tier C; NULL now
  recorded_at_ms INTEGER NOT NULL);
CREATE UNIQUE INDEX IF NOT EXISTS uq_trial_samples_key ON trial_samples(run_id, item_id, sample_index);
```
"Attempt" is not a table: it is `attempt_index` + `retry_of_run_id` on `trial_runs` (TRIAL-04's "new linked run_id"). Existing near-duplicates to keep out of the new domain's way: `research_eval_runs/samples` (scientia.rs:93-121; legacy `vox research eval` keeps writing them when no campaign is given), `harness_eval_*` (harness_eval.rs).

**`model_benchmark_results` options for the human gate** (D-09):
| Option | Effect | Assessment |
|---|---|---|
| A. Separate writer, documented boundary (recommended) | `vox models bench` keeps writing it; contract doc states the boundary; optionally later add nullable link columns | no migration risk, no routing effect |
| B. Nullable `trial_run_id`/`campaign_id` columns on it | needs explicit `ALTER` in `migrate` (precedent open.rs:187-203) | only worth it if bench runs become campaigns |
| C. Projection/view over `trial_samples` | table already exists with data at 95 (would need DROP+recreate); columns (`suite`, `task_category`, `tokens_per_sec`) are not generic sample fields | **reject**: its only reader, `model_benchmark_summary` (ops_model_bench.rs:118-175), aggregates all rows per model into routing evidence (`bench_evidence::load`, bench_evidence.rs:28-39), so projecting campaign samples would change live routing |
12-09's must-have "read for that run returns one row per sample as a projection" and its Task 3 test `benchmark_results_projection_matches_samples` must be amended under A/B. Note eval-corpus does not write this table at all; it writes `model_scoreboard` behind `should_write_back` (eval_corpus.rs:310-314,1319-1329).

**Human gate package (what the checkpoint presents):** the DDL above, the grain decision, the vocabularies, the `model_benchmark_results` option, the guard text, and baseline number 96. Put it in `docs/src/architecture/research-trial-tier-a-schema-2026.md` with the required YAML frontmatter (`title`, `description`, `category`; AGENTS.md "Authored Markdown Frontmatter").

#### Q4 New crate and crate edges (PROPOSALS ONLY)

- **Layer:** `vox-research-trial-core = { layer = 2 }` in `docs/src/architecture/layers.toml` `[crates]` (L2 domain library; peers `vox-eval = { layer = 2 }` :185, `vox-config` :165). Its only workspace dep is `vox-crypto` (L1, :141), so same-or-down. Required: crate `description` (rule `description = "error"`), a `where-things-live.md` row (rule `where_things_live = "error"`, parity rule 12), an in-tree consumer (orphan = "error": `vox-cli-research`). Also WTL row 307 still says `crates/vox-prereg/` for prereg code that now lives in `vox-orchestrator/src/preregistration/` (stale; fix in the same PR).
- **How the guard works** [VERIFIED: crates/vox-cli-ci/src/crate_edges.rs:116-145]: every live normal/build edge must be in the frozen `edges` baseline or the `exceptions` ledger; dev-deps are ignored; layer presence is checked unconditionally; `--tighten` is removal-only. So a brand-new crate's every edge is a `NewEdge` violation until excepted. Entry shape: `{ "from", "to", "reason", "date", "authorized_by" }` (crate_edges.rs:44-51).
- **Proposed ledger entries** (executor writes none; one `checkpoint:human-action` early in Wave 1 so branch CI is not red for long):

| # | from → to | Why | Needed by |
|---|---|---|---|
| 1 | `vox-research-trial-core` → `vox-crypto` | ed25519 sign/verify, trusty hash, canonical bytes | TRIAL-02/05 |
| 2 | `vox-cli-research` → `vox-research-trial-core` | runner uses engine | D-02 |
| 3 | `vox-cli-research` → `vox-eval` | baseline statistics (D-03); existing baseline has `vox-cli → vox-eval` but not this edge | D-03 |
| 4 (optional) | `vox-cli` → `vox-research-trial-core` | 12-09 bundle code uses `SignerAllowlist`/identity | 12-09; alternative: `vox-cli-research` re-exports the module (no edge, since `vox-cli → vox-cli-research` exists) |
Current `vox-cli-research` edges (baseline, 8): `vox-db, vox-foundation, vox-orchestrator, vox-publisher, vox-repository, vox-research-shim, vox-search, vox-secrets` - so the Clavis loader needs no new edge, and `vox-crypto` is *not* an edge there: keep signing behind a trial-core `TrialSigner::from_hex` so entry 4' (cli-research→vox-crypto) is avoided.
- **Avoided edges:** no `vox-db → trial-core` (records are plain structs in `vox-db-types`, mapped in the runner), no `trial-core → vox-secrets/vox-research-events/vox-eval`.
- **Do not** write `max_loc`/`max_dependents` guesses; `loc_budget` is "off" (layers.toml:67).

#### Q5 TELEM-01/02

- **Allowlist landed:** `crates/vox-telemetry/src/research_trial.rs` - `RESEARCH_TRIAL_ALLOWLIST` types `research_trial.{campaign,run,stage,model_call,tool_call}`; `IDENTITY_KEYS = ["campaign_id","run_id","arm_id","attempt_index","replicate_id"]`; strings ≤128 chars matching `[A-Za-z0-9_.:+-]` (model-id keys also `/` `@`); duplicate keys, nested values, unknown keys, case-variant prefixes rejected; wired at `types.rs:254` inside `validate_research_metric_row`; both writers call it (`record_research_metric` research_pipeline.rs:150, `append_research_metric` codex_metrics_packages.rs:21); contract `contracts/telemetry/research-trial.v1.schema.json` + drift test `contract_schema_matches_allowlist`; event catalog row in `contracts/telemetry/events.v1.yaml`.
- **Remaining TELEM-01:** (i) **zero producers** exist for `research_trial.*` (grep: only tests); the runner must be the first, emitting `research_trial.campaign` (start/end) and `research_trial.run` (per attempt, from `RunUsage`); `stage`/`model_call`/`tool_call` rows can wait. (ii) The allowlist covers only `research_trial.`-prefixed rows. The legacy bridge still writes content into `research_metrics`: `scientia.finding_candidate_proposed` carries the whole candidate JSON (`research_event_metrics_bridge.rs:~134-150`) and `ClaimVerified` carries verifier model names. Decide whether trial runs disable that bridge (recommended: the trial runner builds its `ResearchConfig` with the content-bearing emitter off so "research-trial telemetry is structural only" is literally true). (iii) The unified LLM-turn table still stores full prompt/response (`chat.rs:~320-340` `record_unified_llm_turn`) under whatever `telemetry_session_id` is set: a STORE-03/Phase 9 classification decision, flagged not fixed. (iv) The canary test (OBSERVABILITY red test 2) is absent. (v) `CompositeRecorder::record` (recorder.rs:39-47) still fans out unfiltered; low value now that the write choke point validates, leave.
- **TELEM-02 landed:** `metering::scope` wraps each run (pipeline.rs:58) and `run_trace_scope` reuses an outer `TRACE_CTX` else mints one (run_trace.rs); `attempt_session_key` is unique per attempt (helpers.rs:86-99); `create_research_session` is `INSERT ... RETURNING id` and errors on a duplicate key (research_pipeline.rs:21-48). **Remaining:** bind the label to the Phase 7 `run_id` (Q2), and one hermetic integration proof that all stage calls in a run share a trace id and the label.
- `INSERT OR IGNORE` sites still present, none a session resolver: `scientia_claims` (research_pipeline.rs:196, keyed by FNV-1a of claim text, so identical claims across arms merge: Phase 8/9 concern) and `knowledge_nodes` (:834). `last_insert_rowid()` is still used by 20+ sites (codex_chat.rs, research.rs, harness_eval.rs); none is a research-session resolver.
- Trace context carries no campaign/run fields (`TraceContext`: task_id, parent_task_id, trace_id, span_depth, caller_agent_id; span.rs:11-22). Not needed in Phase 7: identity travels in the metering label and metric metadata.

#### Q6 METER-01

- `LlmResponse` fields [VERIFIED: actor-runtime/src/llm/types.rs:368-408]: `content`, `prompt_tokens: u32`, `completion_tokens: u32`, `model`, `cost_usd: Option<f64>`, `tool_calls`, `latency_ms: u64`, `cache_read_tokens: u32`, `ttft_ms: Option<u64>` (non-streaming path reports `latency_ms` here; doc comment says so), plus TPOT.
- **Landed (`2ff39c01c`, `edd0ab725`):** `metering.rs` - `Ttft::{NotApplicable, Measured(u64)}` (:25), `RunUsage {llm_calls, failed_llm_calls, prompt/completion/cache_read tokens, cost_usd, calls_with_unknown_cost, llm_latency_ms, tool_calls, ttft}` (:33), task-local `scope`/`snapshot`/`meter_response`/`meter_failure`/`set_run_session`/`tag_candidates`; call sites in `stages.rs:544-545`, `planner.rs:79,89`, `claims.rs:179,183`, `verifier.rs:388,392`; `ResearchMetadata.usage` (types.rs:349) and `refresh_run_totals` after persistence (pipeline.rs:1161-1164). `meter_response` always passes `streamed=false`, so TTFT is explicitly `NotApplicable` for the whole research path.
- **Remaining:** (1) **eval never reads `usage`** - eval.rs has no reference to `usage`, `duration_ms`, `ttft` (grep); samples carry only wall-clock latency (eval.rs:33,62). (2) Tool-call count is *model-requested* calls only; search-provider calls live in `retrieval_diagnostics.providers[].calls` (types.rs:130,139) - BASE-01 "tool calls/steps" should sum both and add `llm_calls`. (3) Cost: `cost_usd` sums known costs; `calls_with_unknown_cost > 0` must surface as Missing, never 0. Store micros as integer in Tier A. (4) Calls on separately spawned tasks leave the scope (metering.rs module doc) - `vox-research-shim` research dir has one `tokio::spawn` (mesh_subscriber.rs:155, unrelated); verifier uses `buffered` on the same task, fine. (5) Cache hits report `usage = Default` (pipeline.rs:86): stratify or bypass. (6) Real streaming TTFT primitive now exists (`llm_stream_measured` + `vox_llm_egress::MeasuredCompletion`, added by `139bdd20e`, stream.rs:~72-108) but records no telemetry and is not used by research: keep `NotApplicable`; do not wire it in Phase 7.
- **Reuse from Phase 12:** `LlmConfig.seed` (types.rs:70) - trial runner should record it in the campaign's `condition`, not change defaults; nothing else in 12-01..12-08 touches `LlmResponse`.

#### Q7 SCORE-04 on the current tree

| Item | State at HEAD | Evidence |
|---|---|---|
| R1 hallucinated no-evidence answer ≠ abstention | **fixed** (abstention = leading `ABSTENTION_MARKER`) | eval.rs:362-366; test eval.rs:597; marker `pub const ABSTENTION_MARKER: &str = "[INSUFFICIENT_EVIDENCE]";` config.rs:12; synthesis prompt asks for it stages.rs:197 |
| R2 zero-citation answer ≠ precision 1.0 | **fixed at eval level**; the pipeline audit still returns 1.0 for zero citations (pipeline.rs:1274-1277) and a test pins it (pipeline.rs:~1588) | `effective_citation_precision` eval.rs:354-360 |
| R3 negation-aware groundedness | **fixed** (per-sentence polarity) | evaluation.rs:100-126; tests semcov_wave21_tests.rs:267-290 |
| R4 short/vacuous answer ≠ auto-pass | **fixed** (vacuous → 0.0) | evaluation.rs:79-81; test :260 |
| R5 no 0.5 imputation for missing gold | **partly**: `legacy_quality_score` fixed (eval.rs:322-327) but `build_eval_report` still does `s.recall_at_5.unwrap_or(0.0)` over all samples (eval.rs:260) so missing gold counts as 0; `calculate_recall_at_5` returns 1.0 for an empty gold word set (evaluation.rs:56-58) | residual |
| R6 judge failure is Missing, not a score | **open**: judge error uses `config.fallback_quality_score` = 40 (pipeline.rs:911, config.rs:173) although the field doc says "`quality_score` is then 0, never a synthetic default" (types.rs:319-321 area); `judge_error` is recorded. Switching 40→0 does not move the 50/70 finding bars (pipeline.rs:985-986), so low behavioural risk | residual |
| R7 report validates against schema | **partial**: test compares key sets only (eval.rs:661-679); `results.v1` has just 5 metrics (results.v1.schema.json) and `baseline.v1.json` is an offline placeholder (`run_id: "baseline-local-offline"`, latency 0.0) - **not** a BASE-01 baseline | needs v2 schema |
| R8 median / multi-hop substring | p50 **fixed** (eval.rs:279-291, test :633); multi-hop **open**: `query.contains("then")` etc. (eval.rs:369-373, duplicated in the test helper :388-392) matches "authentication" | residual |
| R9 fabricated `[9]` | fixed when no audit (eval.rs:329-351, test :567); unverified when the audit path is used | check |
| failed queries = worst score | **fixed** (`failed_query_sample`, eval.rs:294-319) | |
Also: default golden queries have no gold answers (eval.rs `default_golden_queries`), so any "answer quality" metric from recall is NotApplicable; see Q8. Not executed this session: run `cargo nextest run -p vox-search -E 'test(semcov_wave21)'` and `-p vox-cli-research --lib` in the background to confirm greens before relying on them.

#### Q8 TEST-01 and BASE-01

**TEST-01.** There is **no injection seam** in the pipeline: providers come from `ProviderRegistry::from_env_with_config` (pipeline.rs:~84, provider.rs delegates to `WebSearchDispatcher`) and LLM calls go through `chat_with_cascade`. Hermetic options, cheapest first: (1) the runner takes a `TrialExecutor` trait; the default-lane tests use a `FakeExecutor` returning canned `ResearchResult` JSON (types round-trip already tested: `scientia_phase_0a_types_round_trip.rs`); (2) split `evaluate_single_query_pipeline` into live-produce + pure `score_result(&ResearchResult,&GoldenQueryItem)` so R-tests need no pipeline (QUALITY-EVAL §5); (3) copy the `vox-search` `ctor` egress guard (proxy env → 127.0.0.1:9; lib.rs:100-120) into `vox-cli-research` and `vox-research-shim` test binaries as defence in depth. **Known default-lane offender:** `vox-cli-research/tests/eval_e2e_test.rs` runs `ResearchConfig::default()` through the real pipeline (live providers + LLM keys) with no `#[ignore]`; mark it `#[ignore = "live ..."]` (project style: `scientia_phase_0a_pipeline_smoke.rs:249`) and replace with a fake-executor test. `misguidance_attribution_test.rs` and `pipeline_failure_provider_log_test.rs` reference `ResearchConfig` with no provider-disable lines in a quick grep: audit them. Process-env mutation in tests (`stages.rs:730-733` `set_var`) is global; keep behind `serial_test`.

**Pipeline hygiene needed for a *valid* baseline** (measurement hygiene, not policy): the result cache short-circuits on a key that omits config and the pipeline writes back (pipeline.rs:83-92,1136; `cache_ttl_secs: 3600`, config.rs:186), so replicates 2..n of a query would be cache hits (tokens 0, latency tiny, reproducibility trivially 1.0). `cache_ttl_secs = 0` is not a clean bypass (`saturating_sub(...) <= 0` is true within the same second, pipeline_cache.rs:19-22). Add a default-false `ResearchConfig.bypass_result_cache` set only by the trial runner (read and write). The 14-day claim-verdict cache also leaks across replicates; for Phase 7 *record* it: add `verdict_cache_hits` to `ResearchMetadata` (serde default) - it is the numerator of BASE-01's knowledge-reuse rate - and leave isolation to RACE-02.

**BASE-01 metrics** (all from existing fields unless noted; NotApplicable/Missing explicit, never 0):
| Metric | Source | Aggregation (vox-eval) | Rule |
|---|---|---|---|
| answer quality | judge `quality_score` (Missing if `judge_error`); token recall only when gold exists | mean + `bootstrap_ci(v,reps,seed)` | default corpus has no gold → judge + claim support |
| groundedness | `calculate_groundedness` (lexical proxy, label it) and claim support = supported verdicts / `claims_extracted_count` | mean + CI | capped-out claims count unsupported |
| citation precision | `effective_citation_precision` | mean + CI | 0 citations: NotApplicable if abstained else 0 |
| citation diversity | `retrieval_diagnostics.distinct_domain_count` vs `min_distinct_domains` (types.rs:123) | mean | NotApplicable when abstained |
| retrieval hit rate | `retrieval_diagnostics.hit_rate` (types.rs:120) | mean | |
| latency / TTFT | `ResearchMetadata.duration_ms` (covers verification+persist after 2ff39c01c); TTFT = `Ttft::NotApplicable` | true p50/p95, cache-served stratified | |
| tokens / cost | `usage.prompt_tokens+completion_tokens`; `usage.cost_usd` + `calls_with_unknown_cost` | sums/means | unknown cost → Missing flag |
| tool calls / steps | `usage.tool_calls` + Σ `providers[].calls`; steps = `llm_calls` + provider calls | means | |
| completion rate | non-error attempts / attempts | `wilson_interval(successes,n,z)` | |
| reproducibility | needs ≥2 replicates with cache bypass: Jaccard of retrieved-source sets + |Δ groundedness| (numbers only stored) | mean | definition is a recommendation [A4] |
| knowledge-reuse rate | `served_from_cache` or `verdict_cache_hits>0` | rate, diagnostic only | |
Print `min_detectable_difference(n)` next to every comparison (n=30 resolves only ~25 points, QUALITY-EVAL §4). Output schema: new `contracts/reports/research-eval/results.v2.schema.json` (+ real validation, not key parity; `vox-jsonschema-util` is an L1 option). **Command shape (RACE-01 says one CLI surface):** extend the existing `ResearchCmd::Eval { queries, output, concurrency }` (lib.rs:88-110,205-209) with flags `--sign <draft>`, `--campaign <signed.json>`, `--baseline`, `--replicates N`; no new subcommand group, so no new `command-registry.yaml` row (new flags on an existing path), but re-run `vox ci command-sync` if flag docs are generated (AGENTS.md). The live baseline needs network + provider keys + spend: plan it as a final `checkpoint:human-verify` run; hermetic tests prove the metric plumbing with a fake executor.

#### Q9 Plan decomposition (tracer-first; gate = STORE-01)

| Plan | Wave | Content | Reqs | Gate |
|---|---|---|---|---|
| 07-01 | 1 | **Tracer.** `vox_crypto::canonical` (tests first) + new crate `vox-research-trial-core`: `CampaignPreregV1`, `sign_campaign`/`verify_campaign` typestate, `SignerAllowlist`, campaign lifecycle machine, `RunIdentity`. `checkpoint:human-action` for ledger entries 1-3 (4 if chosen) and the layers/WTL rows | TRIAL-01 (types), 02, 03, 05 (core) | pre-gate |
| 07-02 | 1 | SCORE-04 residuals (R5/R6/R8, citation recall imputation), pure `score_result` split, fixtures, `eval_e2e_test` → ignored + fake-executor equivalent, egress guard | SCORE-04, TEST-01 | pre-gate |
| 07-03 | 1 | Clavis SecretIds (signer private key, signer allowlist CSV), `secret-env-guard`/`secrets-parity` regen | TRIAL-05 (key source) | pre-gate |
| 07-04 | 2 | Orchestrator `trusty_uri` rewired onto the generic canonicalizer (golden-bytes + mutation both ways); `verify_campaign` id-recompute proof | TRIAL-05 | pre-gate |
| 07-05 | 2 | Pipeline attempt seam: `bypass_result_cache`, pre-seeded `metering` label, judge failure Missing, `verdict_cache_hits` | TELEM-02, METER-01, SCORE-04 | pre-gate |
| 07-06 | 2 | **STORE-01 contract doc + `checkpoint:decision`** (DDL, grain, vocab, model_benchmark_results option, baseline 96, guard) | STORE-01 | **the gate** |
| 07-07 | 3 | vox-db `trials` domain: records, DDL, manifest/policy digest, legacy list, loud guard, exclusive-tx helper, persist + failpoint, conditional lifecycle ops | STORE-01, STORE-02 | post-gate |
| 07-08 | 4 | Runner in `vox-cli-research`: Clavis allowlist loader, typestate gate call, run loop, precreated session `trial:{run_id}`, interrupted→retry, `research_trial.*` producers, canary test, fake-executor tests, mutation proofs | TRIAL-01,04,05, TELEM-01,02, METER-01, TEST-01 | post-gate |
| 07-09 | 5 | BASE-01: metrics aggregation on `vox-eval`, `results.v2` schema, `--baseline/--replicates`, docs, `checkpoint:human-verify` live baseline | BASE-01, METER-01 | post-gate |
12-09 unblocks after 07-07 and 07-08; send Phase 12 the amended interface (Q1 bound change, Q3 option A, loader location). Lean verification per project memory: `cargo check` + new tests only; hosted CI is the gate; commits pathspec-scoped (shared tree).

### Anti-Patterns to Avoid
- **Copying `delete_knowledge_node` for the persist:** not isolated on the shared connection (Q3).
- **A second canonicalizer or `sha2` import** (crypto policy; detector `vox/crypto/banned-crate-import`).
- **Reading signing keys via `std::env`:** Clavis only (AGENTS.md).
- **Adding SQL CHECK/triggers:** Turso rejects them; they silently cannot exist.
- **Treating the legacy `PreregGate` as the TRIAL-05 gate:** wrong input type, no caller; build the typestate gate.
- **Storing query/answer/URL text in Tier A or telemetry** (G24): ids and hashes only.
- **Baselining with the result cache on:** invalid replicates (Q8).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Signing / hashing | own envelope or `sha2`/`ed25519_dalek` import | `vox_crypto::facades::{sign, verify, verify_signature_hex, secure_hash, interop_sha256}` | AGENTS.md; banned-import detector |
| Bootstrap CI, Wilson, McNemar, Holm, MDD | new stats code in trial-core | `vox_eval::corpus_stats` | D-06; already tested |
| Metric validation | new key filter | `vox_telemetry::research_trial` + `validate_research_metric_row` | landed, contract-drift tested |
| Usage accumulation | new counters | `vox_research_shim::research::metering::RunUsage` | landed |
| DB writes | raw SQL from the runner | `VoxDb` methods in `vox-db` (Tier A owner) | `data-storage-policy.v1.yaml` |
| Median/quantile | ad hoc | extend `vox-eval` if a helper is needed (D-06), not eval.rs | single stats home |

**Key insight:** the dangerous parts here are not algorithms but bypass paths (a gate no one calls, a transaction other tasks can join, a cache that answers for a different config); tests must prove the bypass is closed, by mutation.

## Runtime State Inventory

Phase 7 adds tables and a crate; it renames nothing, but it moves identity for existing things:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | existing `research_eval_runs/samples` rows (`run_id` = uuid), `scientia_research_sessions` rows keyed `research:...` / `research_async:...`, `model_benchmark_results` (baseline 95 data) | none migrated; campaign mode writes only `trial_*`; legacy mode unchanged; document the boundary |
| Live service config | none | None - verified: no external service stores campaign state |
| OS-registered state | none | None - verified |
| Secrets/env vars | new SecretIds + env names (`VOX_*` canonical) once added; `VOX_NANOPUB_SIGNING_KEY_HEX` untouched | secrets contracts regenerated by `vox ci secrets-parity` flow |
| Build artifacts | `contracts/ci/crate-graph.v1.json`, `crate-build-map.v1.json`, `workspace-hack` (hakari) will list/need the new crate | regenerate via the repo's commands; never hand-edit baselines to admit edges |

## Common Pitfalls

### Pitfall 1: Tampered `campaign_id` verifies
**What goes wrong:** copying `verify_prereg` verbatim leaves `id` unauthenticated. **Why:** `canonical_json` blanks `id`. **Avoid:** recompute and compare in `verify_campaign`. **Warning sign:** no test flips `id`.

### Pitfall 2: Transaction not actually atomic under concurrency
**What goes wrong:** another task's statement joins the open BEGIN and is rolled back (or `BEGIN` nests). **Why:** per-statement mutex. **Avoid:** exclusive helper holding the lock for the closure. **Warning sign:** flaky `forced_mid_write_failure` under `multi_thread` runtime.

### Pitfall 3: Shared-DB version stamp without tables
See Q3 guard. Also `current_version > BASELINE_VERSION` is already a hard error for forward-incompatible binaries.

### Pitfall 4: Baseline on cached results
Replicates become cache hits; tokens/latency/reproducibility are fiction. Bypass flag + stratify `served_from_cache`.

### Pitfall 5: Silent imputation returns
`unwrap_or(0.0)` in report builders (eval.rs:260) and `fallback_quality_score` (pipeline.rs:911). Treat absence as `Missing`/`NotApplicable`.

### Pitfall 6: Canonical float formatting is serde_json's
Third-party offline verifiers must reproduce it. Golden-bytes test; document; consider JCS later.

### Pitfall 7: Crate edge ledger red CI
New crate + deps make `vox ci crate-edges` fail until the user writes entries. Sequence the human-action early; do not regenerate baselines.

### Pitfall 8: `last_insert_rowid()` and `INSERT OR IGNORE`
Racy on the shared connection (lib.rs:455-471). Use text primary keys or `RETURNING`.

### Pitfall 9: `LEGACY_EXPORT_TABLES` order/coverage
Missing a new table fails `legacy_export_covers_all_baseline_tables`; wrong order can break FK import.

### Pitfall 10: Phase 12 contract drift
12-09 assumes `Serialize`-only canonicalizer, ambient `verify_signer_allowlisted(pubkey)`, a projection, "no new SecretId". Send amendments with the SUMMARY (pass the allowlist explicitly so offline third parties are not tied to ambient config).

## Code Examples

### Typestate gate (shape, not final names)
```rust
// vox-research-trial-core (L2, no I/O). Source: pattern from orchestrator/preregistration/{gate,signing}.rs
pub struct SignedCampaign { /* prereg + signer_pubkey_hex + signature_hex */ }
pub struct VerifiedCampaign(SignedCampaign);          // private ctor: only verify_campaign builds it

pub fn verify_campaign(
    s: &SignedCampaign, allow: &SignerAllowlist, _now_ms: i64,
) -> Result<VerifiedCampaign, GateRefusal> {
    if !allow.contains(&s.signer_pubkey_hex) { return Err(GateRefusal::SignerNotAllowlisted); } // check BEFORE crypto
    let bytes = vox_crypto::canonical::canonical_json_bytes(&s.prereg_without_id())?;           // every depth, rejects NaN
    if !vox_crypto::verify_signature_hex(&s.signer_pubkey_hex, &bytes, &s.signature_hex)
        .map_err(GateRefusal::BadKey)? { return Err(GateRefusal::BadSignature); }
    if compute_trusty_id(&bytes) != s.campaign_id { return Err(GateRefusal::IdMismatch); }       // missing in verify_prereg today
    Ok(VerifiedCampaign(s.clone()))
}
// runner: pub async fn run_campaign(signed: SignedCampaign, allow: &SignerAllowlist, ...) { let v = verify_campaign(&signed, allow, now)?; ... }
```

### Exclusive transaction with failpoint (shape)
```rust
// vox-db. The closure runs with the GuardedConnection mutex held; `raw` is the inner turso::Connection.
pub async fn persist_trial_batch(&self, b: &TrialRunBatch, fp: PersistFailpoint) -> Result<(), StoreError> {
    self.conn.exclusive(|raw| async move {
        raw.execute("BEGIN IMMEDIATE", ()).await?;
        let r = async {
            insert_or_finish_run(raw, b).await?;
            if fp == PersistFailpoint::AfterRunRow { return Err(StoreError::Internal("failpoint".into())); }
            insert_samples(raw, b).await?;
            raw.execute("COMMIT", ()).await?; Ok::<_, StoreError>(())
        }.await;
        if r.is_err() { let _ = raw.execute("ROLLBACK", ()).await; }
        r
    }).await
}
```

### Loud baseline guard (shape)
```rust
// store/open.rs, after the `if current_version < BASELINE_VERSION { ... }` block
else { let missing = missing_tables(conn, TRIAL_REQUIRED_TABLES).await?;
       if !missing.is_empty() { return Err(StoreError::BaselineStampedWithoutTables { version: current_version, missing }); } }
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Gate trusts embedded key | explicit trusted-signer set | `0cde42c9b` (2026-10-11) | allowlist exists; caller + key source missing |
| Top-level-only key sort, NaN→null | recursive sort, non-finite rejected | `0cde42c9b` | promote, don't rewrite |
| `INSERT OR IGNORE` + `last_insert_rowid` session | `INSERT ... RETURNING` + per-attempt key | `edd0ab725`, `24b1aa5a8` | TELEM-02 mostly done |
| LLM usage dropped | `RunUsage` task-local accumulator | `2ff39c01c` | METER-01 consumption missing |
| `model_benchmark_results` uncommitted | committed at 95, drives routing | `139bdd20e` | cannot be a projection |

**Deprecated/outdated:** TRIAL-DESIGN §1 claims about `trusty_uri.rs:13`/`:38-41` (sha2, top-level sort) and OBSERVABILITY §1-3 gap list are fixed or changed; `baseline.v1.json` is a placeholder.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Hosting the canonicalizer in `vox-crypto` is acceptable (adds serde/serde_json to the crypto crate) | Q1 | fallback: `vox-research-events`, +1 edge each for trial-core/cli-research |
| A2 | Signer key id = verifying-key hex | Q2 | 12-09 may want a short fingerprint; cheap to add later |
| A3 | Single runner per campaign; recovery via explicit `--recover` | Q2 | concurrent runners need a lease protocol |
| A4 | Reproducibility = source-set Jaccard + groundedness spread | Q8 | metric definition; user may prefer claim-verdict agreement |
| A5 | New SecretIds are acceptable (vs reusing `VoxNanopubSigningKeyHex`) | Q1 | extra secret surface vs key conflation |
| A6 | uuid v4 for `run_id` | Q2 | none material |
| A7 | 96 is unclaimed everywhere (only main=94, HEAD=95 verified) | Q3 | re-check before migrating |
| A8 | JCS crate alternative not evaluated/legitimacy-checked | Q1 | only matters if chosen |
| A9 | serde_json float output is stable enough for signatures within a release | Q1 | golden test catches drift |
| A10 | `cargo hakari` needs a workspace-hack refresh for the new crate | Stack | CI hakari check fails if missed |
| A11 | Live baseline of 30 queries x N replicates fits a modest spend cap | Q8 | cost unknown; set `--max-usd` style cap |
| A12 | Tests named in this document pass at HEAD | Q7 | not executed this session |

## Open Questions

1. **Run grain (gate).** Item-batch grain (Q2/Q3 recommendation) vs per-item runs vs per-invocation runs. Recommend item-batch; the human must confirm because 12-09's persist and Phase 8's reservation both bind to it.
2. **`model_benchmark_results` relation (gate).** Recommend option A (separate writer, documented boundary); C is unsafe because routing reads the table. Amend 12-09.
3. **Signer secrets.** Two new SecretIds (recommended) or reuse `VoxNanopubSigningKeyHex` for the private key? Where does the verifier get the allowlist when run by a third party (explicit parameter, recommended)?
4. **Canonicalizer home.** `vox-crypto` (recommended, zero edges) or `vox-research-events`.
5. **Content in the unified LLM-turn table** (`record_unified_llm_turn`, full prompt/response): leave for STORE-03, or suppress for trial sessions now?
6. **Baseline "answer quality" without gold.** Judge score + claim support (recommended) or author a small gold set? Also confirm corpus (30 default queries) and replicate count.
7. **Who runs the live baseline** (keys, network, spend cap) and where the baseline artifact is committed.
8. **Legacy event bridge in trial runs:** disable (recommended) or keep and scope the canary to `research_trial.*` rows?
9. **Supersession:** does superseding a non-terminal campaign force it to `Stopped{Superseded}`?
10. **Edge #4** (`vox-cli → trial-core`) vs re-export through `vox-cli-research`.
11. **`max_loc`/budget** for the new crate (loc_budget guard is off; growth detection skips crates without `max_loc`).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | all code plans | ✓ (broker-wrapped alias) | toolchain `1.98.1` (rust-toolchain.toml) | — |
| cargo-nextest | validation | ✓ | `~/.cargo/bin/cargo-nextest` | `cargo test` |
| jq | contract inspection | ✓ | /opt/homebrew/bin/jq | — |
| `vox` CLI | `vox ci ...` gates | ✓ | `~/.cargo/bin/vox` (version not probed) | `cargo run -p vox-cli -- ci ...` |
| Provider API keys / network / SearXNG | live BASE-01 run | not probed | — | none: human-run live lane; hermetic tests cover plumbing |
| Docker / `act` | local CI replica | not probed | — | hosted CI is the gate |

**Missing dependencies with no fallback:** none for code. **Live baseline** is blocked until a human supplies keys and approves spend.

## Validation Architecture

Framework: Rust `#[test]`/`#[tokio::test]` under cargo-nextest (profiles `default`, `ci`, `ci-gate` in `.config/nextest.toml`). `workflow.nyquist_validation` is absent from `.planning/config.json`, so this section is required. Run every cargo command in the background with output to the scratchpad (project rule); test names below are proposals for the planner to bind.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest 1.98.1 toolchain; `wiremock`/`serial_test` for the few HTTP/env tests |
| Config file | `.config/nextest.toml` |
| Quick run command | `cargo nextest run -p <crate> -E 'test(<filter>)'` |
| Full suite command | hosted CI `ci-gate`; local equivalent `vox ci pre-push --complete` (executors do not run it) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TRIAL-02 | sign freezes; any field flip, `campaign_id` flip, or NaN fails; supersession links | unit | `cargo nextest run -p vox-research-trial-core -E 'test(campaign::)'` | ❌ Wave 0 |
| TRIAL-03 | illegal campaign/run transitions rejected | unit | `cargo nextest run -p vox-research-trial-core -E 'test(lifecycle::)'` | ❌ Wave 0 |
| TRIAL-05 | foreign signer / unsigned / tampered refused by the gate and by the runner entry | unit | `cargo nextest run -p vox-research-trial-core -E 'test(gate::)'` and `-p vox-cli-research --lib -E 'test(trial::runner::refuses)'` | ❌ Wave 0 |
| TRIAL-05 | canonical bytes: key order independent at depth, NaN rejected, golden bytes | unit | `cargo nextest run -p vox-crypto -E 'test(canonical::)'` | ❌ Wave 0 |
| TRIAL-05 | orchestrator trusty URI unchanged after rewire | unit | `cargo nextest run -p vox-orchestrator --lib -E 'test(preregistration::)'` | ✅ (existing) |
| TRIAL-01 | identity present on pipeline label, session key, `trial_*` rows, `research_trial.*` rows for one attempt | integration (fake executor, in-memory DB) | `cargo nextest run -p vox-cli-research --lib -E 'test(trial::identity_)'` | ❌ Wave 0 |
| TRIAL-04 | interrupted run terminalized immutable; retry has `attempt_index+1`, `retry_of_run_id` | integration | `cargo nextest run -p vox-cli-research --lib -E 'test(trial::retry_)'` | ❌ Wave 0 |
| STORE-01 | baseline 96 digest matches policy; new tables present; stamped-without-tables errors loudly | unit | `cargo nextest run -p vox-db --lib -E 'test(baseline_policy_matches_compiled_schema) \| test(trials::guard_)'` | ✅ policy test / ❌ guard |
| STORE-01 | `LEGACY_EXPORT_TABLES` covers new tables | unit | `cargo nextest run -p vox-db --lib -E 'test(legacy_export_covers_all_baseline_tables)'` | ✅ |
| STORE-02 | failpoint mid-write leaves no run/sample/campaign row, also under concurrent writers | integration | `cargo nextest run -p vox-db -E 'test(trials::forced_mid_write_failure)'` | ❌ Wave 0 |
| TELEM-01 | unknown/content keys rejected; trial runner emits only allowlisted rows; canary text never appears | unit + integration | `cargo nextest run -p vox-telemetry --lib -E 'test(research_trial)'` ✅; `-p vox-cli-research --lib -E 'test(trial::canary_)'` ❌ | mixed |
| TELEM-02 | all stage calls of one run share `trace_id` and the run label; two attempts of one query get distinct session keys | unit/integration | `cargo nextest run -p vox-actor-runtime --lib -E 'test(run_trace)'` ✅; `-p vox-research-shim --lib -E 'test(metering::)'` ✅ + new label test ❌ | mixed |
| METER-01 | usage/duration flow into run + sample scalars; unknown cost is Missing; TTFT NotApplicable | unit | `cargo nextest run -p vox-cli-research --lib -E 'test(trial::meter_)'` | ❌ Wave 0 |
| SCORE-04 | residual R5/R6/R8; failed query worst score; schema v2 validated | unit | `cargo nextest run -p vox-cli-research --lib -E 'test(eval::tests::)'` and `-p vox-research-shim --lib -E 'test(judge_)'` | ✅ partial |
| TEST-01 | default lane makes no network/model calls | unit | `cargo nextest run -p vox-cli-research -E 'not test(live_)'` with the egress guard ctor; `eval_e2e_test` ignored | ❌ Wave 0 |
| BASE-01 | fake-executor baseline emits every metric in the BASE-01 list with explicit Missing/NotApplicable; no policy field changed | integration | `cargo nextest run -p vox-cli-research --lib -E 'test(trial::baseline_)'` | ❌ Wave 0 |
| Crate edges | only approved edges | CI gate | `cargo run -q -p vox-cli -- ci crate-edges` and `cargo run -q -p vox-arch-check` | ✅ |
| Secrets | new ids registered/parity | CI gate | `vox ci secret-env-guard` ; `vox ci secrets-parity` | ✅ |

### Required mutation proofs (each: record the grep count before, break, show the named test FAILS, restore, show count and green)
1. Runner entry calls `verify_campaign` (count of the call in the runner file = 1): remove → `runner_refuses_unsigned_campaign`, `runner_refuses_foreign_signer` fail.
2. Allowlist membership check in `verify_campaign`: remove → `foreign_signer_refused` fails.
3. Trusty-id recompute: remove → `tampered_campaign_id_fails` fails.
4. `sort_keys` recursion / non-finite rejection in `vox_crypto::canonical`: revert → `nested_key_order_independent`, `nan_rejected` fail.
5. Failpoint rollback: commit before the failpoint (or drop `ROLLBACK`) → `forced_mid_write_failure_leaves_no_partial_run` fails.
6. Baseline guard arm: remove → `stamped_version_without_tables_errors` fails.
7. Terminal immutability (`WHERE state='running'` + `changes()==1`): remove → `terminal_run_is_immutable` fails.
8. Telemetry write choke point: bypass `validate_research_metric_row` in the runner's emitter → canary test fails.
9. Judge-failure Missing (07-05): restore `fallback_quality_score` use → `judge_error_is_missing` fails.

### Sampling Rate
- **Per task commit:** `cargo check -p <touched crates>` + that task's new tests (background).
- **Per wave merge:** all new tests of the wave, one nextest filter per crate.
- **Phase gate:** hosted CI green (`ci-gate`), plus the nine mutation proofs recorded in SUMMARY files.

### Wave 0 Gaps
- [ ] `vox-research-trial-core` crate + test modules; `vox-crypto/src/canonical.rs` tests
- [ ] `vox-db` trials tests (guard, failpoint, lifecycle CAS) using `DbConfig::Memory`
- [ ] `vox-cli-research` `trial::` test modules, fake `TrialExecutor`, egress guard `ctor` dev-dependency (already a workspace dep; adds no new package)
- [ ] fixtures: canned `ResearchResult` JSON archetypes (good, hallucinated-no-evidence, uncited, negation flip, fabricated `[9]`, abstention, failure)

## Security Domain

`security_enforcement` is absent from config, so it applies.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (signer identity) | ed25519 via `vox-crypto`; allowlisted verifying keys; typestate gate |
| V3 Session Management | no | — |
| V4 Access Control | yes | only Clavis-configured keys may sign; allowlist empty = deny; Tier A writes only through `vox-db` |
| V5 Input Validation | yes | `deny_unknown_fields` on prereg/campaign JSON; telemetry allowlist; Rust-validated vocabularies |
| V6 Cryptography | yes | `vox-crypto` only; no `sha2`/`ed25519_dalek` imports outside it; secret key never logged or serialized |
| V7 Error handling/logging | yes | refusals carry reason codes, never key material or prompt text |
| V8 Data protection | yes | no query/answer/URL in telemetry or Tier A (G24) |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Self-signed campaign by an arbitrary key | Spoofing | allowlist check before crypto; foreign-signer test |
| Edited prereg / edited `campaign_id` | Tampering | canonical bytes + id recompute |
| Replay of an old signed campaign as new | Tampering/Repudiation | `campaign_id` is content-addressed; `signed_at`/`supersedes` inside the signed bytes; Running CAS prevents double start |
| TOCTOU between verify and run | Tampering | typestate `VerifiedCampaign` consumed by the runner |
| Canonicalization ambiguity (duplicate keys, NaN, key order) | Tampering | one canonicalizer, round-trip check, golden bytes |
| Partial writes | Tampering | exclusive transaction + failpoint test |
| Content leak via telemetry | Information disclosure | row allowlist + canary test; trial emitter off the content bridge |
| Secret in logs/bundles | Information disclosure | `SecretId` resolve only; no `LlmConfig` serialization (12-09 T-12-09-05) |
| Allowlist read from attacker-controlled env | Elevation | document that ambient config is the trust root; offline verifiers take the allowlist as an explicit argument |

## Sources

### Primary (HIGH confidence - read this session)
- `07-CONTEXT.md`, `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`, `12-09-PLAN.md`, `12-CONTEXT.md` (D-09/D-10), `12-VALIDATION.md`
- `.planning/research/{SUMMARY,TRIAL-DESIGN,OBSERVABILITY,QUALITY-EVAL}.md` (re-verified; stale parts noted)
- `crates/vox-orchestrator/src/preregistration/{gate,signing,trusty_uri,mod}.rs`, `research_gate.rs`
- `crates/vox-crypto/{Cargo.toml,src/facades.rs}`; `crates/vox-secrets/src/spec/{ids.rs,registry/*.rs,mod.rs}`; `crates/vox-publisher/src/scholarly/flags.rs`
- `crates/vox-db/src/{schema/manifest.rs,schema/domains/{scientia,harness_eval,publish_cloud}.rs,store/open.rs,lib.rs,migration.rs,codex_legacy.rs,codex_schema.rs,research_pipeline.rs,store/ops_model_bench.rs,store/ops_memory/knowledge.rs,store/ops_a2a.rs}`; `contracts/db/{baseline-version-policy,data-storage-policy.v1}.yaml`
- `crates/vox-telemetry/src/{research_trial,types,span,recorder}.rs`; `contracts/telemetry/{research-trial.v1.schema.json,events.v1.yaml}`
- `crates/vox-research-shim/src/research/{metering,types,provider}.rs`, `orchestrator/{pipeline,pipeline_cache,helpers,config,stages}.rs`, `research_event_metrics_bridge.rs`
- `crates/vox-actor-runtime/src/llm/{types,chat,run_trace,stream}.rs`
- `crates/vox-cli-research/src/{lib,eval}.rs`, `tests/eval_e2e_test.rs`; `crates/vox-cli/src/commands/model/{eval_corpus,bench/mod}.rs`; `crates/vox-eval/src/{lib,corpus_stats,provenance,report}.rs`; `crates/vox-search/src/{evaluation,lib}.rs`; `crates/vox-orchestrator/src/models/bench_evidence.rs`
- `docs/src/architecture/layers.toml`; `contracts/ci/crate-edges.allow.v1.json`; `crates/vox-cli-ci/src/crate_edges.rs`; `AGENTS.md`
- git: `git show --stat` of `0cde42c9b`, `139bdd20e`, `b03cc6491`, `409e5711c`, `b9caf4108`, `d0b799378`, `24b1aa5a8`, `6f5d7ddb3`, `6ca8d70fc`, `2ff39c01c`, `edd0ab725`, `5ff0c75dd`; `git show main:.../manifest.rs`

### Secondary (MEDIUM)
- none (no web research was needed; all questions were about this tree)

### Tertiary (LOW)
- RFC 8785 JCS crate alternative and hakari behaviour: `[ASSUMED]`, not checked

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new third-party packages; every API read in source
- Architecture: MEDIUM-HIGH - crate-edge/layer rules and DB mechanics read in source; grain and Phase 12 interplay are recommendations pending the gate
- Pitfalls: HIGH for items cited with file:line; MEDIUM for concurrency behaviour (reasoned from `GuardedConnection`, not exercised)
- Test status: LOW-MEDIUM - nothing was executed this session

**Research date:** 2026-10-12
**Valid until:** 2026-10-19 (fast-moving shared tree; re-check `origin/main` baseline number and any new SecretId/crate-edge commits before planning waves 3+)
