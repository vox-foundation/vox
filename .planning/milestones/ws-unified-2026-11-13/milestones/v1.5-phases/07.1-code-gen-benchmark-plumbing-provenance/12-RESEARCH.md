# Phase 12: Code-Gen Benchmark Plumbing & Provenance - Research

**Researched:** 2026-10-10
**Domain:** Rust workspace plumbing (LLM facade routing, seeded sampling, paired statistics, provenance records, training lineage, signed run bundles)
**Confidence:** HIGH for repo facts (every claim below was read in this session), MEDIUM for Phase 7 interface (Phase 7 has no plans yet), LOW for two policy-fit items flagged in the Assumptions Log.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01 One identity scheme.** Benchmark runs are Phase 7 campaigns and runs (`campaign_id`, `run_id`, signed preregistration, atomic Tier A persistence). There is no second identity scheme. `model_benchmark_results` (scientia, uncommitted on this branch) becomes a projection or view over these tables, not a parallel writer.
- **D-02 Routing.** `eval_corpus.rs::generate_candidate` must resolve the model through the registry and `vox_actor_runtime::llm`, including local `mens/*` checkpoints. Remove the hardcoded `LlmConfig::openrouter`. There is no MENS special-case path: same corpus, verifier and scoring as every other model.
- **D-03 Sampling.** n is fixed, there is never an early stop, and the seed is recorded and passed. `vox mens eval-local` pass@k must use `vox_eval::pass_at_k` (the unbiased estimator), not "any sample passed".
- **D-04 Statistics.** Reuse `vox-eval` `corpus_stats` (`bootstrap_ci`, `paired_compare`/exact McNemar, `holm_reject`, `min_detectable_difference`). Write no new stats code. CI overlap is never a significance test.
- **D-05 Provenance fields** are the design doc §5 list. A missing required field forces `run_complete = false` and exclusion from comparison. Nothing is backfilled after the fact.
- **D-06 Training lineage.** The training lane writes `AdapterCard` (`crates/vox-populi/src/mens/tensor/adapter_card.rs`) with the `train.jsonl` sha256 (a hash, not a path), the corpus hash, the base revision, the adapter sha256 and the training syntax fingerprint. Until Phase 13 lands the behavioral fingerprint, the training syntax fingerprint uses the existing `compute_ebnf_hash()`, and its algorithm is recorded in a field (`syntax_fingerprint_kind`).
- **D-07 Signing.** Signing and hashing go through `vox-crypto`; there are no direct `sha2`/`blake3`/`ed25519` imports outside it. Reuse the preregistration signing in `crates/vox-orchestrator/src/preregistration/signing.rs` or the Phase 7 equivalent.
- **D-08 Out of scope for Phase 12:** the behavioral syntax fingerprint, workflow triggers, new corpus tiers and cross-language verifiers (all Phase 13).

### Claude's Discretion
- (CONTEXT.md names none explicitly.) Treated as discretion: wave/plan decomposition, the typed-report location, the compare surface (subcommand vs flag), the per-sample seed derivation, the `top_p` handling, and the Phase 7 interface boundary.

### Deferred Ideas (OUT OF SCOPE)
- Results server (default: shared vox-db via `VOX_DB_URL`) and MENS inference host (recommendation: local-host submission first) are "Open (not needed for Phase 12)".
- Everything in D-08 (BENCH-07..BENCH-12).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BENCH-01 | `eval-corpus` resolves the model through registry + `vox_actor_runtime::llm`, incl. `mens/*` | Resolver exists (`resolve_target` + `llm_config_for_spec`); needs `MensCatalog` refresh and a no-synthesize guard for `mens/*`. No new crate edge needed. See "BENCH-01". |
| BENCH-02 | Seeded, recorded sampling; all n drawn; `eval-local` uses unbiased pass@k | Facade has **no seed anywhere**; local MENS sampler is unseeded `thread_rng`; `eval_local` draws one sample. Needs seed plumbing in 4 layers. See "BENCH-02". |
| BENCH-03 | Reports use bootstrap CI, exact McNemar + Holm, resolution floor | All primitives exist with zero callers; `bootstrap_ci` is single-arm, so a paired difference is `bootstrap_ci` over per-problem differences. No compare command exists. See "BENCH-03". |
| BENCH-04 | Every run record carries §5 fields; missing => `run_complete=false` | Report is an untyped `json!` today; most fields are obtainable in-repo. See provenance source table. |
| BENCH-05 | Training writes `AdapterCard` with lineage; eval refuses unlabeled `mens/*` | A writer exists only in the cloud arm; local arm writes none. Card lacks 4 fields. See "BENCH-05". |
| BENCH-06 | Phase 7 run + `vox-crypto`-signed bundle verifying offline | Phase 7 not started; sign/verify primitives exist; no generic canonical-JSON exists. See "BENCH-06" and the interface boundary. |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **Crypto:** all hashing/signing via `vox-crypto` (`secure_hash` BLAKE3, `compliance_hash` SHA3, ed25519 `sign`/`verify`); direct `sha2`/`blake3`/`ed25519-dalek` imports outside it are banned (detector `vox/crypto/banned-crate-import`). `interop_sha256` exists only "for external formats that fix the algorithm".
- **LLM boundary:** all calls through `vox_actor_runtime::llm`; no vendor hostnames; model choice in registry (`vox-orchestrator::models`).
- **Crate edges:** `exceptions` ledger entries are USER-AUTHORIZED-ONLY; never regenerate `crate-edges.allow.v1.json` to admit an edge. Prefer an existing edge or a thin wrapper (defactor rule: <50 lines may be duplicated).
- **Secrets:** Clavis (`vox_secrets::resolve_secret`); a new key needs `SecretId` in `crates/vox-secrets/src/spec/ids.rs` + registry entry + `vox ci secrets-parity` + `docs/src/reference/secrets-ssot.md`.
- **Test-first:** every new `pub fn` in `crates/*/src/**` needs a `#[test]` in the same file (detector `skeleton/untested-pub-api`).
- **Glue = VoxScript**, no new `.sh/.py/.ps1`.
- **Never block on cargo** (background every build/test; read output file later). **Lean verification:** executors run `cargo check -p <touched>` plus only the plan's new tests; CI covers the rest; **mutation proofs for guards stay required** (guard the mutation with a before/after presence check).
- **Docs:** any new `docs/src/**` page needs frontmatter; research docs belong in `docs/src/architecture/`, not IDE memory. Do not read `docs/src/archive/`.
- **Command registry:** a new CLI subcommand changes `contracts/cli/command-registry.yaml` and `contracts/operations/catalog.v1.yaml` (regenerate with `cargo run -p vox-cli -- ci command-sync`; hooks auto-stage). New flags on an existing command do not add registry rows (the registry lists `eval-corpus` / `eval-local` by name only: `command-registry.yaml:9271`, `:8870`).
- **Branch state:** the working tree carries 71 uncommitted modified files (`git diff --stat`), including `LlmConfig::direct` (`types.rs`, not in HEAD: `git show HEAD:... | grep -c "pub fn direct"` = 0), `resolve_egress.rs` direct-provider routing, and `llm_config_for_spec`/`registry.rs` edits. Phase 12 plans must be executed on top of that work landing; they do not own it.

## Summary

Phase 12 is mostly **wiring that already has its parts on the shelf**. The routing resolver (`resolve_target` in `crates/vox-cli/src/commands/model/bench/mod.rs` + `llm_config_for_spec` in `crates/vox-orchestrator/src/models/registry.rs`) exists, vox-cli already holds every crate edge it needs (`vox-actor-runtime`, `vox-orchestrator`, `vox-eval`, `vox-crypto`, `vox-corpus`, `vox-populi`, `vox-grammar-export`, `vox-db`), and the stats primitives, `pass_at_k`, `AdapterCard`, ed25519 sign/verify and `interop_sha256` are all present. **No new crate edge, no new external package, and no new stats code is required** if the work is placed as recommended below.

Three findings change the plan relative to the design doc:
1. **Seed does not exist in the facade at all** (`LlmConfig`, `vox_llm_egress::ChatParams`, `OpenAiChatRequest` have temperature/top_p/max_tokens only), and **local MENS inference samples with an unseeded `rand::thread_rng()`** and `vox mens serve`'s `/v1/chat/completions` request type has no `seed`/`top_p` field. "Recorded and passed" therefore requires a change in four layers (facade, egress wire, serve, candle plugin sampler), and for MENS the seed is currently unobtainable.
2. **`eval_local` does not compute "any sample passes"; it draws exactly one sample** (comment "Greedy decoding is deterministic, so one generation suffices for pass@k", then `(v.pass, v.pass, vec![sample])`). `samples` and `seed_base` are only echoed into the report. BENCH-02 for eval-local means *adding the sample loop*, not just swapping an aggregator.
3. **`is_infra_error` does not recognise the facade's own rate-limit and context errors** (prefix `RATE_LIMITED: ` + text `rate limit exceeded` lowercases to `rate_limited: rate limit exceeded`, which contains neither `"rate limited"` nor `"429"`), so rate-limited samples are scored as model failures today. Also `vox mens serve` answers a wrong `mens/<run>` with HTTP 409, which would be scored as a model failure for every sample.

Phase 7 has **no plans yet** (`phases/` holds only phase 12). Work for D-02, D-03, D-04, D-06 and the provenance *record + completeness gate* (D-05) has no Phase 7 dependency and should land first; only identity, allowlist verification, canonical JSON and atomic persistence (BENCH-06 + the `run_complete` identity fields) block on Phase 7.

