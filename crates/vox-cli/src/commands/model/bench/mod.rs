//! `vox model bench` — graded live benchmarks across Intelligence (graded suites),
//! Responsiveness (time to first token, tokens/sec, p50/p95 latency, error rate) and
//! Efficiency (cost and tokens per solved case), plus reliability (error classes, variance
//! across `--repeats`) and effective context (needle depth). Every call stays under
//! `--max-usd`; rows land in `model_benchmark_results`, which routing reads back as
//! measured quality and Wilson-gated confidence (`models::bench_evidence`).

pub mod budget;
pub mod suites;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clap::Args;
use futures::StreamExt;
use owo_colors::OwoColorize;
use vox_actor_runtime::llm::{LlmChatMessage, llm_chat, llm_stream_measured};
use vox_orchestrator::models::autonomic::{ModelConfidence, PromotionEvidence, record_promotion};
use vox_orchestrator::models::leaderboard::{BENCH_CATEGORIES, MODES, mode_routing_table};
use vox_orchestrator::models::{
    ModelCapabilities, ModelRegistry, ModelSpec, PricingSource, ProviderType, TaskCategory,
    bench_evidence::BenchEvidence, llm_config_for_spec, wilson_score_interval,
};

use budget::{Budget, Price, estimate_case_usd, price_for};
use suites::{BenchCase, Grader, Suite, bench_tools, builtin_cases, grade_text, grade_tool_call};

#[derive(Args, Debug, Clone)]
pub struct BenchArgs {
    /// Models to bench: catalog ids, or `provider:model` where provider is one of
    /// openrouter, ollama, voxlocal, groq, cerebras, mistral, deepseek, sambanova, anthropic, gemini.
    #[arg(long = "model", value_delimiter = ',')]
    pub models: Vec<String>,
    /// Also bench up to this many catalog models that routing does not yet trust
    /// (confidence below Confirmed and a usable key), cheapest first.
    #[arg(long, default_value_t = 0)]
    pub provisional: usize,
    /// Also bench up to this many models that a `~vendor/…-latest` alias currently points
    /// at (the newest release of each line), cheapest first.
    #[arg(long, default_value_t = 0)]
    pub latest: usize,
    /// Suites to run (default: all): reasoning, codegen, json, tool-call, needle, vox-gen, chat.
    #[arg(long, value_delimiter = ',')]
    pub suites: Vec<String>,
    /// Run every case this many times (variance / reliability).
    #[arg(long, default_value_t = 1)]
    pub repeats: u32,
    /// Hard spend cap in USD for the whole run. The run refuses to start when the
    /// worst-case estimate exceeds it, and stops before any call that would cross it.
    #[arg(long)]
    pub max_usd: f64,
    /// Models benched concurrently.
    #[arg(long, default_value_t = 4)]
    pub concurrency: usize,
    /// Print the plan and estimate, make no calls.
    #[arg(long)]
    pub dry_run: bool,
    /// Do not persist results (routing will not see them).
    #[arg(long)]
    pub no_write: bool,
    /// Write a JSON report here (the global `--json` flag is a different switch).
    #[arg(long)]
    pub report: Option<PathBuf>,
    /// Per-call timeout in milliseconds.
    #[arg(long, default_value_t = 120_000)]
    pub timeout_ms: u64,
}

/// One executed case.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CaseOutcome {
    pub model_id: String,
    pub served_model: Option<String>,
    pub suite: &'static str,
    pub case_id: String,
    pub repeat: u32,
    pub score: f64,
    pub passed: bool,
    pub ttft_ms: Option<u64>,
    pub latency_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tokens_per_sec: Option<f64>,
    pub cost_usd: f64,
    pub cost_reported: bool,
    pub error_class: Option<String>,
}

/// Per-model aggregate across the three axes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelSummary {
    pub model_id: String,
    pub served_model: Option<String>,
    pub cases: usize,
    pub passed: usize,
    pub pass_rate: f64,
    pub pass_ci95: Option<(f64, f64)>,
    pub mean_score: f64,
    pub by_suite: BTreeMap<String, f64>,
    pub error_rate: f64,
    pub p50_ttft_ms: Option<u64>,
    pub p50_latency_ms: Option<u64>,
    pub p95_latency_ms: Option<u64>,
    pub mean_tokens_per_sec: Option<f64>,
    pub total_cost_usd: f64,
    pub cost_per_solve_usd: Option<f64>,
    pub tokens_per_solve: Option<f64>,
    /// Mean per-case standard deviation of score across repeats (`None` with one repeat).
    pub score_stddev: Option<f64>,
    /// Deepest needle the model found: "32k", "8k" or "<8k" (`None` when not run).
    pub effective_context: Option<String>,
}

fn nearest_rank(sorted: &[u64], q: f64) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

