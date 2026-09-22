//! `vox mens corpus rft` — rejection-sampling fine-tuning data (STaR / RFT;
//! pipeline audit 2026-09-20 §6, proposal 4).
//!
//! Samples K completions per task description, keeps those that pass the
//! eval-local verifier (parse + typecheck + anti-stub, `@test` execution when
//! present), dedups by normalized code, caps per task, and emits vox_codegen
//! pairs. Tasks and completions overlapping the held-out bench are excluded
//! with the eval-gate leakage check.

use anyhow::Result;
use std::path::PathBuf;

use super::synth::{self, ChatBackend};
use crate::commands::mens::eval_gate::{BenchTask, leaked_bench_task, load_bench_texts};

/// Output tokens budgeted per sampled completion.
const SAMPLE_MAX_TOKENS: usize = 1024;

pub(crate) struct RftOpts {
    pub input: PathBuf,
    pub output: PathBuf,
    pub bench: PathBuf,
    pub k: usize,
    pub max_per_task: usize,
    pub max_tasks: usize,
    pub temperature: f32,
    pub max_spend_usd: f64,
    pub usd_per_1k_tokens: f64,
    pub apply: bool,
}

#[derive(Debug, Default)]
pub(crate) struct RftSummary {
    pub tasks: usize,
    pub excluded_leaky_tasks: usize,
    pub planned_calls: usize,
    pub estimated_usd: f64,
    pub calls: usize,
    pub spent_usd: f64,
    pub failed_verification: usize,
    pub written: usize,
}

fn task_text(row: &serde_json::Value) -> Option<&str> {
    ["prompt", "instruction", "description"]
        .iter()
        .find_map(|k| row.get(*k).and_then(|v| v.as_str()))
        .filter(|s| !s.trim().is_empty())
}