**Primary recommendation:** Put typed report/provenance/compare types in `vox-eval` (already L2, already an edge of vox-cli and vox-ml-cli), resolve models in `eval_corpus.rs` with the existing `resolve_target` + a `MensCatalog` refresh, add `seed` through `LlmConfig`→`ChatParams`→wire (+ serve + metal plugin), and isolate every Phase 7 dependency in one final plan behind three string ids, one canonicalizer, one verify-against-allowlist fn and one atomic persist fn.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Model id -> `LlmConfig` resolution | CLI/binary (L5 vox-cli) | Orchestrator registry (L3) | Registry + `llm_config_for_spec` live in vox-orchestrator; CLI composes them; `vox-actor-runtime` must not depend on the orchestrator (stated in `registry.rs` doc comment on `llm_config_for_spec`). |
| Seed/temperature/top_p on the wire | LLM facade (L3 vox-actor-runtime) | `vox-llm-egress` wire | One wire builder (`wire.rs::build_request`) serves chat and stream. |
| Local MENS sampling determinism | Plugin (L4 candle-metal / candle-cuda) | `vox mens serve` (vox-ml-cli) | Sampler is in the plugin; serve only forwards. |
| Verification of candidates | `vox-corpus` humaneval_runner (L3) | vox binary subprocess | Unchanged: `verify_program`. |
| Report stats (CI, floor, compare) | `vox-eval` (L2, pure) | vox-cli renders | Pure functions over `Vec<bool>`/`Vec<f64>`; no I/O. |
| Provenance record + completeness rule | `vox-eval` (types, `missing_required`) | vox-cli collectors (git sha, binary hash, problem-set hash) | Hashing needs `vox-crypto`; vox-eval has no such edge (it has only `xxhash-rust`), so collection stays in vox-cli. |
| Training lineage | vox-ml-cli `train_arm.rs` (host) | vox-populi `AdapterCard` (type + builder) | Both local and cloud arms converge in `train_arm.rs`; vox-populi has no edge to `vox-grammar-export`/`vox-compiler`, so the fingerprint string is passed in. |
| Signed bundle | vox-cli (assembly) | `vox-crypto` (sign/verify), Phase 7 (identity, canonicalizer, allowlist) | AGENTS.md crypto policy. |
| Durable record | Database Tier A (Phase 7 tables) | Tier C evidence (Phase 9) | D-01. |

## Standard Stack

### Core (all already in the workspace; nothing to install)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vox-eval` | workspace | `pass_at_k`, `corpus_pass_at_k`, `score_corpus`, `bootstrap_ci`, `paired_compare`, `mcnemar_exact_p`, `holm_reject`, `wilson_interval`, `min_detectable_difference` | D-04: reuse, write no new stats. [VERIFIED: crates/vox-eval/src/corpus_stats.rs, corpus_score.rs] |
| `vox-crypto` | workspace | `secure_hash` (BLAKE3), `interop_sha256`, `sign`, `verify`, `verify_signature_hex`, `hex_encode` | D-07. [VERIFIED: crates/vox-crypto/src/facades.rs] |
| `vox_actor_runtime::llm` | workspace | `llm_chat`, `LlmConfig`, `RATE_LIMITED_PREFIX`, `CONTEXT_EXCEEDED_PREFIX` | D-02 / AGENTS.md. [VERIFIED: crates/vox-actor-runtime/src/llm/mod.rs] |
| `vox_orchestrator::models` | workspace | `ModelRegistry`, `llm_config_for_spec`; `catalog::MensCatalog` | Existing resolver. [VERIFIED: registry.rs:1294; catalog.rs:1002,1218] |
| `wiremock` (vox-cli dev-dep) | workspace | loopback HTTP mock for generation tests | Already a dev-dependency of vox-cli (`Cargo.toml:340`) and used by `vox-llm-egress/tests/wire_mock.rs`. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| In-process `compute_ebnf_hash()` for syntax identity | Hash `vox grammar` stdout of the verifier binary | Describes the verifier rather than the harness, but breaks equality with the training-side algorithm D-06 mandates. Use in-process hash and separately compare the verifier's `--version` line to the harness `VOX_VERSION`. |
| New `eval-compare` subcommand | `--baseline <report.json>` flag on `eval-corpus` | Flag needs no registry row, but compares one arm only and cannot Holm-correct across arms. Recommend the subcommand (one registry row, regenerated by `ci command-sync`). |
| `StdRng::seed_from_u64` in the plugin sampler | In-house xorshift (as `bootstrap_ci` does) | Xorshift has a stable stream across dependency bumps; `StdRng` is simpler but not stream-stable across `rand` versions [ASSUMED A3]. Either is acceptable; record `rng` name in the provenance. |

**Installation:** none. **Package Legitimacy Audit:** not applicable — this phase recommends **no new external packages** (every crate above is an existing workspace member or dependency), so `gsd-tools package-legitimacy check` was not needed.

## Architecture Patterns

### System Architecture Diagram

```text
 vox model eval-corpus --model <id> --seed S -n N -k K ...
        |
        v
 [resolve_eval_model(registry, id, repo_root)]            (BENCH-01, eval_corpus.rs)
   ModelRegistry::from_cache() + MensCatalog::refresh()   <-- local fs only, registers mens/<run>
   resolve_target(): catalog hit | provider:model | (mens/* miss => HARD ERROR, never synthesize)
   llm_config_for_spec(spec, CodeGen) -> LlmConfig{provider,model,base_url}
        |                                  mens/<run> -> provider "voxlocal" -> VOX_LOCAL_ENDPOINT
        v
 [AdapterCard gate for mens/*]   read <repo>/mens/runs/<run>/adapter_card.json   (BENCH-05)
   absent/invalid lineage => row refused (run_complete=false, label withheld)
        |
        v
 for fixture in selected (sorted by id):                    NO early break (D-03)
   for i in 0..N:
      seed_i = derive(S, fixture.id, i) & 0x7fffffff
      llm_chat(LlmConfig{seed:Some(seed_i), temperature, top_p, max_tokens})   (BENCH-02)
           |-- Err(infra) -> infra counter + SampleRecord{outcome: infra}  (not in attempts)
           |-- Err(409 "Loaded model is") -> ABORT run (config error, not a model failure)
           v
      verify_program(vox_bin, candidate, tests)  -> compiled/tests_passed/cheated
      push AttemptOutcome  +  SampleRecord{fixture,i,seed_i,code,class,tokens,...}
        |
        v
 score_corpus(outcomes,k) + report_stats(): bootstrap_ci, wilson(n==1), min_detectable_difference(n)  (BENCH-03)
        |
        v
 RunProvenance::collect(...) -> missing_required() -> run_complete = infra_ok && spend_ok && prov_ok   (BENCH-04)
        |
        +--> write-back model_scoreboard ONLY IF run_complete && live        (guard; today unconditional)
        +--> EvalCorpusReport (typed, schema_version 2)  --output
        v
 [Phase 7 boundary]  BenchBundle{record, samples[], failure_classes, code}  canonical-JSON -> vox_crypto::sign
        |   identity: campaign_id/run_id/preregistration_id (Phase 7)
        v
 atomic persist (Phase 7 Tier A) ; model_benchmark_results = projection   (BENCH-06, D-01)

 vox model eval-compare A.json B.json ... --baseline A.json              (BENCH-03)
   filter run_complete, same problem_set_hash -> paired_compare(first-attempt bools) + bootstrap_ci(diffs)
   -> holm_reject over preregistered claims -> verdict {significant | not_significant | unresolved(<MDD)}
```

### Recommended Project Structure
```text
crates/vox-eval/src/
├── corpus_score.rs        # existing: pass_at_k, score_corpus
├── corpus_stats.rs        # existing: stats primitives (unchanged)
├── report.rs              # NEW: EvalCorpusReport (typed), ReportStats, ci/floor assembly
├── provenance.rs          # NEW: RunProvenance, required-field list, missing_required()
└── compare.rs             # NEW: align by fixture_id, eligibility, Holm over claims, verdict
crates/vox-cli/src/commands/model/
├── eval_corpus.rs         # resolve_eval_model, sample_seed, collectors, run() wiring
└── eval_compare.rs        # NEW thin clap wrapper over vox_eval::compare
crates/vox-populi/src/mens/tensor/adapter_card.rs   # +4 lineage fields (serde default), validate_lineage()
crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs  # card hook after train_res.is_ok()
crates/vox-actor-runtime/src/llm/types.rs + crates/vox-llm-egress/src/{lib,wire}.rs  # seed
crates/vox-plugin-mens-candle-metal/src/inference.rs      # PromptRequest.seed -> seeded rng
```

### Pattern 1: Dependency-injected resolver (hermetic routing tests, TEST-01 style)
**What:** `resolve_eval_model(registry: &ModelRegistry, id: &str, mens_specs: Vec<ModelSpec>) -> Result<ResolvedModel>`; production passes `ModelRegistry::from_cache()` plus `MensCatalog::new(repo_root).refresh().await`, tests pass `ModelRegistry::default()` and a hand-built `ModelSpec`. Existing precedent: `bench/mod.rs` tests use `ModelRegistry::default()` (`reg = ModelRegistry::default()` at lines 948 and 983).
**Example:**
```rust
// Source: bench/mod.rs:243-250 (resolve_target) + registry.rs:1294 (llm_config_for_spec)
let (spec, synthesized) = resolve_target(&registry, raw);
anyhow::ensure!(!(synthesized && raw.starts_with("mens/")),
    "mens/* id `{raw}` not found under mens/runs — refusing to route it to OpenRouter");
let mut cfg = llm_config_for_spec(&spec, TaskCategory::CodeGen);
```

### Pattern 2: Record-everything-but-score-only-attempts
Keep `AttemptOutcome`/`FixtureOutcome` serde shape unchanged (scoring depends on `attempts.len()` as n). Carry generated code, per-sample seed, failure class, and infra samples in a **parallel** `Vec<SampleRecord>`; infra errors must never enter `attempts`.

