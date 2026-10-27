//! Thinking-mode scoring for collateral-damage benches.
//! A bench directory that contains `reasoning.jsonl` (one
//! `{"id","question","answer"}` object per line, exactly [`SLICE_SIZE`] rows) is
//! scored by generating from the bare Qwen3 thinking-mode prefix — no
//! `/no_think`, no prefilled empty think block — and grading the final answer.
//! Think-initiation rate is recorded beside accuracy.
//!
//! The scorer fails closed: a slice that does not load, a model that does not
//! load, or any per-item failure (inference error, unparseable response, empty
//! text) writes a `not_scored` report (`n` 0, no score keys) when the adapter
//! directory exists, and always returns `Err`. A failing item is never graded as
//! a wrong answer. A score is recorded only for a full slice, with `n` stored
//! beside it. `VOX_REASONING_EVAL_MAX_TOKENS` overrides the 2048-token cap; a
//! value that is set but not an integer is a `not_scored` failure, not 2048.
use anyhow::{Context, Result};
use std::path::Path;
use vox_eval::qwen_reasoning;
use vox_eval::reasoning_bench::{BenchOutcome, ReasoningItem, SLICE_SIZE, load_slice};

/// Accuracy and think-initiation for one reasoning bench.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReasoningReport {
    /// Fraction of items whose final answer matched the gold answer.
    pub(crate) accuracy: f64,
    /// Fraction of completions that opened a non-empty `<think>` block.
    pub(crate) think_initiation_rate: f64,
    /// Items actually generated. The collateral writer reads it and refuses any
    /// count other than `SLICE_SIZE`.
    pub(crate) n: usize,
}

/// `(id, completion, correct, opened_think)` for one generated item.
type Row = (String, String, bool, bool);

/// Score one reasoning bench. `Ok` only for a full, error-free run. This is the
/// only place a [`ReasoningReport`] is built, and only from `BenchOutcome::Scored`.
pub(crate) fn score_reasoning_file(
    bench: &str,
    items_path: &Path,
    adapter: &Path,
) -> Result<ReasoningReport> {
    score_reasoning_file_with(bench, items_path, adapter, max_tokens_env().as_deref())
}

/// [`score_reasoning_file`] with the raw token-cap value passed in, so callers
/// and tests do not depend on the process environment.
fn score_reasoning_file_with(
    bench: &str,
    items_path: &Path,
    adapter: &Path,
    raw_max_tokens: Option<&str>,
) -> Result<ReasoningReport> {
    if !adapter.is_dir() {
        anyhow::bail!(
            "reasoning bench '{bench}': adapter {} is not an existing directory; \
             scored 0 of {SLICE_SIZE} items",
            adapter.display()
        );
    }
    let max_tokens = match parse_max_tokens(raw_max_tokens) {
        Ok(n) => n,
        Err(e) => {
            return Err(fail_closed(bench, adapter, &format!("{e:#}"), SLICE_SIZE));
        }
    };
    let items = match load_slice(items_path, SLICE_SIZE) {
        Ok(items) => items,
        Err(e) => {
            return Err(fail_closed(
                bench,
                adapter,
                &format!("loading slice: {e}"),
                SLICE_SIZE,
            ));
        }
    };
    let (outcome, rows) = match run_backend(bench, &items, adapter, max_tokens) {
        Ok(done) => done,
        Err(e) => {
            return Err(fail_closed(bench, adapter, &format!("{e:#}"), items.len()));
        }
    };
    write_outcome(bench, adapter, &outcome, &rows)?;
    match outcome {
        BenchOutcome::Scored(scored) => Ok(ReasoningReport {
            accuracy: scored.accuracy,
            think_initiation_rate: scored.think_initiation_rate,
            n: scored.n,
        }),
        BenchOutcome::NotScored { reason, .. } => {
            anyhow::bail!("reasoning bench '{bench}' not scored: {reason}")
        }
    }
}

/// Write a `not_scored` report and build the error to return.
fn fail_closed(bench: &str, adapter: &Path, detail: &str, n_requested: usize) -> anyhow::Error {
    let reason = format!("scored 0 of {n_requested} items: {detail}");
    let outcome = BenchOutcome::NotScored {
        reason: reason.clone(),
        n_requested,
    };
    match write_outcome(bench, adapter, &outcome, &[]) {
        Ok(()) => anyhow::anyhow!("reasoning bench '{bench}' not scored: {reason}"),
        Err(e) => anyhow::anyhow!(
            "reasoning bench '{bench}' not scored: {reason} (the not_scored report could not be written: {e})"
        ),
    }
}

