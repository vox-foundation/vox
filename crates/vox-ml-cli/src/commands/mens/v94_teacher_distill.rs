//! MIT / Apache-2.0 frontier teacher distillation via `vox_actor_runtime::llm`
//! (`FacadeBackend`) with execution-verified harness checking, single-retry
//! compiler/assertion repair, strict `$3.00` USD budget cap, and v1/v2 leakage
//! filtering.

use anyhow::{Context, Result, bail};
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;

use super::super::eval_gate::leakage::{
    BenchTask, leaked_bench_task, leaked_bench_task_contained, load_bench_answers,
};
use super::super::metrics::verify_completion_with_harness;
use super::repo_root;
use crate::commands::corpus::synth::{
    ChatBackend, FacadeBackend, SynthLlmArgs, clean_code, estimate_usd,
};

const HARD_BUDGET_CAP_USD: f64 = 3.00;
const STOP_SPAWN_USD: f64 = 2.85;
const FALLBACK_USD_PER_1K: f64 = 0.002;
const TEACHER_SYSTEM_PROMPT: &str = "\
You are an expert Vox programmer. Write clean, idiomatic, complete Vox code matching the exact specification and declared names/signatures.\n\
Critical Vox syntax rules:\n\
- Functions use `to` for return types: `fn foo(a: int, b: str) to int { ... }`.\n\
- Query/mutation endpoints use `query foo(...) to T { ... }` or `mutation foo(...) to T { ... }` (NEVER `@query` or `@mutation` decorators).\n\
- Algebraic types: `type Foo =\\n    | A(x: int)\\n    | B` and `match v { A(x) => { ... } B => { ... } }`.\n\
- Booleans: `and`, `or`, `not` (NEVER `&&`, `||`, `!`). Comparisons: `==` or `is`.\n\
- Conditionals: ALWAYS use braces `if cond { ... } else { ... }`. NEVER use `if ... then ... else ...`.\n\
- Data structures & loops: NEVER use hashmaps/dicts, tuple destructuring `for (k, v) in ...`, `.items()`, or Python syntax. Use `while i < xs.len() { ... i = i + 1 }`, `for x in xs { ... }`, and parallel `list[int]` / `list[str]` when frequency/KV mapping is needed.\n\
- Mutable locals: `let mut x: int = 0` (always type-annotate empty lists: `let mut xs: list[int] = []`). Always write explicit `return val` inside functions.\n\
- Lists: `list[int]`, `list[str]`, `list[list[int]]`; indexing yields Option so use `xs[i].unwrap()`; push returns a new list `xs = xs.push(val)`; remove uses `xs = xs.remove_at(i)`.\n\
- Loops: use `while i < xs.len() { ... i = i + 1 }` or `for x in xs { ... }`.\n\
- Strings: `.len()`, `.chars()`, `.chars_count()`, `.split(sep)`, `.join(sep)`, `.trim()`, `.to_lower()`, `.to_upper()`, `.slice(start, end)`, `.starts_with(p)`, `.ends_with(s)`, `.contains(sub)`, `.ord()`, `.is_digit()`, `.is_alnum()`, `.to_int().unwrap()`.\n\
- Output ONLY raw Vox source code. No prose, no explanations, no TODO/stubs.";

struct DistillSeed {
    id: String,
    category: String,
    spec_prose: String,
    signature: String,
    expected_contains: Vec<String>,
    harness: String,
    require_exact_types: bool,
}

fn seed_prompt(seed: &DistillSeed, idx: usize) -> String {
    if seed.require_exact_types || idx.is_multiple_of(2) {
        format!(
            "Implement in Vox:\n\n{}\n\nRequired declarations/signatures:\n{}",
            seed.spec_prose, seed.signature
        )
    } else {
        let sig_summary = seed
            .signature
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "{}\n\nExact Vox signature contract: {sig_summary}.\nReturn only valid Vox code.",
            seed.spec_prose
        )
    }
}

fn license_class_for_model(slug: &str) -> &'static str {
    let lo = slug.to_ascii_lowercase();
    if lo.contains("deepseek") || lo.contains("kimi") {
        "MIT"
    } else if lo.contains("qwen")
        || lo.contains("mistral")
        || lo.contains("glm")
        || lo.contains("minimax")
    {
        "Apache-2.0"
    } else {
        "open-weights-configured"
    }
}

fn is_mit_or_apache_slug(slug: &str) -> bool {
    let lo = slug.to_ascii_lowercase();
    lo.contains("deepseek")
        || lo.contains("qwen")
        || lo.contains("mistral")
        || lo.contains("glm")
        || lo.contains("kimi")
        || lo.contains("minimax")
}

fn candidate_models() -> Vec<String> {
    let prefs = vox_actor_runtime::model_resolution::RouteResolutionInput::default();
    let mut cands = Vec::new();
    // Prefer explicitly MIT frontier coder models first per CAP-27B-02 decision.
    for fixed in ["deepseek/deepseek-chat", "qwen/qwen3-coder"] {
        cands.push(fixed.to_string());
    }
    let cfg = prefs.openrouter_model.trim();
    if !cfg.is_empty() && is_mit_or_apache_slug(cfg) && !cands.iter().any(|x| x == cfg) {
        cands.push(cfg.to_string());
    }
    if !cfg.is_empty() && !cands.iter().any(|x| x == cfg) {
        cands.push(cfg.to_string());
    }
    cands
}

fn build_facade(model: &str) -> Result<FacadeBackend> {
    let args = SynthLlmArgs {
        provider: "openrouter".to_string(),
        model: Some(model.to_string()),
        base_url: None,
        max_spend_usd: HARD_BUDGET_CAP_USD,
        usd_per_1k_tokens: FALLBACK_USD_PER_1K,
        apply: true,
    };
    FacadeBackend::from_args(&args, 1024)
}