### Anti-Patterns to Avoid
- **Serializing `LlmConfig` into a report or bundle.** It carries `api_key: Option<String>`. Build an explicit `SamplingRecord{temperature, top_p, n, k, seed, max_tokens}`.
- **A second identity scheme** (e.g. keying rows on `config_digest` alone). D-01.
- **Hand-written JSON (`serde_json::json!`) for the report.** It is how provenance fields get silently dropped; use a typed struct whose required fields are non-`Option` or checked by `missing_required()`.
- **Averaging conditions or declaring significance from `intervals_overlap`.** `corpus_stats::intervals_overlap` is display-only by its own doc.

## Findings by requirement

### BENCH-01 — Routing (answers question 1)

- **Hardcode:** `let mut config = LlmConfig::openrouter(model_id);` [VERIFIED: crates/vox-cli/src/commands/model/eval_corpus.rs:253]. Signature at `:244-249` takes `model_id: &str`.
- **Existing resolver, reuse it:** `pub fn resolve_target(registry: &ModelRegistry, raw: &str) -> (ModelSpec, bool)` [VERIFIED: crates/vox-cli/src/commands/model/bench/mod.rs:243]; then `llm_config_for_spec(&spec, category)` [VERIFIED: bench/mod.rs:352 `let mut cfg = llm_config_for_spec(spec, category);`; definition crates/vox-orchestrator/src/models/registry.rs:1294]. `vox chat --model auto` uses the same pair (`crates/vox-cli/src/commands/chat.rs:100,122`). **Do not use** `resolve_chat_config` (chat.rs) — it is string-prefix matching that builds a `voxlocal` literal with `max_tokens: Some(2048)` and `telemetry_skip_interaction: true`, not registry-driven.
- **`mens/<run>` shape:** `MensCatalog::refresh` emits `id: format!("mens/{}", name)`, `provider: "voxlocal".to_string()`, `provider_type: ProviderType::VoxLocal` for each dir under `<root>/mens/runs/` that `mens_run_dir_is_listable` accepts (adapter `candle_qlora_adapter.safetensors` + `tokenizer.json` + `adapter_manifest.json`, at run root or in `final/`|`checkpoint-*`) [VERIFIED: crates/vox-orchestrator/src/catalog.rs:1218-1245; tests at :542-565]. `llm_config_for_spec` maps `ProviderType::VoxLocal` to `LlmConfig::direct("voxlocal", spec.id.clone())` [VERIFIED: registry.rs:1318 (working tree)]. The egress resolver turns `"voxlocal" | "populi_local"` into `$VOX_LOCAL_ENDPOINT` (default `http://127.0.0.1:11434`) `+ /v1/chat/completions` [VERIFIED: crates/vox-config/src/resolve_egress.rs:199-203].
- **`ModelRegistry::from_cache()` does NOT contain fresh runs.** It seeds hard-coded `mens/runs/qwen3_27b_metal_check/...` and `vox-mens-8b-v0.6` via `register_mens_local_candidates` (registry.rs:774-819) and whatever a prior refresh cached. A newly trained run is only present after `MensCatalog::new(&repo_root).refresh()` (precedent: registry.rs:485, `catalog_live.rs:112`). `ModelRegistry::get` also resolves `mens/runs/foo`, `vox-local/...`, and synthesizes `mens/foo/checkpoint-500` from parent `mens/foo` (registry.rs:1200-1255).
- **Silent mis-route risk:** `resolve_target` on an unknown id synthesizes a spec with `ProviderType::OpenRouter` (bench/mod.rs:262-275). For `mens/*` that reproduces the original bug. Guard: refuse when `synthesized && id.starts_with("mens/")`.
- **Crate edges:** vox-cli already has edges to `vox-actor-runtime`, `vox-orchestrator`, `vox-eval`, `vox-crypto`, `vox-corpus`, `vox-populi`, `vox-db`, `vox-grammar-export`, `vox-build-meta` [VERIFIED: contracts/ci/crate-edges.allow.v1.json — python membership check, all `True`]. **No new edge, no authorization needed for BENCH-01.** `vox-orchestrator` is built with the `runtime` feature in vox-cli (bench uses `llm_config_for_spec`, which is `#[cfg(feature = "runtime")]`).
- **How local MENS is served:** `vox mens serve` (vox-ml-cli `commands/ai/serve`) exposes OpenAI-shaped `POST /v1/chat/completions` over one loaded model; default port `pub const DEFAULT_INFERENCE_PORT: u16 = 11435;` [VERIFIED: crates/vox-ml-cli/src/commands/ai/inference_defaults.rs:12], whereas the `voxlocal` egress default is port **11434** (Ollama's). The operator must `export VOX_LOCAL_ENDPOINT=http://127.0.0.1:11435`. Serve checks the requested model against the loaded file stem and returns **409** `Loaded model is {loaded}, not {requested}` [VERIFIED: crates/vox-ml-cli/src/commands/ai/model_id.rs:19-25]; `mens/<run>` matches stem `<run>` (tests at model_id.rs:32-45). Ollama (`ProviderType::Ollama` → `LlmConfig::direct("ollama", ...)`) and MLX/vLLM-LoRA (`serving/vllm_lora.rs`, a client library) are alternative hosts; the design doc's recommended first path is a local host.
- **No MENS run exists on this machine:** `ls mens/runs` shows only `logs`. A true end-to-end MENS check cannot run here; all Phase 12 routing proofs must be hermetic (wiremock).

### BENCH-02 — Seeded sampling and unbiased pass@k (answers question 2)

- **Facade has no seed.** `grep -rn seed crates/vox-actor-runtime/src crates/vox-llm-egress/src` returns nothing. `LlmConfig` has `temperature`, `top_p`, `max_tokens` [VERIFIED: crates/vox-actor-runtime/src/llm/types.rs, struct LlmConfig]. `pub struct ChatParams<'a> { temperature, top_p, max_tokens, response_format, tools, tool_choice }` [VERIFIED: crates/vox-llm-egress/src/lib.rs:143-151] and `OpenAiChatRequest { model, messages, temperature, top_p, max_tokens, response_format, tools, tool_choice, stream }` [VERIFIED: crates/vox-llm-egress/src/wire.rs:255-272]. `ChatParams` is built at `chat.rs:157`, `stream.rs:53`, `stream.rs:98`; every other construction site (`vox-gamify transport.rs:534`, `vox-code-audit review/client.rs:241`, `tests/wire_mock.rs:180`) already uses `..Default::default()`, so adding `seed: Option<u64>` to `ChatParams` costs 3 edits.
- **`LlmConfig` has ~31 struct-literal construction sites in 14 files** (`rg "LlmConfig {"`: cascade.rs 1, chat.rs 2, stream.rs 2, model_resolution.rs 5, vox-cli chat.rs 2, vox-effort-audit 1, vox-effort-route 4, vox-orchestrator-mcp 6, registry.rs 1, vox-orchestrator tests 1, vox-research-shim metering 2, vox-search 4). Adding `seed: Option<u64>` is a mechanical `seed: None,` after each `telemetry_skip_interaction:` line; give it its own plan and run `cargo check -p` for each touched crate only. The new field serializes as `null`; `LlmConfig` derives `Deserialize` and `Option` defaults to `None`, so stored configs still parse.
- **Provider support (do not over-claim):** OpenRouter documents `seed` as "sample deterministically, such that repeated requests with the same seed and parameters should return the same result", **not guaranteed for some models** [CITED: openrouter.ai/docs/api-reference/parameters]. Ollama's OpenAI-compatible `/v1/chat/completions` lists `seed`, `temperature`, `top_p`, `max_tokens` as supported [CITED: docs.ollama.com/api/openai-compatibility]. Anthropic's OpenAI-compat layer is believed to ignore `seed` [ASSUMED A4]. Therefore record the field as **`seed_requested`**, and treat "identical sample sets" (success criterion 2) as testable only for deterministic backends: assert on the wire body (hermetic) and on a seeded local sampler (unit), not on a live provider.
- **Local MENS sampler is unseeded:** in the candle-metal plugin `generate` does `let mut rng = rand::thread_rng();` and `PromptRequest` has `prompt, system, max_tokens, temperature, top_k, output_mode` — no `seed`, no `top_p` [VERIFIED: crates/vox-plugin-mens-candle-metal/src/inference.rs, `generate_from_json` ~596-633 and `generate` ~635-660]. The CUDA plugin's `inference.rs` has diverged (1208 vs 1579 lines, different imports), so it is a separate edit (follow-up; metal is the Mac path). `vox mens serve`'s `ChatCompletionRequest` has `messages, model, max_tokens, temperature, tools` only [VERIFIED: crates/vox-ml-cli/src/commands/ai/serve/schema.rs:112-127], and the handler defaults `max_tokens` to 256 and `temperature` to 0.7 when omitted [VERIFIED: handlers.rs:295-298]. Required chain: serve schema (+`seed`) → `GenerateRequest` → worker → `prompt_json` → plugin `PromptRequest.seed` → `rand::rngs::StdRng::seed_from_u64`. `top_p` is not implemented by the sampler (`top_k` only): record `top_p: null` for MENS honestly; it is not a required provenance field.
- **Per-sample seed:** derive `seed_i = u64::from_le_bytes(secure_hash(format!("{base}|{fixture_id}|{i}"))[..8]) & 0x7fff_ffff` (31 bits: safe for i32/u32/i64 and JS-double providers [ASSUMED A2]). Pure fn, test-first, `vox-crypto` only.
- **eval-corpus plumbing:** `EvalCorpusArgs` (eval_corpus.rs:34-91) has `n`, `k`, `temperature`, `max_tokens`; **no `--seed`, no `--top-p`**. Add both (explicit default constants), put them in `config_digest`, and in the report. `n` is already drawn fully with no early break (loop at :391-464, comment :461-463) — keep and cover with a test that a fake generator is called exactly n times per fixture even when sample 0 passes.
- **eval_local exact lines** [VERIFIED: crates/vox-ml-cli/src/commands/mens/eval_local.rs]:
  - signature `:29-38` accepts `samples: usize, seed_base: u64` but they are only echoed (`"k": samples.max(1)` `:279`, `"seed_base": seed_base` `:415`).
  - `:188` comment `// Greedy decoding is deterministic, so one generation suffices for pass@k.`; `:189-194` `prompt_json` carries only `prompt`, `max_tokens`, `temperature`.
  - `:233` `(v.pass, v.pass, vec![sample])` — pass@1 and pass@k are the same single boolean.
  - `:264-268` `semantic_pass_at_k = samples_json.iter().any(...)` and `:297-303` `anti_stub_passed_k` use `any()`; `:290-293` and `:360-364` accumulate `passed_k` as a count and divide by `total`.
  - Replace with: loop `0..samples.max(1)` generating with `seed_base + i`, collect per-sample `pass` into `(n, c)`, compute `vox_eval::corpus_score::pass_at_k(n, c, k)` per problem, report the mean. Extract `fn per_problem_pass_at_k(samples: &[bool], k: usize) -> f64` (pure, un-gated by `cfg(feature = "gpu")`) so it is unit-testable; the inference body is `#[cfg(feature = "gpu")]` (`:185-245`), the non-gpu branch returns an error sample (`:246-261`).
  - `vox-eval` **is already a dependency of vox-ml-cli** [VERIFIED: crates/vox-ml-cli/Cargo.toml:61 `vox-eval = { workspace = true }`] — no edge change. Keep `semantic`/`anti_stub` aggregates as-is (they are not pass@k) but compute them over the same sample set, not `any()`, or label them "any-sample".
  - **Test vectors** (computed from the product form in `pass_at_k`): `pass_at_k(5,1,1)=0.2` (any-pass would say 1.0), `pass_at_k(5,2,3)=0.9`, `pass_at_k(5,1,5)=1.0`.

### BENCH-03 — Statistics in reports (answers question 3)

- **Primitives (all with zero callers outside vox-eval):** `bootstrap_ci(per_problem: &[f64], reps: usize, seed: u64) -> (f64, f64)` (xorshift, 2.5/97.5 percentiles, single arm), `paired_compare(a: &[bool], b: &[bool]) -> PairedResult{b_only,c_only,p_value,difference}` (asserts equal length), `mcnemar_exact_p`, `holm_reject(&[f64], alpha) -> Vec<bool>`, `wilson_interval(successes, trials, z) -> ConfidenceInterval` ("Valid ONLY for single-attempt pass@1"), `min_detectable_difference(n)` step table `0..=40 => 0.25, 41..=80 => 0.18, 81..=130 => 0.13, 131..=250 => 0.10, 251..=450 => 0.07, _ => 0.05`, `intervals_overlap` (display hint only) [VERIFIED: crates/vox-eval/src/corpus_stats.rs]. `docs/…` and `grep` confirm no callers: `grep -rn "bootstrap_ci\|paired_compare\|min_detectable_difference\|holm_reject" crates | grep -v corpus_stats.rs` returned only the `evaluate_gate` self-reference.
- **Paired cluster bootstrap needs no new stats code:** call `bootstrap_ci(&diffs, reps, seed)` where `diffs[i] = pass_at_k_A(problem i) - pass_at_k_B(problem i)` over the aligned problem set. Single-arm CI = `bootstrap_ci(&per_problem_estimates, ...)`, where the per-problem estimates are `pass_at_k(n,c,k)` for fixtures with `n>0` (the same exclusion `corpus_pass_at_k` applies). Record `bootstrap_reps` and the analysis seed in the report; the analysis seed is distinct from the sampling seed.
- **McNemar input is one bool per problem:** `CorpusScore.per_problem_pass_at_1` is the FIRST attempt's `tests_passed && !cheated` per fixture ("intentionally ignores samples after the first" — corpus_score.rs). Use it for `paired_compare` / Holm; use the bootstrap diff for pass@k. Both must be reported with that distinction labelled.
- **Report struct today:** an untyped `serde_json::json!({ "schema_version": 1, "model_id", "harness_id", "config_digest", "condition", "context_hash", "cutoff", "held_out_only", "run_complete", "n_infra_errors", "score", "fixtures" })` [VERIFIED: eval_corpus.rs:511-524] written only when `--output` is set. `CorpusScore` fields: `n_fixtures, k, pass_at_1, pass_at_k, compile_rate, n_cheated, n_infra_errors, total_tokens, tokens_per_pass, p50_ms, cumulative_cost_usd, cost_per_success_usd, per_problem_pass_at_1`. **`score_corpus` hard-codes `n_infra_errors: 0`**; only the top-level `n_infra_errors` is real. Fix by setting it after scoring.
- **No compare command exists** (`grep -rn "compare" crates/vox-cli/src/commands/model/*.rs` empty). Recommended `vox model eval-compare`: load reports (use `vox_bounded_fs::read_utf8_path_capped`, check `schema_version`), drop `run_complete=false`, require identical `problem_set_hash` (error otherwise), align fixtures by `fixture_id`, compute per-claim p-values for a **preregistered** list of `(arm, baseline)` pairs, `holm_reject` across the list, print `min_detectable_difference(n_aligned)` beside each comparison, and label `|difference| < MDD` as `unresolved`, never "no difference". Differing `condition` is allowed (that is estimand E2) but must be printed as a column, not merged.
- **Resolution floor facts:** 31 held-out problems => 0.25; 150 => 0.10; 164 => 0.10 (the function, not the design doc's "≈9% power" prose, is what the code prints).

### BENCH-04 — Provenance record (answers question 4)

Design doc §5 field -> obtainable-from table (all sources verified present):

| Field | Source in repo | Note |
|---|---|---|
| `syntax_fingerprint` + `syntax_fingerprint_kind` | `vox_grammar_export::versioning::compute_ebnf_hash()` (sha256 of `emit_ebnf()`); kind `"ebnf-sha256"` | vox-cli has the edge. Embeds `CARGO_PKG_VERSION` (`ebnf.rs:17`), so it flips on every version bump — acceptable only because D-06 says so and the kind field is recorded. It has no caller outside its crate except `verify_grammar_alignment`. |
| `cargo_version` | `env!("CARGO_PKG_VERSION")` / `vox_cli::VOX_VERSION` (`lib.rs:~89`: `0.x.y+build.N (githash)`) | |
| `vox_git_sha`, `vox_binary_sha256` | Run `<vox_bin> --version`, parse the `(githash)` suffix; `interop_sha256(std::fs::read(vox_bin))` (binary is 163 MB, fine) | **Describe the verifier binary** (`resolve_vox_binary`, eval_corpus.rs:200-223), not the harness. `VOX_GIT_HASH` is a short sha with no dirty marker, so on a dirty tree the binary hash is the only true identity. If the verifier's `--version` line != the harness `VOX_VERSION`, set `verifier_matches_harness=false` and force `run_complete=false`. |
| `tier`, `problem_set_hash` | New: BLAKE3 over, for each selected fixture sorted by id, `(id, spec.toml bytes, reference.vox bytes, tests.vox bytes)` | `Fixture` today carries only `tests_path` (manifest.rs:11-27); the manifest `files:` block has `spec`, `reference`, `tests` per fixture. Add `spec_path`, `reference_path` to `Fixture`. vox-corpus has **no** edge to `vox-crypto` (checked) — hash in vox-cli. The manifest's own `corpus_hash: "sha256:8a2f…"` is a hand-maintained string with no recomputing code found; do not rely on it. Record both selected-set hash and the manifest's `corpus_hash`. |
| `prompt_template_hash` | BLAKE3 of the template text in `build_prompt` (conditions.rs:107-125) | Lift the template to a `const` so it can be hashed. |
| `condition`, `context_hash` | `Condition::id()`, `PromptContext.context_hash` | **`hash_context` is FNV-1a 64-bit** ("FNV-1a: adequate for a row-identity fingerprint… avoids adding a new crate dependency", conditions.rs:62-71). Replace with BLAKE3 of the text via vox-cli, or keep and add `context_text_hash`; `vox-corpus` lacks a `vox-crypto` edge so compute it in vox-cli from `ctx.context_text`. |
| `harness_version` | `VOX_VERSION` | |
| Model (external) | `ModelSpec.id`, `LlmConfig.provider`, `LlmResponse.model` (provider-reported string), request date | The reported model string is per response; record the set of distinct values seen. |
| Model (MENS) | `AdapterCard` (see BENCH-05) | |
| Sampling | `temperature, top_p, n, k, seed, max_tokens` | `top_p` is the one field that may be `null` (explicit "not applicable"); the rest are required. |
| Runner | `std::env::consts::{OS,ARCH}`, runner label (`--runner`/`GITHUB_ACTIONS`), `--timeout-secs` | |
| Integrity | `preregistration_id`, signature, `run_complete`, infra count | Phase 7 for the first two. |

- **Completeness rule:** `RunProvenance::missing_required(&self) -> Vec<&'static str>` is the single gate; `run_complete = no_spend_cap && infra_rate <= max_infra_rate && missing_required().is_empty() && verifier_matches_harness`. Today `run_complete` is false only on the spend cap (`:380-395`). **Required keys must be present-but-null-distinguishable**: use `Option<T>` with no `#[serde(default)]` on required fields so a record with an absent key fails to deserialize rather than silently defaulting.
- **Uncommitted `model_benchmark_results`:** the diff adds a per-case table `(run_id, model_id, resolved_model_id, provider, suite, task_category, case_id, passed, score, ttft_ms, latency_ms, input_tokens, output_tokens, tokens_per_sec, cost_usd, error_class, recorded_at_ms)` with `suite` commented `codegen | reasoning | json | tool_call | needle | vox_gen` and bumps `pub const BASELINE_VERSION: i64 = 95;`, with store ops in the untracked `ops_model_bench.rs` ("This table is the only writer of *measured* model quality") [VERIFIED: git diff of scientia.rs, manifest.rs; ops_model_bench.rs:1-4]. It is a **`vox models bench` per-case table, not a campaign/run table**; eval-corpus does not write it and Phase 12 must not either (D-01). **Coordination hazard:** STORE-01 plans the Phase 7 migration at "the next free DB baseline (95 as of 2026-10-01)" with a guard that fails if a shared DB already carries 95 without its tables (REQUIREMENTS.md:21). This branch already claims 95. Phase 7 must take 96 (or sequence after this lands). Surface to the Phase 7 planner and to the STORE-01 human schema gate now.
- **Existing defects to fix in the same plan** (they corrupt provenance/identity): (a) `config_digest` uses `sha2::Sha256` directly (eval_corpus.rs:124-125, a policy violation) and omits model id, seed, top_p and problem-set hash despite its doc "over every knob that can change a score"; move to `vox_crypto::facades::secure_hash` + `hex_encode` (the digest is only written to the report JSON — `rg config_digest` finds no other consumer in vox-cli). (b) The `model_scoreboard` write-back (`:493-508`) runs for incomplete runs and for `--from-dir` rows; guard it with `run_complete && measured`. (c) `is_infra_error` (see Pitfall 1).

### BENCH-05 — Training lineage (answers question 5)

- **`AdapterCard` today** [VERIFIED: adapter_card.rs:6-25]: `base_hf_id, base_revision, base_rung, quantization, lora_rank: u32, lora_alpha: f32, seed: u64, corpus_hash: String, preset_version, metrics: serde_json::Value, cost_usd: f64, provider, git_sha, created`. **Missing:** `adapter_sha256`, `train_data_sha256`, `training_syntax_fingerprint`, `syntax_fingerprint_kind`. No `#[serde(default)]` anywhere, so add the four as `#[serde(default)] String` for forward-compat reads of existing cards. Keep `validate()` as is (serve depends on it; `vllm_lora.rs` calls `is_compatible_with`) and add `validate_lineage()` which fails on empty lineage; the eval gate calls that.
- **Writers today:** exactly one — `card_from_manifest` in `crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs:518-553`, called only from `post_training_flow` on `EvalGateOutcome::PassedBase` (`:566-570`), fed by the **cloud** `TrainingManifest{base_hf_id, base_revision, rung, quantization, preset, rank, alpha, seed, corpus_hash, metrics, cost_usd, provider, git_sha, created_at}` (`:422-443`). The design doc's G10 "AdapterCard has no writer" is therefore only half true: the cloud arm writes one when the eval gate passes; the **local arm (`--cloud local`) writes none**.
- **`corpus_hash` is weak:** `compute_corpus_fingerprint` is an xxh3 XOR-fold over `WATCHED_FILES` paths and contents, 16 hex chars, used for idempotency (`preflight_part1.rs:3-14`). It is not `train.jsonl`'s hash. The card's new `train_data_sha256` must hash the file the trainer actually consumed.
- **Where training finishes (local):** `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs:532-586` `train::run_train(...).await`, then `:588-612` a post-success block (`if train_res.is_ok() && let Some(ref r) = workspace_root`) that already runs host-side logic with `data_dir`, `output_dir`, `effective_model`, `seed`, `rank`, `alpha`, `effective_preset` in scope. **Put the local-arm card write here**, and add the same fields to the cloud `TrainingManifest` (`:240-255`) so both arms share one builder in vox-populi. Background mode returns early (`:524-526` `spawn_train_with_log`); the child process re-enters without `--background`, so only the child writes the card. The plugin-local `TrainingManifest` (`vox-plugin-mens-candle-core/src/manifest.rs:46`, `train_file: String` is a **path**, `seed`, `git_sha`, `base_model`) is not the right home: two vendor plugin copies and an L4 crate that must not depend on vox-populi.
- **Syntax fingerprint edge problem:** `vox-ml-cli -> vox-grammar-export` and `vox-populi -> vox-grammar-export` are **absent** from the allowed edge set; `vox-ml-cli -> vox-compiler` and `vox-compiler -> vox-grammar-export` are present. No-authorization path: add a thin wrapper next to the existing `vox_compiler::llm_prompt::vox_grammar_prompt()` (which already wraps `vox_grammar_export::compact_prompt::emit_compact_llm_prompt()`, llm_prompt.rs:9), e.g. `pub fn grammar_fingerprint() -> (String /*hash*/, &'static str /*kind*/)` delegating to `compute_ebnf_hash()`. `train_arm.rs` calls it and passes the `String` into the vox-populi builder. conditions.rs:12-15 documents this exact "zero new workspace crate edges" pattern.
- **Hashing:** `adapter_sha256` and `train_data_sha256` are sha256 by name in D-06/BENCH-05. The only SHA-256 in `vox-crypto` is `interop_sha256` (staged diff, "only for external formats that fix the algorithm (e.g. Trusty URIs)"; `secure_hash` is BLAKE3). These two hashes plausibly meet that bar (HF LFS oids are sha256; the trainer's train set is uploaded to HF as `train_data_hf`), but this is a policy judgment: see A1/Open Question 1. `vox-crypto` has no streaming API, so read the file (`std::fs::read`); adapters are tens-hundreds of MB, `train.jsonl` a few MB [ASSUMED A9].
- **What reads cards:** `vllm_lora.rs` `ensure_adapter_loaded` -> `card.is_compatible_with(serve_rung, serve_quant)` (provenance check only on rung/quant), `DomainRouter::register` (skips adapters with no card), tests. Nothing reads lineage yet, so adding fields is non-breaking.
- **Eval-side gate:** for `mens/<run>`: locate `<repo>/mens/runs/<run>/` (the same dir `MensCatalog` listed), `AdapterCard::read_sidecar(<run>/candle_qlora_adapter.safetensors)` (reads `adapter_card.json` next to the adapter, adapter_card.rs:58-69), run `validate()` and `validate_lineage()`, **re-hash the adapter file and compare to `adapter_sha256`** (detects a swapped adapter), and compare `training_syntax_fingerprint` to the current in-process fingerprint to set `syntax_drift: bool`. No card or failed check => refuse to label the row, set `run_complete=false` (per D-05, never backfill). Sub-checkpoints (`mens/foo/checkpoint-500`) resolve to the parent spec; the card lives with the parent run's adapter — use the resolved adapter file's directory.

### BENCH-06 — Signing, bundle, Phase 7 boundary (answers question 6)

- **Existing signing:** `sign_prereg`/`verify_prereg` are typed to `PreregistrationV1` and sign `canonical_json(prereg)` bytes with `vox_crypto::facades::{sign, verify, verifying_key_from_bytes, to_verifying_key}` [VERIFIED: crates/vox-orchestrator/src/preregistration/signing.rs:52-106]. They embed the verifying key in the signed object (so any self-generated key verifies; TRIAL-DESIGN §1 already lists the allowlist as a Phase 7 fix). `compute_trusty_uri` now uses `vox_crypto::facades::interop_sha256` (staged change; previously direct `sha2`).
- **No generic canonicalizer exists:** `canonical_json` is `pub(crate)` and takes `&PreregistrationV1` (trusty_uri.rs:46); `sort_keys` is private; `rg "fn canonical_json|fn sort_keys|fn canonical_bytes"` finds only these. TRIAL-05 (Phase 7) requires: "canonicalization sorts keys at every depth and rejects NaN/non-finite numbers; hashing and signing go through `vox-crypto`". Bundle contents (pass rates) can legitimately be NaN (0/0), so the non-finite rejection is load-bearing.
- **Offline verification primitive:** `vox_crypto::facades::verify_signature_hex(pubkey_hex, message, signature_hex) -> Result<bool, String>` (facades.rs:259) is exactly an offline verify.
- **Signing key source:** `SecretId::VoxNanopubSigningKeyHex` exists (hex Ed25519 private key "for nanopublication signing via vox-crypto", registry/scholarly.rs:337-346). Reusing a nanopublication key for benchmark bundles conflates purposes; Phase 7 will define the signer allowlist (`signer_key_id`). Either Phase 7 supplies the key id, or Phase 12 adds a dedicated `SecretId` (ids.rs + registry + parity + secrets-ssot.md). See Open Question 3.
- **What Phase 7 will provide (inferred from REQUIREMENTS.md and `.planning/research/TRIAL-DESIGN.md`; Phase 7 has no plans, so every item is `[ASSUMED A7]` as to location/names):** `campaign_id` (trusty id of the signed `CampaignPreregV1`), `run_id`, `replicate_id`, `attempt_index`, `retry_of_run_id`; signer-allowlist gate; recursive canonicalizer; atomic campaign/run/sample-batch persistence; a new Tier A fragment.
- **Minimal interface boundary (the only things Phase 12 consumes from Phase 7):**
  1. `RunIdentity { campaign_id: String, run_id: String, replicate_id: u32, attempt_index: u32, retry_of_run_id: Option<String> }` (strings/ints only; no Phase 7 types leak into vox-eval).
  2. `preregistration_id: String` and `signer_key_id: String`.
  3. `fn canonical_json_bytes<T: Serialize>(&T) -> Result<Vec<u8>, CanonicalError>` (recursive sort, rejects non-finite). Fallback if Phase 7 slips: promote `sort_keys` + the lossy round-trip check from trusty_uri.rs into a generic pub fn (<30 lines) in the same module — D-07 explicitly allows "or the Phase 7 equivalent".
  4. `fn verify_signer_allowlisted(pubkey_hex: &str) -> Result<(), _>`.
  5. `async fn persist_run_atomic(db, identity, record_json, samples) -> Result<()>` — one transaction.
  Everything else (record, bundle, sign, offline verify) is Phase 12-owned and can use `Option<String>`/`None` for the three identity fields until Phase 7 lands; those `None`s are *required fields*, so such runs are honestly `run_complete=false`. This preserves D-01 (no second identity scheme is invented) while letting Waves 1-2 ship.
- **Schema ask for the STORE-01 gate (give it to the Phase 7 planner now):** add to the Tier A run row a single `provenance_json TEXT NOT NULL` (the canonical §5 record) plus indexed scalar columns `syntax_fingerprint`, `model_id`, `condition`, `problem_set_hash`, `run_complete`, rather than ~30 columns; this keeps the human-approved schema small and lets Phase 13 add fields without a migration.
- **Bundle content:** `{ record, samples: [{fixture_id, sample_index, seed, code, compiled, tests_passed, cheated, failure_class, tokens, latency_ms, cost_usd}], infra_samples: [...] }`. Failure class from existing facts only: `cheated` / `infra` / `timeout` (`detail` contains `"timed out"` — verify.rs, `format!("timed out after {}s", ...)`) / `check_error` (`!compiled`) / `runtime_or_wrong_answer` (`compiled && !tests_passed`). Parse-vs-type is not separable from the current `VerifyOutcome` (design §8 asks for that split; defer, state it). Never serialize `LlmConfig`.

### Test infrastructure (answers question 7)

- eval_corpus.rs tests today are 8 pure unit tests (`mens_row_identity`, `config_digest`, `is_infra_error`, `extract_vox_code`, `load_solution_dir`, `resolve_vox_binary`) — no generation, no network, no verifier [VERIFIED: eval_corpus.rs:536-657]. `run()` is untested (needs a vox binary, DB, network).
- **Hermetic generation test:** refactor `generate_candidate(model_id: &str, ...)` to `generate_candidate(config: LlmConfig, prompt, ...)`; test with `wiremock::MockServer` (already a vox-cli dev-dep) by setting `config.base_url = Some(format!("{}/v1/chat/completions", server.uri()))`, `provider = "voxlocal"`, asserting via `body_partial_json` that the request carries `seed`, `temperature`, `model`. Pattern: `crates/vox-llm-egress/tests/wire_mock.rs`. This satisfies TEST-01's spirit (loopback only) without waiting for Phase 7's fixtures.
- **Routing test:** `ModelRegistry::default()` (hermetic under `cfg(test)`) + a `ModelSpec` for `mens/foo` with `ProviderType::VoxLocal`; assert `cfg.provider == "voxlocal"`, `cfg.model == "mens/foo"`; and an unknown `mens/zzz` errors rather than returning `openrouter`.
- **Serve 409 test:** feed a mock returning `409 {"text":"Loaded model is x, not y"}` and assert the run aborts rather than recording failed samples.
- **Gated code:** anything under `#[cfg(feature = "gpu")]` (eval_local inference) cannot run in default CI; extract pure fns.
- **Never-block-on-cargo / lean verification:** each plan's verify step is `cargo check -p <crate>` + the plan's own tests (`cargo nextest run -p <crate> -E 'test(<name>)'`), launched in the background; broader suites go to hosted CI. Mutation proofs: before/after presence-check on the mutated line.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Unbiased pass@k | A new estimator or an `any()` | `vox_eval::corpus_score::pass_at_k` / `corpus_pass_at_k` | Numerically-stable product form, tested; D-03. |
| Paired CI | A paired-bootstrap routine | `bootstrap_ci(&per_problem_differences, reps, seed)` | Resampling per-problem differences *is* the paired cluster bootstrap. |
| Pairwise significance | CI-overlap test | `paired_compare` -> `mcnemar_exact_p`, `holm_reject` | Audit C4; overlap fires at alpha ≈ 0.005. |
| Resolution floor | Power calc | `min_detectable_difference(n)` | Single table, already documented. |
| Model-id routing | `if id.starts_with("mens/") { voxlocal }` | `resolve_target` + `llm_config_for_spec` + `MensCatalog::refresh` | The string-prefix approach is `resolve_chat_config`; it ignores the registry and D-02 forbids a MENS special case. |
| Hashing / signing | `sha2`, `blake3`, `ed25519-dalek` imports | `vox_crypto::facades::{secure_hash, interop_sha256, sign, verify, verify_signature_hex, hex_encode}` | AGENTS.md; detector `vox/crypto/banned-crate-import`. |
| Canonical JSON | A third copy of `sort_keys` | Phase 7's canonicalizer (TRIAL-05); fallback = promote the existing one | One definition across prereg, campaign and bundle. |
| LLM transport in tests | A mock `reqwest` layer | `wiremock` loopback | Already used by `vox-llm-egress` tests. |
| Card read/write | New JSON helpers | `AdapterCard::write_sidecar` / `read_sidecar` | Already round-trip tested. |

**Key insight:** every primitive exists; the work is *removing the places where a result can be produced without the primitive* (unconditional scoreboard write, untyped `json!` report, string-prefix routing, single-sample "pass@k").

## Common Pitfalls

### Pitfall 1: Rate-limit and context errors scored as model failures
**What goes wrong:** `is_infra_error` lowercases the message and checks `"rate limited"`, `"429"`, `"context length"`, `"502"`, `"503"`, `"timeout"` (eval_corpus.rs:140-148). The facade prefixes errors with `RATE_LIMITED: ` (`chat.rs:55`) around `EgressError::RateLimited`'s Display `"rate limit exceeded"` (lib.rs:197-199), giving `rate_limited: rate limit exceeded` which matches none of them; `CONTEXT_LENGTH_EXCEEDED: ` likewise. Infra failures become zero-score samples.
**How to avoid:** match on `vox_actor_runtime::llm::{RATE_LIMITED_PREFIX, CONTEXT_EXCEEDED_PREFIX}` (both are re-exported from `llm/mod.rs`) plus the existing substrings; add a test using the real prefixes (the current test uses a hand-written `"rate limited: 429 too many requests"`). Add `"transport-error"`-class messages (`HTTP request failed:`) deliberately: decide whether connection refused is infra (yes for a down MENS server) — and treat a **409 `Loaded model is`** as fatal config error, not infra and not a model failure.
**Warning signs:** a live run with high `compile_rate` of 0 and no `infra_errors`.

### Pitfall 2: `mens/*` silently routed to OpenRouter
**What goes wrong:** `resolve_target` synthesizes an OpenRouter spec for unknown ids; `from_cache()` lacks new runs.
**How to avoid:** `MensCatalog::refresh` first; hard error on synthesized `mens/*`; test both.

### Pitfall 3: Recorded sampling != effective sampling
**What goes wrong:** `vox mens serve` ignores `seed`/`top_p` and defaults `max_tokens=256`, `temperature=0.7` when absent; the candle sampler draws from `thread_rng`. A report claiming `seed=S` for MENS would be false.
**How to avoid:** plumb seed end-to-end (serve + plugin) *before* claiming it; until then record `seed_requested` and `seed_effective: false` for `voxlocal`; always send `max_tokens` and `temperature` explicitly (the facade does).

### Pitfall 4: Port mismatch for local serving
`vox mens serve` binds 11435; the `voxlocal` egress default is 11434. Document and preflight: if `VOX_LOCAL_ENDPOINT` is unset and provider is `voxlocal`, warn (or probe both) rather than hitting Ollama and getting a model-not-found.

### Pitfall 5: Incomplete runs poisoning routing
Unconditional `upsert_model_scoreboard` writes `quality_score = pass_at_1` under `vox-codegen` that routing reads. Guard on `run_complete && measured && !no_write_back`.

### Pitfall 6: Concurrency of seeds with retries
`execute_activity` retries by `ActivityOptions.retries`; a retried call with the same seed is fine, but a new `seed_i` per retry would break reproducibility. Derive from `(base, fixture, sample_index)` only, never from a global counter or attempt number.

### Pitfall 7: Hashing the wrong training file / hashing too early
`data_dir/train.jsonl` may differ from what the trainer reads (mix config, `target/dogfood/train_mixed.jsonl`, refreshed corpus: `refresh_stale_training_corpus` at train_arm.rs:788). Hash the path recorded as consumed (plugin manifest `train_file`) immediately after a successful run in the same process; record the path only as a non-identity convenience field.

### Pitfall 8: Floats in signed JSON
NaN serializes to `null`; a signed bundle would verify but lose information. Reject non-finite values in the canonicalizer (TRIAL-05) and represent undefined rates as `null` with an explicit `n=0` field.

### Pitfall 9: `Fixture` change fan-out
Adding `spec_path`/`reference_path` to `vox_corpus::humaneval_runner::Fixture` touches every construction site; `rg "Fixture {"` shows only the one in `manifest.rs:91` for this type, but check `vox-ml-cli`/`vox-audit` before assuming.

### Pitfall 10: Branch drift
71 uncommitted files, including `LlmConfig::direct` and `resolve_egress`. Plans touching `types.rs`, `chat.rs`, `stream.rs`, `registry.rs`, `scientia.rs` will conflict textually with the in-flight work. Sequence after that work is committed; prefer small hunks.

## Code Examples

### Unbiased vs any-sample (test vector for BENCH-02)
```rust
// Source: crates/vox-eval/src/corpus_score.rs pass_at_k (product form)
use vox_eval::corpus_score::pass_at_k;
assert!((pass_at_k(5, 1, 1) - 0.2).abs() < 1e-12);   // any-sample would report 1.0
assert!((pass_at_k(5, 2, 3) - 0.9).abs() < 1e-12);
assert_eq!(pass_at_k(5, 1, 5), 1.0);
```

### Paired difference CI with existing primitives
```rust
// Source: crates/vox-eval/src/corpus_stats.rs (bootstrap_ci, paired_compare, holm_reject)
let diffs: Vec<f64> = a_est.iter().zip(&b_est).map(|(x, y)| x - y).collect();
let (lo, hi) = bootstrap_ci(&diffs, reps, analysis_seed);
let p = paired_compare(&a_first_bool, &b_first_bool).p_value;
let rejected = holm_reject(&p_values_of_preregistered_claims, 0.05);
let mdd = min_detectable_difference(n_aligned);
```

### Per-sample seed (pure, vox-crypto only)
```rust
// proposed; tests: deterministic, input-sensitive, < 2^31
pub fn sample_seed(base: u64, fixture_id: &str, i: usize) -> u64 {
    let h = vox_crypto::facades::secure_hash(format!("{base}|{fixture_id}|{i}").as_bytes());
    u64::from_le_bytes(h[..8].try_into().unwrap()) & 0x7fff_ffff
}
```

### Wire body carries the seed (hermetic proof)
```rust
// Source pattern: crates/vox-llm-egress/tests/wire_mock.rs (body_partial_json matcher)
Mock::given(method("POST")).and(path("/v1/chat/completions"))
    .and(body_partial_json(serde_json::json!({ "seed": 1234, "temperature": 0.8 })))
    .respond_with(ResponseTemplate::new(200).set_body_json(/* choices/usage */))
    .expect(1).mount(&server).await;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| "any attempt passed" / k derived from data | Unbiased Chen et al. estimator, k a config input | audit 2026-09-01 (C2/C3) | `eval_local` still on the old path (single sample). |
| CI-overlap significance | Exact McNemar + Holm, MDD | audit C4/C5 | Implemented in `vox-eval`, unwired. |
| EBNF-text hash as language identity | Behavioral syntax fingerprint | Phase 13 (BENCH-07) | Phase 12 uses `compute_ebnf_hash` + `syntax_fingerprint_kind`. |

**Deprecated/outdated within this repo:** `hash_context` FNV for provenance identity; direct `sha2` in `config_digest`; `resolve_chat_config` string routing as a model for new code.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `interop_sha256` is an acceptable facade for `adapter_sha256`/`train_data_sha256`/`vox_binary_sha256` because those names fix SHA-256 for HF/GitHub interop; AGENTS.md says Vox-owned hashing uses BLAKE3/SHA3 | BENCH-04/05 | A reviewer may call it a crypto-policy violation; fallback is `*_blake3` field names (changes locked D-06 wording) |
| A2 | Masking seeds to 31 bits is safe across providers (i32/u32/JS-double) | BENCH-02 | Narrower seed space; harmless for reproducibility |
| A3 | `rand::rngs::StdRng` streams are not stable across `rand` versions | BENCH-02 | If stable, the "record rng name" field is unnecessary overhead |
| A4 | Anthropic's OpenAI-compat endpoint ignores `seed`; other direct providers vary | BENCH-02 | Reports label `seed_requested` only; if honored, nothing breaks |
| A5 | A default max infra-error rate (suggest 5%) for `run_complete` | BENCH-04 | Needs a preregistered value from the user; design says "preregistered rate" without a number |
| A6 | `vox run --mode interp` executes model-generated code without ambient file/network capabilities by default (ADR-048 caps) | Security | If caps default permissive, model code could touch the host; check before widening the panel |
| A7 | Phase 7 will expose the canonicalizer / allowlist verify / atomic persist as TRIAL-05 / STORE-02 describe | BENCH-06 | Interface boundary names must be re-aligned at Phase 7 planning |
| A8 | Phase 7's Tier A migration will not use baseline 95 | BENCH-04 | Migration collision on shared DBs (guard in STORE-01 would fail loudly) |
| A9 | Whole-file in-memory hashing is fine (adapter ≤ few hundred MB; vox binary 163 MB measured) | BENCH-05 | A multi-GB merged model would need a streaming hash in `vox-crypto` (new pub fn, test-first) |

## Open Questions

1. **Hash algorithm for the `*_sha256` fields.**
   - Known: D-06/BENCH-05 name sha256; `vox-crypto` offers `interop_sha256` only for "external formats that fix the algorithm".
   - Unclear: whether the user accepts the HF/GitHub-interop justification.
   - Recommendation: use `interop_sha256`, name the fields as specified, and ask the user to confirm at plan review (cheap checkpoint). Vox-owned `problem_set_hash`, `context_hash`, `prompt_template_hash` use `secure_hash` (BLAKE3).
2. **Baseline 95 collision** between this branch's `model_benchmark_results` and Phase 7's STORE-01. Needs a decision from the Phase 7 planner/user (take 96, or fold the table into the Phase 7 fragment as the projection D-01 describes).
3. **Bundle signing key.** Dedicated `SecretId` (Clavis checklist) vs Phase 7's signer key vs reusing the nanopublication key. Recommend: Phase 7's allowlisted signer; add a dedicated id only if Phase 7 lacks one.
4. **Infra-error `run_complete` threshold** (A5): choose a number and put it in the signed preregistration.
5. **MENS seed in `vox mens serve`:** confirm scope includes adding `seed` to serve + metal plugin now (recommended; without it MENS rows are non-reproducible) and CUDA as follow-up.
6. **Compare surface:** subcommand (recommended) vs flag; subcommand costs one registry regeneration.
7. **Which `train.jsonl`** the local trainer consumes in each mode (mix/curriculum paths) — confirm at plan time by reading `train::run_train` -> plugin `train_file`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo (broker shim) | all builds/tests | ✓ | `/Users/brbrainerd/.vox/build-broker/bin/cargo` | — |
| `vox` binary (verifier) | eval-corpus E2E, binary hash | ✓ | `target/debug/vox`, `target/release/vox` (163,522,240 bytes, Oct 10 02:56) | build via broker |
| Ollama | optional MENS/GGUF host | ✓ | `/opt/homebrew/bin/ollama` | — |
| A trained MENS run | `mens/*` E2E | ✗ | `mens/runs/` contains only `logs` | hermetic wiremock + synthetic run dir fixtures |
| Network / OpenRouter key | external-model live sweep | not probed | — | wiremock; live runs are manual/ignored lane |
| GPU/Metal inference in CI | `eval_local` gpu path | ✗ (hosted CI) | — | extract pure aggregation; gpu path verified on Mac host manually |

**Missing with no fallback:** none for planning. **Missing with fallback:** MENS run and CI GPU (hermetic tests + a documented manual Mac smoke).

## Validation Architecture

(`workflow.nyquist_validation` is absent from `.planning/config.json` => enabled.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `#[test]`/`#[tokio::test]` under cargo-nextest; `wiremock` for loopback HTTP; `serial_test` for env mutation |
| Config file | workspace `.config/nextest.toml` (profiles `ci`, `ci-gate`) — verify name at plan time |
| Quick run command | `cargo nextest run -p <crate> -E 'test(<name>)'` (background; redirect to scratchpad) |
| Full suite command | hosted CI (`vox ci pre-push --complete` is the local equivalent; not for executors) |

### Phase Requirements -> Test Map (names are proposals)
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BENCH-01 | `mens/<run>` resolves to `voxlocal` config via registry; unknown `mens/*` errors, never `openrouter` | unit | `cargo nextest run -p vox-cli -E 'test(eval_corpus::tests::resolve_)'` | ❌ Wave 0 |
| BENCH-01 | generation hits the configured base URL with model + sampling | integration (wiremock) | `cargo nextest run -p vox-cli -E 'test(eval_corpus::tests::generate_candidate_)'` | ❌ Wave 0 |
| BENCH-01 | 409 "Loaded model is" aborts the run; `RATE_LIMITED:`/`CONTEXT_LENGTH_EXCEEDED:` classed infra | unit | `cargo nextest run -p vox-cli -E 'test(eval_corpus::tests::infra_)'` | ❌ Wave 0 |
| BENCH-02 | wire body carries `seed` (and omits it when `None`) | unit + wiremock | `cargo nextest run -p vox-llm-egress -E 'test(seed)'` | ❌ Wave 0 |
| BENCH-02 | `sample_seed` deterministic, input-sensitive, `< 2^31`; fake generator called exactly n times per fixture even on early pass | unit | `cargo nextest run -p vox-cli -E 'test(eval_corpus::tests::sample_)'` | ❌ Wave 0 |
| BENCH-02 | `per_problem_pass_at_k` equals `pass_at_k` on (5,1,1)=0.2, (5,2,3)=0.9; not `any()` | unit | `cargo nextest run -p vox-ml-cli -E 'test(eval_local::tests::unbiased_)'` | ❌ Wave 0 |
| BENCH-02 | seeded sampler reproducible for equal seed, differs for different | unit | `cargo nextest run -p vox-plugin-mens-candle-metal -E 'test(seeded_)'` | ❌ Wave 0 |
| BENCH-03 | report carries CI (`lo<=point<=hi`), reps, analysis seed, MDD(n) | unit | `cargo nextest run -p vox-eval -E 'test(report::)'` | ❌ Wave 0 |
| BENCH-03 | compare: Holm applied, `<MDD` labelled unresolved, mismatched `problem_set_hash` errors, `run_complete=false` excluded, no overlap-based verdict anywhere | unit | `cargo nextest run -p vox-eval -E 'test(compare::)'` | ❌ Wave 0 |
| BENCH-04 | each required field removed in turn => listed by `missing_required()` and `run_complete=false` (mutation: delete the gate call => test fails) | unit | `cargo nextest run -p vox-eval -E 'test(provenance::)'` | ❌ Wave 0 |
| BENCH-04 | scoreboard write-back skipped when incomplete/`--from-dir` | unit (pure predicate) | `cargo nextest run -p vox-cli -E 'test(eval_corpus::tests::write_back_)'` | ❌ Wave 0 |
| BENCH-04 | `problem_set_hash` changes when one test/reference/spec byte changes; stable across runs | unit | `cargo nextest run -p vox-cli -E 'test(eval_corpus::tests::problem_set_hash_)'` | ❌ Wave 0 |
| BENCH-05 | `AdapterCard` round-trips with lineage; old JSON (no lineage) still parses; `validate_lineage` rejects empty | unit | `cargo nextest run -p vox-populi -E 'test(adapter_card::)'` | ✅ file, ❌ new tests |
| BENCH-05 | eval refuses `mens/*` with no card / swapped adapter hash; flags syntax drift | unit (tempdir) | `cargo nextest run -p vox-cli -E 'test(eval_corpus::tests::card_)'` | ❌ Wave 0 |
| BENCH-05 | `train_arm` local success writes `adapter_card.json` with sha of the consumed file | unit/integration (tempdir, fake trainer result) | `cargo nextest run -p vox-ml-cli -E 'test(train_arm::)'` | ✅ file, ❌ new tests |
| BENCH-06 | bundle signs and `verify_signature_hex` passes offline; any tampered byte (record, sample code, seed) fails; NaN rejected | unit | `cargo nextest run -p vox-cli -E 'test(eval_bundle::)'` | ❌ Wave 0 (Phase 7 canonicalizer) |
| BENCH-06 | atomic persist: forced mid-write failure leaves no partial run | integration (vox-db test-support) | `cargo nextest run -p vox-db -E 'test(bench_run_atomic)'` | ❌ blocked on Phase 7 |

### Sampling Rate
- **Per task commit:** `cargo check -p <touched crates>` + that task's new tests only (background).
- **Per wave merge:** all new tests of the wave in one nextest filter per crate.
- **Phase gate:** hosted CI green (`ci-gate` profile) before `/gsd-verify-work`; docs lint if any doc page is added.

### Wave 0 Gaps
- [ ] `vox-eval/src/{report,provenance,compare}.rs` with their `#[cfg(test)]` modules (test-first, per the Test-First Policy)
- [ ] eval_corpus.rs test additions (resolver, generate_candidate via wiremock, infra classification, sample_seed, problem_set_hash, write-back predicate, card gate)
- [ ] vox-llm-egress wire seed test (extend `tests/wire_mock.rs`)
- [ ] eval_local pure aggregator tests; metal plugin seeded-sampler test
- [ ] mutation proofs (presence-checked): (1) remove the provenance gate call, (2) route `mens/*` back to `openrouter`, (3) drop `seed` from `build_request`, (4) replace `pass_at_k` with any-sample, (5) skip signature verification in bundle verify, (6) remove `run_complete` filter in compare
- [ ] Framework install: none

## Security Domain

(`security_enforcement` is absent in config => enabled.)

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (signer allowlist, Phase 7) | allowlisted verifying keys; never trust the key embedded in the signed object |
| V5 Input Validation | yes | `serde` typed reports with `schema_version` check; `vox_bounded_fs::read_utf8_path_capped` for report/bundle reads; `load_solution_dir` already errors on ambiguous ids |
| V6 Cryptography | yes | `vox-crypto` only; ed25519 sign/verify; no hand-rolled nonce/KDF/envelope |
| V8 Data Protection | yes | never serialize `LlmConfig` (carries `api_key`); Clavis for the signing key; bundles contain model output only |
| V12 Files/Resources | yes | model-generated code is executed by `vox run --mode interp` (A6); workdir under temp, removed after run |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Tampered report/bundle to inflate a result | Tampering | canonical-JSON ed25519 signature over record + samples; offline verify; allowlisted signer |
| Self-signed bundle by an arbitrary key | Spoofing | allowlist check (Phase 7 TRIAL-05), not embedded-key trust |
| Swapped adapter under a `mens/<run>` label | Spoofing/Tampering | `adapter_sha256` re-hash vs card at eval time; serve's 409 model-stem check |
| Candidate disables its own tests | Tampering | existing `canary` + `rejects_at_ingest`; `cheated` counts as failure |
| Incomplete/forged-complete run steering routing | Tampering | `run_complete` gate before `model_scoreboard` write-back; comparison excludes incomplete |
| API key leakage into artifacts | Information disclosure | explicit `SamplingRecord`, never `LlmConfig` |
| Model-generated code executing on host | Elevation of privilege | interpreter isolation, no `--caps` granted (verify, A6) |

## Recommended plan decomposition and sequencing

**Wave 1 (no Phase 7 dependency; plans 12-01..12-03 can run in parallel — they touch different crates):**
1. **12-01 Seed through the facade** (`LlmConfig.seed`, `ChatParams.seed`, `OpenAiChatRequest.seed`; ~14 files mechanical; wire test).
2. **12-02 Routing + infra classification** (`resolve_eval_model`, `generate_candidate(config)`, `MensCatalog` refresh, 409 fatal, prefix-based `is_infra_error`, `--seed`/`--top-p`, `sample_seed`, wiremock tests). Depends on 12-01 only for the `seed` field (stub with `None` first if parallel).
3. **12-03 MENS sampling reproducibility + `eval_local`** (serve schema `seed`, metal plugin seeded rng, `eval_local` sample loop + `per_problem_pass_at_k`). CUDA parity as a trailing task.

**Wave 2:**
4. **12-04 Typed report + stats + compare** (`vox-eval` report/compare modules; `eval-compare` subcommand; `n_infra_errors` fix). Test-first.
5. **12-05 Provenance record + completeness gate** (`vox-eval::provenance`, collectors in vox-cli, `Fixture` path fields, `problem_set_hash`, `config_digest` migration to vox-crypto, scoreboard guard, `run_complete` formula). Depends on 12-04's typed report.
6. **12-06 Training lineage** (`AdapterCard` fields + `validate_lineage`, `vox_compiler` fingerprint wrapper, `train_arm.rs` hook for local and cloud, eval card gate + drift flag). Independent of 12-04/05 except the gate hook; may run in parallel with 12-04.

**Wave 3 (blocked on Phase 7):**
7. **12-07 Bundle + identity + atomic persistence** (bundle struct, canonical JSON via Phase 7, sign/verify, allowlist, `persist_run_atomic`, `model_benchmark_results` projection). The sign/verify core can be developed against a stub canonicalizer behind the 5-item boundary above, but must not ship until Phase 7's canonicalizer replaces the stub.

Per the lean-verification memory, 12-04/12-05/12-06 are small enough that tail tasks may be batched into one executor; ask before batching plans touching different heavy crates (vox-populi/vox-ml-cli vs vox-cli).

## Sources

### Primary (HIGH confidence — read this session)
- `.planning/workstreams/research-trial-flywheel/phases/12-.../12-CONTEXT.md`; `docs/src/architecture/vox-mens-continuous-benchmark-provenance-design-2026-10-10.md` (§§2,5,8,9); REQUIREMENTS.md (BENCH-01..06, STORE-01, TRIAL-05, TEST-01); ROADMAP.md (Phase 7/12 sections); STATE.md; `.planning/research/TRIAL-DESIGN.md`.
- `crates/vox-cli/src/commands/model/{eval_corpus.rs,bench/mod.rs,mod.rs}`, `crates/vox-cli/src/commands/chat.rs`, `crates/vox-cli/Cargo.toml`
- `crates/vox-ml-cli/src/commands/mens/eval_local.rs`, `.../populi/train_arm.rs`, `.../ai/serve/{schema,handlers,config,mod}.rs`, `.../ai/{model_id,inference_defaults}.rs`
- `crates/vox-eval/src/{corpus_stats,corpus_score,lib}.rs`
- `crates/vox-actor-runtime/src/llm/{types,chat,stream,cascade,mod}.rs`, `crates/vox-llm-egress/src/{lib,wire}.rs`, `tests/wire_mock.rs`, `crates/vox-config/src/{resolve_egress,inference}.rs`
- `crates/vox-orchestrator/src/{catalog.rs,models/registry.rs,models/key_guard.rs,preregistration/{signing,trusty_uri}.rs}`
- `crates/vox-populi/src/mens/{tensor/adapter_card.rs,cloud/pipeline_dispatch.rs,serving/}`; `crates/vox-plugin-mens-candle-core/src/manifest.rs`; `crates/vox-plugin-mens-candle-metal/src/{inference,backend}.rs`
- `crates/vox-grammar-export/src/versioning.rs`; `crates/vox-crypto/src/facades.rs` (+ `git diff --cached` for `interop_sha256`); `crates/vox-corpus/src/humaneval_runner/{manifest,conditions,verify,mod}.rs`; `contracts/eval/humaneval-vox/manifest.v1.yaml`
- `contracts/ci/crate-edges.allow.v1.json` (programmatic edge membership), `docs/src/architecture/layers.toml`
- `git diff` of `crates/vox-db/src/{schema/domains/scientia.rs,schema/manifest.rs,store/*}` and `ops_model_bench.rs`

### Secondary (MEDIUM confidence)
- OpenRouter parameters doc (seed semantics): https://openrouter.ai/docs/api-reference/parameters
- Ollama OpenAI compatibility (supported fields incl. `seed`): https://docs.ollama.com/api/openai-compatibility

### Tertiary (LOW confidence)
- A3, A4, A6 (training-knowledge, flagged in the Assumptions Log)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every component located and read; edge permissions checked programmatically.
- Architecture: HIGH for Wave 1-2; MEDIUM for Wave 3 (Phase 7 unplanned).
- Pitfalls: HIGH for 1,2,3,4,5 (derived from code read); MEDIUM for 7,9.

**Research date:** 2026-10-10
**Valid until:** 2026-10-24 (the working tree has 71 uncommitted files in the same crates; re-check `LlmConfig`, `registry.rs`, `scientia.rs` before planning if they have landed)
