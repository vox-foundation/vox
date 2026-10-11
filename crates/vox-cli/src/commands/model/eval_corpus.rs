//! `vox model eval-corpus` — score one (model, harness, condition) tuple
//! against the held-out HumanEval-Vox corpus.
//!
//! Two input modes:
//!   * `--from-dir <dir>`: score pre-generated solutions. This is how external
//!     harnesses (Claude Code, Cursor, Warp, Grok) enter the leaderboard — we
//!     cannot drive their UIs, but we can score their output with the identical
//!     verifier, which is what makes the comparison fair. Tokens/latency/cost
//!     are `None` for these rows, never a fabricated `0`.
//!   * live generation (default): generate through the `vox_actor_runtime::llm`
//!     facade for a registry model id, drawing `n` samples per fixture with NO
//!     early stop (the unbiased pass@k estimator needs every outcome).
//!
//! Correctness comes from `vox_corpus::humaneval_runner::verify_program` alone
//! — compiler and test exit codes, canary-checked against oracle-neutralizing
//! candidates. No LLM judge participates in the correctness path.
//!
//! See `docs/superpowers/plans/2026-09-01-vox-efficacy-benchmark-v2.md` and
//! `docs/src/architecture/vox-efficacy-benchmark-adversarial-audit-2026-09-01.md`.

use anyhow::{Context, Result};
use clap::Parser;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vox_actor_runtime::llm::LlmConfig;
use vox_corpus::humaneval_runner::conditions::{
    Condition, PromptContext, build_context, build_prompt,
};
use vox_corpus::humaneval_runner::{
    Fixture, VerifyOutcome, eligible_after, held_out, load_corpus, verify_program,
};
use vox_crypto::facades::{hex_encode, interop_sha256, secure_hash};
use vox_eval::corpus_score::{AttemptOutcome, CorpusScore, FixtureOutcome};
use vox_eval::provenance::{MensLineage, RunProvenance, SamplingRecord};
use vox_eval::report::{
    DEFAULT_BOOTSTRAP_REPS, DEFAULT_MAX_INFRA_RATE, EvalCorpusReport, FailureClass, ReportInputs,
    SampleRecord, VerifyFacts, classify_failure,
};
use vox_orchestrator::catalog::{MensCatalog, ModelCatalog};
use vox_orchestrator::models::{ModelRegistry, ModelSpec, TaskCategory, llm_config_for_spec};
use vox_populi::mens::tensor::adapter_card::check_for_eval;

use super::bench::resolve_target;

/// `vox model eval-corpus` arguments.
#[derive(Parser, Debug, Clone)]
pub struct EvalCorpusArgs {
    /// Model id to score (registry id for live mode, or a label for `--from-dir`).
    #[arg(long)]
    pub model: String,
    /// Harness that produced the solutions (e.g. `vox-harness`, `claude-code`, `cursor`, `warp`).
    #[arg(long, default_value = "vox-harness")]
    pub harness: String,
    /// Local model checkpoint label (e.g. a MENS build id). Folded into the row's
    /// model id so successive checkpoints are separate leaderboard rows.
    #[arg(long)]
    pub checkpoint: Option<String>,
    /// Prompt condition: C0 zero-shot, C1 grammar, C2 few-shot, C3 full docs.
    #[arg(long, default_value = "C1")]
    pub condition: String,
    /// Samples per fixture. All are drawn; there is no early stop, so the
    /// unbiased pass@k estimator is computable.
    #[arg(long, default_value_t = 1)]
    pub n: usize,
    /// k for pass@k. Must satisfy k <= n.
    #[arg(long, default_value_t = 1)]
    pub k: usize,
    /// Sampling temperature. Use 0.0 with n=1 (greedy headline) or >=0.6 with n>=10.
    #[arg(long, default_value_t = 0.0)]
    pub temperature: f32,
    #[arg(long, default_value_t = 4096)]
    pub max_tokens: u64,
    /// Nucleus sampling mass; 1.0 disables truncation. Always sent so the recorded value is
    /// the requested value.
    #[arg(long, default_value_t = 1.0)]
    pub top_p: f32,
    /// Base sampling seed; each sample's seed is derived from (seed, fixture id, sample
    /// index) and recorded.
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
    /// Score pre-generated `<fixture-id>.vox` solutions from this directory instead of generating.
    #[arg(long)]
    pub from_dir: Option<PathBuf>,
    /// Corpus root.
    #[arg(long, default_value = "contracts/eval/humaneval-vox")]
    pub corpus: PathBuf,
    /// Score only fixtures added strictly after this ISO date (a model's
    /// training cutoff — prefer the OpenRouter catalog's `knowledge_cutoff`
    /// over a hand-typed date).
    #[arg(long)]
    pub cutoff: Option<String>,
    /// Include training-eligible fixtures. Off by default: only held-out
    /// fixtures support an external claim.
    #[arg(long, default_value_t = false)]
    pub include_training_eligible: bool,
    /// Path to the `vox` binary. Defaults to `target/release/vox[.exe]`, then
    /// `target/debug/vox[.exe]`.
    #[arg(long)]
    pub vox_bin: Option<PathBuf>,
    /// Per-subprocess timeout in seconds.
    #[arg(long, default_value_t = 60)]
    pub timeout_secs: u64,
    /// Abort the sweep once spend exceeds this; the run is marked incomplete.
    #[arg(long)]
    pub max_spend_usd: Option<f64>,
    /// Write the run report JSON here.
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Skip the `model_scoreboard` write-back.
    #[arg(long, default_value_t = false)]
    pub no_write_back: bool,
    /// Compiler-feedback repair turns per failed sample (0-3). pass@1/pass@k always use the
    /// original attempt; repaired rates are reported per turn.
    #[arg(long, default_value_t = 0)]
    pub feedback_turns: u32,
    /// Where the run executed: `github-hosted`, or a named local host. Defaults to
    /// `github-hosted` when `GITHUB_ACTIONS=true`; otherwise unset, which leaves the run
    /// incomplete with `runner` named as missing.
    #[arg(long)]
    pub runner: Option<String>,
}

/// Resolve the `(model_id, harness_id)` pair a leaderboard row is keyed by.
///
/// A local checkpoint label is folded into the model id as `<model>@<checkpoint>`
/// so successive MENS builds are separate rows rather than silently overwriting
/// one another. External models pass `None` and keep their bare registry id.
/// MENS thereby enters the leaderboard as a registry id like any other model,
/// scored by identical code on identical axes — no special case anywhere here.
#[must_use]
pub fn mens_row_identity(model: &str, harness: &str, checkpoint: Option<&str>) -> (String, String) {
    match checkpoint {
        Some(c) => (format!("{model}@{c}"), harness.to_string()),
        None => (model.to_string(), harness.to_string()),
    }
}

/// Every knob that can change a score; the input of [`config_digest`].
#[derive(Debug, Clone)]
pub struct ConfigKnobs<'a> {
    pub model_id: &'a str,
    pub harness: &'a str,
    pub n: usize,
    pub k: usize,
    pub condition: &'a str,
    pub context_hash: &'a str,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u64,
    pub seed: u64,
    pub problem_set_hash: &'a str,
    pub feedback_turns: u32,
}

/// BLAKE3 row identity (16 hex chars) over every knob that can change a score.
///
/// An earlier design hashed with `DefaultHasher`, which Rust does not guarantee stable
/// across releases; hashing goes through `vox-crypto` per the crypto policy.
#[must_use]
pub fn config_digest(c: &ConfigKnobs<'_>) -> String {
    let joined = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        c.model_id,
        c.harness,
        c.n,
        c.k,
        c.condition,
        c.context_hash,
        c.temperature,
        c.top_p,
        c.max_tokens,
        c.seed,
        c.problem_set_hash,
        c.feedback_turns
    );
    hex_encode(&secure_hash(joined.as_bytes())[..8])
}

/// SHA-256 (per D-11, so it can be checked outside Vox) over the problem content.
///
/// For each fixture sorted by id: the id, then the raw bytes of `spec.toml`,
/// `reference.vox` and `tests.vox`, each as a u64-LE length followed by the bytes. Raw
/// bytes: no UTF-8 decoding and no newline normalization. Selection order is irrelevant;
/// a repeated id is an error.
pub fn problem_set_hash(fixtures: &[Fixture]) -> Result<String> {
    let mut refs: Vec<&Fixture> = fixtures.iter().collect();
    refs.sort_by(|a, b| a.id.cmp(&b.id));
    if let Some(w) = refs.windows(2).find(|w| w[0].id == w[1].id) {
        anyhow::bail!(
            "duplicate fixture id `{}` in the selected problem set",
            w[0].id
        );
    }
    let mut buf = Vec::new();
    let mut put = |bytes: &[u8]| {
        buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(bytes);
    };
    for f in refs {
        put(f.id.as_bytes());
        for path in [&f.spec_path, &f.reference_path, &f.tests_path] {
            let bytes =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            put(&bytes);
        }
    }
    Ok(hex_encode(&interop_sha256(&buf)))
}

/// BLAKE3 of the prompt template with placeholder context, signature and task, so the
/// hash changes exactly when the template text changes.
#[must_use]
pub fn prompt_template_hash() -> String {
    let ctx = PromptContext {
        condition: Condition::C0ZeroShot,
        context_text: "{context}".to_string(),
        context_hash: String::new(),
    };
    let rendered = build_prompt(&ctx, "{signature}", "{task}");
    hex_encode(&secure_hash(rendered.as_bytes()))
}

/// BLAKE3 over the condition id, a 0x00 separator and the context text.
// vox-corpus keeps its own FNV `hash_context` (no vox-crypto edge); the record uses this one.
#[must_use]
pub fn context_hash_blake3(condition_id: &str, context_text: &str) -> String {
    let mut buf = Vec::with_capacity(condition_id.len() + 1 + context_text.len());
    buf.extend_from_slice(condition_id.as_bytes());
    buf.push(0);
    buf.extend_from_slice(context_text.as_bytes());
    hex_encode(&secure_hash(&buf))
}

/// Split `vox --version` output (`vox 0.6.0+build.12 (abc1234)`) into the version string
/// (same shape as `VOX_VERSION`) and the git sha in parentheses.
#[must_use]
pub fn parse_vox_version_line(line: &str) -> Option<(String, String)> {
    let (_program, version) = line.trim().split_once(' ')?;
    let version = version.trim();
    let open = version.rfind('(')?;
    let sha = version[open + 1..].strip_suffix(')')?.trim();
    (!sha.is_empty()).then(|| (version.to_string(), sha.to_string()))
}