/// Fold executed cases into per-model summaries (pure; unit-tested).
#[must_use]
pub fn summarize(outcomes: &[CaseOutcome]) -> Vec<ModelSummary> {
    let mut by_model: BTreeMap<&str, Vec<&CaseOutcome>> = BTreeMap::new();
    for o in outcomes {
        by_model.entry(o.model_id.as_str()).or_default().push(o);
    }
    by_model
        .into_iter()
        .map(|(model_id, rows)| {
            let cases = rows.len();
            let passed = rows.iter().filter(|r| r.passed).count();
            let errors = rows.iter().filter(|r| r.error_class.is_some()).count();
            let mean_score = rows.iter().map(|r| r.score).sum::<f64>() / cases.max(1) as f64;
            let mut suite_scores: BTreeMap<String, (f64, usize)> = BTreeMap::new();
            for r in &rows {
                let e = suite_scores.entry(r.suite.to_string()).or_default();
                e.0 += r.score;
                e.1 += 1;
            }
            let ok: Vec<&&CaseOutcome> = rows.iter().filter(|r| r.error_class.is_none()).collect();
            let mut ttft: Vec<u64> = ok.iter().filter_map(|r| r.ttft_ms).collect();
            ttft.sort_unstable();
            let mut lat: Vec<u64> = ok.iter().map(|r| r.latency_ms).collect();
            lat.sort_unstable();
            let tps: Vec<f64> = ok.iter().filter_map(|r| r.tokens_per_sec).collect();
            let total_cost: f64 = rows.iter().map(|r| r.cost_usd).sum();
            let total_tokens: u64 = rows.iter().map(|r| r.input_tokens + r.output_tokens).sum();
            let mut per_case: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
            for r in &rows {
                per_case
                    .entry(r.case_id.as_str())
                    .or_default()
                    .push(r.score);
            }
            let sds: Vec<f64> = per_case
                .values()
                .filter(|v| v.len() > 1)
                .map(|v| {
                    let m = v.iter().sum::<f64>() / v.len() as f64;
                    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt()
                })
                .collect();
            let found = |id: &str| rows.iter().any(|r| r.case_id == id && r.passed);
            let ran = |id: &str| rows.iter().any(|r| r.case_id == id);
            let effective_context = if !ran("needle-8k") && !ran("needle-32k") {
                None
            } else if found("needle-32k") {
                Some("32k".to_string())
            } else if found("needle-8k") {
                Some("8k".to_string())
            } else {
                Some("<8k".to_string())
            };
            ModelSummary {
                model_id: model_id.to_string(),
                served_model: rows.iter().rev().find_map(|r| r.served_model.clone()),
                cases,
                passed,
                pass_rate: passed as f64 / cases.max(1) as f64,
                pass_ci95: wilson_score_interval(passed as i64, cases as i64),
                mean_score,
                by_suite: suite_scores
                    .into_iter()
                    .map(|(k, (s, n))| (k, s / n as f64))
                    .collect(),
                error_rate: errors as f64 / cases.max(1) as f64,
                p50_ttft_ms: nearest_rank(&ttft, 0.5),
                p50_latency_ms: nearest_rank(&lat, 0.5),
                p95_latency_ms: nearest_rank(&lat, 0.95),
                mean_tokens_per_sec: (!tps.is_empty())
                    .then(|| tps.iter().sum::<f64>() / tps.len() as f64),
                total_cost_usd: total_cost,
                cost_per_solve_usd: (passed > 0).then(|| total_cost / passed as f64),
                tokens_per_solve: (passed > 0).then(|| total_tokens as f64 / passed as f64),
                score_stddev: (!sds.is_empty()).then(|| sds.iter().sum::<f64>() / sds.len() as f64),
                effective_context,
            }
        })
        .collect()
}

const PROVIDER_PREFIXES: [(&str, fn() -> ProviderType); 10] = [
    ("openrouter", || ProviderType::OpenRouter),
    ("ollama", || ProviderType::Ollama),
    ("voxlocal", || ProviderType::VoxLocal),
    ("groq", || ProviderType::Groq),
    ("cerebras", || ProviderType::Cerebras),
    ("mistral", || ProviderType::Mistral),
    ("deepseek", || ProviderType::DeepSeek),
    ("sambanova", || ProviderType::SambaNova),
    ("anthropic", || ProviderType::Anthropic),
    ("gemini", || ProviderType::GoogleDirect),
];

/// Models a `-latest` alias currently resolves to, reachable and not already targeted,
/// cheapest first.
fn latest_alias_targets(
    models: Vec<ModelSpec>,
    already: &[(ModelSpec, bool)],
    limit: usize,
    reachable: impl Fn(&ModelSpec) -> bool,
) -> Vec<ModelSpec> {
    let mut picked: Vec<ModelSpec> = models
        .into_iter()
        .filter(|m| m.capabilities.is_alias_target && reachable(m))
        .filter(|m| !already.iter().any(|(t, _)| t.id == m.id))
        .collect();
    picked.sort_by(|a, b| {
        let (pa, pb) = (price_for(a), price_for(b));
        (pa.input_per_1k + pa.output_per_1k)
            .total_cmp(&(pb.input_per_1k + pb.output_per_1k))
            .then_with(|| a.id.cmp(&b.id))
    });
    picked.truncate(limit);
    picked
}