fn effective_call_cost(reported: f64, sys: &str, user: &str, reply_text: &str) -> f64 {
    let est = estimate_usd(
        sys.len() + user.len(),
        reply_text.len().div_ceil(4).max(16),
        FALLBACK_USD_PER_1K,
    );
    if reported > 0.0 {
        reported.max(est * 0.25)
    } else {
        est
    }
}

async fn verify_async(
    code: String,
    expected: Vec<String>,
    harness: String,
    id: String,
    idx: usize,
) -> Result<super::super::metrics::CompletionVerification> {
    tokio::task::spawn_blocking(move || {
        let tmp = tempfile::tempdir()?;
        Ok(verify_completion_with_harness(
            &code,
            tmp.path(),
            "",
            &id,
            idx,
            &expected,
            Some(&harness),
        ))
    })
    .await
    .context("verification spawn_blocking panicked")?
}

fn extract_diagnostic_summary(checks: &serde_json::Value) -> String {
    let mut parts = Vec::new();
    if let Some(pe) = checks["parse_error"].as_str() {
        parts.push(format!("parse_or_typecheck_error: {pe}"));
    }
    if let Some(he) = checks["harness_error"].as_str() {
        parts.push(format!("harness_exec_error: {he}"));
    }
    if let Some(de) = checks["diag_errors"].as_u64()
        && de > 0
        && parts.is_empty()
    {
        parts.push(format!("compiler_diag_errors={de}"));
    }
    if checks["pass_exec"].as_bool() == Some(false) && parts.is_empty() {
        parts.push("execution assertion failed inside __vox_eval_harness".to_string());
    }
    if checks["semantic_pass"].as_bool() == Some(false) {
        parts.push("missing required declaration name from prompt signature".to_string());
    }
    if parts.is_empty() {
        "did not satisfy compile + execution + anti-stub check".to_string()
    } else {
        parts.join("; ")
    }
}

fn check_bench_leakage(code: &str, bench: &[BenchTask]) -> Option<String> {
    if let Some((tid, _)) = leaked_bench_task(code, bench) {
        return Some(format!("jaccard:{tid}"));
    }
    if let Some((tid, _)) = leaked_bench_task_contained(bench, code) {
        return Some(format!("contained:{tid}"));
    }
    None
}

async fn probe_working_backend() -> Result<(Arc<FacadeBackend>, String, String, f64)> {
    let mut errors = Vec::new();
    let probe_user = "Write a Vox function `fn ping_double(n: int) to int` returning `n * 2`. Return raw Vox code only.";
    for model in candidate_models() {
        let backend = match build_facade(&model) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("{model}: build_facade: {e}"));
                continue;
            }
        };
        match backend.chat(TEACHER_SYSTEM_PROMPT, probe_user, 0.0).await {
            Ok(reply) if !reply.text.trim().is_empty() => {
                let cost = effective_call_cost(
                    reply.cost_usd,
                    TEACHER_SYSTEM_PROMPT,
                    probe_user,
                    &reply.text,
                );
                let resolved = if reply.model.trim().is_empty() {
                    model.clone()
                } else {
                    reply.model.clone()
                };
                let lic = license_class_for_model(&resolved).to_string();
                eprintln!(
                    "TEACHER_PROBE_OK: requested={model} resolved={resolved} license={lic} cost_usd={cost:.5}"
                );
                return Ok((Arc::new(backend), resolved, lic, cost));
            }
            Ok(_) => {
                errors.push(format!("{model}: empty probe reply"));
            }
            Err(e) => {
                errors.push(format!("{model}: {e}"));
            }
        }
    }
    bail!(
        "all candidate OpenRouter teacher models failed probe:\n{}",
        errors.join("\n")
    );
}

async fn solve_one_seed(
    backend: Arc<FacadeBackend>,
    seed: DistillSeed,
    idx: usize,
    allow_repair: bool,
    bench: Arc<Vec<BenchTask>>,
    model_used: String,
    license_class: String,
) -> SeedAttemptResult {
    let prompt = seed_prompt(&seed, idx);
    let reply = match backend.chat(TEACHER_SYSTEM_PROMPT, &prompt, 0.2).await {
        Ok(r) => r,
        Err(e) => {
            return SeedAttemptResult::Error {
                id: seed.id,
                err: e.to_string(),
                cost_usd: 0.0,
            };
        }
    };
    let mut total_cost =
        effective_call_cost(reply.cost_usd, TEACHER_SYSTEM_PROMPT, &prompt, &reply.text);
    let mut code = clean_code(&reply.text);
    let mut repairs = 0usize;

    let mut ver = match verify_async(
        code.clone(),
        seed.expected_contains.clone(),
        seed.harness.clone(),
        seed.id.clone(),
        idx,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return SeedAttemptResult::Error {
                id: seed.id,
                err: e.to_string(),
                cost_usd: total_cost,
            };
        }
    };

    if !(ver.pass && ver.pass_compile && ver.pass_exec == Some(true) && ver.anti_stub_pass)
        && allow_repair
    {
        repairs = 1;
        let diag = extract_diagnostic_summary(&ver.checks);
        let repair_prompt = format!(
            "Fix this Vox code so it compiles cleanly and passes all behavioral requirements.\n\nSpecification:\n{}\n\nSignature:\n{}\n\nPrevious attempt:\n{}\n\nObserved failure: {}\n\nReturn ONLY corrected valid Vox code.",
            seed.spec_prose, seed.signature, code, diag
        );
        if let Ok(r2) = backend
            .chat(TEACHER_SYSTEM_PROMPT, &repair_prompt, 0.1)
            .await
        {
            total_cost +=
                effective_call_cost(r2.cost_usd, TEACHER_SYSTEM_PROMPT, &repair_prompt, &r2.text);
            let candidate_code = clean_code(&r2.text);
            if let Ok(v2) = verify_async(
                candidate_code.clone(),
                seed.expected_contains.clone(),
                seed.harness.clone(),
                seed.id.clone(),
                idx,
            )
            .await
            {
                code = candidate_code;
                ver = v2;
            }
        }
    }

    if !(ver.pass && ver.pass_compile && ver.pass_exec == Some(true) && ver.anti_stub_pass) {
        return SeedAttemptResult::RejectedCompile {
            id: seed.id,
            cost_usd: total_cost,
            repairs,
            diag: extract_diagnostic_summary(&ver.checks),
        };
    }

    if let Some(reason) = check_bench_leakage(&code, &bench) {
        return SeedAttemptResult::RejectedLeakage {
            id: seed.id,
            cost_usd: total_cost,
            reason,
        };
    }

    SeedAttemptResult::Accepted {
        row: json!({
            "id": seed.id,
            "prompt": prompt,
            "response": code,
            "output": code,
            "category": seed.category,
            "source": format!("mit_teacher_rft_v94/{}.vox", seed.id),
            "origin": "mit_teacher_rft_v94",
            "teacher_model": model_used,
            "teacher_license": license_class,
            "expected_contains": seed.expected_contains,
            "test_harness": seed.harness,
            "verified_exec": true,
            "repair_attempts": repairs,
            "cost_usd": total_cost,
            "rating": 5,
            "schema_version": "vox_dogfood_v1",
            "lane": "vox_codegen",
            "response_mode": "code_only",
            "task_family": "vox_codegen",
        }),
        cost_usd: total_cost,
        repairs,
    }
}