/// Which exact `vox` binary verified the candidates: version, git sha, and a SHA-256 of
/// its bytes. Any failure is an error; the caller records the identity as missing.
fn verifier_identity(vox_bin: &Path) -> Result<VerifierIdentity> {
    let out = std::process::Command::new(vox_bin)
        .arg("--version")
        .output()
        .with_context(|| format!("running {} --version", vox_bin.display()))?;
    anyhow::ensure!(
        out.status.success(),
        "`{} --version` failed",
        vox_bin.display()
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout.lines().next().unwrap_or_default().trim().to_string();
    let parsed = parse_vox_version_line(&line);
    let bytes = std::fs::read(vox_bin).with_context(|| format!("reading {}", vox_bin.display()))?;
    Ok(VerifierIdentity {
        version_line: parsed.as_ref().map_or(line, |(v, _)| v.clone()),
        git_sha: parsed.map(|(_, sha)| sha),
        binary_sha256: hex_encode(&interop_sha256(&bytes)),
    })
}

/// Lineage of a `mens/<run>` checkpoint from its training card, or the reasons the label
/// is refused. Non-`mens/` ids need no lineage: `(None, [])`.
///
/// Wraps `adapter_card::check_for_eval`: a missing or invalid card, or an adapter whose
/// SHA-256 differs from the card, yields `None`, so the completeness gate marks the run
/// incomplete through the ordinary missing-field path. Syntax drift is reported, not hidden.
#[must_use]
pub fn mens_lineage(
    repo_root: &Path,
    model_id: &str,
    fingerprint: &str,
    kind: &str,
) -> (Option<MensLineage>, Vec<String>) {
    let Some(run) = model_id.strip_prefix("mens/") else {
        return (None, Vec::new());
    };
    let run_dir = repo_root.join("mens").join("runs").join(run);
    let check = check_for_eval(&run_dir, fingerprint, kind);
    if !check.problems.is_empty() {
        return (None, check.problems);
    }
    let Some(card) = check.card else {
        return (None, vec!["no adapter card".to_string()]);
    };
    let base = if card.base_hf_id.contains('@') {
        card.base_hf_id.clone()
    } else {
        format!("{}@{}", card.base_hf_id, card.base_revision)
    };
    let lineage = MensLineage {
        base_hf_id_at_revision: Some(base),
        adapter_sha256: Some(card.adapter_sha256),
        training_manifest_digest: Some(card.training_manifest_digest),
        train_data_sha256: Some(card.train_data_sha256),
        training_syntax_fingerprint: Some(card.training_syntax_fingerprint),
        training_syntax_fingerprint_kind: Some(card.syntax_fingerprint_kind),
        syntax_drift: check.syntax_drift,
    };
    (Some(lineage), Vec::new())
}

/// Only a complete, generated run may reach the `model_scoreboard` routing reads; an
/// incomplete or ingested (`--from-dir`) run never does.
#[must_use]
pub fn should_write_back(run_complete: bool, measured: bool, no_write_back: bool) -> bool {
    run_complete && measured && !no_write_back
}

/// The manifest's own hand-maintained `corpus.corpus_hash`; informational only.
fn corpus_manifest_hash(corpus_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(corpus_root.join("manifest.v1.yaml")).ok()?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
    doc.get("corpus")?
        .get("corpus_hash")?
        .as_str()
        .map(str::to_string)
}

/// How a generation error must be accounted for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenErrorKind {
    /// Provider-side failure; never scored as a model failure.
    Infra,
    /// The server is not serving what was asked for; the whole run is invalid.
    FatalConfig,
    /// The model produced no usable output; scored as a miss.
    Model,
}

/// Classify a facade error string. The facade prefixes rate limits and context overflow
/// (`RATE_LIMITED: ` / `CONTEXT_LENGTH_EXCEEDED: `); other provider failures arrive as
/// `LLM API returned error (<code>): ...` or `HTTP request failed: ...`.
#[must_use]
pub fn classify_generation_error(msg: &str) -> GenErrorKind {
    use vox_actor_runtime::llm::{CONTEXT_EXCEEDED_PREFIX, RATE_LIMITED_PREFIX};
    // `vox mens serve` answers 409 "Loaded model is X, not Y" when asked for another model.
    if msg.contains("Loaded model is") {
        return GenErrorKind::FatalConfig;
    }
    if msg.starts_with(RATE_LIMITED_PREFIX) || msg.starts_with(CONTEXT_EXCEEDED_PREFIX) {
        return GenErrorKind::Infra;
    }
    let m = msg.to_ascii_lowercase();
    const INFRA_MARKERS: &[&str] = &[
        "rate limited",
        "429",
        "context length",
        "http error:",
        "http request failed:",
        "provider status 500",
        "provider status 502",
        "provider status 503",
        "provider status 504",
        "returned error (500)",
        "returned error (502)",
        "returned error (503)",
        "returned error (504)",
        "timeout",
        "timed out",
        "activity cancelled",
    ];
    if INFRA_MARKERS.iter().any(|k| m.contains(k)) {
        GenErrorKind::Infra
    } else {
        GenErrorKind::Model
    }
}

/// Per-sample seed from `(base, fixture id, sample index)`: BLAKE3 over unambiguous framing
/// (fixed-width base, length-prefixed id, fixed-width index), masked to 31 bits so providers
/// that parse seeds as i32/u32/JS numbers accept it.
// vox:defactored-from vox-ml-cli 2026-10-11
#[must_use]
pub fn sample_seed(base: u64, problem_id: &str, sample_index: usize) -> u64 {
    let mut buf = Vec::with_capacity(32 + problem_id.len());
    buf.extend_from_slice(&base.to_le_bytes());
    buf.extend_from_slice(&(problem_id.len() as u64).to_le_bytes());
    buf.extend_from_slice(problem_id.as_bytes());
    buf.extend_from_slice(&(sample_index as u64).to_le_bytes());
    let digest = vox_crypto::facades::secure_hash(&buf);
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(head) & 0x7fff_ffff
}

/// Reject sampling settings that cannot produce a meaningful score, before any request.
pub fn validate_sampling(args: &EvalCorpusArgs) -> Result<()> {
    anyhow::ensure!(args.n >= 1, "--n must be at least 1");
    anyhow::ensure!(
        (1..=args.n).contains(&args.k),
        "--k ({}) must satisfy 1 <= k <= --n ({}) — pass@k needs at least k samples",
        args.k,
        args.n
    );
    anyhow::ensure!(
        args.top_p.is_finite() && args.top_p > 0.0 && args.top_p <= 1.0,
        "--top-p ({}) must be in (0, 1]",
        args.top_p
    );
    anyhow::ensure!(
        args.temperature.is_finite() && args.temperature >= 0.0,
        "--temperature ({}) must be a finite, non-negative number",
        args.temperature
    );
    anyhow::ensure!(
        args.feedback_turns <= MAX_FEEDBACK_TURNS,
        "--feedback-turns ({}) must be at most {MAX_FEEDBACK_TURNS}",
        args.feedback_turns
    );
    Ok(())
}

/// Upper bound on `--feedback-turns`: each turn multiplies requests (design section 8 asks for 1 and 3).
pub const MAX_FEEDBACK_TURNS: u32 = 3;

/// Follow-up user message that hands the verifier's one-line diagnostic back to the model.
#[must_use]
pub fn feedback_prompt(detail: &str) -> String {
    format!(
        "The Vox verifier rejected that solution:\n{detail}\n\nReply with ONLY the corrected Vox source for the same function."
    )
}

/// Seed of repair turn `turn` (1-based) for one sample: the ordinary `sample_seed` over a
/// per-turn problem id, so turns and samples never share a seed.
#[must_use]
pub fn repair_seed(base: u64, fixture_id: &str, turn: u32, sample_index: usize) -> u64 {
    sample_seed(base, &format!("{fixture_id}#turn{turn}"), sample_index)
}

/// Pull Vox source from a completion, unwrapping a fenced block when present.
#[must_use]
pub fn extract_vox_code(completion: &str) -> String {
    let Some(open) = completion.find("```") else {
        return completion.trim().to_string();
    };
    let after = &completion[open + 3..];
    let body = &after[after.find('\n').map_or(0, |i| i + 1)..];
    match body.find("```") {
        Some(c) => body[..c].trim().to_string(),
        None => body.trim().to_string(),
    }
}

/// Load `<id>[-slug].vox` solutions, erroring on ambiguity rather than
/// silently picking one by directory order.
pub fn load_solution_dir(dir: &Path) -> Result<HashMap<String, String>> {
    let mut out: HashMap<String, String> = HashMap::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("vox") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let id = stem.split('-').next().unwrap_or(&stem).to_string();
        if out.contains_key(&id) {
            anyhow::bail!(
                "ambiguous fixture id `{id}` in {} — two files map to the same fixture",
                dir.display()
            );
        }
        out.insert(id, std::fs::read_to_string(&path)?);
    }
    Ok(out)
}

/// Resolve the `vox` binary to verify with: an explicit path, else the release
/// build, else the dev build.
///
/// The dev fallback is deliberate. A release build of this workspace needs
/// multi-GB of RAM to borrow-check at `opt-level=3` with LTO, and on a
/// contended machine rustc can die with an allocation failure
/// (`STATUS_STACK_BUFFER_OVERRUN` / `0xc0000409`) that looks nothing like an
/// out-of-memory error. Verification only needs `vox check`/`vox run` to be
/// correct, not fast — published latency comes from the generation call, not
/// this subprocess, so the profile cannot bias a metric.
pub fn resolve_vox_binary(explicit: Option<&Path>) -> Result<PathBuf> {
    let name = if cfg!(windows) { "vox.exe" } else { "vox" };
    if let Some(p) = explicit {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        anyhow::bail!(
            "vox binary not found at {} — build it first: cargo build -p vox-cli --release \
             (add `-j 2` if rustc dies with an allocation failure on a contended machine)",
            p.display()
        );
    }
    for profile in ["release", "debug"] {
        let candidate = PathBuf::from("target").join(profile).join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "no vox binary at target/release/{name} or target/debug/{name} — build one first: \
         cargo build -p vox-cli --release (add `-j 2` if rustc dies with an allocation \
         failure on a contended machine)"
    )
}

/// Select the fixtures this run will score, applying the held-out and
/// rolling-window filters.
fn select_fixtures(all: &[Fixture], args: &EvalCorpusArgs) -> Vec<Fixture> {
    let base: Vec<Fixture> = if args.include_training_eligible {
        all.to_vec()
    } else {
        held_out(all).into_iter().cloned().collect()
    };
    let mut selected: Vec<Fixture> = match &args.cutoff {
        Some(cutoff) => eligible_after(&base, cutoff).into_iter().cloned().collect(),
        None => base,
    };
    // Fixed iteration order: identical seeds then give identical sample lists.
    selected.sort_by(|a, b| a.id.cmp(&b.id));
    selected
}

