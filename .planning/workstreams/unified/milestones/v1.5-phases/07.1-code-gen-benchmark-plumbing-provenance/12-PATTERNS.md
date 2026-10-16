# Phase 12: Code-Gen Benchmark Plumbing & Provenance - Pattern Map

**Mapped:** 2026-10-10
**Files analyzed:** 22 new/modified
**Analogs found:** 21 / 22 (one is Phase-7-blocked, partial analog)

All analog paths below were checked against `git ls-files`. **One exception that the planner must know about:** `crates/vox-cli/src/commands/model/bench/` is **untracked** (`??` in `git status`) and holds `resolve_target`, the routing resolver this phase must reuse. It has no tracked origin. Plans that call `resolve_target` depend on that in-flight branch work landing first (RESEARCH Pitfall 10). The same applies to the uncommitted hunks in `types.rs` (`LlmConfig::direct`) and `registry.rs` (`llm_config_for_spec` VoxLocal arm).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/vox-actor-runtime/src/llm/types.rs` (+`seed`) | model/config | request-response | itself (`top_p` field) | exact |
| `crates/vox-actor-runtime/src/llm/{chat,stream}.rs` (forward seed) | service | request-response | itself (`ChatParams {..}` blocks) | exact |
| `crates/vox-llm-egress/src/lib.rs` (`ChatParams.seed`) | model | request-response | itself (`top_p`) | exact |
| `crates/vox-llm-egress/src/wire.rs` (`OpenAiChatRequest.seed`) | utility | request-response | itself (`top_p`) | exact |
| `crates/vox-llm-egress/tests/wire_mock.rs` (+seed tests) | test | request-response | same file `body_partial_json` tests | exact |
| `crates/vox-ml-cli/src/commands/ai/serve/schema.rs` (+`seed`) | model | request-response | itself (`temperature`) | exact |
| `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs` (forward seed) | controller | request-response | itself (`GenerateRequest {..}` build) | exact |
| `crates/vox-plugin-mens-candle-metal/src/inference.rs` (seeded rng) | service | transform/streaming | itself (`PromptRequest`, `sample_next_token`) | exact |
| `crates/vox-cli/src/commands/model/eval_corpus.rs` (routing, seed, infra, gate, typed report) | controller | batch + request-response | itself + `bench/mod.rs::run_case` | exact |
| `crates/vox-ml-cli/src/commands/mens/eval_local.rs` (sample loop, unbiased pass@k) | controller | batch | itself + `vox_eval::corpus_score::pass_at_k` | role-match |
| `crates/vox-eval/src/report.rs` (NEW) | model/utility | transform | `crates/vox-eval/src/corpus_score.rs` | exact |
| `crates/vox-eval/src/provenance.rs` (NEW) | model | transform | `corpus_score.rs` (+ `AdapterCard::validate`) | role-match |
| `crates/vox-eval/src/compare.rs` (NEW) | utility | transform | `crates/vox-eval/src/corpus_stats.rs` | exact |
| `crates/vox-eval/src/lib.rs` (`pub mod` lines) | config | n/a | lines 9-11 | exact |
| `crates/vox-cli/src/commands/model/eval_compare.rs` (NEW) | controller | file-I/O + transform | `model/shadow.rs` (thin clap wrapper) | role-match |
| `crates/vox-cli/src/commands/model/mod.rs` (register variant) | config | n/a | itself (`EvalCorpus` variant + match arm) | exact |
| `crates/vox-cli/src/commands/model/eval_bundle.rs` (NEW, Wave 3) | service | file-I/O | `orchestrator/preregistration/signing.rs` + `trusty_uri.rs` | role-match |
| `crates/vox-populi/src/mens/tensor/adapter_card.rs` (+4 lineage fields, `validate_lineage`) | model | CRUD | itself | exact |
| `crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs` (`card_from_manifest`) | service | CRUD | itself (lines 518-551) | exact |
| `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` (local card hook) | controller | file-I/O | itself (post-success block, 588-612) | exact |
| `crates/vox-compiler/src/llm_prompt.rs` (`grammar_fingerprint` wrapper) | utility | transform | itself (`vox_grammar_prompt`) | exact |
| `crates/vox-corpus/src/humaneval_runner/manifest.rs` (+`spec_path`, `reference_path`) | model | file-I/O | itself (`tests_path`) | exact |
| Phase 7 `persist_run_atomic` / `model_benchmark_results` projection (Wave 3) | service/migration | CRUD | none (Phase 7 unplanned) | none |

## Pattern Assignments

### Seed plumbing: `types.rs`, `chat.rs`, `stream.rs`, `egress lib.rs`, `wire.rs` (config/wire, request-response)

**Analog:** the existing `top_p` field at every one of these sites. Add `seed` next to it with identical shape.

`LlmConfig` (`crates/vox-actor-runtime/src/llm/types.rs:64-69`):
```rust
    /// Sampling temperature when supported by the endpoint.
    pub temperature: Option<f32>,
    /// Sampling top_p when supported by the endpoint.
    pub top_p: Option<f32>,
    /// Maximum tokens to generate when supported.
    pub max_tokens: Option<u64>,
