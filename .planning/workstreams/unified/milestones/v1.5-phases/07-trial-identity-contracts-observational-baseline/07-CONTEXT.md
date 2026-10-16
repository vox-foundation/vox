# Phase 7 Context: Trial Identity, Contracts & Observational Baseline

**Source of decisions:** the user chose on 2026-10-11 to synthesize this from existing material rather than run discuss-phase. The sources are:

- `.planning/workstreams/research-trial-flywheel/REQUIREMENTS.md` (grill rulings G1–G25, review rulings R1–R9);
- `.planning/research/{SUMMARY,TRIAL-DESIGN,OBSERVABILITY,QUALITY-EVAL}.md`, researched 2026-10-01 and partly stale (see "Changed since research");
- the user's answers on 2026-10-11 (D-01 to D-03 below);
- Phase 12's binding asks (D-07 to D-09).

Those research docs predate this week's commits, so re-verify any `file:line` they cite.

## Locked decisions

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

## Decisions after research (user, 2026-10-12)

- **D-10 Canonicalizer home: `vox_crypto::canonical`.** One generic recursive canonicalizer that rejects non-finite values through a Serialize→Deserialize→PartialEq round-trip. Trial-core, the orchestrator `trusty_uri` and Phase 12 bundles all use it. It needs no new crate edges.
- **D-11 Signer keys: two new Clavis SecretIds.** One holds the trial signing key; `VoxResearchTrialSignerAllowlist` holds a CSV of hex Ed25519 pubkeys. `VoxNanopubSigningKeyHex` is not reused. Run `vox ci secret-env-guard` and `vox ci secrets-parity`, and update `docs/src/reference/secrets-ssot.md`.
- **D-12 Legacy content-bearing event bridge is disabled in trial runs.** When a run label/trial identity is set, the finding-candidate JSON bridge into `research_metrics` is skipped. Non-trial research keeps today's behavior. This needs a test that proves it.
- **D-13 BASE-01 answer quality is judge score plus claim-support rate.** They are separate versioned metrics with explicit Missing (no imputation). Gold answers are out of scope.
- **Defaults (planner may apply, no user input needed):**
  - Superseding a non-terminal campaign forces it to `Stopped{Superseded}`.
  - For 12-09, prefer a re-export through `vox-cli-research` over a new `vox-cli → vox-research-trial-core` edge, unless that is impossible.
  - The live baseline run is a `checkpoint:human-verify` that the user runs, because it needs keys, network and spend.
- **Still for the STORE-01 gate (07-06 checkpoint):**
  - Run grain: the research recommends item-batch.
  - The `model_benchmark_results` option: the research recommends A, a separate writer with a documented boundary. That is because it feeds live routing, so making it a projection would be a policy change.

## Changed since research (verify before planning)

- **Nine 2026-10-01 commits already shipped most of SCORE-04, TELEM-01, TELEM-02 and METER-01.** These are `b03cc6491`, `409e5711c`, `b9caf4108`, `d0b799378`, `24b1aa5a8`, `6f5d7ddb3`, `6ca8d70fc`, `2ff39c01c` and `edd0ab725`. Plans cover only their residuals (see 07-RESEARCH.md).

- **TRIAL-05 is partly delivered by `0cde42c9b`** ("allowlist signers, canonicalize at every depth"):
  - `PreregGate::new(trusted_signers)` refuses keys outside the allowlist;
  - `canonical_json` sorts at every depth and rejects NaN/inf;
  - `trusty_uri` hashes through `vox_crypto::facades::interop_sha256`.
  Still missing: a **production caller** of the gate (mutation-tested), the new `CampaignPreregV1`, and the allowlist's real key source.
- **`139bdd20e`** shipped measured-quality model routing, `vox models bench` and `model_benchmark_results` at baseline 95.
- **Phase 12 plans 12-01 to 12-08 landed**, including `LlmConfig.seed`, the typed eval report and provenance gate in `vox-eval`, eval-compare and the AdapterCard lineage. The METER-01/TELEM work should reuse what touches `LlmResponse`/`LlmConfig`.

## Scope fences

- Phase 7 is only **identity, contracts, atomic persistence, structural telemetry, metering, evaluator correctness, hermetic tests, and one observational baseline**.
- **No policy change:** BASE-01 runs the current pipeline as is.
- **Out of scope:**
  - race harness, budgets, stopping (Phase 8);
  - Tier C evidence storage (Phase 9, STORE-03); until then campaigns are observational, never replay- or promotion-eligible;
  - promotion (Phase 10).
- **Hermetic default tests (TEST-01):** no model or HTTP access; live coverage runs only in an ignored/live lane.

## Project rules that bind every plan

- Test-first for every new `pub fn`.
- Crypto only via `vox-crypto`.
- LLM calls only via the `vox_actor_runtime::llm` facade.
- Secrets via Clavis.
- No new crate edges beyond D-02/D-03, and those are user-approved.
- Background every cargo command.
- Lean executor verification: `cargo check` plus new tests.
- Prove security guards by mutation, with before/after presence checks.
- The tree is shared with another active session: commits are pathspec-scoped only.