/// A catalog spec for `raw`, or a synthesized one for `provider:model` (`bool` = synthesized).
#[must_use]
pub fn resolve_target(registry: &ModelRegistry, raw: &str) -> (ModelSpec, bool) {
    let raw = raw.trim();
    if let Some(spec) = registry.get(raw) {
        return (spec.clone(), false);
    }
    let (provider_type, id) = raw
        .split_once(':')
        .and_then(|(p, rest)| {
            PROVIDER_PREFIXES
                .iter()
                .find(|(name, _)| *name == p)
                .map(|(_, pt)| (pt(), rest.to_string()))
        })
        .unwrap_or((ProviderType::OpenRouter, raw.to_string()));
    if let Some(spec) = registry
        .get(&id)
        .filter(|s| s.provider_type == provider_type)
    {
        return (spec.clone(), false);
    }
    let local = matches!(provider_type, ProviderType::Ollama | ProviderType::VoxLocal);
    let provider = PROVIDER_PREFIXES
        .iter()
        .find(|(_, pt)| pt() == provider_type)
        .map_or("openrouter", |(n, _)| *n)
        .to_string();
    let spec = ModelSpec {
        id: id.clone(),
        canonical_slug: id,
        provider,
        provider_type,
        max_tokens: 4096,
        cost_per_1k: 0.0,
        cost_per_1k_input: 0.0,
        cost_per_1k_output: 0.0,
        is_free: local,
        observed_cost_per_1k: None,
        strengths: vec![
            vox_orchestrator::models::StrengthTag::Generalist,
            vox_orchestrator::models::StrengthTag::Codegen,
        ],
        capabilities: ModelCapabilities {
            max_context: 32_000,
            supports_json: true,
            ..Default::default()
        },
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source: PricingSource::Unknown,
        supported_parameters: vec![],
    };
    (spec, true)
}

fn classify_error(e: &str) -> String {
    let l = e.to_ascii_lowercase();
    if l.contains("rate limit") || l.contains("429") {
        "rate_limited"
    } else if l.contains("timed out") || l.contains("timeout") || l.contains("deadline") {
        "timeout"
    } else if l.contains("401")
        || l.contains("403")
        || l.contains("api key")
        || l.contains("unauthorized")
    {
        "auth"
    } else if l.contains("404") || l.contains("not found") || l.contains("no endpoints") {
        "not_found"
    } else if l.contains("status 5") || l.contains(" 50") {
        "server"
    } else if l.contains("status 4") || l.contains(" 40") {
        "client"
    } else {
        "transport"
    }
    .to_string()
}

fn supports_tools(spec: &ModelSpec) -> bool {
    spec.supported_parameters.is_empty() || spec.supported_parameters.iter().any(|p| p == "tools")
}

fn cases_for(spec: &ModelSpec, cases: &[BenchCase]) -> Vec<BenchCase> {
    cases
        .iter()
        .filter(|c| c.suite != Suite::ToolCall || supports_tools(spec))
        .filter(|c| {
            c.suite != Suite::Needle
                || spec.capabilities.max_context == 0
                || budget::estimate_prompt_tokens(c.prompt.len()) + c.max_tokens
                    <= spec.capabilities.max_context
        })
        .cloned()
        .collect()
}