```
Add `pub seed: Option<u64>,` (stored configs still parse: `Option` defaults to `None`). Every `LlmConfig {..}` struct literal (about 31 in 14 files) needs `seed: None,` after `telemetry_skip_interaction:`.

`ChatParams` (`crates/vox-llm-egress/src/lib.rs:143-152`), derives `Default`, so only the 3 constructors in `chat.rs:157`, `stream.rs:53`, `stream.rs:98` need the new line; other callers use `..Default::default()`:
```rust
#[derive(Clone, Debug, Default)]
pub struct ChatParams<'a> {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u64>,
    ...
}
```

Facade forwarding (`chat.rs:157-164`, same shape at `stream.rs:53` and `:98`):
```rust
let params = vox_llm_egress::ChatParams {
    temperature: config.temperature,
    top_p: config.top_p,
    max_tokens: config.max_tokens,
    response_format: config.response_format.as_ref(),
    tools: wire_tools.as_deref(),
    tool_choice: config.tool_choice.as_ref(),
};
```
Add `seed: config.seed,`.

Wire struct (`crates/vox-llm-egress/src/wire.rs:255-302`). Copy the `skip_serializing_if` idiom so a `None` seed leaves the body byte-identical for existing providers:
```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    ...
    // in build_request(): 
    top_p: params.top_p,