enum SeedAttemptResult {
    Accepted {
        row: serde_json::Value,
        cost_usd: f64,
        repairs: usize,
    },
    RejectedCompile {
        id: String,
        cost_usd: f64,
        repairs: usize,
        diag: String,
    },
    RejectedLeakage {
        id: String,
        cost_usd: f64,
        reason: String,
    },
    Error {
        id: String,
        err: String,
        cost_usd: f64,
    },
}

fn fmt_i(xs: &[i64]) -> String {
    let p: Vec<String> = xs.iter().map(|v| v.to_string()).collect();
    format!("[{}]", p.join(", "))
}

fn fmt_s(xs: &[&str]) -> String {
    let p: Vec<String> = xs
        .iter()
        .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect();
    format!("[{}]", p.join(", "))
}

fn fmt_m(m: &[Vec<i64>]) -> String {
    let p: Vec<String> = m.iter().map(|r| fmt_i(r)).collect();
    format!("[{}]", p.join(", "))
}

fn build_teacher_seed_bank() -> Vec<DistillSeed> {
    let mut seeds = Vec::with_capacity(180);

    // 1. data_structures (25 varied hard problems)
    for k in 0..25usize {
        let cap = 3 + (k as i64 % 4);
        let weight = 2 + (k as i64 % 5);
        match k % 5 {
            0 => {
                let fname = format!("td_deque_sliding_{k}");
                let win = 2 + (k as i64 % 3);
                let oracle = |xs: &[i64]| -> Vec<i64> {
                    if xs.len() < win as usize {
                        return vec![];
                    }
                    xs.windows(win as usize)
                        .map(|w| *w.iter().min().unwrap() * weight)
                        .collect()
                };
                let t1 = [9, 3, 7, 1, 8, 5];
                let t2 = [4, -2, 6];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "data_structures".into(),
                    spec_prose: format!(
                        "Implement `{fname}(xs)` returning `{weight}` times the minimum of each contiguous `{win}`-element window across `xs` (empty list if fewer than `{win}` elements)."
                    ),
                    signature: format!("fn {fname}(xs: list[int]) to list[int]"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == [])\n    return \"ok\"\n}}",
                        fmt_i(&t1),
                        fmt_i(&oracle(&t1)),
                        fmt_i(&t2),
                        fmt_i(&oracle(&t2))
                    ),
                    require_exact_types: false,
                });
            }
            1 => {
                let fname = format!("td_priority_bag_{k}");
                let oracle = |ops: &[&str]| -> i64 {
                    let mut bag: Vec<i64> = Vec::new();
                    let mut popped = 0i64;
                    for &op in ops {
                        if let Some(rest) = op.strip_prefix("ins:") {
                            bag.push(rest.parse::<i64>().unwrap_or(0));
                        } else if op == "pop_min" && !bag.is_empty() {
                            let (mi, &mv) =
                                bag.iter().enumerate().min_by_key(|(_, v)| **v).unwrap();
                            bag.remove(mi);
                            popped += mv;
                        }
                    }
                    popped * weight + bag.iter().sum::<i64>()
                };
                let o1 = ["ins:8", "ins:2", "ins:5", "pop_min", "ins:1", "pop_min"];
                let o2 = ["pop_min", "ins:7"];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "data_structures".into(),
                    spec_prose: format!(
                        "Implement `{fname}(ops)` maintaining a min-bag supporting `\"ins:<n>\"` and `\"pop_min\"` (sum popped minimums), returning `popped_sum * {weight} + remaining_sum`."
                    ),
                    signature: format!("fn {fname}(ops: list[str]) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == 0)\n    return \"ok\"\n}}",
                        fmt_s(&o1),
                        oracle(&o1),
                        fmt_s(&o2),
                        oracle(&o2)
                    ),
                    require_exact_types: false,
                });
            }
            2 => {
                let fname = format!("td_ring_fifo_{k}");
                let oracle = |ops: &[&str]| -> i64 {
                    let mut q = Vec::new();
                    let mut dropped = 0i64;
                    for &op in ops {
                        if let Some(rest) = op.strip_prefix("enq:") {
                            let v = rest.parse::<i64>().unwrap_or(0);
                            if q.len() < cap as usize {
                                q.push(v);
                            } else {
                                dropped += 1;
                            }
                        } else if op == "deq" && !q.is_empty() {
                            q.remove(0);
                        }
                    }
                    q.iter().sum::<i64>() + dropped * weight
                };
                let o1 = ["enq:5", "enq:6", "enq:7", "enq:8", "enq:9", "deq", "enq:4"];
                let o2 = ["deq", "enq:11"];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "data_structures".into(),
                    spec_prose: format!(
                        "Implement `{fname}(ops)` simulating a bounded FIFO queue of capacity `{cap}` (`\"enq:<n>\"` drops when full and increments `dropped`, `\"deq\"` pops front if non-empty), returning `sum(remaining) + dropped * {weight}`."
                    ),
                    signature: format!("fn {fname}(ops: list[str]) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == 0)\n    return \"ok\"\n}}",
                        fmt_s(&o1),
                        oracle(&o1),
                        fmt_s(&o2),
                        oracle(&o2)
                    ),
                    require_exact_types: false,
                });
            }
            3 => {
                let fname = format!("td_freq_mode_{k}");
                let oracle = |xs: &[i64]| -> i64 {
                    if xs.is_empty() {
                        return 0;
                    }
                    let mut counts = BTreeMap::<i64, i64>::new();
                    for &v in xs {
                        *counts.entry(v).or_default() += 1;
                    }
                    let (&best_k, _) = counts
                        .iter()
                        .max_by(|(k1, c1), (k2, c2)| c1.cmp(c2).then_with(|| k2.cmp(k1)))
                        .unwrap();
                    best_k
                };
                let t1 = [4, 2, 4, 2, 9, 4];
                let t2 = [7, 3, 7, 3];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "data_structures".into(),
                    spec_prose: format!(
                        "Implement `{fname}(xs)` returning the most frequent integer in `xs` (break ties in favor of the smallest integer value; `0` if empty)."
                    ),
                    signature: format!("fn {fname}(xs: list[int]) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == 0)\n    return \"ok\"\n}}",
                        fmt_i(&t1),
                        oracle(&t1),
                        fmt_i(&t2),
                        oracle(&t2)
                    ),
                    require_exact_types: false,
                });
            }
            _ => {
                let fname = format!("td_span_stack_{k}");
                let oracle = |prices: &[i64]| -> Vec<i64> {
                    (0..prices.len())
                        .map(|i| {
                            let mut sp = 1i64;
                            let mut j = i as isize - 1;
                            while j >= 0 && prices[j as usize] <= prices[i] {
                                sp += 1;
                                j -= 1;
                            }
                            sp
                        })
                        .collect()
                };
                let t1 = [100, 80, 60, 70, 60, 75, 85];
                let t2 = [10, 20, 30];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "data_structures".into(),
                    spec_prose: format!(
                        "Implement `{fname}(prices)` computing the daily span for each day `i`: count of consecutive previous days (including `i`) where `prices[j] <= prices[i]`."
                    ),
                    signature: format!("fn {fname}(prices: list[int]) to list[int]"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == [])\n    return \"ok\"\n}}",
                        fmt_i(&t1),
                        fmt_i(&oracle(&t1)),
                        fmt_i(&t2),
                        fmt_i(&oracle(&t2))
                    ),
                    require_exact_types: false,
                });
            }
        }
    }

    // 2. endpoint_handler (25 query & mutation backend specifications)
    for k in 0..25usize {
        let base = 200 + (k as i64 % 5) * 100;
        let rate = 5 + (k as i64 % 6) * 3;
        let tier = 50 + (k as i64 % 4) * 25;
        match k % 2 {
            0 => {
                let qname = format!("td_api_invoice_quote_{k}");
                let eval = |seats: i64, annual: bool| -> i64 {
                    if seats <= 0 {
                        return 0;
                    }
                    let mut cost = base + seats * rate;
                    if seats > tier {
                        cost -= (seats - tier) * 2;
                    }
                    if annual { cost - cost / 5 } else { cost }
                };
                seeds.push(DistillSeed {
                    id: qname.clone(),
                    category: "endpoint_handler".into(),
                    spec_prose: format!(
                        "Define a Vox `query` `{qname}(seats: int, annual: bool) to int`: `0` when `seats <= 0`; else `{base} + seats * {rate}` minus a volume discount of `2 * (seats - {tier})` for seats strictly over `{tier}`; if `annual` is true, subtract `cost / 5` (20% discount)."
                    ),
                    signature: format!("query {qname}(seats: int, annual: bool) to int"),
                    expected_contains: vec![format!("query {qname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({qname}(0, true) == 0)\n    assert({qname}(10, false) == {})\n    assert({qname}({}, true) == {})\n    return \"ok\"\n}}",
                        eval(10, false),
                        tier + 20,
                        eval(tier + 20, true)
                    ),
                    require_exact_types: true,
                });
            }
            _ => {
                let mname = format!("td_api_credit_mutate_{k}");
                let eval = |bal: i64, delta: i64, hold: bool| -> i64 {
                    if hold || delta == 0 {
                        return bal.max(0);
                    }
                    let nxt = bal + delta;
                    if delta < 0 && nxt < 0 {
                        return bal.max(0);
                    }
                    nxt.clamp(0, base * 4)
                };
                seeds.push(DistillSeed {
                    id: mname.clone(),
                    category: "endpoint_handler".into(),
                    spec_prose: format!(
                        "Define a Vox `mutation` `{mname}(balance: int, delta: int, on_hold: bool) to int`: if `on_hold` is true or a debit (`delta < 0`) would drop balance below `0`, keep `max(0, balance)`; otherwise apply `balance + delta` clamped into `[0, {}]`.",
                        base * 4
                    ),
                    signature: format!(
                        "mutation {mname}(balance: int, delta: int, on_hold: bool) to int"
                    ),
                    expected_contains: vec![format!("mutation {mname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({mname}(100, -150, false) == 100)\n    assert({mname}(100, 50, true) == 100)\n    assert({mname}({}, 500, false) == {})\n    return \"ok\"\n}}",
                        base * 4 - 100,
                        eval(base * 4 - 100, 500, false)
                    ),
                    require_exact_types: true,
                });
            }
        }
    }

    // 3. matrix_grid (25 non-trivial matrix algorithms)
    for k in 0..25usize {
        let th = 1 + (k as i64 % 3);
        match k % 3 {
            0 => {
                let fname = format!("td_grid_local_maxima_{k}");
                let oracle = |g: &[Vec<i64>]| -> i64 {
                    if g.is_empty() {
                        return 0;
                    }
                    let (r, c) = (g.len(), g[0].len());
                    let mut hits = 0i64;
                    for i in 0..r {
                        for j in 0..c {
                            let v = g[i][j];
                            let ok = (i == 0 || v > g[i - 1][j])
                                && (i + 1 == r || v > g[i + 1][j])
                                && (j == 0 || v > g[i][j - 1])
                                && (j + 1 == c || v > g[i][j + 1]);
                            if ok && v >= th {
                                hits += 1;
                            }
                        }
                    }
                    hits
                };
                let g1 = vec![vec![5, 2, 8], vec![1, 9, 4], vec![6, 3, 7]];
                let g2 = vec![vec![0, 0], vec![0, 0]];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "matrix_grid".into(),
                    spec_prose: format!(
                        "Implement `{fname}(grid)` counting cells `grid[i][j] >= {th}` that are strictly greater than all existing 4-orthogonal neighbors."
                    ),
                    signature: format!("fn {fname}(grid: list[list[int]]) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == 0)\n    assert({fname}([]) == 0)\n    return \"ok\"\n}}",
                        fmt_m(&g1),
                        oracle(&g1),
                        fmt_m(&g2)
                    ),
                    require_exact_types: false,
                });
            }
            1 => {
                let fname = format!("td_grid_row_col_equal_{k}");
                let oracle = |g: &[Vec<i64>]| -> i64 {
                    let n = g.len();
                    let mut pairs = 0i64;
                    for i in 0..n {
                        for j in 0..n {
                            if g[i].len() == n && (0..n).all(|idx| g[i][idx] == g[idx][j]) {
                                pairs += 1;
                            }
                        }
                    }
                    pairs
                };
                let g1 = vec![vec![3, 2, 1], vec![1, 7, 6], vec![2, 7, 7]];
                let g2 = vec![vec![1, 2], vec![2, 1]];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "matrix_grid".into(),
                    spec_prose: format!(
                        "Implement `{fname}(grid)` for an `n x n` square matrix counting pairs `(r, c)` where row `r` equals column `c` element-by-element (`0` if empty)."
                    ),
                    signature: format!("fn {fname}(grid: list[list[int]]) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == 0)\n    return \"ok\"\n}}",
                        fmt_m(&g1),
                        oracle(&g1),
                        fmt_m(&g2),
                        oracle(&g2)
                    ),
                    require_exact_types: false,
                });
            }
            _ => {
                let fname = format!("td_grid_zero_cross_{k}");
                let oracle = |g: &[Vec<i64>]| -> Vec<Vec<i64>> {
                    if g.is_empty() {
                        return vec![];
                    }
                    let (r, c) = (g.len(), g[0].len());
                    let z_rows: Vec<bool> = (0..r).map(|i| g[i].contains(&0)).collect();
                    let z_cols: Vec<bool> = (0..c).map(|j| (0..r).any(|i| g[i][j] == 0)).collect();
                    (0..r)
                        .map(|i| {
                            (0..c)
                                .map(|j| if z_rows[i] || z_cols[j] { 0 } else { g[i][j] })
                                .collect()
                        })
                        .collect()
                };
                let g1 = vec![vec![1, 2, 3], vec![4, 0, 6], vec![7, 8, 9]];
                let g2 = vec![vec![5, 6], vec![7, 8]];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "matrix_grid".into(),
                    spec_prose: format!(
                        "Implement `{fname}(grid)` returning a new matrix where if any cell in original row `r` or column `c` is `0`, every cell in row `r` and column `c` becomes `0`."
                    ),
                    signature: format!("fn {fname}(grid: list[list[int]]) to list[list[int]]"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == [])\n    return \"ok\"\n}}",
                        fmt_m(&g1),
                        fmt_m(&oracle(&g1)),
                        fmt_m(&g2),
                        fmt_m(&oracle(&g2))
                    ),
                    require_exact_types: false,
                });
            }
        }
    }

    // 4. string_parsing (25 parser/formatter problems)
    for k in 0..25usize {
        let shift = 1 + (k as i64 % 5);
        match k % 3 {
            0 => {
                let fname = format!("td_str_rle_len_{k}");
                let oracle = |s: &str| -> String {
                    let chars: Vec<char> = s.chars().collect();
                    if chars.is_empty() {
                        return String::new();
                    }
                    let mut out = String::new();
                    let mut i = 0;
                    while i < chars.len() {
                        let ch = chars[i];
                        let mut j = i + 1;
                        while j < chars.len() && chars[j] == ch {
                            j += 1;
                        }
                        out.push(ch);
                        if j - i > 1 {
                            out.push_str(&(j - i).to_string());
                        }
                        i = j;
                    }
                    out
                };
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "string_parsing".into(),
                    spec_prose: format!(
                        "Implement `{fname}(s)` run-length encoding consecutive identical characters by appending decimal count ONLY when count >= 2 (e.g., `\"aaabbc\" -> \"a3b2c\"`)."
                    ),
                    signature: format!("fn {fname}(s: str) to str"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}(\"aaabbc\") == \"{}\")\n    assert({fname}(\"xyz\") == \"xyz\")\n    assert({fname}(\"\") == \"\")\n    return \"ok\"\n}}",
                        oracle("aaabbc")
                    ),
                    require_exact_types: false,
                });
            }
            1 => {
                let fname = format!("td_str_eval_addsub_{k}");
                let oracle = |expr: &str| -> i64 {
                    let mut acc = 0i64;
                    let mut sign = 1i64;
                    for tok in expr.split_whitespace() {
                        match tok {
                            "+" => sign = 1,
                            "-" => sign = -1,
                            n => acc += sign * n.parse::<i64>().unwrap_or(0),
                        }
                    }
                    acc + shift
                };
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "string_parsing".into(),
                    spec_prose: format!(
                        "Implement `{fname}(expr)` evaluating space-separated integer addition/subtraction (e.g., `\"10 - 3 + 5\"`) and adding offset `{shift}` (return `{shift}` on empty input)."
                    ),
                    signature: format!("fn {fname}(expr: str) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}(\"10 - 3 + 5\") == {})\n    assert({fname}(\"-4 + 9 - 2\") == {})\n    assert({fname}(\"\") == {shift})\n    return \"ok\"\n}}",
                        oracle("10 - 3 + 5"),
                        oracle("-4 + 9 - 2")
                    ),
                    require_exact_types: false,
                });
            }
            _ => {
                let fname = format!("td_str_redact_digits_{k}");
                let oracle = |s: &str| -> String {
                    let cs: Vec<char> = s.chars().collect();
                    let keep = cs.len().saturating_sub(shift as usize);
                    cs.into_iter()
                        .enumerate()
                        .map(|(i, c)| {
                            if i < keep && c.is_ascii_alphanumeric() {
                                '*'
                            } else {
                                c
                            }
                        })
                        .collect()
                };
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "string_parsing".into(),
                    spec_prose: format!(
                        "Implement `{fname}(s)` masking alphanumeric characters with `'*'` EXCEPT in the last `{shift}` character positions (preserve non-alphanumeric punctuation everywhere)."
                    ),
                    signature: format!("fn {fname}(s: str) to str"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}(\"ab-123456\") == \"{}\")\n    assert({fname}(\"hi\") == \"hi\")\n    assert({fname}(\"\") == \"\")\n    return \"ok\"\n}}",
                        oracle("ab-123456")
                    ),
                    require_exact_types: false,
                });
            }
        }
    }

    // 5. state_machine (20 non-trivial FSMs)
    for k in 0..20usize {
        let max_fail = 2 + (k as i64 % 3);
        let probe_ok = 2 + ((k / 3) as i64 % 2);
        let fname = format!("td_fsm_cb_exec_{k}");
        let oracle = |events: &[&str]| -> i64 {
            // 0=Closed, 1=Open, 2=HalfOpen
            let (mut st, mut fails, mut ok_streak, mut allowed) = (0i64, 0i64, 0i64, 0i64);
            for &ev in events {
                if st == 0 {
                    allowed += 1;
                    if ev == "ok" {
                        fails = 0;
                    } else if ev == "fail" {
                        fails += 1;
                        if fails >= max_fail {
                            st = 1;
                            fails = 0;
                        }
                    }
                } else if st == 1 {
                    if ev == "probe" {
                        st = 2;
                        ok_streak = 0;
                    }
                } else {
                    if ev == "ok" {
                        allowed += 1;
                        ok_streak += 1;
                        if ok_streak >= probe_ok {
                            st = 0;
                            fails = 0;
                        }
                    } else if ev == "fail" {
                        st = 1;
                    }
                }
            }
            allowed
        };
        let e1 = [
            "ok", "fail", "fail", "fail", "ok", "probe", "ok", "ok", "ok",
        ];
        let e2 = ["fail", "fail", "fail", "probe", "fail", "ok"];
        seeds.push(DistillSeed {
            id: fname.clone(),
            category: "state_machine".into(),
            spec_prose: format!(
                "Implement `{fname}(events)` simulating a 3-state circuit breaker (`Closed`, `Open`, `HalfOpen`) over `\"ok\"`, `\"fail\"`, `\"probe\"`: start `Closed`; `{max_fail}` consecutive `\"fail\"`s trip `Closed -> Open` (every `\"ok\"`/`\"fail\"` arrived while `Closed` counts as an allowed call); `Open` rejects `\"ok\"`/`\"fail\"`, and `\"probe\"` shifts `Open -> HalfOpen`; in `HalfOpen`, count `\"ok\"` as allowed and recover to `Closed` after `{probe_ok}` consecutive `\"ok\"`s, while any `\"fail\"` re-opens to `Open`. Return total allowed calls."
            ),
            signature: format!("fn {fname}(events: list[str]) to int"),
            expected_contains: vec![format!("fn {fname}")],
            harness: format!(
                "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == 0)\n    return \"ok\"\n}}",
                fmt_s(&e1),
                oracle(&e1),
                fmt_s(&e2),
                oracle(&e2)
            ),
            require_exact_types: false,
        });
    }

    // 6. algorithm (20 non-trivial DP / two-pointer / greedy problems)
    for k in 0..20usize {
        let cap = 7 + (k as i64 % 5) * 2;
        match k % 2 {
            0 => {
                let fname = format!("td_algo_max_trap_{k}");
                let step = 1 + (k as i64 % 3);
                let oracle = |hs: &[i64]| -> i64 {
                    if hs.len() < 3 {
                        return 0;
                    }
                    let n = hs.len();
                    let mut lmax = vec![0i64; n];
                    let mut rmax = vec![0i64; n];
                    lmax[0] = hs[0];
                    for i in 1..n {
                        lmax[i] = lmax[i - 1].max(hs[i]);
                    }
                    rmax[n - 1] = hs[n - 1];
                    for i in (0..n - 1).rev() {
                        rmax[i] = rmax[i + 1].max(hs[i]);
                    }
                    let mut water = 0i64;
                    for i in 0..n {
                        let w = lmax[i].min(rmax[i]) - hs[i];
                        if w > 0 {
                            water += w;
                        }
                    }
                    water * step
                };
                let t1 = [0, 1, 0, 2, 1, 0, 1, 3, 2, 1, 2, 1];
                let t2 = [4, 2, 0, 3, 2, 5];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "algorithm".into(),
                    spec_prose: format!(
                        "Implement `{fname}(heights)` computing rainwater units trapped between elevation bars `heights`, multiplied by `{step}` (`0` if fewer than 3 bars)."
                    ),
                    signature: format!("fn {fname}(heights: list[int]) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([3, 2]) == 0)\n    return \"ok\"\n}}",
                        fmt_i(&t1),
                        oracle(&t1),
                        fmt_i(&t2),
                        oracle(&t2)
                    ),
                    require_exact_types: false,
                });
            }
            _ => {
                let fname = format!("td_algo_rob_house_{k}");
                let oracle = |nums: &[i64]| -> i64 {
                    let (mut p2, mut p1) = (0i64, 0i64);
                    for &v in nums {
                        let take = v.min(cap).max(0) + p2;
                        let best = p1.max(take);
                        p2 = p1;
                        p1 = best;
                    }
                    p1
                };
                let t1 = [2, 7, 9, 3, 1, 12];
                let t2 = [4, 1, 1, 4];
                seeds.push(DistillSeed {
                    id: fname.clone(),
                    category: "algorithm".into(),
                    spec_prose: format!(
                        "Implement `{fname}(nums)` finding maximum sum of non-adjacent elements where each element is first clamped into `[0, {cap}]`."
                    ),
                    signature: format!("fn {fname}(nums: list[int]) to int"),
                    expected_contains: vec![format!("fn {fname}")],
                    harness: format!(
                        "fn __vox_eval_harness() to str {{\n    assert({fname}({}) == {})\n    assert({fname}({}) == {})\n    assert({fname}([]) == 0)\n    return \"ok\"\n}}",
                        fmt_i(&t1),
                        oracle(&t1),
                        fmt_i(&t2),
                        oracle(&t2)
                    ),
                    require_exact_types: false,
                });
            }
        }
    }

    // 7. adt_match (20 algebraic type + match evaluators)
    for k in 0..20usize {
        let ty = format!("TdPlanAction{k}");
        let fname = format!("td_eval_plan_actions_{k}");
        let tax = 2 + (k as i64 % 5);
        let sig = format!(
            "type {ty} =\n    | AddCharge(cents: int)\n    | PercentCut(pct: int)\n    | ApplyFlatTax\n\nfn {fname}(start: int, actions: list[{ty}]) to int"
        );
        let oracle = |st: i64, acts: &[(u8, i64)]| -> i64 {
            let mut cur = st;
            for &(tag, v) in acts {
                match tag {
                    0 => cur += v,
                    1 => cur -= (cur * v) / 100,
                    _ => cur += tax,
                }
            }
            cur.max(0)
        };
        seeds.push(DistillSeed {
            id: fname.clone(),
            category: "adt_match".into(),
            spec_prose: format!(
                "Define algebraic type `{ty}` (`AddCharge(cents: int)`, `PercentCut(pct: int)`, `ApplyFlatTax`) and implement `{fname}(start, actions)` applying each sequentially: `AddCharge(c)` adds `c`, `PercentCut(p)` subtracts `(cur * p) / 100`, `ApplyFlatTax` adds `{tax}`; clamp final result at `>= 0`."
            ),
            signature: sig,
            expected_contains: vec![format!("type {ty}"), format!("fn {fname}")],
            harness: format!(
                "fn __vox_eval_harness() to str {{\n    assert({fname}(100, [AddCharge(50), PercentCut(10), ApplyFlatTax]) == {})\n    assert({fname}(10, [AddCharge(-50)]) == 0)\n    return \"ok\"\n}}",
                oracle(100, &[(0, 50), (1, 10), (2, 0)])
            ),
            require_exact_types: true,
        });
    }

    // 8. multi_fn_module (20 composed 2-function modules)
    for k in 0..20usize {
        let lo = 10 + (k as i64 % 5) * 4;
        let bonus = 3 + (k as i64 % 4);
        let hname = format!("td_mod_score_item_{k}");
        let fname = format!("td_mod_batch_total_{k}");
        let sig = format!("fn {hname}(x: int) to int\nfn {fname}(xs: list[int]) to int");
        let eval_h = |x: i64| if x < lo { 0 } else { x * 2 + bonus };
        let eval_f = |xs: &[i64]| -> i64 { xs.iter().map(|&x| eval_h(x)).sum() };
        let t1 = [1, lo, lo + 5];
        seeds.push(DistillSeed {
            id: fname.clone(),
            category: "multi_fn_module".into(),
            spec_prose: format!(
                "Write a 2-function Vox module: `{hname}(x)` returning `0` if `x < {lo}` else `x * 2 + {bonus}`, and `{fname}(xs)` summing `{hname}(x)` across `xs`."
            ),
            signature: sig,
            expected_contains: vec![format!("fn {hname}"), format!("fn {fname}")],
            harness: format!(
                "fn __vox_eval_harness() to str {{\n    assert({hname}(2) == 0)\n    assert({hname}({lo}) == {})\n    assert({fname}({}) == {})\n    return \"ok\"\n}}",
                eval_h(lo),
                fmt_i(&t1),
                eval_f(&t1)
            ),
            require_exact_types: true,
        });
    }

    seeds
}