/// Generation token cap from a raw `VOX_REASONING_EVAL_MAX_TOKENS` value.
/// `None` (unset) is 2048; an integer below 1 is raised to 1. A value that is
/// set but not an integer after trimming is an error, never the default: the
/// run must not be scored under a cap the caller did not ask for.
fn parse_max_tokens(raw: Option<&str>) -> Result<usize> {
    let Some(raw) = raw else {
        return Ok(2048);
    };
    let n: i64 = raw
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("VOX_REASONING_EVAL_MAX_TOKENS={raw:?} is not an integer"))?;
    Ok(usize::try_from(n.max(1)).unwrap_or(usize::MAX))
}

/// The raw `VOX_REASONING_EVAL_MAX_TOKENS` value; a non-UTF-8 value counts as set.
fn max_tokens_env() -> Option<String> {
    std::env::var_os("VOX_REASONING_EVAL_MAX_TOKENS").map(|v| v.to_string_lossy().into_owned())
}

/// Load the backend plugin and adapter, then score every item through it.
/// `Err` means the model never loaded.
fn run_backend(
    bench: &str,
    items: &[ReasoningItem],
    adapter: &Path,
    max_tokens: usize,
) -> Result<(BenchOutcome, Vec<Row>)> {
    let plugin_id = vox_populi::mens::select_mens_backend(
        vox_populi::mens::DeviceKind::Best,
        &vox_plugin_host::probe(),
    );
    let loaded = vox_plugin_host::cached_code_plugin(plugin_id).with_context(|| {
        format!("{plugin_id} plugin not found — install vox-plugin-{plugin_id}")
    })?;
    let backend = loaded
        .plugin
        .as_ml_backend()
        .into_option()
        .ok_or_else(|| anyhow::anyhow!("{plugin_id} plugin does not provide MlBackend"))?;
    let handle = backend
        .load_model(adapter.to_string_lossy().as_ref().into())
        .into_result()
        .map_err(|e| anyhow::anyhow!("load_model({}): {e}", adapter.display()))?;

    let mut infer = |prompt_json: &str| -> Result<String> {
        backend
            .run_inference(&handle, prompt_json.into())
            .into_result()
            .map(|resp| resp.as_str().to_string())
            .map_err(|e| anyhow::anyhow!("{e}"))
    };
    let done = score_loaded(bench, items, max_tokens, &mut infer);
    // `loaded` is a cache reference and must outlive `handle`; it drops here
    // with the other locals, which is after the last inference.
    let _plugin = loaded;
    Ok(done)
}

/// `generated_text` of a response, only when it parses and is non-empty.
fn usable_text(resp: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(resp)
        .ok()?
        .get("generated_text")?
        .as_str()
        .filter(|t| !t.trim().is_empty())
        .map(str::to_string)
}