/// Register every listable `<repo_root>/mens/runs/*` checkpoint on `base`, so a freshly
/// trained `mens/<run>` resolves like any other registry id.
///
/// Production passes `ModelRegistry::from_cache()`; tests pass `ModelRegistry::default()`.
pub async fn with_mens_runs(mut base: ModelRegistry, repo_root: &Path) -> Result<ModelRegistry> {
    for spec in MensCatalog::new(repo_root).refresh().await? {
        base.register(spec);
    }
    Ok(base)
}

/// Resolve a `--model` id to its spec and a dispatchable facade config, the same
/// `resolve_target` + `llm_config_for_spec` pair `vox models bench` uses.
///
/// An unknown `mens/*` id is a hard error: `resolve_target` would otherwise synthesize an
/// OpenRouter spec for it, and a score for a local checkpoint must never come from (or
/// leak prompts to) a cloud model.
pub fn resolve_eval_model(registry: &ModelRegistry, raw: &str) -> Result<(ModelSpec, LlmConfig)> {
    let (spec, synthesized) = resolve_target(registry, raw);
    if synthesized && raw.trim().starts_with("mens/") {
        anyhow::bail!(
            "`{}` was not found under mens/runs (no listable checkpoint with that name); \
             it will not be routed to a cloud provider. Train it first or check the name.",
            raw.trim()
        );
    }
    let config = llm_config_for_spec(&spec, TaskCategory::CodeGen);
    Ok((spec, config))
}

/// One generated candidate plus the measurements recorded with it.
#[derive(Debug, Clone)]
pub struct Generated {
    pub code: String,
    /// The completion exactly as the model wrote it (sent back verbatim in repair turns).
    pub raw: String,
    pub total_tokens: u32,
    pub latency_ms: i64,
    pub cost_usd: Option<f64>,
    /// Model string the provider reported (provenance: what actually answered).
    pub provider_model: String,
}

/// Generate one candidate solution through the model-agnostic LLM facade.
///
/// `config` already carries model, temperature, top_p, max_tokens and seed; they are not
/// overridden here. All LLM traffic goes through `vox_actor_runtime::llm` per the
/// workspace's model-agnostic boundary — never a vendor SDK or hostname.
async fn generate_candidate(config: &LlmConfig, prompt: &str) -> Result<Generated, String> {
    generate_chat(config, vec![chat_message("user", prompt)]).await
}

fn chat_message(role: &str, content: &str) -> vox_actor_runtime::llm::LlmChatMessage {
    vox_actor_runtime::llm::LlmChatMessage {
        role: role.to_string(),
        content: content.to_string(),
        ..Default::default()
    }
}

/// The one LLM call path for first attempts and repair turns alike.
async fn generate_chat(
    config: &LlmConfig,
    messages: Vec<vox_actor_runtime::llm::LlmChatMessage>,
) -> Result<Generated, String> {
    use vox_actor_runtime::ActivityOptions;
    use vox_actor_runtime::llm::llm_chat;

    let mut config = config.clone();
    config.telemetry_task_category = Some("eval-corpus".to_string());

    let started = std::time::Instant::now();
    let outcome = llm_chat(&ActivityOptions::new(), messages, config).await;
    let latency_ms = started.elapsed().as_millis() as i64;

    let response = match outcome {
        vox_actor_runtime::ActivityResult::Ok(inner) => inner,
        vox_actor_runtime::ActivityResult::Failed(e) => return Err(e.to_string()),
        vox_actor_runtime::ActivityResult::Cancelled => return Err("activity cancelled".into()),
    }?;

    Ok(Generated {
        code: extract_vox_code(&response.content),
        raw: response.content.clone(),
        total_tokens: response.prompt_tokens + response.completion_tokens,
        latency_ms,
        cost_usd: response.cost_usd,
        provider_model: response.model,
    })
}

/// Where candidate solutions come from.
pub enum SampleSource<'a> {
    /// Generate through the facade with `config` (one request per sample).
    Live {
        config: &'a LlmConfig,
        ctx: &'a PromptContext,
    },
    /// Pre-generated solutions keyed by fixture id; one attempt per fixture.
    FromDir(&'a HashMap<String, String>),
}

/// How many samples to draw and under which budget.
#[derive(Debug, Clone, Copy)]
pub struct SamplePlan {
    pub n: usize,
    pub base_seed: u64,
    pub max_spend_usd: Option<f64>,
    /// Repair turns per failed live sample (0 = none).
    pub feedback_turns: u32,
}

/// Everything one sweep produced.
#[derive(Debug, Clone)]
pub struct DrawOutcome {
    pub outcomes: Vec<FixtureOutcome>,
    pub samples: Vec<SampleRecord>,
    pub n_infra_errors: usize,
    pub n_samples_planned: usize,
    pub spend_cap_hit: bool,
}

fn missed_attempt() -> AttemptOutcome {
    AttemptOutcome {
        compiled: false,
        tests_passed: false,
        cheated: false,
        total_tokens: 0,
        latency_ms: 0,
        cost_usd: None,
    }
}

fn classify_outcome(code: &str, outcome: &VerifyOutcome) -> FailureClass {
    let parse_ok = vox_compiler::parser::parse(vox_compiler::lexer::lex(code)).is_ok();
    classify_failure(&VerifyFacts {
        compiled: outcome.compiled,
        tests_passed: outcome.tests_passed,
        cheated: outcome.cheated,
        timed_out: outcome.detail.contains("timed out"),
        parse_ok,
        assertion_failed: outcome.assertion_failed,
    })
}

/// Failure classes whose diagnostic can guide a repair (not cheats, timeouts or infra).
fn is_repairable(class: FailureClass) -> bool {
    matches!(
        class,
        FailureClass::ParseError
            | FailureClass::CheckError
            | FailureClass::RuntimeError
            | FailureClass::WrongAnswer
    )
}

/// Draw `plan.n` samples for every fixture (one per fixture for `FromDir`).
///
/// Infra errors never enter `attempts`; model failures and missing solutions are
/// all-false attempts; a served-model mismatch aborts the run. Every sample, including
/// infra and generation failures, yields one `SampleRecord`, ordered by
/// `(fixture_id, sample_index)`.
pub async fn draw_samples(
    fixtures: &[Fixture],
    source: SampleSource<'_>,
    plan: SamplePlan,
    verify: &mut dyn FnMut(&str, &Fixture) -> Result<VerifyOutcome>,
) -> Result<DrawOutcome> {
    let per_fixture = match source {
        SampleSource::Live { .. } => plan.n,
        SampleSource::FromDir(_) => 1,
    };
    let mut out = DrawOutcome {
        outcomes: Vec::with_capacity(fixtures.len()),
        samples: Vec::with_capacity(fixtures.len() * per_fixture),
        n_infra_errors: 0,
        n_samples_planned: fixtures.len() * per_fixture,
        spend_cap_hit: false,
    };
    let mut cumulative_spend = 0.0f64;

    'fixtures: for fixture in fixtures {
        let mut attempts = Vec::with_capacity(per_fixture);

        for i in 0..per_fixture {
            if let Some(cap) = plan.max_spend_usd {
                if cumulative_spend >= cap {
                    out.spend_cap_hit = true;
                    break 'fixtures;
                }
            }

            // Pre-generated solutions are not sampled, so they carry no seed.
            let seed = match source {
                SampleSource::Live { .. } => sample_seed(plan.base_seed, &fixture.id, i),
                SampleSource::FromDir(_) => 0,
            };
            let mut record = SampleRecord {
                fixture_id: fixture.id.clone(),
                sample_index: i,
                seed,
                code: None,
                failure_class: FailureClass::GenerationError,
                compiled: false,
                tests_passed: false,
                cheated: false,
                total_tokens: 0,
                latency_ms: 0,
                cost_usd: None,
                provider_model: None,
                passed_by_turn: Vec::new(),
                repair_tokens: 0,
                repair_cost_usd: None,
            };

            let prompt = match &source {
                SampleSource::Live { ctx, .. } => {
                    build_prompt(ctx, &fixture.signature, &fixture.prompt)
                }
                SampleSource::FromDir(_) => String::new(),
            };
            let generated: Option<Generated> = match &source {
                SampleSource::FromDir(map) => map.get(&fixture.id).map(|src| Generated {
                    code: extract_vox_code(src),
                    raw: String::new(),
                    total_tokens: 0,
                    latency_ms: 0,
                    cost_usd: None,
                    provider_model: String::new(),
                }),
                SampleSource::Live { config, ctx } => {
                    let mut config = (*config).clone();
                    config.seed = Some(seed);
                    match generate_candidate(&config, &prompt).await {
                        Ok(g) => Some(g),
                        Err(reason) => match classify_generation_error(&reason) {
                            GenErrorKind::FatalConfig => anyhow::bail!(
                                "{reason} — the server is serving a different model than requested"
                            ),
                            GenErrorKind::Infra => {
                                out.n_infra_errors += 1;
                                tracing::warn!(model = %config.model, fixture = %fixture.id, error = %reason, "infra error, not counted as a model failure");
                                record.failure_class = FailureClass::InfraError;
                                out.samples.push(record);
                                continue;
                            }
                            GenErrorKind::Model => {
                                tracing::warn!(model = %config.model, fixture = %fixture.id, error = %reason, "generation failed");
                                None
                            }
                        },
                    }
                }
            };

            // A missing solution or a model that produced nothing is a miss, not a skip:
            // skipping would shrink the denominator and inflate the rate.
            let Some(g) = generated else {
                attempts.push(missed_attempt());
                out.samples.push(record);
                continue;
            };

            if let Some(c) = g.cost_usd {
                cumulative_spend += c;
            }

            let outcome = verify(&g.code, fixture)?;
            record.failure_class = classify_outcome(&g.code, &outcome);
            record.compiled = outcome.compiled;
            record.tests_passed = outcome.tests_passed;
            record.cheated = outcome.cheated;
            record.total_tokens = g.total_tokens;
            record.latency_ms = g.latency_ms;
            record.cost_usd = g.cost_usd;
            record.provider_model = (!g.provider_model.is_empty()).then_some(g.provider_model);
            record.code = Some(g.code);

            // Compiler-feedback repair turns. They only fill `passed_by_turn` and the repair_*
            // fields: the AttemptOutcome pushed below is always the original attempt, so
            // pass@1 and pass@k can never be inflated by a repair.
            let mut trajectory = vec![record.failure_class == FailureClass::Pass];
            if let SampleSource::Live { config, .. } = &source {
                if is_repairable(record.failure_class) {
                    let (mut prev_raw, mut prev_detail) = (g.raw.clone(), outcome.detail.clone());
                    for turn in 1..=plan.feedback_turns {
                        if plan
                            .max_spend_usd
                            .is_some_and(|cap| cumulative_spend >= cap)
                        {
                            out.spend_cap_hit = true;
                            break;
                        }
                        let messages = vec![
                            chat_message("user", &prompt),
                            chat_message("assistant", &prev_raw),
                            chat_message("user", &feedback_prompt(&prev_detail)),
                        ];
                        let mut repair_cfg = (*config).clone();
                        repair_cfg.seed = Some(repair_seed(plan.base_seed, &fixture.id, turn, i));
                        let r = match generate_chat(&repair_cfg, messages).await {
                            Ok(r) => r,
                            Err(reason) => match classify_generation_error(&reason) {
                                GenErrorKind::FatalConfig => anyhow::bail!(
                                    "{reason} — the server is serving a different model than requested"
                                ),
                                GenErrorKind::Infra => {
                                    out.n_infra_errors += 1;
                                    tracing::warn!(fixture = %fixture.id, turn, error = %reason, "infra error in repair turn");
                                    break;
                                }
                                GenErrorKind::Model => {
                                    tracing::warn!(fixture = %fixture.id, turn, error = %reason, "repair generation failed");
                                    break;
                                }
                            },
                        };
                        record.repair_tokens += r.total_tokens;
                        if let Some(c) = r.cost_usd {
                            cumulative_spend += c;
                            *record.repair_cost_usd.get_or_insert(0.0) += c;
                        }
                        let repaired = verify(&r.code, fixture)?;
                        let class = classify_outcome(&r.code, &repaired);
                        trajectory.push(class == FailureClass::Pass);
                        if !is_repairable(class) {
                            break; // passed, cheated or timed out: this sample is done
                        }
                        (prev_raw, prev_detail) = (r.raw, repaired.detail);
                    }
                }
            }
            // Carry the last value forward: cumulative after a pass, flat after a stop.
            let last = trajectory.last().copied().unwrap_or(false);
            trajectory.resize(plan.feedback_turns as usize + 1, last);
            record.passed_by_turn = trajectory;
            out.samples.push(record);

            attempts.push(AttemptOutcome {
                compiled: outcome.compiled,
                tests_passed: outcome.tests_passed,
                cheated: outcome.cheated,
                total_tokens: g.total_tokens,
                latency_ms: g.latency_ms,
                cost_usd: g.cost_usd,
            });
            // Deliberately NO early break on a pass: the unbiased pass@k
            // estimator needs (n, c) — every attempt's outcome, not just the
            // first success.
        }

        let c = attempts.iter().filter(|a| a.tests_passed).count();
        out.outcomes.push(FixtureOutcome {
            fixture_id: fixture.id.clone(),
            n: attempts.len(),
            c,
            attempts,
        });
    }

    out.samples.sort_by(|a, b| {
        (a.fixture_id.as_str(), a.sample_index).cmp(&(b.fixture_id.as_str(), b.sample_index))
    });
    Ok(out)
}