async fn run_case(
    spec: &ModelSpec,
    case: &BenchCase,
    repeat: u32,
    price: Price,
    timeout_ms: u64,
) -> CaseOutcome {
    let category: TaskCategory = case
        .suite
        .task_category()
        .parse()
        .unwrap_or(TaskCategory::General);
    let mut cfg = llm_config_for_spec(spec, category);
    cfg.max_tokens = Some(case.max_tokens);
    cfg.temperature = Some(0.0);
    cfg.timeout_ms = Some(timeout_ms);
    cfg.telemetry_skip_interaction = true;
    let messages = vec![LlmChatMessage {
        role: "user".to_string(),
        content: case.prompt.clone(),
        ..Default::default()
    }];
    let mut out = CaseOutcome {
        model_id: spec.id.clone(),
        served_model: None,
        suite: case.suite.as_str(),
        case_id: case.id.clone(),
        repeat,
        score: 0.0,
        passed: false,
        ttft_ms: None,
        latency_ms: 0,
        input_tokens: 0,
        output_tokens: 0,
        tokens_per_sec: None,
        cost_usd: 0.0,
        cost_reported: false,
        error_class: None,
    };
    let started = std::time::Instant::now();
    let reported_cost = if case.suite == Suite::ToolCall {
        cfg.tools = Some(bench_tools());
        cfg.tool_choice = Some(serde_json::json!("auto"));
        let opts = vox_actor_runtime::ActivityOptions::new()
            .with_timeout(std::time::Duration::from_millis(timeout_ms));
        let flat = match llm_chat(&opts, messages, cfg).await {
            vox_actor_runtime::ActivityResult::Ok(inner) => inner,
            vox_actor_runtime::ActivityResult::Failed(e) => Err(e.to_string()),
            vox_actor_runtime::ActivityResult::Cancelled => Err("cancelled".to_string()),
        };
        match flat {
            Ok(resp) => {
                let calls: Vec<(String, serde_json::Value)> = resp
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|c| (c.name, c.arguments))
                    .collect();
                tracing::debug!(model = %spec.id, case = %case.id, ?calls, "bench tool-call reply");
                out.score = grade_tool_call(&case.grader, &calls);
                out.served_model = Some(resp.model).filter(|m| !m.is_empty());
                out.input_tokens = u64::from(resp.prompt_tokens);
                out.output_tokens = u64::from(resp.completion_tokens);
                out.latency_ms = if resp.latency_ms > 0 {
                    resp.latency_ms
                } else {
                    started.elapsed().as_millis() as u64
                };
                resp.cost_usd
            }
            Err(e) => {
                out.error_class = Some(classify_error(&e));
                tracing::debug!(model = %spec.id, case = %case.id, error = %e, "bench call failed");
                out.latency_ms = started.elapsed().as_millis() as u64;
                None
            }
        }
    } else {
        match llm_stream_measured(messages, cfg).await {
            Ok(m) => {
                tracing::debug!(model = %spec.id, case = %case.id, reply = %m.content, "bench reply");
                out.score = if matches!(case.grader, Grader::ToolCall { .. }) {
                    0.0
                } else {
                    grade_text(&case.grader, &m.content)
                };
                if m.content.trim().is_empty() {
                    out.error_class = Some("empty".to_string());
                }
                out.tokens_per_sec = m.tokens_per_sec();
                out.ttft_ms = m.ttft_ms;
                out.latency_ms = m.latency_ms;
                out.served_model = m.served_model;
                out.input_tokens = u64::from(m.prompt_tokens);
                out.output_tokens = u64::from(m.completion_tokens);
                m.cost_usd
            }
            Err(e) => {
                out.error_class = Some(classify_error(&e));
                tracing::debug!(model = %spec.id, case = %case.id, error = %e, "bench call failed");
                out.latency_ms = started.elapsed().as_millis() as u64;
                None
            }
        }
    };
    out.passed = out.score >= 1.0 - 1e-9;
    if out.input_tokens == 0 && out.error_class.is_none() {
        out.input_tokens = budget::estimate_prompt_tokens(case.prompt.len());
    }
    out.cost_reported = reported_cost.is_some();
    out.cost_usd = reported_cost.unwrap_or_else(|| price.cost(out.input_tokens, out.output_tokens));
    out
}

/// Registry evidence for one model, built from this run's summary rather than the DB.
fn evidence_from_summary(s: &ModelSummary) -> BenchEvidence {
    BenchEvidence {
        cases: s.cases as i64,
        passed: s.passed as i64,
        errors: (s.error_rate * s.cases as f64).round() as i64,
        mean_score: s.mean_score,
        resolved_model_id: s.served_model.clone(),
        p50_ttft_ms: s.p50_ttft_ms.map(|v| v as i64),
        p50_latency_ms: s.p50_latency_ms.map(|v| v as i64),
        p95_latency_ms: s.p95_latency_ms.map(|v| v as i64),
        mean_tokens_per_sec: s.mean_tokens_per_sec,
        cost_per_solve_usd: s.cost_per_solve_usd,
        tokens_per_solve: s.tokens_per_solve,
        last_recorded_at_ms: now_ms(),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub async fn run(args: BenchArgs) -> anyhow::Result<()> {
    let report = execute(args.clone()).await?;
    if let Some(path) = &args.report {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&report)?)?;
        println!("Wrote bench report to {}", path.display().bright_cyan());
    }
    if report["aborted"].as_bool() == Some(true) {
        anyhow::bail!("bench stopped at the --max-usd cap before finishing");
    }
    Ok(())
}