```
Both chat and stream go through `build_request` (`wire.rs:327`, `:447`), so one edit covers both.

**Test pattern** (copy from `crates/vox-llm-egress/tests/wire_mock.rs:1-60, 165-188`): `req(base)` helper, `MockServer`, `body_partial_json(serde_json::json!({"seed": 1234}))`, `.respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}})))`, call `chat_once(&r, &[], &ChatParams { seed: Some(1234), ..Default::default() })`. Add the negative twin (seed `None` => body has no `seed` key; use a `wiremock` custom matcher or `received_requests()` and assert the key is absent).

---

### `crates/vox-cli/src/commands/model/eval_corpus.rs` (controller, batch + request-response)

**Analog:** itself for structure; `crates/vox-cli/src/commands/model/bench/mod.rs` (untracked, see header) for routing.

**Routing to copy** (`bench/mod.rs:352-356`, plus `resolve_target` at `:243-300`):
```rust
let mut cfg = llm_config_for_spec(spec, category);
cfg.max_tokens = Some(case.max_tokens);
cfg.temperature = Some(0.0);
cfg.timeout_ms = Some(timeout_ms);
cfg.telemetry_skip_interaction = true;
```
Imports it uses (`bench/mod.rs:18-24`):
```rust
use vox_actor_runtime::llm::{LlmChatMessage, llm_chat, llm_stream_measured};
use vox_orchestrator::models::{
    ModelCapabilities, ModelRegistry, ModelSpec, PricingSource, ProviderType, TaskCategory,
    llm_config_for_spec, ...
};
```
Make `resolve_target` reachable from `eval_corpus.rs` (it is `pub fn` in `bench/mod.rs`, so `use super::bench::resolve_target;`). Add the `mens/*` no-synthesize guard from RESEARCH Pattern 1 and `MensCatalog::new(&repo_root).refresh().await` (`crates/vox-orchestrator/src/catalog.rs:1218-1245` emits `id: format!("mens/{}", name)`, `provider_type: ProviderType::VoxLocal`).

**The line to replace** (`eval_corpus.rs:244-256`):
```rust
async fn generate_candidate(model_id: &str, prompt: &str, temperature: f32, max_tokens: u64)
    -> Result<(String, u32, i64, Option<f64>), String> {
    ...
    let mut config = LlmConfig::openrouter(model_id);
    config.max_tokens = Some(max_tokens);
    config.temperature = Some(temperature);
    config.telemetry_task_category = Some("eval-corpus".to_string());
```
Change the first parameter to `config: LlmConfig` (resolved once per run, then `.clone()` and set `config.seed = Some(sample_seed(..))` per sample). That signature makes the wiremock test possible.

**Core call pattern to keep** (`eval_corpus.rs:264-272`):
```rust
let outcome = llm_chat(&ActivityOptions::new(), messages, config).await;
let response = match outcome {
    vox_actor_runtime::ActivityResult::Ok(inner) => inner,
    vox_actor_runtime::ActivityResult::Failed(e) => return Err(e.to_string()),
    vox_actor_runtime::ActivityResult::Cancelled => return Err("activity cancelled".into()),
}?;
```

**Infra classification to fix** (`eval_corpus.rs:138-148`; current test uses a hand-written string, so add one using the real prefixes `vox_actor_runtime::llm::{RATE_LIMITED_PREFIX, CONTEXT_EXCEEDED_PREFIX}` re-exported from `llm/mod.rs`):
```rust
pub fn is_infra_error(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("rate limited") || m.contains("429") || m.contains("context length")
        || m.contains("502") || m.contains("503") || m.contains("timeout")
}
```

**Sample loop to keep unchanged in shape** (`eval_corpus.rs:384-473`): `'fixtures:` label, `for _ in 0..n_attempts`, spend-cap `break 'fixtures`, infra branch does `n_infra_errors += 1; continue;` (infra never enters `attempts`), no early break on pass (comment at `:461-463`). Add a parallel `Vec<SampleRecord>` (RESEARCH Pattern 2); do not change `AttemptOutcome`/`FixtureOutcome` serde shape.

**Report to replace** (`eval_corpus.rs:511-524`): the `serde_json::json!({ "schema_version": 1, ... })` literal becomes a typed `vox_eval::report::EvalCorpusReport` (schema_version 2). Write via the existing tail (`create_dir_all` + `std::fs::write(path, serde_json::to_string_pretty(&report)?)`).

**Digest to migrate** (`eval_corpus.rs:115-136`, direct `sha2` is a policy violation): replace the `use sha2::Digest` block with `vox_crypto::facades::{secure_hash, hex_encode}`; widen inputs to include model id, seed, top_p, problem_set_hash:
```rust
let d = vox_crypto::facades::secure_hash(format!("{harness}|{n}|...").as_bytes());
vox_crypto::facades::hex_encode(&d[..8])
```

**Write-back guard** (`eval_corpus.rs:493-508`): wrap in `if !args.no_write_back && run_complete && measured`; extract the predicate as a pure `fn should_write_back(run_complete: bool, measured: bool, no_write_back: bool) -> bool` (test-first, `eval_corpus::tests::write_back_*`).

**Args to extend** (`eval_corpus.rs:34-91`): add `--seed` and `--top-p` in the existing style:
```rust
    /// Sampling temperature. Use 0.0 with n=1 (greedy headline) or >=0.6 with n>=10.
    #[arg(long, default_value_t = 0.0)]
    pub temperature: f32,
    #[arg(long, default_value_t = 4096)]
    pub max_tokens: u64,
```
No new command-registry row for new flags on an existing command.

**Test pattern:** existing tests (`eval_corpus.rs:536-657`) are pure `#[test]` fns in `mod tests { use super::*; }`. Add `#[tokio::test]` wiremock tests per RESEARCH "Test infrastructure" (`wiremock` is already a vox-cli dev-dep; mirror `wire_mock.rs`). Routing tests use `ModelRegistry::default()` (precedent `bench/mod.rs:946-960`):
```rust
let reg = ModelRegistry::default();
let (g, synth) = resolve_target(&reg, "groq:llama-3.3-70b-versatile");
assert!(synth);
assert_eq!(g.provider_type, ProviderType::Groq);
```

---

### `crates/vox-eval/src/report.rs`, `provenance.rs`, `compare.rs` (NEW; pure transform)

**Analogs:** `corpus_score.rs` (serde structs + pure fns + test module) and `corpus_stats.rs` (paired/stat helpers). Both are tracked, L2, no I/O.

**Module header + imports to copy** (`corpus_score.rs:1-14`): a `//!` doc block naming the audit finding it implements, then:
```rust
use serde::{Deserialize, Serialize};
```
**Struct style** (`corpus_score.rs:15-37`): `#[derive(Debug, Clone, Serialize, Deserialize)]`, doc comment per field stating the fact source.
**Registration** (`crates/vox-eval/src/lib.rs:9-11`):
```rust
pub mod corpus_score;
pub mod corpus_stats;
pub mod mens;
```
Append `pub mod report; pub mod provenance; pub mod compare;` in the same list.

**Primitives to call, never reimplement** (`corpus_stats.rs`, signatures):
`bootstrap_ci(per_problem: &[f64], reps: usize, seed: u64) -> (f64, f64)` (:114), `paired_compare(a: &[bool], b: &[bool]) -> PairedResult` (:60, asserts equal length), `holm_reject(p_values: &[f64], alpha: f64) -> Vec<bool>` (:88), `wilson_interval` (:143, single-attempt pass@1 only), `min_detectable_difference(n) -> f64` (:178), `corpus_score::pass_at_k(n, c, k)` (:42). `intervals_overlap` (:167) is display-only; no compare verdict may use it.

**Test module style** (`corpus_stats.rs:246-276`): `#[cfg(test)] mod tests { use super::*; #[test] fn <behavior_as_sentence>() { assert!(..., "message explaining why"); } }`. The research test map names `report::`, `compare::`, `provenance::` as the nextest filters, so name the modules to match.

**Provenance completeness** (`provenance.rs`): `RunProvenance::missing_required(&self) -> Vec<&'static str>` is the single gate. Required fields are `Option<T>` with **no** `#[serde(default)]` so an absent key fails deserialization (RESEARCH BENCH-04). The fail-closed validation idiom to mirror is `AdapterCard::validate` (`adapter_card.rs:27-38`):
```rust
if self.base_rung.is_empty() { anyhow::bail!("AdapterCard.base_rung must not be empty"); }
```
but return the list of missing names instead of bailing on the first.

---

### `crates/vox-cli/src/commands/model/eval_compare.rs` (NEW, controller) and `model/mod.rs` registration

**Analog:** `crates/vox-cli/src/commands/model/shadow.rs` (small clap `Args` struct + `pub async fn run(args) -> anyhow::Result<()>` that delegates to a library fn, with a pure-fn unit test). Body is a thin wrapper over `vox_eval::compare`.

Args/run shape (`shadow.rs:1-60`):
```rust
use clap::Args;
use owo_colors::OwoColorize;

#[derive(Args, Debug)]
pub struct ShadowArgs {
    pub model_id: String,
    #[arg(long, value_delimiter = ',')]
    pub suites: Vec<String>,
    #[arg(long, default_value_t = 0.25)]
    pub max_usd: f64,
}
pub async fn run(args: ShadowArgs) -> anyhow::Result<()> { ... }
```
Report loading: use `vox_bounded_fs::read_utf8_path_capped` and check `schema_version` (RESEARCH V5).

**Registration** (`model/mod.rs`): three places, copy the `EvalCorpus` entries.
```rust
pub mod eval_corpus;                                   // module list (alphabetical)
/// Score a model or external harness against the held-out HumanEval-Vox corpus.
EvalCorpus(Box<eval_corpus::EvalCorpusArgs>),          // enum variant (Box if large)
ModelCmd::EvalCorpus(args) => eval_corpus::run(*args).await,   // match arm
```
Adding a subcommand changes `contracts/cli/command-registry.yaml` and `contracts/operations/catalog.v1.yaml`: regenerate with `cargo run -p vox-cli -- ci command-sync` (hooks auto-stage). Both files are already dirty on this branch; keep hunks small.

---

### `crates/vox-ml-cli/src/commands/mens/eval_local.rs` (controller, batch)

**Analog:** itself (loop scaffold) + `vox_eval::corpus_score::pass_at_k`. `vox-eval` is already a dependency of vox-ml-cli.

**Current single-sample shape to turn into a loop** (`eval_local.rs:185-245`, inside `#[cfg(feature = "gpu")]`):
```rust
// Greedy decoding is deterministic, so one generation suffices for pass@k.
let prompt_json = serde_json::json!({
    "prompt": &prompt, "max_tokens": max_tokens, "temperature": temperature,
}).to_string();
match backend.run_inference(handle, prompt_json.as_str().into()).into_result() {
    Ok(resp) => { ... (v.pass, v.pass, vec![sample]) }
    Err(e) => ( false, false, vec![serde_json::json!({"sample_index": 0, "pass": false, "error": format!("run_inference: {e}")})] ),
}
```
Wrap in `for i in 0..samples.max(1)`, add `"seed": seed_base + i as u64` to `prompt_json`, set `"sample_index": i`. The aggregation that must change is `:264-268` (`samples_json.iter().any(...)`) and the `passed_k` count accumulations at `:290-293`, `:297-303`, `:360-364`.

**Pure, un-gated aggregator** (extract so it runs in default CI; the inference body stays `cfg(feature = "gpu")`):
```rust
fn per_problem_pass_at_k(samples: &[bool], k: usize) -> f64 {
    let n = samples.len();
    let c = samples.iter().filter(|b| **b).count();
    vox_eval::corpus_score::pass_at_k(n, c, k)
}
```
Test vectors (RESEARCH): `(5,1,1)=0.2`, `(5,2,3)=0.9`, `(5,1,5)=1.0`. Test name filter `eval_local::tests::unbiased_*`.

---

### `crates/vox-ml-cli/src/commands/ai/serve/{schema,handlers}.rs` and metal `inference.rs` (MENS seed chain)

**Serve request** (`schema.rs:111-127`): add next to `temperature`:
```rust
    #[serde(default)]
    pub temperature: Option<f32>,
    // add:
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub top_p: Option<f32>,
```
**Handler forward** (`handlers.rs:295-303`): `GenerateRequest { prompt, max_tokens: req.max_tokens.unwrap_or(256), temperature: req.temperature.unwrap_or(0.7), model: req.model.clone(), ... }`; add `seed: req.seed`, and add `#[serde(default)] pub seed: Option<u64>` to `GenerateRequest` (`schema.rs:8-27`, same `#[serde(default)] pub model: Option<String>` idiom).

**Plugin request** (`crates/vox-plugin-mens-candle-metal/src/inference.rs:597-616`): add to the local `PromptRequest`:
```rust
            #[serde(default)]
            top_k: usize,
            // add:
            #[serde(default)]
            seed: Option<u64>,
```
**Sampler** (`inference.rs:655`): replace `let mut rng = rand::thread_rng();` with a seeded rng when `req.seed` is `Some` (pass `seed` into `generate`, signature at `:635-641`):
```rust
use rand::SeedableRng;
let mut rng: rand::rngs::StdRng = match seed {
    Some(s) => rand::rngs::StdRng::seed_from_u64(s),
    None => rand::rngs::StdRng::from_entropy(),
};
```
`sample_next_token(logits, temperature, top_k, rng: &mut impl rand::Rng)` (`:857-862`) is already generic over `Rng`, so it needs no change. Existing tests (`:1418-1440`) call it with `rand::thread_rng()`; add a `seeded_*` test: same seed and same logits sequence => identical token sequence, different seed => differs.
CUDA copy is a separate file (diverged 1208 vs 1579 lines): same change or a recorded deviation (D-12). `top_p` is not implemented by the sampler (`top_k` only): record `null` honestly.

---

### `crates/vox-populi/src/mens/tensor/adapter_card.rs` (model, CRUD) and its writers

**Analog:** itself. Add four fields with `#[serde(default)]` for forward-compat reads (current struct has none, `adapter_card.rs:5-25`):
```rust
    pub corpus_hash: String,
    ...
    // add (Strings; empty = absent lineage):
    #[serde(default)] pub adapter_sha256: String,
    #[serde(default)] pub train_data_sha256: String,
    #[serde(default)] pub training_syntax_fingerprint: String,
    #[serde(default)] pub syntax_fingerprint_kind: String,
```
Update `AdapterCard::for_test` (`:78-95`) and the one non-test struct literal in `card_from_manifest` (`pipeline_dispatch.rs:522-540`). Keep `validate()` untouched (serve depends on it). Add `validate_lineage()` in the same fail-closed style as `validate()` (`:27-38`):
```rust
pub fn validate_lineage(&self) -> Result<()> {
    if self.adapter_sha256.is_empty() { anyhow::bail!("AdapterCard.adapter_sha256 must not be empty"); }
    ...
}
```
Sidecar IO to reuse, not rewrite: `write_sidecar` (`:44-52`) and `read_sidecar` (`:57-69`, returns `Ok(None)` when absent, which the eval gate must treat as "refuse to label").
**Existing tests** (`:104+`): `validate_rejects_empty_rung` style (`AdapterCard::for_test(..)`, mutate one field, `assert!(card.validate().is_err(), "msg")`). Add: lineage round-trip, old JSON without lineage still parses, `validate_lineage` rejects empty.

**Hashing** (D-11): `vox_crypto::facades::{interop_sha256, hex_encode}` (`facades.rs:96-103`, `:46`):
```rust
let digest = vox_crypto::facades::hex_encode(&vox_crypto::facades::interop_sha256(&std::fs::read(path)?));
```
Check that `vox-populi` has a `vox-crypto` edge before using it there; otherwise compute the hashes in `train_arm.rs` (vox-ml-cli) and pass strings into the builder (the edge note in RESEARCH BENCH-05 says the fingerprint string is also passed in).

---

### `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` (controller, file-I/O)

**Analog:** itself, the post-success block (`train_arm.rs:588-612`) that already runs host-side logic after `train_res.is_ok()`:
```rust
    if train_res.is_ok()
        && let Some(ref r) = workspace_root
    {
        let mixed = r.join("target/dogfood/train_mixed.jsonl");
        ...
    }
    train_res
```
Insert the local-arm card write here (hash the file the trainer actually consumed, immediately, in the same process; RESEARCH Pitfall 7), non-fatal warnings styled like the existing `eprintln!("  {} ...", "⚠️".yellow())` lines. Cloud arm: add the same fields to cloud `TrainingManifest` (`train_arm.rs:240-255` per research) so both converge on `card_from_manifest`.
**Test:** file has `#[test]`s at `:879` and `:892` (tempdir style); put new `train_arm::` tests beside them.

---

### `crates/vox-compiler/src/llm_prompt.rs` (utility, transform)

**Analog:** the sibling wrapper in the same file (`llm_prompt.rs:9-11`):
```rust
pub fn vox_grammar_prompt() -> String {
    vox_grammar_export::compact_prompt::emit_compact_llm_prompt()
}
```
Add `pub fn grammar_fingerprint() -> (String, &'static str)` delegating to `vox_grammar_export::versioning::compute_ebnf_hash()` with kind `"ebnf-sha256"`. This is the zero-new-edge route (`vox-ml-cli -> vox-compiler` exists; `vox-ml-cli -> vox-grammar-export` and `vox-populi -> vox-grammar-export` do not). Needs a `#[test]` in the same file (Test-First Policy; file is under 30 non-blank lines in the excerpt but add one anyway: non-empty, 64 hex chars, stable across two calls).

---

### `crates/vox-corpus/src/humaneval_runner/manifest.rs` (model, file-I/O)

**Analog:** itself. `Fixture` (`manifest.rs:11-27`) carries only `tests_path: PathBuf`. Add `spec_path` and `reference_path` with the same doc-comment form (`/// Absolute path to the fixture's ...`) and populate them in the single construction site in `load_corpus` (`manifest.rs:~91`). `problem_set_hash` is computed in vox-cli (vox-corpus has no `vox-crypto` edge): BLAKE3 over `(id, spec.toml, reference.vox, tests.vox)` for fixtures sorted by id, via `secure_hash`. Pitfall 9: grep other `Fixture {` sites in vox-ml-cli / vox-audit before assuming one.

---

### `crates/vox-cli/src/commands/model/eval_bundle.rs` (NEW, Wave 3, file-I/O + signing)

**Analog:** `crates/vox-orchestrator/src/preregistration/signing.rs` (sign/verify flow, error enums, tamper tests) and `trusty_uri.rs` (canonicalizer). The signing key is the Phase 7 allowlisted campaign signer (D-10); no new `SecretId`.

Imports/sign (`signing.rs:16-20, 71-74`):
```rust
use thiserror::Error;
use vox_crypto::facades::{SigningKey, sign, to_verifying_key, verify, verifying_key_from_bytes, verifying_key_to_bytes};
...
let canonical = canonical_json(prereg).map_err(|e| SignError::Serialization(e.to_string()))?;
let sig_bytes = sign(signing_key, canonical.as_bytes());
Ok(Signature(hex::encode(sig_bytes)))
```
Offline verify primitive: `vox_crypto::facades::verify_signature_hex(pubkey_hex, message, signature_hex) -> Result<bool, String>` (`facades.rs:259`). **Do not trust the key embedded in the bundle**; check it against the Phase 7 allowlist before accepting (signing.rs verifies against the embedded key, which is the known weakness).

Canonicalizer to promote or replace (`trusty_uri.rs:46-86`): `canonical_json` is `pub(crate)` and typed to `PreregistrationV1`; `sort_keys` is private:
```rust
fn sort_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> =
                map.into_iter().map(|(k, v)| (k, sort_keys(v))).collect();
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(items.into_iter().map(sort_keys).collect()),
        other => other,
    }
}
```
Use Phase 7's generic `canonical_json_bytes<T: Serialize>` if it exists; fallback is promoting this into a generic pub fn (<30 lines) that also rejects non-finite floats (bundle rates can be NaN; Pitfall 8). Never serialize `LlmConfig` (carries `api_key`); build an explicit `SamplingRecord`.
**Test pattern** (`signing.rs:155-190`): `sign_and_verify_round_trip`, `tamper_detection_fails_verify` (mutate a field, assert `is_err`), `wrong_signature_hex_fails_verify` (`"ff".repeat(64)`), using `generate_signing_keypair()`. Add: tampered sample code, tampered seed, NaN rejected. Mutation proof: skip signature verification, test must fail.

---

### Phase 7 persistence / `model_benchmark_results` projection (Wave 3)

**No analog** (Phase 7 has no plans). Per D-09, Phase 12 adds no migration of its own and must not claim a baseline version. The `model_benchmark_results` table exists only as uncommitted work (`crates/vox-db/src/schema/domains/scientia.rs`, `schema/manifest.rs` `BASELINE_VERSION = 95`, untracked `ops_model_bench.rs`); it becomes a projection over Phase 7's campaign/run tables. The planner should write this plan against the 5-item interface boundary in RESEARCH "BENCH-06" and treat `persist_run_atomic` as a stub-able trait/fn until Phase 7 lands.

## Shared Patterns

### Crypto: `vox-crypto` facade only
**Source:** `crates/vox-crypto/src/facades.rs` (`secure_hash` :11 BLAKE3, `interop_sha256` :98, `hex_encode` :46, `sign`/`verify`, `verify_signature_hex` :259)
**Apply to:** `eval_corpus.rs` (`config_digest`, `sample_seed`, `problem_set_hash`, `prompt_template_hash`, `vox_binary_sha256`), `adapter_card` lineage hashing, `eval_bundle.rs`.
Rules: BLAKE3 (`secure_hash`) for Vox-owned identity (`problem_set_hash`, `context_hash`, `config_digest`, per-sample seed); `interop_sha256` for the four D-11 externally checkable hashes (`adapter_sha256`, `train_data_sha256`, `problem_set_hash` per D-11, `vox_binary_sha256`). Note D-11 lists `problem_set_hash` as SHA-256, while RESEARCH BENCH-04 sketched BLAKE3 for it; **D-11 (locked, later) wins**, use `interop_sha256`. No `sha2`/`blake3`/`ed25519-dalek` imports outside `vox-crypto` (detector `vox/crypto/banned-crate-import`). Per-sample seed (from RESEARCH, pure and test-first):
```rust
pub fn sample_seed(base: u64, fixture_id: &str, i: usize) -> u64 {
    let h = vox_crypto::facades::secure_hash(format!("{base}|{fixture_id}|{i}").as_bytes());
    u64::from_le_bytes(h[..8].try_into().unwrap()) & 0x7fff_ffff
}
```

### LLM boundary
**Source:** `crates/vox-actor-runtime/src/llm/` (`llm_chat`, `LlmConfig`, `RATE_LIMITED_PREFIX`, `CONTEXT_EXCEEDED_PREFIX`)
**Apply to:** `eval_corpus.rs` only. No vendor hostnames; model choice via `llm_config_for_spec` (`crates/vox-orchestrator/src/models/registry.rs:1294`). Do not copy `resolve_chat_config` from `chat.rs` (string-prefix routing).

### Error handling
**Source:** `eval_corpus.rs` (`anyhow::Result` + `with_context(|| format!(..))`, `anyhow::ensure!`/`bail!` with remediation text, `tracing::warn!` for non-fatal) and `signing.rs` (`thiserror` enums for library-style crates).
**Apply to:** vox-cli files use `anyhow`; `vox-eval` new modules use `thiserror` enums or plain `Result<_, String>` consistent with `corpus_stats.rs` (which is infallible/pure); `vox-orchestrator`-style bundle verify uses `thiserror`.

### Test-first and mutation proofs
**Source:** Test-First Policy (AGENTS.md); `signing.rs` tamper tests; lean-verification memory.
**Apply to:** every new `pub fn` gets a `#[test]` in the same file. Executors run `cargo check -p <crate>` plus only the plan's own tests, backgrounded (never block on cargo). Required mutation proofs (with before/after presence check on the mutated line): (1) remove provenance gate call, (2) route `mens/*` back to openrouter, (3) drop `seed` from `build_request`, (4) replace `pass_at_k` with any-sample, (5) skip signature verification, (6) remove `run_complete` filter in compare.

### Command registry
**Apply to:** `eval_compare.rs` + `model/mod.rs` only. Regenerate with `cargo run -p vox-cli -- ci command-sync`.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| Phase 7 `persist_run_atomic` / run-table projection | service | CRUD | Phase 7 not planned; use the interface boundary stub (RESEARCH BENCH-06) |

Partial analogs worth flagging: generic canonical-JSON (`trusty_uri.rs` is prereg-typed and `pub(crate)`), and Phase-7 signer-allowlist verification (none exists; `verify_prereg` trusts the embedded key).

## Metadata

**Analog search scope:** `crates/vox-cli/src/commands/model/`, `crates/vox-eval/src/`, `crates/vox-llm-egress/`, `crates/vox-actor-runtime/src/llm/`, `crates/vox-ml-cli/src/commands/{mens,ai}/`, `crates/vox-populi/src/mens/`, `crates/vox-orchestrator/src/{preregistration,catalog.rs,models/registry.rs}`, `crates/vox-plugin-mens-candle-metal/`, `crates/vox-compiler/src/llm_prompt.rs`, `crates/vox-corpus/src/humaneval_runner/`
**Files scanned:** ~30 (read or range-read); tracked-source check run on all named analogs via `git ls-files`
**Untracked analog:** `crates/vox-cli/src/commands/model/bench/mod.rs` (in-flight branch work)
**Pattern extraction date:** 2026-10-10