async fn run_distillation_async(root: &Path) -> Result<serde_json::Value> {
    let mut bench = load_bench_answers(&root.join("mens/data/heldout_bench/manifest.json"))?;
    bench.extend(load_bench_answers(
        &root.join("mens/data/heldout_bench_v2/manifest.json"),
    )?);
    let bench = Arc::new(bench);

    let (backend, model_used, license_class, probe_cost) = probe_working_backend().await?;
    let mut spent_usd = probe_cost;
    let seeds = build_teacher_seed_bank();
    let total_seeds = seeds.len();

    let mut accepted_rows: Vec<serde_json::Value> = Vec::new();
    let mut passed_first_try = 0usize;
    let mut passed_after_repair = 0usize;
    let mut rejected_compile_or_exec = 0usize;
    let mut rejected_leakage = 0usize;
    let mut errors = 0usize;
    let mut stopped_on_budget = false;
    let mut by_category: BTreeMap<String, usize> = BTreeMap::new();
    let mut sample_diags: Vec<String> = Vec::new();

    let batch_size = 10usize;
    let mut idx = 0usize;
    let mut it = seeds.into_iter().peekable();
    while it.peek().is_some() {
        if spent_usd >= STOP_SPAWN_USD {
            stopped_on_budget = true;
            break;
        }
        let mut set = tokio::task::JoinSet::new();
        for _ in 0..batch_size {
            let Some(seed) = it.next() else { break };
            let cur_idx = idx;
            idx += 1;
            let allow_repair = spent_usd + 0.05 < STOP_SPAWN_USD;
            let b = Arc::clone(&backend);
            let bn = Arc::clone(&bench);
            let mu = model_used.clone();
            let lc = license_class.clone();
            set.spawn(
                async move { solve_one_seed(b, seed, cur_idx, allow_repair, bn, mu, lc).await },
            );
        }
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(SeedAttemptResult::Accepted {
                    row,
                    cost_usd,
                    repairs,
                }) => {
                    spent_usd += cost_usd;
                    if repairs == 0 {
                        passed_first_try += 1;
                    } else {
                        passed_after_repair += 1;
                    }
                    if let Some(cat) = row["category"].as_str() {
                        *by_category.entry(cat.to_string()).or_default() += 1;
                    }
                    accepted_rows.push(row);
                }
                Ok(SeedAttemptResult::RejectedCompile {
                    id,
                    cost_usd,
                    repairs,
                    diag,
                }) => {
                    spent_usd += cost_usd;
                    rejected_compile_or_exec += 1;
                    if sample_diags.len() < 10 {
                        sample_diags.push(format!("{id} (repairs={repairs}): {diag}"));
                    }
                }
                Ok(SeedAttemptResult::RejectedLeakage {
                    id,
                    cost_usd,
                    reason,
                }) => {
                    spent_usd += cost_usd;
                    rejected_leakage += 1;
                    if sample_diags.len() < 10 {
                        sample_diags.push(format!("{id}: leakage({reason})"));
                    }
                }
                Ok(SeedAttemptResult::Error { id, err, cost_usd }) => {
                    spent_usd += cost_usd;
                    errors += 1;
                    if sample_diags.len() < 10 {
                        sample_diags.push(format!("{id}: err={err}"));
                    }
                }
                Err(je) => {
                    errors += 1;
                    if sample_diags.len() < 10 {
                        sample_diags.push(format!("join_err: {je}"));
                    }
                }
            }
        }
        if spent_usd >= HARD_BUDGET_CAP_USD {
            stopped_on_budget = true;
            break;
        }
    }

    let out_jsonl = root.join("mens/data/mix_sources/mit_teacher_rft_v94.jsonl");
    if let Some(p) = out_jsonl.parent() {
        std::fs::create_dir_all(p)?;
    }
    {
        let mut wf = std::fs::File::create(&out_jsonl)?;
        for row in &accepted_rows {
            serde_json::to_writer(&mut wf, row)?;
            writeln!(wf)?;
        }
    }

    let report = json!({
        "schema": "mens_mit_teacher_distill_v94",
        "model_used": model_used,
        "license_class": license_class,
        "total_seeds": total_seeds,
        "attempted": idx,
        "passed_first_try": passed_first_try,
        "passed_after_repair": passed_after_repair,
        "emitted": accepted_rows.len(),
        "rejected_compile_or_exec": rejected_compile_or_exec,
        "rejected_leakage": rejected_leakage,
        "errors": errors,
        "emitted_by_category": by_category,
        "total_cost_usd": spent_usd,
        "budget_cap_usd": HARD_BUDGET_CAP_USD,
        "stopped_on_budget": stopped_on_budget,
        "output_jsonl": "mens/data/mix_sources/mit_teacher_rft_v94.jsonl",
        "sample_diagnostics": sample_diags,
    });
    let report_path = root.join("contracts/reports/mens-mit-teacher-distill-v94.json");
    if let Some(p) = report_path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)? + "\n")?;
    Ok(report)
}

#[test]
#[ignore]
fn distill_v94_mit_teacher_rows() {
    let root = repo_root();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build current-thread tokio runtime");
    let report = rt
        .block_on(run_distillation_async(&root))
        .expect("MIT teacher distillation run");
    eprintln!(
        "V94_TEACHER_DISTILL_REPORT: {}",
        serde_json::to_string(&report).unwrap()
    );
    let cost = report["total_cost_usd"].as_f64().unwrap_or(0.0);
    assert!(
        cost <= HARD_BUDGET_CAP_USD,
        "budget cap exceeded: spent ${cost:.4} > ${HARD_BUDGET_CAP_USD:.2}"
    );
    let emitted = report["emitted"].as_u64().unwrap_or(0);
    assert!(
        emitted >= 50,
        "expected >= 50 execution-verified teacher rows, got {emitted}: {report}"
    );
}