/// Run the bench and return the JSON report (shared with `vox model shadow`).
pub async fn execute(args: BenchArgs) -> anyhow::Result<serde_json::Value> {
    let mut registry = ModelRegistry::from_cache();
    vox_orchestrator::catalog::sync_installed_ollama(
        &mut registry,
        vox_config::local_ollama_populi_base_url(),
    )
    .await;
    let db =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::CliWorkspace, true)
            .await;
    if db.is_none() && !args.no_write && !args.dry_run {
        eprintln!(
            "{} the Vox database could not be opened (a running daemon may hold its lock), so \
             these results route only within this run and are not saved. Set VOX_DATA_DIR to \
             bench into a separate store.",
            "warning:".yellow().bold()
        );
    }
    if let Some(db) = &db {
        registry.inject_evidence_from_db(db).await;
    }

    let suites: Vec<Suite> = if args.suites.is_empty() {
        Suite::ALL.to_vec()
    } else {
        args.suites
            .iter()
            .map(|s| Suite::parse(s).ok_or_else(|| anyhow::anyhow!("unknown suite `{s}`")))
            .collect::<anyhow::Result<_>>()?
    };
    let all_cases: Vec<BenchCase> = builtin_cases()
        .into_iter()
        .filter(|c| suites.contains(&c.suite))
        .collect();

    let mut targets: Vec<(ModelSpec, bool)> = args
        .models
        .iter()
        .filter(|m| !m.trim().is_empty())
        .map(|m| resolve_target(&registry, m))
        .collect();
    if args.provisional > 0 {
        let mut pending: Vec<ModelSpec> = registry
            .list_models()
            .into_iter()
            .filter(|m| registry.routing_confidence(m) != ModelConfidence::Confirmed)
            .filter(|m| {
                vox_orchestrator::models::key_guard::inference_wire_for(&m.provider_type).is_some()
            })
            .filter(|m| !targets.iter().any(|(t, _)| t.id == m.id))
            .collect();
        pending.sort_by(|a, b| {
            let pa = price_for(a);
            let pb = price_for(b);
            (pa.input_per_1k + pa.output_per_1k)
                .total_cmp(&(pb.input_per_1k + pb.output_per_1k))
                .then_with(|| a.id.cmp(&b.id))
        });
        targets.extend(
            pending
                .into_iter()
                .take(args.provisional)
                .map(|m| (m, false)),
        );
    }
    if args.latest > 0 {
        let reachable = |m: &ModelSpec| {
            vox_orchestrator::models::key_guard::inference_wire_for(&m.provider_type).is_some()
        };
        let picked = latest_alias_targets(registry.list_models(), &targets, args.latest, reachable);
        targets.extend(picked.into_iter().map(|m| (m, false)));
    }
    if targets.is_empty() {
        anyhow::bail!("nothing to bench: pass --model <id>, --provisional <n> or --latest <n>");
    }

    // Pre-flight: the worst case (every output token used) must fit the cap.
    let repeats = args.repeats.max(1);
    let mut plan: Vec<(ModelSpec, bool, Vec<BenchCase>, Price, f64)> = Vec::new();
    let mut total_est = 0.0;
    for (spec, synthesized) in targets {
        let price = price_for(&spec);
        let cases = cases_for(&spec, &all_cases);
        let est: f64 = cases
            .iter()
            .map(|c| estimate_case_usd(&price, c.prompt.len(), c.max_tokens))
            .sum::<f64>()
            * f64::from(repeats);
        total_est += est;
        plan.push((spec, synthesized, cases, price, est));
    }
    println!(
        "{} {} model(s), {} suite(s), {} repeat(s); worst-case estimate ${:.4} (cap ${:.2})",
        " BENCH ".on_blue().white().bold(),
        plan.len(),
        suites.len(),
        repeats,
        total_est,
        args.max_usd
    );
    for (spec, _, cases, price, est) in &plan {
        println!(
            "  {:<52} {:>3} cases  est ${:.4}{}",
            spec.id,
            cases.len() as u32 * repeats,
            est,
            if price.assumed {
                "  (unpriced: assumed high)"
            } else {
                ""
            }
        );
    }
    if total_est > args.max_usd {
        anyhow::bail!(
            "pre-flight estimate ${total_est:.4} exceeds --max-usd ${:.2}; drop models or suites, or raise the cap",
            args.max_usd
        );
    }
    let before: HashMap<String, ModelConfidence> = plan
        .iter()
        .map(|(s, ..)| (s.id.clone(), registry.routing_confidence(s)))
        .collect();
    if args.dry_run {
        return Ok(
            serde_json::json!({ "schema_version": 1, "dry_run": true, "estimate_usd": total_est }),
        );
    }

    let run_id = format!("bench-{}", now_ms());
    let budget = Arc::new(Mutex::new(Budget::new(args.max_usd)));
    let aborted = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let db = db.map(Arc::new);
    let jobs = plan.iter().map(|(spec, _, cases, price, _)| {
        let budget = Arc::clone(&budget);
        let aborted = Arc::clone(&aborted);
        let db = db.clone();
        let run_id = run_id.clone();
        let no_write = args.no_write;
        let timeout_ms = args.timeout_ms;
        async move {
            let mut rows = Vec::new();
            let mut last_log = std::time::Instant::now();
            for repeat in 0..repeats {
                for case in cases {
                    let est = estimate_case_usd(price, case.prompt.len(), case.max_tokens);
                    let reserved = budget.lock().map(|mut b| b.reserve(est)).unwrap_or(false);
                    if !reserved {
                        aborted.store(true, std::sync::atomic::Ordering::SeqCst);
                        return rows;
                    }
                    let o = run_case(spec, case, repeat, *price, timeout_ms).await;
                    if let Ok(mut b) = budget.lock() {
                        let reported = o.cost_reported.then_some(o.cost_usd);
                        b.settle(est, reported, o.cost_usd);
                    }
                    if !no_write {
                        if let Some(db) = &db {
                            let row = vox_db::NewModelBenchmarkResult {
                                run_id: run_id.clone(),
                                model_id: o.model_id.clone(),
                                resolved_model_id: o.served_model.clone(),
                                provider: spec.provider.clone(),
                                suite: o.suite.to_string(),
                                task_category: case.suite.task_category().to_string(),
                                case_id: o.case_id.clone(),
                                passed: o.passed,
                                score: o.score.clamp(0.0, 1.0),
                                ttft_ms: o.ttft_ms.map(|v| v as i64),
                                latency_ms: Some(o.latency_ms as i64),
                                input_tokens: Some(o.input_tokens as i64),
                                output_tokens: Some(o.output_tokens as i64),
                                tokens_per_sec: o.tokens_per_sec,
                                cost_usd: Some(o.cost_usd),
                                error_class: o.error_class.clone(),
                                recorded_at_ms: now_ms(),
                            };
                            if let Err(e) = db.record_model_benchmark_result(&row).await {
                                tracing::warn!(error = %e, model = %o.model_id, "bench: result not recorded");
                            }
                        }
                    }
                    if last_log.elapsed() >= std::time::Duration::from_secs(3) {
                        tracing::info!(model = %spec.id, done = rows.len() + 1, "bench progress");
                        last_log = std::time::Instant::now();
                    }
                    rows.push(o);
                }
            }
            rows
        }
    });
    let outcomes: Vec<CaseOutcome> = futures::stream::iter(jobs)
        .buffer_unordered(args.concurrency.max(1))
        .collect::<Vec<Vec<CaseOutcome>>>()
        .await
        .into_iter()
        .flatten()
        .collect();
    let spent = budget.lock().map(|b| b.spent_usd).unwrap_or(0.0);
    let aborted = aborted.load(std::sync::atomic::Ordering::SeqCst);
    let summaries = summarize(&outcomes);
    print_summary(&summaries);
    println!(
        "spent ${spent:.4} of ${:.2}{}",
        args.max_usd,
        if aborted { " — STOPPED at cap" } else { "" }
    );

    // Adaptation: re-read evidence, let synthesized targets compete, and show what moved.
    for (spec, synthesized, ..) in &plan {
        if *synthesized {
            registry.register(spec.clone());
        }
    }
    match db.as_ref().filter(|_| !args.no_write) {
        Some(db) => registry.inject_evidence_from_db(db).await,
        None => {
            let mut evidence: HashMap<String, BenchEvidence> = registry
                .list_models()
                .iter()
                .filter_map(|m| {
                    registry
                        .bench_evidence(&m.id)
                        .map(|e| (m.id.clone(), e.clone()))
                })
                .collect();
            for s in &summaries {
                evidence.insert(s.model_id.clone(), evidence_from_summary(s));
            }
            registry.inject_benchmark_evidence(evidence);
        }
    }
    let mut confidence = Vec::new();
    for (spec, ..) in &plan {
        let from = before
            .get(&spec.id)
            .copied()
            .unwrap_or(ModelConfidence::Provisional);
        let to = registry
            .get(&spec.id)
            .map_or(from, |s| registry.routing_confidence(&s));
        if from != to {
            record_promotion(&spec.id, from, to, PromotionEvidence::BenchmarkVerdict);
        }
        confidence.push(serde_json::json!({
            "model_id": spec.id,
            "from": from.as_str(),
            "to": to.as_str(),
            "measured_quality": registry.get(&spec.id).and_then(|s| s.capabilities.quality_prior.map(|p| p.value)),
        }));
        println!(
            "  confidence {:<52} {} -> {}",
            spec.id,
            from.as_str(),
            to.as_str()
        );
    }
    let routes = mode_routing_table(&registry, &MODES, &BENCH_CATEGORIES, 5);
    println!("{}", "Routing after this run (mode x category):".bold());
    for r in &routes {
        println!(
            "  {:<10} {:<18} -> {} [{}{}]",
            r.mode,
            r.category,
            r.model_id.as_deref().unwrap_or("(none)"),
            r.quality_source,
            if r.bench_cases > 0 {
                format!(", {} cases", r.bench_cases)
            } else {
                String::new()
            }
        );
    }

    Ok(serde_json::json!({
        "schema_version": 1,
        "run_id": run_id,
        "max_usd": args.max_usd,
        "estimate_usd": total_est,
        "spent_usd": spent,
        "aborted": aborted,
        "persisted": !args.no_write && db.is_some(),
        "summaries": summaries,
        "confidence": confidence,
        "routes": routes,
        "cases": outcomes,
    }))
}