/// Build the scoreboard row recording a corpus run.
///
/// `quality_score` is the measured pass@1 — replacing the router's
/// `log10(max_tokens)` heuristic with a real number under the `vox-codegen`
/// task category, never overwriting the general-purpose row `vox model eval`
/// maintains (that lives under `task_category = "general"`).
#[must_use]
pub fn scoreboard_row_from_corpus(
    model_id: &str,
    harness_id: &str,
    score: &CorpusScore,
) -> vox_db::store::types::ModelScoreboardRow {
    vox_db::store::types::ModelScoreboardRow {
        model_id: model_id.to_string(),
        task_category: "vox-codegen".to_string(),
        strength_tag: harness_id.to_string(),
        window_days: 7,
        n_calls: score.n_fixtures as i64,
        success_rate: score.pass_at_1,
        p50_latency_ms: score.p50_ms,
        p99_latency_ms: None,
        cost_per_success_usd: score.cost_per_success_usd,
        quality_score: score.pass_at_1,
        updated_at_ms: vox_db::now_unix_ms() as i64,
        success_count: (score.pass_at_1 * score.n_fixtures as f64).round() as i64,
        cumulative_cost_usd: score.cumulative_cost_usd.unwrap_or(0.0),
        // The corpus harness measures pass@1, p50 latency and cost; it never observes
        // streaming timings. `None` is "not measured" — the honest value, and the one the
        // scoreboard renders as an empty cell rather than as a confident `0.0`. (These three
        // are display-only columns; the Pareto axes are reliability, cost and p50 latency.)
        p95_ttft_ms: None,
        p95_tpot_ms: None,
        goodput_tokens_per_sec: None,
    }
}

/// Identity of the `vox` binary that verified the candidates.
#[derive(Debug, Clone)]
pub struct VerifierIdentity {
    /// Version string as `vox --version` printed it, minus the program name
    /// (same shape as `crate::VOX_VERSION`).
    pub version_line: String,
    /// Git sha parsed from the version string.
    pub git_sha: Option<String>,
    /// SHA-256 of the binary's bytes.
    pub binary_sha256: String,
}

/// Everything `collect_provenance` needs that the sweep itself does not carry.
pub struct ProvenanceInputs<'a> {
    pub args: &'a EvalCorpusArgs,
    /// Row model id (`model` or `model@checkpoint`).
    pub row_model_id: String,
    /// Resolved provider; `None` for `--from-dir` rows (nothing generated them here).
    pub provider: Option<String>,
    /// UTC date at run start.
    pub request_date: String,
    pub samples: &'a [SampleRecord],
    pub context_hash: Option<String>,
    pub problem_set_hash: Option<String>,
    pub prompt_template_hash: Option<String>,
    pub corpus_manifest_hash: Option<String>,
    /// `(fingerprint, kind)` of the syntax surface under test.
    pub fingerprint: Option<(String, String)>,
    pub verifier: Option<VerifierIdentity>,
    pub mens: Option<MensLineage>,
    pub runner: Option<String>,
}

/// The `--runner` rule: an explicit label wins; otherwise `github-hosted` under Actions;
/// otherwise unset (the run is then incomplete with `runner` named missing).
#[must_use]
pub fn resolve_runner(flag: Option<&str>, github_actions: Option<&str>) -> Option<String> {
    match flag.map(str::trim).filter(|s| !s.is_empty()) {
        Some(r) => Some(r.to_string()),
        None => (github_actions == Some("true")).then(|| "github-hosted".to_string()),
    }
}

/// `<corpus dir name>:held-out|all[:after-<cutoff>]`.
#[must_use]
pub fn tier_label(args: &EvalCorpusArgs) -> String {
    let dir = args
        .corpus
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("corpus");
    let scope = if args.include_training_eligible {
        "all"
    } else {
        "held-out"
    };
    match &args.cutoff {
        Some(c) => format!("{dir}:{scope}:after-{c}"),
        None => format!("{dir}:{scope}"),
    }
}

/// Fill the §5 record with everything the harness observed. Fields it could not observe
/// stay `None`; `EvalCorpusReport::assemble` names them and marks the run incomplete.
#[must_use]
pub fn collect_provenance(inp: &ProvenanceInputs<'_>) -> RunProvenance {
    let a = inp.args;
    let measured = a.from_dir.is_none();
    let (n, k) = if measured { (a.n, a.k) } else { (1, 1) };
    let mut provider_reported_models: Vec<String> = inp
        .samples
        .iter()
        .filter_map(|s| s.provider_model.clone())
        .collect();
    provider_reported_models.sort();
    provider_reported_models.dedup();
    let v = inp.verifier.as_ref();
    RunProvenance {
        syntax_fingerprint: inp.fingerprint.as_ref().map(|f| f.0.clone()),
        syntax_fingerprint_kind: inp.fingerprint.as_ref().map(|f| f.1.clone()),
        cargo_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        vox_git_sha: v.and_then(|v| v.git_sha.clone()),
        vox_binary_sha256: v.map(|v| v.binary_sha256.clone()),
        verifier_version: v.map(|v| v.version_line.clone()),
        verifier_matches_harness: v.map(|v| v.version_line == crate::VOX_VERSION),
        tier: Some(tier_label(a)),
        problem_set_hash: inp.problem_set_hash.clone(),
        corpus_manifest_hash: inp.corpus_manifest_hash.clone(),
        prompt_template_hash: inp.prompt_template_hash.clone(),
        condition: Some(a.condition.clone()),
        context_hash: inp.context_hash.clone(),
        harness_version: Some(crate::VOX_VERSION.to_string()),
        model_id: Some(inp.row_model_id.clone()),
        provider: inp.provider.clone(),
        request_date: Some(inp.request_date.clone()),
        provider_reported_models,
        mens: inp.mens.clone(),
        sampling: Some(SamplingRecord {
            temperature: a.temperature,
            top_p: a.top_p,
            n,
            k,
            seed: if measured { a.seed } else { 0 },
            max_tokens: a.max_tokens,
            seed_enforced: inp.provider.as_deref() == Some("voxlocal"),
            feedback_turns: if measured { a.feedback_turns } else { 0 },
        }),
        runner: inp.runner.clone(),
        os: Some(std::env::consts::OS.to_string()),
        arch: Some(std::env::consts::ARCH.to_string()),
        verifier_timeout_secs: Some(a.timeout_secs),
        // Supplied by Phase 7 run identity and preregistration (12-09); never filled in here.
        identity: None,
        preregistration_id: None,
    }
}

/// Report-level facts that are not provenance.
#[derive(Debug, Clone)]
pub struct ReportMeta {
    pub model_id: String,
    pub harness_id: String,
    pub config_digest: String,
    pub cutoff: Option<String>,
    pub held_out_only: bool,
    pub measured: bool,
    /// Effective k (1 for ingested solutions).
    pub k: usize,
    /// Bootstrap seed; distinct in role from the sampling seed.
    pub analysis_seed: u64,
}

/// Hand the sweep and its provenance to the single completeness gate.
#[must_use]
pub fn finalize_report(
    draw: DrawOutcome,
    prov: RunProvenance,
    meta: &ReportMeta,
) -> EvalCorpusReport {
    EvalCorpusReport::assemble(ReportInputs {
        model_id: meta.model_id.clone(),
        harness_id: meta.harness_id.clone(),
        config_digest: meta.config_digest.clone(),
        cutoff: meta.cutoff.clone(),
        held_out_only: meta.held_out_only,
        measured: meta.measured,
        k: meta.k,
        outcomes: draw.outcomes,
        samples: draw.samples,
        n_infra_errors: draw.n_infra_errors,
        n_samples_planned: draw.n_samples_planned,
        max_infra_rate: DEFAULT_MAX_INFRA_RATE,
        spend_cap_hit: draw.spend_cap_hit,
        provenance: prov,
        bootstrap_reps: DEFAULT_BOOTSTRAP_REPS,
        analysis_seed: meta.analysis_seed,
    })
}

fn fmt_arm(a: Option<&vox_eval::report::ArmStats>) -> String {
    a.map_or_else(
        || "n/a".to_string(),
        |a| {
            format!(
                "{:.1}% [{:.1}, {:.1}]",
                a.point * 100.0,
                a.ci_low * 100.0,
                a.ci_high * 100.0
            )
        },
    )
}