/// Generate and grade every item through `infer` (prompt JSON in, raw response
/// out). The first inference error, unparseable response, or empty text stops the
/// run as `NotScored`; that item is neither graded nor counted. Grading rules are
/// the unchanged `qwen_reasoning` ones.
fn score_loaded(
    bench: &str,
    items: &[ReasoningItem],
    max_tokens: usize,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> (BenchOutcome, Vec<Row>) {
    let n = items.len();
    let mut rows: Vec<Row> = Vec::with_capacity(n);
    for (i, item) in items.iter().enumerate() {
        let prompt = qwen_reasoning::thinking_mode_prompt(&item.question);
        let prompt_json = serde_json::json!({
            "prompt": prompt,
            "max_tokens": max_tokens,
            "temperature": 0.0,
            "seed": 0,
        })
        .to_string();
        let detail = match infer(&prompt_json) {
            Ok(resp) => match usable_text(&resp) {
                Some(completion) => {
                    let correct = qwen_reasoning::grade_final_answer(&completion, &item.answer);
                    let thinking = qwen_reasoning::opened_nonempty_think(&completion);
                    eprintln!(
                        "  {bench} {}/{n} {} correct={correct} think={thinking}",
                        i + 1,
                        item.id
                    );
                    rows.push((item.id.clone(), completion, correct, thinking));
                    continue;
                }
                None => "response had no non-empty generated_text".to_string(),
            },
            Err(e) => format!("inference error: {e:#}"),
        };
        eprintln!("  {bench} {} failed: {detail}", item.id);
        let reason = format!("item {}: {detail}; scored 0 of {n} items", item.id);
        return (
            BenchOutcome::NotScored {
                reason,
                n_requested: n,
            },
            rows,
        );
    }
    let scored = qwen_reasoning::score_completions(
        rows.iter()
            .zip(items.iter())
            .map(|((_, completion, _, _), item)| (completion.as_str(), item.answer.as_str())),
    );
    (BenchOutcome::from_score(scored, n), rows)
}

/// Write `reasoning_report.json` for any outcome. The adapter directory must
/// already exist; `std::fs::write` truncates, so an older report is replaced.
fn write_outcome(bench: &str, adapter: &Path, outcome: &BenchOutcome, rows: &[Row]) -> Result<()> {
    let out_dir = adapter.join("collateral_eval").join(bench);
    std::fs::create_dir_all(&out_dir)?;
    let mut body = outcome.to_json();
    if let Some(map) = body.as_object_mut() {
        map.insert("benchmark".to_string(), serde_json::json!(bench));
        if matches!(outcome, BenchOutcome::Scored(_)) {
            let items: Vec<_> = rows
                .iter()
                .map(|(id, completion, correct, thinking)| {
                    serde_json::json!({
                        "id": id,
                        "correct": correct,
                        "opened_think": thinking,
                        "completion_preview": completion.chars().take(240).collect::<String>(),
                    })
                })
                .collect();
            map.insert("temperature".to_string(), serde_json::json!(0.0));
            map.insert("prompt".to_string(), serde_json::json!("thinking_mode"));
            map.insert("items".to_string(), serde_json::json!(items));
        }
    }
    std::fs::write(
        out_dir.join("reasoning_report.json"),
        serde_json::to_string_pretty(&body)?,
    )?;
    match outcome {
        BenchOutcome::Scored(scored) => eprintln!(
            "  {bench}: accuracy {:.1}%  think {:.1}%  (n={})",
            scored.accuracy * 100.0,
            scored.think_initiation_rate * 100.0,
            scored.n
        ),
        BenchOutcome::NotScored { reason, .. } => eprintln!("  {bench}: not scored: {reason}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gsm8k_slice() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/data/gsm8k_think/reasoning.jsonl")
    }

    fn read_report(adapter: &Path, bench: &str) -> serde_json::Value {
        let p = adapter
            .join("collateral_eval")
            .join(bench)
            .join("reasoning_report.json");
        let text = std::fs::read_to_string(&p).expect("report exists");
        serde_json::from_str(&text).expect("report is JSON")
    }

    #[test]
    fn an_adapter_dir_without_a_loadable_model_writes_not_scored() {
        let adapter = tempfile::tempdir().expect("tempdir");
        let err = score_reasoning_file_with("gsm8k_think", &gsm8k_slice(), adapter.path(), None)
            .expect_err("an empty adapter dir cannot be scored");
        let msg = format!("{err:#}");
        assert!(msg.contains("scored 0 of 32"), "message was: {msg}");
        let v = read_report(adapter.path(), "gsm8k_think");
        assert_eq!(v["status"], "not_scored");
        assert_eq!(v["n"], 0);
        let obj = v.as_object().expect("object");
        assert!(!obj.contains_key("accuracy"));
        assert!(!obj.contains_key("think_initiation_rate"));
    }

    #[test]
    fn a_missing_adapter_path_is_err_and_not_created() {
        let root = tempfile::tempdir().expect("tempdir");
        let adapter = root.path().join("no-such-adapter");
        let err = score_reasoning_file_with("gsm8k_think", &gsm8k_slice(), &adapter, None)
            .expect_err("a missing adapter cannot be scored");
        assert!(format!("{err:#}").contains("scored 0 of 32"));
        assert!(!adapter.exists(), "the adapter path must not be created");
    }

    fn synthetic_items(n: usize) -> Vec<ReasoningItem> {
        (0..n)
            .map(|i| ReasoningItem {
                id: format!("syn-{i:04}"),
                question: format!("what is {i}?"),
                answer: i.to_string(),
            })
            .collect()
    }

    /// Gold the prompt asks about: the synthetic question is `what is {i}?`.
    fn gold_from_prompt(prompt_json: &str) -> String {
        let v: serde_json::Value = serde_json::from_str(prompt_json).expect("prompt json");
        let prompt = v["prompt"].as_str().expect("prompt");
        let after = prompt.split("what is ").nth(1).expect("question in prompt");
        after.split('?').next().expect("number").to_string()
    }

    fn boxed_response(gold: &str) -> String {
        serde_json::json!({
            "generated_text": format!("<think>\nx\n</think>\n\n\\boxed{{{gold}}}")
        })
        .to_string()
    }

    #[test]
    fn every_answer_correct_scores_32() {
        let items = synthetic_items(32);
        let mut infer = |p: &str| -> Result<String> { Ok(boxed_response(&gold_from_prompt(p))) };
        let (outcome, rows) = score_loaded("syn", &items, 2048, &mut infer);
        match outcome {
            BenchOutcome::Scored(s) => {
                assert_eq!(s.n, 32);
                assert_eq!(s.accuracy, 1.0);
                assert_eq!(s.think_initiation_rate, 1.0);
            }
            other => panic!("expected Scored, got {other:?}"),
        }
        let ids: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
        let want: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
        assert_eq!(ids, want.iter().map(String::as_str).collect::<Vec<_>>());
    }

    #[test]
    fn three_of_32_correct_stores_exact_accuracy() {
        let items = synthetic_items(32);
        let mut infer = |p: &str| -> Result<String> {
            let gold: usize = gold_from_prompt(p).parse().expect("number");
            Ok(boxed_response(&if gold < 3 {
                gold.to_string()
            } else {
                "999999".to_string()
            }))
        };
        let (outcome, rows) = score_loaded("syn", &items, 2048, &mut infer);
        assert_eq!(rows.len(), 32);
        match outcome {
            BenchOutcome::Scored(s) => {
                assert_eq!(s.accuracy, 3.0 / 32.0);
                assert_eq!(s.n, 32);
            }
            other => panic!("expected Scored, got {other:?}"),
        }
    }

    #[test]
    fn an_inference_error_on_any_item_is_not_scored() {
        let items = synthetic_items(32);
        for bad in [0usize, 31] {
            let mut calls = 0usize;
            let mut infer = |p: &str| -> Result<String> {
                let i = calls;
                calls += 1;
                if i == bad {
                    anyhow::bail!("backend exploded");
                }
                Ok(boxed_response(&gold_from_prompt(p)))
            };
            let (outcome, rows) = score_loaded("syn", &items, 2048, &mut infer);
            match outcome {
                BenchOutcome::NotScored {
                    reason,
                    n_requested,
                } => {
                    assert_eq!(n_requested, 32);
                    assert!(
                        reason.contains(&format!("syn-{bad:04}")),
                        "reason must name the item: {reason}"
                    );
                    assert!(reason.contains("scored 0 of 32 items"), "{reason}");
                }
                other => panic!("bad={bad}: expected NotScored, got {other:?}"),
            }
            assert_eq!(rows.len(), bad, "the failing item is not graded or counted");
        }
    }

    #[test]
    fn empty_missing_or_unparseable_text_is_not_scored() {
        let items = synthetic_items(32);
        let bodies = [
            r#"{"generated_text": ""}"#,
            r#"{"generated_text": "   "}"#,
            r#"{"other": "x"}"#,
            r#"{"generated_text": 7}"#,
            "not json",
        ];
        for body in bodies {
            let mut calls = 0usize;
            let mut infer = |p: &str| -> Result<String> {
                calls += 1;
                if calls == 5 {
                    Ok(body.to_string())
                } else {
                    Ok(boxed_response(&gold_from_prompt(p)))
                }
            };
            let (outcome, rows) = score_loaded("syn", &items, 2048, &mut infer);
            match outcome {
                BenchOutcome::NotScored { reason, .. } => {
                    assert!(reason.contains("syn-0004"), "{body}: {reason}");
                }
                other => panic!("{body}: expected NotScored, got {other:?}"),
            }
            assert_eq!(rows.len(), 4);
        }
    }

    #[test]
    fn an_empty_slice_is_never_scored() {
        let mut infer = |_: &str| -> Result<String> { Ok(boxed_response("1")) };
        let (outcome, rows) = score_loaded("syn", &[], 2048, &mut infer);
        assert!(matches!(outcome, BenchOutcome::NotScored { .. }));
        assert!(rows.is_empty());
    }

    #[test]
    fn a_not_scored_write_replaces_a_stale_scored_report() {
        use vox_eval::qwen_reasoning::ReasoningScore;
        let adapter = tempfile::tempdir().expect("tempdir");
        let scored = BenchOutcome::Scored(ReasoningScore {
            accuracy: 0.9,
            think_initiation_rate: 1.0,
            n: 32,
        });
        write_outcome("gsm8k_think", adapter.path(), &scored, &[]).expect("write scored");
        let before = read_report(adapter.path(), "gsm8k_think");
        assert_eq!(before["status"], "scored");
        assert_eq!(before["accuracy"].as_f64(), Some(0.9));

        let not_scored = BenchOutcome::NotScored {
            reason: "scored 0 of 32 items: model went away".to_string(),
            n_requested: 32,
        };
        write_outcome("gsm8k_think", adapter.path(), &not_scored, &[]).expect("write not scored");
        let after = read_report(adapter.path(), "gsm8k_think");
        assert_eq!(after["status"], "not_scored");
        assert_eq!(after["n"], 0);
        let obj = after.as_object().expect("object");
        assert!(!obj.contains_key("accuracy"), "stale accuracy survived");
        assert!(!obj.contains_key("think_initiation_rate"));
        assert!(!obj.contains_key("items"));
    }

    #[test]
    fn a_wrong_size_slice_is_not_scored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let slice = dir.path().join("reasoning.jsonl");
        let rows: String = (0..31)
            .map(|i| format!("{{\"id\":\"w-{i:04}\",\"question\":\"q{i}\",\"answer\":\"{i}\"}}\n"))
            .collect();
        std::fs::write(&slice, rows).expect("write slice");
        let adapter = tempfile::tempdir().expect("tempdir");
        let err = score_reasoning_file_with("gsm8k_think", &slice, adapter.path(), None)
            .expect_err("a 31-row slice must not be scored");
        let msg = format!("{err:#}");
        assert!(msg.contains("scored 0 of 32"), "message was: {msg}");
        let v = read_report(adapter.path(), "gsm8k_think");
        assert_eq!(v["status"], "not_scored");
        assert_eq!(v["n"], 0);
        assert_eq!(v["n_requested"], 32);
        assert!(!v.as_object().expect("object").contains_key("accuracy"));
    }

    #[test]
    fn max_tokens_parse_rules() {
        assert_eq!(parse_max_tokens(None).expect("unset"), 2048);
        assert_eq!(parse_max_tokens(Some("4096")).expect("4096"), 4096);
        assert_eq!(parse_max_tokens(Some(" 512\n")).expect("trimmed"), 512);
        assert_eq!(parse_max_tokens(Some("1")).expect("1"), 1);
        assert_eq!(parse_max_tokens(Some("0")).expect("0"), 1);
        assert_eq!(parse_max_tokens(Some("-7")).expect("negative"), 1);
        for bad in ["4k", "", "  ", "1.5", "abc", "0x10", "4 096"] {
            assert!(
                parse_max_tokens(Some(bad)).is_err(),
                "{bad:?} must not parse"
            );
        }
    }

    #[test]
    fn an_unparseable_max_tokens_is_not_scored_and_never_2048() {
        let adapter = tempfile::tempdir().expect("tempdir");
        let err =
            score_reasoning_file_with("gsm8k_think", &gsm8k_slice(), adapter.path(), Some("4k"))
                .expect_err("a non-integer cap must not be scored");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("VOX_REASONING_EVAL_MAX_TOKENS"),
            "message was: {msg}"
        );
        assert!(msg.contains("scored 0 of 32"), "message was: {msg}");
        let v = read_report(adapter.path(), "gsm8k_think");
        assert_eq!(v["status"], "not_scored");
        assert_eq!(v["n"], 0);
        let obj = v.as_object().expect("object");
        assert!(!obj.contains_key("accuracy"));
        assert!(!obj.contains_key("think_initiation_rate"));
        assert!(!obj.contains_key("items"));
    }

    #[test]
    fn the_token_cap_reaches_the_prompt() {
        let items = synthetic_items(1);
        let mut seen = Vec::new();
        let mut infer = |p: &str| -> Result<String> {
            let v: serde_json::Value = serde_json::from_str(p).expect("prompt json");
            seen.push(v["max_tokens"].as_u64().expect("max_tokens"));
            Ok(boxed_response(&gold_from_prompt(p)))
        };
        let _ = score_loaded("syn", &items, 77, &mut infer);
        assert_eq!(seen, vec![77]);
    }
}