/// Tier and strengths read off measured suite scores (`vox model classify --measure`).
/// The tier is only proposed when the catalog does not know it; cost tiers (Local/Free)
/// are a pricing fact, not something a benchmark can establish.
#[must_use]
pub fn classify_from_summary(
    s: &ModelSummary,
    catalog_tier: vox_orchestrator::models::ModelTier,
) -> vox_orchestrator::models::autonomic::ClassificationJudgement {
    use vox_orchestrator::models::{ModelTier, StrengthTag};
    const STRONG: f64 = 0.75;
    let suite = |k: &str| s.by_suite.get(k).copied().unwrap_or(0.0);
    let mut strengths = Vec::new();
    if suite("codegen").max(suite("vox-gen")) >= STRONG {
        strengths.push(StrengthTag::Codegen);
    }
    if suite("reasoning") >= STRONG {
        strengths.push(StrengthTag::Logic);
        strengths.push(StrengthTag::Planning);
    }
    if suite("json") >= STRONG {
        strengths.push(StrengthTag::Parsing);
    }
    if suite("needle") >= STRONG {
        strengths.push(StrengthTag::Research);
    }
    if s.mean_score >= 0.6 {
        strengths.push(StrengthTag::Generalist);
    }
    let tier = if catalog_tier != ModelTier::Unknown {
        catalog_tier
    } else if s.mean_score >= 0.85 {
        ModelTier::Pro
    } else if s.mean_score >= 0.6 {
        ModelTier::Fast
    } else {
        ModelTier::Light
    };
    vox_orchestrator::models::autonomic::ClassificationJudgement {
        tier,
        strengths,
        confidence: (s.cases as f32 / (s.cases as f32 + 8.0)).clamp(0.0, 1.0),
        rationale: Some(format!(
            "measured: {}/{} passed, mean score {:.2}, suites {:?}",
            s.passed, s.cases, s.mean_score, s.by_suite
        )),
    }
}