fn print_summary(report: &EvalCorpusReport) {
    println!(
        "pass@1 {} | pass@{} {} | compile {:.1}% | cheated {} | infra-errors {}",
        fmt_arm(report.pass_at_1.as_ref()),
        report.score.k,
        fmt_arm(report.pass_at_k.as_ref()),
        report.score.compile_rate * 100.0,
        report.score.n_cheated,
        report.n_infra_errors,
    );
    if let Some(s) = &report.provenance.sampling {
        // A requested seed is only a guarantee where the harness controls the provider.
        println!(
            "seed {} ({})",
            s.seed,
            if s.seed_enforced {
                "enforced"
            } else {
                "requested; provider may ignore"
            }
        );
    }
    if !report.run_complete {
        println!("{} run incomplete:", "INCOMPLETE".yellow().bold());
        for r in &report.incomplete_reasons {
            println!("  - {r}");
        }
    }
}

pub async fn run(args: EvalCorpusArgs) -> Result<()> {
    validate_sampling(&args)?;
    if args.temperature == 0.0 && args.n > 1 {
        eprintln!(
            "{} temperature=0.0 with n={} draws near-identical samples; pass@k will \
             collapse toward pass@1. Use --temperature >= 0.6 for a meaningful pass@k, \
             or --n 1 for a cheaper greedy headline.",
            "warning:".yellow(),
            args.n
        );
    }

    let request_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let repo_root = std::env::current_dir()?;
    let condition = Condition::parse(&args.condition)?;
    let ctx = build_context(condition, &repo_root)?;

    let all = load_corpus(&args.corpus)?;
    let fixtures = select_fixtures(&all, &args);
    anyhow::ensure!(
        !fixtures.is_empty(),
        "no fixtures selected — a --cutoff after the corpus's newest problem leaves \
         nothing scoreable, which is the honest result, not an error to paper over"
    );

    let (row_model_id, row_harness_id) =
        mens_row_identity(&args.model, &args.harness, args.checkpoint.as_deref());
    let psh = problem_set_hash(&fixtures)?;
    // Pre-generated solutions cannot be re-prompted.
    let feedback_turns = if args.from_dir.is_some() {
        0
    } else {
        args.feedback_turns
    };
    let context_hash = context_hash_blake3(condition.id(), &ctx.context_text);
    let digest = config_digest(&ConfigKnobs {
        model_id: &row_model_id,
        harness: &row_harness_id,
        n: args.n,
        k: args.k,
        condition: condition.id(),
        context_hash: &context_hash,
        temperature: args.temperature,
        top_p: args.top_p,
        max_tokens: args.max_tokens,
        seed: args.seed,
        problem_set_hash: &psh,
        feedback_turns,
    });

    let vox_bin = resolve_vox_binary(args.vox_bin.as_deref())?;
    let verifier = match verifier_identity(&vox_bin) {
        Ok(v) => {
            if v.version_line != crate::VOX_VERSION {
                eprintln!(
                    "{} verifier binary is `{}` but the harness is `{}`; the run will be incomplete",
                    "warning:".yellow(),
                    v.version_line,
                    crate::VOX_VERSION
                );
            }
            Some(v)
        }
        Err(e) => {
            eprintln!(
                "{} verifier identity unavailable: {e:#}",
                "warning:".yellow()
            );
            None
        }
    };
    let (fingerprint, fingerprint_kind) = vox_compiler::llm_prompt::grammar_fingerprint();
    let (mens, mens_problems) =
        mens_lineage(&repo_root, &args.model, &fingerprint, fingerprint_kind);
    if !mens_problems.is_empty() {
        eprintln!("{}", "mens label refused:".red().bold());
        for p in &mens_problems {
            eprintln!("  - {p}");
        }
    }
    if mens.as_ref().and_then(|m| m.syntax_drift) == Some(true) {
        eprintln!(
            "{} the adapter was trained against a different syntax than this harness evaluates",
            "syntax drift:".yellow().bold()
        );
    }
    let timeout = std::time::Duration::from_secs(args.timeout_secs);
    let workdir = std::env::temp_dir().join(format!("vox-eval-corpus-{}", std::process::id()));

    let solutions = match &args.from_dir {
        Some(dir) => Some(load_solution_dir(dir)?),
        None => None,
    };
    let measured = solutions.is_none();
    let live_config: Option<LlmConfig> = if measured {
        let registry = with_mens_runs(ModelRegistry::from_cache(), &repo_root).await?;
        let (_spec, mut config) = resolve_eval_model(&registry, &args.model)?;
        config.max_tokens = Some(args.max_tokens);
        config.temperature = Some(args.temperature);
        config.top_p = Some(args.top_p);
        if config.provider == "voxlocal"
            && std::env::var("VOX_LOCAL_ENDPOINT").map_or(true, |v| v.trim().is_empty())
        {
            eprintln!(
                "{} VOX_LOCAL_ENDPOINT is unset: voxlocal defaults to 127.0.0.1:11434 (Ollama), \
                 but `vox mens serve` listens on 11435. Export \
                 VOX_LOCAL_ENDPOINT=http://127.0.0.1:11435 or this run scores whatever answers \
                 on 11434.",
                "warning:".yellow()
            );
        }
        Some(config)
    } else {
        None
    };

    println!(
        "{} {} / {} [{}] — {} fixture(s), {} attempt(s) each",
        " EVAL-CORPUS ".on_blue().white().bold(),
        row_model_id,
        row_harness_id,
        condition.id(),
        fixtures.len(),
        if measured { args.n } else { 1 }
    );

    let plan = SamplePlan {
        n: args.n,
        base_seed: args.seed,
        max_spend_usd: args.max_spend_usd,
        feedback_turns,
    };
    let source = match (&solutions, &live_config) {
        (Some(map), _) => SampleSource::FromDir(map),
        (None, Some(config)) => SampleSource::Live { config, ctx: &ctx },
        (None, None) => unreachable!("live mode always resolves a config"),
    };
    let mut verify = |candidate: &str, fixture: &Fixture| -> Result<VerifyOutcome> {
        let tests_source = std::fs::read_to_string(&fixture.tests_path)
            .with_context(|| format!("reading {}", fixture.tests_path.display()))?;
        verify_program(&vox_bin, candidate, &tests_source, &workdir, timeout)
    };
    let drawn = draw_samples(&fixtures, source, plan, &mut verify).await?;

    let effective_k = if measured { args.k } else { 1 };
    let prov = collect_provenance(&ProvenanceInputs {
        args: &args,
        row_model_id: row_model_id.clone(),
        provider: live_config.as_ref().map(|c| c.provider.clone()),
        request_date,
        samples: &drawn.samples,
        context_hash: Some(context_hash),
        problem_set_hash: Some(psh),
        prompt_template_hash: Some(prompt_template_hash()),
        corpus_manifest_hash: corpus_manifest_hash(&args.corpus),
        fingerprint: Some((fingerprint, fingerprint_kind.to_string())),
        verifier,
        mens,
        runner: resolve_runner(
            args.runner.as_deref(),
            std::env::var("GITHUB_ACTIONS").ok().as_deref(),
        ),
    });
    let meta = ReportMeta {
        model_id: row_model_id.clone(),
        harness_id: row_harness_id.clone(),
        config_digest: digest,
        cutoff: args.cutoff.clone(),
        held_out_only: !args.include_training_eligible,
        measured,
        k: effective_k,
        analysis_seed: args.seed,
    };
    let report = finalize_report(drawn, prov, &meta);
    print_summary(&report);

    if should_write_back(report.run_complete, measured, args.no_write_back) {
        match vox_db::VoxDb::connect(
            vox_db::DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?,
        )
        .await
        {
            Ok(db) => {
                let row = scoreboard_row_from_corpus(&row_model_id, &row_harness_id, &report.score);
                match db.upsert_model_scoreboard(row).await {
                    Ok(()) => println!("Wrote measured quality to model_scoreboard (vox-codegen)."),
                    Err(e) => tracing::warn!(error = %e, "scoreboard upsert failed"),
                }
            }
            Err(e) => tracing::warn!(error = %e, "DB unavailable; skipping write-back"),
        }
    } else if !args.no_write_back {
        println!(
            "model_scoreboard not updated: {}",
            if report.run_complete {
                "ingested run (--from-dir)"
            } else {
                "run incomplete"
            }
        );
    }

    if let Some(path) = &args.output {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&report)?)?;
        println!("Wrote report to {}", path.display());
    }

    std::fs::remove_dir_all(&workdir).ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mens_checkpoints_are_distinct_leaderboard_rows() {
        let (id_a, _) = mens_row_identity("vox/mens", "vox-harness", Some("2026-09-01-a"));
        let (id_b, _) = mens_row_identity("vox/mens", "vox-harness", Some("2026-09-15-b"));
        assert_ne!(
            id_a, id_b,
            "different checkpoints -> different row identity"
        );
        assert!(
            id_a.contains("2026-09-01-a"),
            "checkpoint is visible in the row id"
        );
    }

    #[test]
    fn external_models_without_a_checkpoint_keep_their_bare_id() {
        let (id, harness) = mens_row_identity("moonshot/kimi-k2.6-thinking", "vox-harness", None);
        assert_eq!(
            id, "moonshot/kimi-k2.6-thinking",
            "no checkpoint -> unchanged id"
        );
        assert_eq!(harness, "vox-harness");
    }

    fn knobs() -> ConfigKnobs<'static> {
        ConfigKnobs {
            model_id: "acme/m",
            harness: "vox-harness",
            n: 10,
            k: 1,
            condition: "C1",
            context_hash: "ctxhash",
            temperature: 0.8,
            top_p: 0.95,
            max_tokens: 4096,
            seed: 7,
            problem_set_hash: "psh",
            feedback_turns: 0,
        }
    }

    #[test]
    fn config_digest_covers_every_knob() {
        let base = knobs();
        let a = config_digest(&base);
        assert_eq!(a, config_digest(&knobs()), "stable");
        assert_eq!(a.len(), 16);
        let variants: [(&str, ConfigKnobs<'static>); 12] = [
            (
                "model_id",
                ConfigKnobs {
                    model_id: "acme/other",
                    ..knobs()
                },
            ),
            (
                "harness",
                ConfigKnobs {
                    harness: "claude-code",
                    ..knobs()
                },
            ),
            ("n", ConfigKnobs { n: 11, ..knobs() }),
            ("k", ConfigKnobs { k: 2, ..knobs() }),
            (
                "condition",
                ConfigKnobs {
                    condition: "C0",
                    ..knobs()
                },
            ),
            (
                "context_hash",
                ConfigKnobs {
                    context_hash: "OTHER",
                    ..knobs()
                },
            ),
            (
                "temperature",
                ConfigKnobs {
                    temperature: 0.2,
                    ..knobs()
                },
            ),
            (
                "top_p",
                ConfigKnobs {
                    top_p: 0.5,
                    ..knobs()
                },
            ),
            (
                "max_tokens",
                ConfigKnobs {
                    max_tokens: 2048,
                    ..knobs()
                },
            ),
            ("seed", ConfigKnobs { seed: 8, ..knobs() }),
            (
                "problem_set_hash",
                ConfigKnobs {
                    problem_set_hash: "other",
                    ..knobs()
                },
            ),
            (
                "feedback_turns",
                ConfigKnobs {
                    feedback_turns: 2,
                    ..knobs()
                },
            ),
        ];
        for (name, v) in &variants {
            assert_ne!(a, config_digest(v), "{name} must be part of row identity");
        }
    }

    /// Three files per fixture under `dir`, with the given contents.
    fn fixture_with(dir: &Path, id: &str, spec: &str, reference: &str, tests: &str) -> Fixture {
        let write = |name: &str, body: &str| {
            let p = dir.join(format!("{id}.{name}"));
            std::fs::write(&p, body).unwrap();
            p
        };
        Fixture {
            id: id.to_string(),
            slug: format!("{id}-x"),
            training_eligible: false,
            added_at: "2026-01-01".to_string(),
            prompt: "p".to_string(),
            signature: "fn f() to int".to_string(),
            tests_path: write("tests", tests),
            spec_path: write("spec", spec),
            reference_path: write("reference", reference),
        }
    }

    #[test]
    fn problem_set_hash_is_byte_sensitive_and_order_independent() {
        let dir = tempfile::tempdir().unwrap();
        let a = fixture_with(dir.path(), "001", "spec-a", "ref-a", "tests-a");
        let b = fixture_with(dir.path(), "002", "spec-b", "ref-b", "tests-b");
        let h = problem_set_hash(&[a.clone(), b.clone()]).unwrap();
        assert_eq!(h.len(), 64);
        assert!(h.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')));
        assert_eq!(h, problem_set_hash(&[a.clone(), b.clone()]).unwrap());
        assert_eq!(
            h,
            problem_set_hash(&[b.clone(), a.clone()]).unwrap(),
            "order-independent"
        );
        for path in [&a.spec_path, &a.reference_path, &a.tests_path] {
            let orig = std::fs::read(path).unwrap();
            std::fs::write(path, [orig.as_slice(), b"x"].concat()).unwrap();
            assert_ne!(
                h,
                problem_set_hash(&[a.clone(), b.clone()]).unwrap(),
                "one byte in {} must change the hash",
                path.display()
            );
            std::fs::write(path, orig).unwrap();
        }
        assert_eq!(
            h,
            problem_set_hash(&[a, b]).unwrap(),
            "restored content, restored hash"
        );
    }

    #[test]
    fn problem_set_hash_rejects_duplicate_ids() {
        let dir = tempfile::tempdir().unwrap();
        let a = fixture_with(dir.path(), "001", "s", "r", "t");
        let err = problem_set_hash(&[a.clone(), a]).unwrap_err().to_string();
        assert!(err.contains("duplicate fixture id"), "{err}");
    }

    #[test]
    fn problem_set_hash_framing_has_no_boundary_collisions() {
        let d1 = tempfile::tempdir().unwrap();
        let d2 = tempfile::tempdir().unwrap();
        let one = fixture_with(d1.path(), "ab", "c", "r", "t");
        let two = fixture_with(d2.path(), "a", "bc", "r", "t");
        assert_ne!(
            problem_set_hash(&[one]).unwrap(),
            problem_set_hash(&[two]).unwrap()
        );
        // The empty set is defined: SHA-256 of the empty framing.
        assert_eq!(
            problem_set_hash(&[]).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn parse_vox_version_line_extracts_git_sha() {
        assert_eq!(
            parse_vox_version_line("vox 0.6.0+build.12 (abc1234)"),
            Some((
                "0.6.0+build.12 (abc1234)".to_string(),
                "abc1234".to_string()
            ))
        );
        assert_eq!(parse_vox_version_line("vox 0.6.0"), None);
        assert_eq!(parse_vox_version_line("vox 0.6.0 ()"), None);
    }

    #[cfg(unix)]
    #[test]
    fn verifier_identity_reads_version_and_hashes_the_binary() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("vox");
        let script = b"#!/bin/sh\necho \"vox 1.2.3+build.4 (deadbee)\"\n";
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let id = verifier_identity(&bin).unwrap();
        assert_eq!(id.version_line, "1.2.3+build.4 (deadbee)");
        assert_eq!(id.git_sha.as_deref(), Some("deadbee"));
        assert_eq!(id.binary_sha256, hex_encode(&interop_sha256(script)));
        assert!(verifier_identity(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn template_and_context_hashes_are_blake3() {
        let t = prompt_template_hash();
        assert_eq!(t.len(), 64);
        assert_eq!(t, prompt_template_hash());
        assert_eq!(
            context_hash_blake3("C1", "text"),
            hex_encode(&secure_hash(b"C1\0text"))
        );
        assert_ne!(
            context_hash_blake3("C0", "same"),
            context_hash_blake3("C1", "same")
        );
    }

    #[test]
    fn infra_classification_uses_the_facade_prefixes() {
        use vox_actor_runtime::llm::{CONTEXT_EXCEEDED_PREFIX, RATE_LIMITED_PREFIX};
        for msg in [
            format!("{RATE_LIMITED_PREFIX}rate limit exceeded"),
            format!("{CONTEXT_EXCEEDED_PREFIX}LLM API returned error (400): too long"),
            "http error: connection refused".to_string(),
            "provider status 503: busy".to_string(),
            "provider status 500: x".to_string(),
            "LLM API returned error (503): overloaded".to_string(),
            "HTTP request failed: connection reset".to_string(),
            "activity cancelled".to_string(),
        ] {
            assert_eq!(
                classify_generation_error(&msg),
                GenErrorKind::Infra,
                "{msg}"
            );
        }
        assert_eq!(
            classify_generation_error("model returned malformed code"),
            GenErrorKind::Model
        );
        assert_eq!(
            classify_generation_error("LLM API returned error (400): bad request"),
            GenErrorKind::Model
        );
    }

    #[test]
    fn infra_409_model_mismatch_is_fatal() {
        assert_eq!(
            classify_generation_error(
                "provider status 409: {\"text\":\"Loaded model is a, not b\"}"
            ),
            GenErrorKind::FatalConfig
        );
        assert_eq!(
            classify_generation_error(
                "LLM API returned error (409): {\"text\":\"Loaded model is a, not b\"}"
            ),
            GenErrorKind::FatalConfig
        );
    }

    #[test]
    fn sample_seed_matches_eval_local_and_is_unambiguous() {
        let s = sample_seed(7, "001", 0);
        // Pinned by 12-03: vox-ml-cli's eval-local returns the same value.
        assert_eq!(s, 780_188_561);
        assert_eq!(s, sample_seed(7, "001", 0));
        assert!(s < 1 << 31);
        assert_ne!(s, sample_seed(8, "001", 0));
        assert_ne!(s, sample_seed(7, "002", 0));
        assert_ne!(s, sample_seed(7, "001", 1));
        assert_ne!(sample_seed(1, "1", 23), sample_seed(1, "12", 3));
    }

    fn args_for_validation() -> EvalCorpusArgs {
        EvalCorpusArgs::parse_from(["eval-corpus", "--model", "m"])
    }

    #[test]
    fn sample_args_are_validated() {
        assert!(validate_sampling(&args_for_validation()).is_ok());
        let bad: [(&str, fn(&mut EvalCorpusArgs)); 6] = [
            ("n = 0", |a| a.n = 0),
            ("k = 0", |a| a.k = 0),
            ("k > n", |a| a.k = a.n + 1),
            ("top_p = 0", |a| a.top_p = 0.0),
            ("top_p = 1.5", |a| a.top_p = 1.5),
            ("temperature NaN", |a| a.temperature = f32::NAN),
        ];
        for (name, mutate) in bad {
            let mut a = args_for_validation();
            mutate(&mut a);
            assert!(validate_sampling(&a).is_err(), "{name} must be rejected");
        }
    }

    #[test]
    fn extract_vox_code_unwraps_fences_and_passes_bare_code() {
        assert!(extract_vox_code("```vox\nfn f() to int { return 1 }\n```").starts_with("fn f()"));
        assert!(extract_vox_code("```\nfn f() to int { return 1 }\n```").starts_with("fn f()"));
        assert_eq!(
            extract_vox_code("fn f() to int { return 1 }").trim(),
            "fn f() to int { return 1 }"
        );
    }

    #[test]
    fn load_solution_dir_errors_on_ambiguous_ids() {
        let d = std::env::temp_dir().join(format!("vox-sol-amb-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("041.vox"), "fn a() to int { return 1 }").unwrap();
        std::fs::write(d.join("041-nth-prime.vox"), "fn b() to int { return 2 }").unwrap();
        assert!(
            load_solution_dir(&d).is_err(),
            "ambiguous fixture id must be an error"
        );
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn load_solution_dir_ignores_non_vox_files() {
        let d = std::env::temp_dir().join(format!("vox-sol-nonvox-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("041.vox"), "fn a() to int { return 1 }").unwrap();
        std::fs::write(d.join("notes.md"), "ignored").unwrap();
        let sols = load_solution_dir(&d).unwrap();
        assert_eq!(sols.len(), 1);
        assert!(sols.contains_key("041"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn resolve_vox_binary_errors_with_build_instructions_when_absent() {
        let missing = std::env::temp_dir().join("definitely-no-vox-binary-here");
        let err = resolve_vox_binary(Some(&missing)).unwrap_err();
        assert!(
            err.to_string().contains("cargo build -p vox-cli --release"),
            "error must tell the operator how to fix it, got: {err}"
        );
        assert!(
            err.to_string().contains("-j 2"),
            "error must mention the low-memory workaround: a release build of this \
             workspace can OOM rustc on a contended machine, and the crash does not \
             look like an out-of-memory error"
        );
    }

    #[test]
    fn resolve_vox_binary_accepts_an_explicit_existing_path() {
        let existing = std::env::current_exe().expect("test binary path");
        assert_eq!(
            resolve_vox_binary(Some(&existing)).expect("resolves"),
            existing
        );
    }

    /// A repo root whose `mens/runs/<name>` holds a serveable checkpoint.
    fn repo_with_run(name: &str) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        let run = tmp.path().join("mens").join("runs").join(name);
        std::fs::create_dir_all(&run).expect("run dir");
        for f in [
            "candle_qlora_adapter.safetensors",
            "tokenizer.json",
            "adapter_manifest.json",
        ] {
            std::fs::write(run.join(f), b"{}").expect("fixture file");
        }
        tmp
    }

    #[tokio::test]
    async fn resolve_mens_run_routes_to_voxlocal() {
        let repo = repo_with_run("demo");
        let registry = with_mens_runs(ModelRegistry::default(), repo.path())
            .await
            .expect("registry");
        let (spec, config) = resolve_eval_model(&registry, "mens/demo").expect("resolves");
        assert_eq!(spec.id, "mens/demo");
        assert_eq!(config.provider, "voxlocal");
        assert_eq!(config.model, "mens/demo");
    }

    #[test]
    fn resolve_unknown_mens_id_is_refused() {
        let err = resolve_eval_model(&ModelRegistry::default(), "mens/zzz").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("mens/runs"), "names where it looked: {msg}");
        assert!(
            msg.contains("cloud provider"),
            "says it will not go to the cloud: {msg}"
        );
    }

    #[tokio::test]
    async fn generate_candidate_sends_model_and_sampling() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let repo = repo_with_run("demo");
        let registry = with_mens_runs(ModelRegistry::default(), repo.path())
            .await
            .expect("registry");
        let (_spec, mut config) = resolve_eval_model(&registry, "mens/demo").expect("resolves");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_partial_json(serde_json::json!({
                "model": "mens/demo", "seed": 1234, "temperature": 0.8, "top_p": 0.9, "max_tokens": 64
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "model": "mens/demo",
                "choices": [{"message": {"role": "assistant", "content": "```vox\nfn f() to int { return 1 }\n```"}}],
                "usage": {"prompt_tokens": 3, "completion_tokens": 4}
            })))
            .expect(1)
            .mount(&server)
            .await;

        config.base_url = Some(format!("{}/v1/chat/completions", server.uri()));
        config.seed = Some(1234);
        config.temperature = Some(0.8);
        config.top_p = Some(0.9);
        config.max_tokens = Some(64);
        config.telemetry_skip_interaction = true;

        let g = generate_candidate(&config, "write f")
            .await
            .expect("generated");
        assert_eq!(g.code, "fn f() to int { return 1 }");
        assert_eq!(g.provider_model, "mens/demo");
        assert_eq!(g.total_tokens, 7);
    }

    fn fixture(id: &str, dir: &Path) -> Fixture {
        let tests_path = dir.join(format!("{id}.vox"));
        std::fs::write(&tests_path, "fn main() to str { return \"ok\" }").unwrap();
        Fixture {
            id: id.to_string(),
            slug: format!("{id}-x"),
            training_eligible: false,
            added_at: "2026-01-01".to_string(),
            prompt: "return one".to_string(),
            signature: "fn f() to int".to_string(),
            spec_path: tests_path.clone(),
            reference_path: tests_path.clone(),
            tests_path,
        }
    }

    fn ctx() -> PromptContext {
        PromptContext {
            condition: Condition::C0ZeroShot,
            context_text: String::new(),
            context_hash: "h".to_string(),
        }
    }

    fn live_config(server_uri: &str) -> LlmConfig {
        let mut c = LlmConfig::direct("voxlocal", "mens/demo");
        c.base_url = Some(format!("{server_uri}/v1/chat/completions"));
        c.telemetry_skip_interaction = true;
        c
    }

    fn verdict(compiled: bool, tests_passed: bool) -> VerifyOutcome {
        VerifyOutcome {
            compiled,
            tests_passed,
            cheated: false,
            assertion_failed: false,
            detail: String::new(),
        }
    }

    fn ok_body() -> serde_json::Value {
        serde_json::json!({
            "model": "mens/demo",
            "choices": [{"message": {"role": "assistant", "content": "fn f() to int { return 1 }"}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 4}
        })
    }

    #[tokio::test]
    async fn sample_loop_draws_exactly_n_without_early_stop() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let dir = tempfile::tempdir().unwrap();
        let fixtures = vec![fixture("001", dir.path()), fixture("002", dir.path())];
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .expect(6)
            .mount(&server)
            .await;

        let cfg = live_config(&server.uri());
        let ctx = ctx();
        let plan = SamplePlan {
            n: 3,
            base_seed: 7,
            max_spend_usd: None,
            feedback_turns: 0,
        };
        let mut verify = |_: &str, _: &Fixture| Ok(verdict(true, true));
        let drawn = draw_samples(
            &fixtures,
            SampleSource::Live {
                config: &cfg,
                ctx: &ctx,
            },
            plan,
            &mut verify,
        )
        .await
        .expect("drawn");

        assert_eq!(drawn.n_samples_planned, 6);
        assert_eq!(drawn.outcomes.len(), 2);
        for o in &drawn.outcomes {
            assert_eq!(
                o.attempts.len(),
                3,
                "all n drawn even though the first passed"
            );
            assert_eq!((o.n, o.c), (3, 3));
        }
        assert_eq!(drawn.samples.len(), 6);
        let order: Vec<(String, usize)> = drawn
            .samples
            .iter()
            .map(|s| (s.fixture_id.clone(), s.sample_index))
            .collect();
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted);
        for s in &drawn.samples {
            assert_eq!(s.seed, sample_seed(7, &s.fixture_id, s.sample_index));
            assert_eq!(s.failure_class, FailureClass::Pass);
            assert_eq!(s.provider_model.as_deref(), Some("mens/demo"));
        }
    }

    #[tokio::test]
    async fn sample_loop_keeps_infra_out_of_attempts() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let dir = tempfile::tempdir().unwrap();
        let fixtures = vec![fixture("001", dir.path())];
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
            .mount(&server)
            .await;

        let cfg = live_config(&server.uri());
        let ctx = ctx();
        let plan = SamplePlan {
            n: 1,
            base_seed: 0,
            max_spend_usd: None,
            feedback_turns: 0,
        };
        let mut verify = |_: &str, _: &Fixture| -> Result<VerifyOutcome> {
            panic!("no candidate exists, so nothing is verified")
        };
        let drawn = draw_samples(
            &fixtures,
            SampleSource::Live {
                config: &cfg,
                ctx: &ctx,
            },
            plan,
            &mut verify,
        )
        .await
        .expect("drawn");

        assert_eq!(drawn.n_infra_errors, 1);
        assert_eq!(drawn.outcomes[0].attempts.len(), 0);
        assert_eq!(drawn.samples.len(), 1);
        assert_eq!(drawn.samples[0].failure_class, FailureClass::InfraError);
        assert!(drawn.samples[0].code.is_none());
    }

    #[tokio::test]
    async fn sample_loop_aborts_on_model_mismatch() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let dir = tempfile::tempdir().unwrap();
        let fixtures = vec![fixture("001", dir.path())];
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(409)
                    .set_body_string("{\"text\":\"Loaded model is a, not b\"}"),
            )
            .mount(&server)
            .await;

        let cfg = live_config(&server.uri());
        let ctx = ctx();
        let plan = SamplePlan {
            n: 2,
            base_seed: 0,
            max_spend_usd: None,
            feedback_turns: 0,
        };
        let mut verify = |_: &str, _: &Fixture| Ok(verdict(true, true));
        let err = draw_samples(
            &fixtures,
            SampleSource::Live {
                config: &cfg,
                ctx: &ctx,
            },
            plan,
            &mut verify,
        )
        .await
        .expect_err("a served-model mismatch aborts the run");
        assert!(err.to_string().contains("different model"), "{err}");
    }

    #[tokio::test]
    async fn sample_loop_from_dir_missing_solution_is_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures = vec![fixture("001", dir.path())];
        let solutions: HashMap<String, String> = HashMap::new();
        let plan = SamplePlan {
            n: 5,
            base_seed: 0,
            max_spend_usd: None,
            feedback_turns: 0,
        };
        let mut verify = |_: &str, _: &Fixture| -> Result<VerifyOutcome> {
            panic!("a missing solution is never verified")
        };
        let drawn = draw_samples(
            &fixtures,
            SampleSource::FromDir(&solutions),
            plan,
            &mut verify,
        )
        .await
        .expect("drawn");

        assert_eq!(
            drawn.n_samples_planned, 1,
            "from-dir is one attempt per fixture"
        );
        let a = &drawn.outcomes[0].attempts;
        assert_eq!(a.len(), 1);
        assert!(!a[0].compiled && !a[0].tests_passed && !a[0].cheated);
        assert_eq!(
            drawn.samples[0].failure_class,
            FailureClass::GenerationError
        );
        assert!(drawn.samples[0].code.is_none());
    }

    // ---- compiler-feedback repair turns (12-08) ----

    fn chat_body(content: &str) -> serde_json::Value {
        serde_json::json!({
            "model": "mens/demo",
            "choices": [{"message": {"role": "assistant", "content": content}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 4}
        })
    }

    /// One fixture, n = 1. The first request answers a candidate the verifier rejects; any
    /// request carrying the feedback text answers one it accepts.
    async fn feedback_run(
        feedback_turns: u32,
        first_expect: u64,
        repair_expect: u64,
    ) -> (DrawOutcome, wiremock::MockServer) {
        use wiremock::matchers::{body_string_contains, method};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let dir = tempfile::tempdir().unwrap();
        let fixtures = vec![fixture("001", dir.path())];
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("The Vox verifier rejected"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(chat_body("fn good() to int { return 1 }")),
            )
            .with_priority(1)
            .expect(repair_expect)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(chat_body("fn bad() to int { return 0 }")),
            )
            .with_priority(5)
            .expect(first_expect)
            .mount(&server)
            .await;

        let cfg = live_config(&server.uri());
        let ctx = ctx();
        let plan = SamplePlan {
            n: 1,
            base_seed: 7,
            max_spend_usd: None,
            feedback_turns,
        };
        let mut verify = |code: &str, _: &Fixture| -> Result<VerifyOutcome> {
            Ok(if code.contains("good") {
                verdict(true, true)
            } else {
                VerifyOutcome {
                    detail: "type error: bad".to_string(),
                    ..verdict(false, false)
                }
            })
        };
        let drawn = draw_samples(
            &fixtures,
            SampleSource::Live {
                config: &cfg,
                ctx: &ctx,
            },
            plan,
            &mut verify,
        )
        .await
        .expect("drawn");
        (drawn, server)
    }

    #[tokio::test]
    async fn feedback_repairs_a_failing_sample() {
        let (drawn, server) = feedback_run(1, 1, 1).await;
        let s = &drawn.samples[0];
        assert_eq!(s.passed_by_turn, vec![false, true]);
        assert_eq!(
            s.repair_tokens, 7,
            "repair tokens are recorded apart from the attempt's"
        );

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 2);
        let body: serde_json::Value = serde_json::from_slice(&reqs[1].body).unwrap();
        let roles: Vec<&str> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["role"].as_str().unwrap())
            .collect();
        assert_eq!(roles, ["user", "assistant", "user"]);
        assert_eq!(
            body["messages"][1]["content"],
            "fn bad() to int { return 0 }"
        );
        assert!(
            body["messages"][2]["content"]
                .as_str()
                .unwrap()
                .contains("type error: bad"),
            "the verifier's diagnostic is sent back"
        );
        assert_eq!(body["seed"], repair_seed(7, "001", 1, 0));
    }

    /// T-12-08-03: a repaired sample must not leak into pass@1 / pass@k inputs.
    #[tokio::test]
    async fn feedback_never_replaces_the_original_attempt() {
        let (drawn, _server) = feedback_run(1, 1, 1).await;
        assert_eq!(drawn.samples[0].passed_by_turn, vec![false, true]);
        let o = &drawn.outcomes[0];
        assert_eq!((o.n, o.c), (1, 0), "the fixture still counts as failed");
        assert!(!o.attempts[0].tests_passed && !o.attempts[0].compiled);
        let s = &drawn.samples[0];
        assert!(!s.tests_passed && !s.compiled);
        assert_eq!(s.failure_class, FailureClass::CheckError);
        assert!(s.code.as_deref().unwrap().contains("bad"));
    }

    #[tokio::test]
    async fn feedback_zero_turns_sends_no_repairs() {
        let (drawn, _server) = feedback_run(0, 1, 0).await;
        assert_eq!(drawn.samples[0].passed_by_turn, vec![false]);
        assert_eq!(drawn.samples[0].repair_tokens, 0);
    }

    #[tokio::test]
    async fn feedback_stops_after_first_pass() {
        let (drawn, _server) = feedback_run(3, 1, 1).await;
        assert_eq!(
            drawn.samples[0].passed_by_turn,
            vec![false, true, true, true]
        );
    }

    #[test]
    fn repair_seed_is_per_turn_and_deterministic() {
        let a = repair_seed(7, "001", 1, 0);
        assert_eq!(a, repair_seed(7, "001", 1, 0));
        assert_eq!(a, sample_seed(7, "001#turn1", 0));
        assert_ne!(a, repair_seed(7, "001", 2, 0));
        assert_ne!(a, repair_seed(7, "001", 1, 1));
        assert_ne!(
            a,
            sample_seed(7, "001", 0),
            "never the first attempt's seed"
        );
    }

    #[test]
    fn sample_args_reject_more_than_three_feedback_turns() {
        let mut a = args_for_validation();
        a.feedback_turns = 3;
        assert!(validate_sampling(&a).is_ok());
        a.feedback_turns = 4;
        assert!(validate_sampling(&a).is_err());
    }

    // ---- provenance collection and the typed report (12-07) ----

    fn passing_draw() -> DrawOutcome {
        let attempt = AttemptOutcome {
            compiled: true,
            tests_passed: true,
            cheated: false,
            total_tokens: 7,
            latency_ms: 5,
            cost_usd: None,
        };
        let outcomes = ["001", "002"]
            .iter()
            .map(|id| FixtureOutcome {
                fixture_id: (*id).to_string(),
                n: 1,
                c: 1,
                attempts: vec![attempt.clone()],
            })
            .collect();
        let samples = ["001", "002"]
            .iter()
            .map(|id| SampleRecord {
                fixture_id: (*id).to_string(),
                sample_index: 0,
                seed: 1,
                code: Some("fn f() to int { return 1 }".to_string()),
                failure_class: FailureClass::Pass,
                compiled: true,
                tests_passed: true,
                cheated: false,
                total_tokens: 7,
                latency_ms: 5,
                cost_usd: None,
                provider_model: Some("m".to_string()),
                passed_by_turn: vec![true],
                repair_tokens: 0,
                repair_cost_usd: None,
            })
            .collect();
        DrawOutcome {
            outcomes,
            samples,
            n_infra_errors: 0,
            n_samples_planned: 2,
            spend_cap_hit: false,
        }
    }

    fn meta_for(args: &EvalCorpusArgs) -> ReportMeta {
        ReportMeta {
            model_id: args.model.clone(),
            harness_id: args.harness.clone(),
            config_digest: "digest".to_string(),
            cutoff: None,
            held_out_only: true,
            measured: true,
            k: args.k,
            analysis_seed: args.seed,
        }
    }

    fn prov_inputs<'a>(
        args: &'a EvalCorpusArgs,
        provider: Option<&str>,
        samples: &'a [SampleRecord],
    ) -> ProvenanceInputs<'a> {
        ProvenanceInputs {
            args,
            row_model_id: args.model.clone(),
            provider: provider.map(str::to_string),
            request_date: "2026-10-11".to_string(),
            samples,
            context_hash: Some("ctx".to_string()),
            problem_set_hash: None,
            prompt_template_hash: None,
            corpus_manifest_hash: None,
            fingerprint: None,
            verifier: None,
            mens: None,
            runner: None,
        }
    }

    #[test]
    fn finalize_report_names_missing_identity() {
        let args = EvalCorpusArgs::parse_from(["eval-corpus", "--model", "acme/m"]);
        let draw = passing_draw();
        let prov = collect_provenance(&prov_inputs(&args, Some("openrouter"), &draw.samples));
        let report = finalize_report(draw, prov, &meta_for(&args));
        assert!(!report.run_complete, "identity is owed by Phase 7");
        assert_eq!(report.schema_version, 2);
        for want in [
            "missing provenance: identity",
            "missing provenance: preregistration_id",
        ] {
            assert!(
                report.incomplete_reasons.iter().any(|r| r == want),
                "{want} in {:?}",
                report.incomplete_reasons
            );
        }
        assert_eq!(report.samples.len(), 2, "every SampleRecord is carried");
    }

    #[test]
    fn seed_is_enforced_only_for_voxlocal() {
        let args = EvalCorpusArgs::parse_from(["eval-corpus", "--model", "acme/m"]);
        for (provider, want) in [("openrouter", false), ("voxlocal", true)] {
            let prov = collect_provenance(&prov_inputs(&args, Some(provider), &[]));
            assert_eq!(prov.sampling.unwrap().seed_enforced, want, "{provider}");
        }
        let prov = collect_provenance(&prov_inputs(&args, None, &[]));
        assert!(
            !prov.sampling.unwrap().seed_enforced,
            "no provider, no guarantee"
        );
    }

    #[test]
    fn provider_models_are_sorted_and_deduplicated() {
        let args = EvalCorpusArgs::parse_from(["eval-corpus", "--model", "acme/m"]);
        let mut samples = passing_draw().samples;
        for (s, m) in samples.iter_mut().zip(["z-model", "a-model"]) {
            s.provider_model = Some(m.to_string());
        }
        let mut extra = samples[0].clone();
        extra.sample_index = 1;
        samples.push(extra);
        let mut none = samples[0].clone();
        none.provider_model = None;
        samples.push(none);
        let prov = collect_provenance(&prov_inputs(&args, Some("openrouter"), &samples));
        assert_eq!(prov.provider_reported_models, vec!["a-model", "z-model"]);
    }

    #[test]
    fn report_json_has_no_credentials() {
        let args = EvalCorpusArgs::parse_from(["eval-corpus", "--model", "acme/m"]);
        let draw = passing_draw();
        let prov = collect_provenance(&prov_inputs(&args, Some("openrouter"), &draw.samples));
        let json = serde_json::to_string(&finalize_report(draw, prov, &meta_for(&args))).unwrap();
        assert!(!json.contains("api_key"), "{json}");
    }

    #[test]
    fn runner_label_rule() {
        assert_eq!(
            resolve_runner(Some("mac-studio"), Some("true")).as_deref(),
            Some("mac-studio")
        );
        assert_eq!(
            resolve_runner(None, Some("true")).as_deref(),
            Some("github-hosted")
        );
        assert_eq!(resolve_runner(None, None), None);
        assert_eq!(resolve_runner(Some("  "), Some("false")), None);
    }

    // ---- mens card check and write-back guard (12-07) ----

    /// `<tmp>/mens/runs/demo` holding an adapter, optionally with a valid lineage card.
    fn mens_repo(with_card: bool) -> tempfile::TempDir {
        use vox_populi::mens::tensor::adapter_card::{AdapterCard, sha256_file_hex};
        let tmp = tempfile::tempdir().unwrap();
        let run = tmp.path().join("mens/runs/demo");
        std::fs::create_dir_all(&run).unwrap();
        let adapter = run.join("candle_qlora_adapter.safetensors");
        std::fs::write(&adapter, b"adapter bytes").unwrap();
        if with_card {
            let mut card = AdapterCard::for_test("qwen3_16g", "qlora");
            card.adapter_sha256 = sha256_file_hex(&adapter).unwrap();
            card.train_data_sha256 = hex_encode(&interop_sha256(b"train data"));
            card.training_manifest_digest = hex_encode(&secure_hash(b"manifest"));
            card.training_syntax_fingerprint = "fp1".to_string();
            card.syntax_fingerprint_kind = "ebnf-sha256".to_string();
            card.write_sidecar(&adapter).unwrap();
        }
        tmp
    }

    #[test]
    fn card_missing_refuses_mens_label() {
        let repo = mens_repo(false);
        let (lineage, problems) = mens_lineage(repo.path(), "mens/demo", "fp1", "ebnf-sha256");
        assert!(lineage.is_none());
        assert!(
            problems.iter().any(|p| p.contains("adapter_card.json")),
            "names the missing card: {problems:?}"
        );
    }

    #[test]
    fn card_valid_fills_lineage_and_drift() {
        let repo = mens_repo(true);
        let (lineage, problems) = mens_lineage(repo.path(), "mens/demo", "fp1", "ebnf-sha256");
        assert!(problems.is_empty(), "{problems:?}");
        let l = lineage.expect("valid card gives lineage");
        assert!(
            l.base_hf_id_at_revision
                .as_deref()
                .unwrap()
                .ends_with("@abc123")
        );
        assert_eq!(
            l.adapter_sha256.as_deref(),
            Some(hex_encode(&interop_sha256(b"adapter bytes")).as_str())
        );
        assert_eq!(l.syntax_drift, Some(false));
        assert_eq!(l.training_syntax_fingerprint.as_deref(), Some("fp1"));
        let (drifted, _) = mens_lineage(repo.path(), "mens/demo", "fp2", "ebnf-sha256");
        assert_eq!(drifted.unwrap().syntax_drift, Some(true));
    }

    #[test]
    fn card_not_required_for_external_models() {
        let repo = mens_repo(false);
        let (lineage, problems) = mens_lineage(repo.path(), "acme/model", "fp1", "ebnf-sha256");
        assert!(lineage.is_none() && problems.is_empty());
    }

    #[test]
    fn write_back_requires_complete_measured_runs() {
        assert!(should_write_back(true, true, false));
        assert!(
            !should_write_back(false, true, false),
            "incomplete never steers routing"
        );
        assert!(
            !should_write_back(true, false, false),
            "ingested rows never steer routing"
        );
        assert!(
            !should_write_back(true, true, true),
            "--no-write-back is honored"
        );
    }
}