pub(crate) async fn run_rft<B: ChatBackend>(opts: &RftOpts, backend: &B) -> Result<RftSummary> {
    let rows = synth::read_jsonl(&opts.input)?;
    let (bench_desc, bench_ans): (Vec<BenchTask>, Vec<BenchTask>) = if opts.bench.is_file() {
        (
            load_bench_texts(&opts.bench, "description")?,
            load_bench_texts(&opts.bench, "answer")?,
        )
    } else {
        tracing::warn!(
            "rft: bench manifest {} missing; leakage exclusion disabled",
            opts.bench.display()
        );
        (Vec::new(), Vec::new())
    };

    let mut s = RftSummary::default();
    let mut seen_tasks = std::collections::HashSet::new();
    let mut tasks = Vec::new();
    for row in &rows {
        if tasks.len() >= opts.max_tasks {
            break;
        }
        let Some(task) = task_text(row) else { continue };
        if !seen_tasks.insert(task.to_string()) {
            continue;
        }
        if leaked_bench_task(task, &bench_desc).is_some() {
            s.excluded_leaky_tasks += 1;
            continue;
        }
        let source = row
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("rft")
            .to_string();
        let difficulty = row.get("difficulty").and_then(|v| v.as_u64()).unwrap_or(5);
        tasks.push((task.to_string(), source, difficulty));
    }
    s.tasks = tasks.len();

    let system_prompt = vox_corpus::training::generate_system_prompt();
    s.planned_calls = s.tasks * opts.k;
    s.estimated_usd = tasks
        .iter()
        .map(|(t, _, _)| {
            synth::estimate_usd(
                system_prompt.len() + t.len(),
                SAMPLE_MAX_TOKENS,
                opts.usd_per_1k_tokens,
            )
        })
        .sum::<f64>()
        * opts.k as f64;
    println!(
        "rft plan: {} tasks ({} excluded as bench-overlapping), k={}, {} LLM calls, est ${:.4} (model {}, cap ${:.2})",
        s.tasks,
        s.excluded_leaky_tasks,
        opts.k,
        s.planned_calls,
        s.estimated_usd,
        backend.model_id(),
        opts.max_spend_usd
    );
    if !opts.apply {
        println!("dry run: no calls made; re-run with --apply to spend");
        return Ok(s);
    }

    let mut out = Vec::new();
    'tasks: for (ti, (task, source, difficulty)) in tasks.iter().enumerate() {
        let mut kept = std::collections::HashSet::new();
        for i in 0..opts.k {
            if kept.len() >= opts.max_per_task {
                break;
            }
            if s.spent_usd >= opts.max_spend_usd {
                println!("spend cap reached (${:.4}); stopping", s.spent_usd);
                break 'tasks;
            }
            let reply = backend.chat(&system_prompt, task, opts.temperature).await?;
            s.calls += 1;
            s.spent_usd += reply.cost_usd;
            let code = synth::clean_code(&reply.text);
            let v = synth::verify(&code, "rft", ti * opts.k + i);
            if !v.pass || leaked_bench_task(&code, &bench_ans).is_some() {
                s.failed_verification += 1;
                continue;
            }
            if !kept.insert(vox_crypto::hash_fast_hex(code.trim().as_bytes())) {
                continue;
            }
            let rating = if v.pass_exec == Some(true) { 5 } else { 4 };
            let mut row = synth::pair_row(task, &code, "rft", source, *difficulty, rating);
            row["model"] = reply.model.into();
            row["verification"] = serde_json::json!({
                "compile": v.pass_compile,
                "tests": v.pass_exec,
            });
            out.push(row);
        }
    }
    s.written = out.len();
    synth::write_jsonl(&opts.output, &out)?;
    println!(
        "rft: {} calls, ${:.4} spent, {} verified pairs ({} rejected) -> {}",
        s.calls,
        s.spent_usd,
        s.written,
        s.failed_verification,
        opts.output.display()
    );
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::super::synth::test_support::MockBackend;
    use super::*;
    use std::cell::Cell;

    const GOOD: &str = "fn double_it(x: int) to int {\n    return x * 2\n}\n";
    const GOOD_TESTED: &str = "fn triple_it(x: int) to int {\n    return x * 3\n}\n@test fn t() to Unit {\n    assert(triple_it(2) == 6)\n}\n";
    const BAD: &str = "fn double_it(x: int) to int {\n    return x *\n";
    const BENCH_ANSWER: &str = "fn secret_bench_solution(values: list[int]) to int {\n    let mut total = 0\n    for v in values {\n        total = total + v\n    }\n    return total\n}";

    fn opts(dir: &std::path::Path, tasks: &[&str], apply: bool) -> RftOpts {
        let input = dir.join("tasks.jsonl");
        std::fs::write(
            &input,
            tasks
                .iter()
                .map(|t| serde_json::json!({"prompt": t}).to_string() + "\n")
                .collect::<String>(),
        )
        .unwrap();
        let bench = dir.join("manifest.json");
        std::fs::write(
            &bench,
            serde_json::json!({"benchmarks": [{
                "id": "bench_sum",
                "description": "define a function named `secret_bench_solution` that sums every value in a list of integers and returns the total",
                "answer": BENCH_ANSWER,
            }]})
            .to_string(),
        )
        .unwrap();
        RftOpts {
            input,
            output: dir.join("rft.jsonl"),
            bench,
            k: 4,
            max_per_task: 2,
            max_tasks: 10,
            temperature: 0.8,
            max_spend_usd: 10.0,
            usd_per_1k_tokens: 0.002,
            apply,
        }
    }

    #[tokio::test]
    async fn dry_run_makes_zero_calls_and_excludes_bench_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let o = opts(
            dir.path(),
            &[
                "Write `double_it` that doubles an int.",
                "define a function named `secret_bench_solution` that sums every value in a list of integers and returns the total",
            ],
            false,
        );
        let mock = MockBackend::new(|_, _| unreachable!("dry run must not call"));
        let s = run_rft(&o, &mock).await.unwrap();
        assert_eq!(mock.calls(), 0);
        assert_eq!(
            (s.tasks, s.excluded_leaky_tasks, s.planned_calls),
            (1, 1, 4)
        );
        assert!(!o.output.exists());
    }

    #[tokio::test]
    async fn compile_gate_drops_failures_dedups_and_caps() {
        let dir = tempfile::tempdir().unwrap();
        let o = opts(
            dir.path(),
            &["Write `double_it` that doubles an int."],
            true,
        );
        // bad, good, good (dup), tested-good -> keep good + tested, cap 2 reached.
        let n = Cell::new(0);
        let mock = MockBackend::new(|_, _| {
            n.set(n.get() + 1);
            match n.get() {
                1 => BAD.into(),
                2 | 3 => format!("```vox\n{GOOD}```"),
                _ => GOOD_TESTED.into(),
            }
        });
        let s = run_rft(&o, &mock).await.unwrap();
        assert_eq!((mock.calls(), s.written, s.failed_verification), (4, 2, 1));
        let rows = synth::read_jsonl(&o.output).unwrap();
        assert!(
            rows.iter()
                .all(|r| !r["response"].as_str().unwrap().contains("```"))
        );
        assert_eq!(rows[0]["rating"], 4);
        assert_eq!(
            rows[1]["rating"], 5,
            "@test-verified completion rates higher"
        );
        assert_eq!(rows[0]["lane"], "vox_codegen");
    }

    #[tokio::test]
    async fn completions_copying_a_bench_answer_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let mut o = opts(dir.path(), &["Sum a list of ints."], true);
        o.k = 1;
        let mock = MockBackend::new(|_, _| BENCH_ANSWER.into());
        let s = run_rft(&o, &mock).await.unwrap();
        assert_eq!((s.written, s.failed_verification), (0, 1));
    }
}