fn print_summary(summaries: &[ModelSummary]) {
    let mut table = comfy_table::Table::new();
    table.set_header(vec![
        "Model",
        "Served as",
        "Pass",
        "Score",
        "TTFT p50",
        "Lat p50/p95",
        "tok/s",
        "Err",
        "$/solve",
        "tok/solve",
        "Ctx",
    ]);
    for s in summaries {
        table.add_row(vec![
            s.model_id.clone(),
            s.served_model
                .clone()
                .filter(|m| *m != s.model_id)
                .unwrap_or_default(),
            format!("{}/{}", s.passed, s.cases),
            format!("{:.2}", s.mean_score),
            s.p50_ttft_ms.map_or("-".into(), |v| format!("{v}ms")),
            format!(
                "{}/{}",
                s.p50_latency_ms.map_or("-".into(), |v| v.to_string()),
                s.p95_latency_ms.map_or("-".into(), |v| v.to_string())
            ),
            s.mean_tokens_per_sec
                .map_or("-".into(), |v| format!("{v:.0}")),
            format!("{:.0}%", s.error_rate * 100.0),
            s.cost_per_solve_usd
                .map_or("-".into(), |v| format!("{v:.5}")),
            s.tokens_per_solve.map_or("-".into(), |v| format!("{v:.0}")),
            s.effective_context
                .clone()
                .unwrap_or_else(|| "-".to_string()),
        ]);
    }
    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(
        model: &str,
        case: &str,
        repeat: u32,
        score: f64,
        lat: u64,
        cost: f64,
        err: Option<&str>,
    ) -> CaseOutcome {
        CaseOutcome {
            model_id: model.into(),
            served_model: Some(format!("{model}-v2")),
            suite: if case.starts_with("needle") {
                "needle"
            } else {
                "reasoning"
            },
            case_id: case.into(),
            repeat,
            score,
            passed: score >= 1.0,
            ttft_ms: err.is_none().then_some(lat / 4),
            latency_ms: lat,
            input_tokens: 100,
            output_tokens: 50,
            tokens_per_sec: err.is_none().then_some(40.0),
            cost_usd: cost,
            cost_reported: true,
            error_class: err.map(str::to_string),
        }
    }

    #[test]
    fn summary_covers_intelligence_responsiveness_efficiency_and_reliability() {
        let rows = vec![
            o("m", "a", 0, 1.0, 100, 0.01, None),
            o("m", "a", 1, 0.0, 300, 0.01, None),
            o("m", "b", 0, 1.0, 200, 0.01, None),
            o("m", "b", 1, 1.0, 400, 0.01, None),
            o("m", "needle-8k", 0, 1.0, 500, 0.0, None),
            o("m", "needle-32k", 0, 0.0, 9000, 0.0, Some("timeout")),
        ];
        let s = &summarize(&rows)[0];
        assert_eq!((s.cases, s.passed), (6, 4));
        assert!((s.error_rate - 1.0 / 6.0).abs() < 1e-9);
        assert_eq!(
            s.p50_latency_ms,
            Some(300),
            "errored calls excluded from latency"
        );
        assert_eq!(s.p95_latency_ms, Some(500));
        assert_eq!(s.p50_ttft_ms, Some(75));
        assert!((s.cost_per_solve_usd.unwrap() - 0.01).abs() < 1e-9);
        assert!((s.tokens_per_solve.unwrap() - 150.0 * 6.0 / 4.0).abs() < 1e-9);
        assert_eq!(s.effective_context.as_deref(), Some("8k"));
        assert_eq!(s.served_model.as_deref(), Some("m-v2"));
        let sd = s.score_stddev.expect("two cases repeated");
        assert!(
            (sd - (0.5f64.sqrt()) / 2.0).abs() < 1e-9,
            "case a sd=0.707, case b sd=0 -> mean {sd}"
        );
        let (lo, hi) = s.pass_ci95.unwrap();
        assert!(lo < s.pass_rate && s.pass_rate < hi);
    }

    #[test]
    fn provider_prefix_synthesizes_a_spec_on_the_right_wire() {
        let reg = ModelRegistry::default();
        let (g, synth) = resolve_target(&reg, "groq:llama-3.3-70b-versatile");
        assert!(synth);
        assert_eq!(g.provider_type, ProviderType::Groq);
        assert_eq!(g.id, "llama-3.3-70b-versatile");
        let (l, _) = resolve_target(&reg, "ollama:qwen3:8b");
        assert_eq!(l.provider_type, ProviderType::Ollama);
        assert_eq!(l.id, "qwen3:8b");
        assert!(l.is_free);
        let (or, _) = resolve_target(&reg, "meta-llama/llama-3.3-70b-instruct:free");
        assert_eq!(or.provider_type, ProviderType::OpenRouter);
        assert_eq!(or.id, "meta-llama/llama-3.3-70b-instruct:free");
    }

    #[test]
    fn classification_follows_suite_scores_and_keeps_known_tiers() {
        use vox_orchestrator::models::{ModelTier, StrengthTag};
        let mut s = summarize(&[o("m", "a", 0, 1.0, 100, 0.0, None)])[0].clone();
        s.by_suite = [("codegen".to_string(), 0.9), ("reasoning".to_string(), 0.5)].into();
        s.mean_score = 0.7;
        s.cases = 8;
        let j = classify_from_summary(&s, ModelTier::Unknown);
        assert_eq!(j.tier, ModelTier::Fast);
        assert!(j.strengths.contains(&StrengthTag::Codegen));
        assert!(!j.strengths.contains(&StrengthTag::Logic));
        assert!(j.strengths.contains(&StrengthTag::Generalist));
        assert!((j.confidence - 0.5).abs() < 1e-6);
        assert_eq!(
            classify_from_summary(&s, ModelTier::Free).tier,
            ModelTier::Free
        );
    }

    #[test]
    fn latest_picks_current_alias_targets_cheapest_first_and_skips_explicit_ones() {
        let reg = ModelRegistry::default();
        let spec = |id: &str, alias: bool, per_1k: f64| {
            let (mut s, _) = resolve_target(&reg, &format!("openrouter:{id}"));
            s.capabilities.is_alias_target = alias;
            s.cost_per_1k_input = per_1k;
            s.cost_per_1k_output = per_1k;
            s.cost_per_1k = per_1k;
            s.pricing_source = vox_orchestrator::models::PricingSource::OpenRouter;
            s
        };
        let models = vec![
            spec("acme/pricey-9", true, 0.05),
            spec("acme/cheap-3", true, 0.001),
            spec("acme/old-2", false, 0.0001),
            spec("acme/mid-5", true, 0.01),
        ];
        let already = vec![(spec("acme/mid-5", true, 0.01), false)];
        let ids: Vec<String> = latest_alias_targets(models.clone(), &already, 5, |_| true)
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(ids, ["acme/cheap-3", "acme/pricey-9"]);
        assert_eq!(
            latest_alias_targets(models.clone(), &[], 1, |_| true)[0].id,
            "acme/cheap-3"
        );
        assert!(latest_alias_targets(models, &[], 5, |_| false).is_empty());
    }

    #[test]
    fn unsaved_run_evidence_carries_counts_rates_and_alias_target() {
        let s = ModelSummary {
            model_id: "~acme/x-latest".into(),
            served_model: Some("acme/x-3".into()),
            cases: 17,
            passed: 16,
            pass_rate: 16.0 / 17.0,
            pass_ci95: None,
            mean_score: 0.95,
            by_suite: BTreeMap::new(),
            error_rate: 2.0 / 17.0,
            p50_ttft_ms: Some(900),
            p50_latency_ms: Some(1500),
            p95_latency_ms: Some(4000),
            mean_tokens_per_sec: Some(80.0),
            total_cost_usd: 0.01,
            cost_per_solve_usd: Some(0.000_625),
            tokens_per_solve: Some(300.0),
            score_stddev: None,
            effective_context: None,
        };
        let ev = evidence_from_summary(&s);
        assert_eq!((ev.cases, ev.passed, ev.errors), (17, 16, 2));
        assert_eq!(ev.resolved_model_id.as_deref(), Some("acme/x-3"));
        assert_eq!((ev.p50_ttft_ms, ev.p95_latency_ms), (Some(900), Some(4000)));
        assert_eq!(ev.verdict(), Some(ModelConfidence::Confirmed));
    }

    #[test]
    fn error_classes_separate_rate_limits_auth_and_timeouts() {
        assert_eq!(
            classify_error("rate limit exceeded, try again in 3s"),
            "rate_limited"
        );
        assert_eq!(classify_error("HTTP status 401: invalid api key"), "auth");
        assert_eq!(classify_error("operation timed out"), "timeout");
        assert_eq!(classify_error("connection reset by peer"), "transport");
    }
}
