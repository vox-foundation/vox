//! Thinking-mode scoring for collateral-damage benches.
//! A bench directory that contains `reasoning.jsonl` (one
//! `{"id","question","answer"}` object per line, exactly [`SLICE_SIZE`] rows) is
//! scored by generating from the bare Qwen3 thinking-mode prefix — no
//! `/no_think`, no prefilled empty think block — and grading the final answer.
//! Think-initiation rate is recorded beside accuracy.
//!
//! The scorer fails closed: a slice that does not load, a model that does not
//! load, or any per-item failure writes a `not_scored` report (`n` 0, no score
//! keys) when the adapter directory exists, and always returns `Err`. A score is
//! recorded only for a full slice, with `n` stored beside it.
//! `VOX_REASONING_EVAL_MAX_TOKENS` overrides the 2048-token generation cap.
use anyhow::{Context, Result};
use std::path::Path;
use vox_eval::qwen_reasoning;
use vox_eval::reasoning_bench::{BenchOutcome, SLICE_SIZE, load_slice};

/// Accuracy and think-initiation for one reasoning bench.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReasoningReport {
    /// Fraction of items whose final answer matched the gold answer.
    pub(crate) accuracy: f64,
    /// Fraction of completions that opened a non-empty `<think>` block.
    pub(crate) think_initiation_rate: f64,
    /// Items actually generated. Written into the JSON report; the gate reads
    /// accuracy and think rate off this struct.
    #[allow(dead_code)]
    pub(crate) n: usize,
}

/// `(id, completion, correct, opened_think)` for one generated item.
type Row = (String, String, bool, bool);

/// Score one reasoning bench. `Ok` only for a full, error-free run.
pub(crate) fn score_reasoning_file(
    bench: &str,
    items_path: &Path,
    adapter: &Path,
) -> Result<ReasoningReport> {
    if !adapter.is_dir() {
        anyhow::bail!(
            "reasoning bench '{bench}': adapter {} is not an existing directory; \
             scored 0 of {SLICE_SIZE} items",
            adapter.display()
        );
    }
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
    let max_tokens = std::env::var("VOX_REASONING_EVAL_MAX_TOKENS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(2048)
        .max(1);

    let (outcome, rows) = match generate_all(bench, &items, adapter, max_tokens) {
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

/// Load the backend plugin and adapter, then generate and grade every item.
/// `Err` means the model never loaded.
fn generate_all(
    bench: &str,
    items: &[vox_eval::reasoning_bench::ReasoningItem],
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

    let mut rows: Vec<Row> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let prompt = qwen_reasoning::thinking_mode_prompt(&item.question);
        let prompt_json = serde_json::json!({
            "prompt": prompt,
            "max_tokens": max_tokens,
            "temperature": 0.0,
            "seed": 0,
        })
        .to_string();
        let completion = match backend
            .run_inference(&handle, prompt_json.as_str().into())
            .into_result()
        {
            Ok(resp) => serde_json::from_str::<serde_json::Value>(resp.as_str())
                .ok()
                .and_then(|v| {
                    v.get("generated_text")
                        .and_then(|x| x.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default(),
            Err(e) => {
                eprintln!("  {bench} {} inference error: {e}", item.id);
                String::new()
            }
        };
        let correct = qwen_reasoning::grade_final_answer(&completion, &item.answer);
        let thinking = qwen_reasoning::opened_nonempty_think(&completion);
        eprintln!(
            "  {bench} {}/{} {} correct={correct} think={thinking}",
            i + 1,
            items.len(),
            item.id
        );
        rows.push((item.id.clone(), completion, correct, thinking));
    }
    // `loaded` is a cache reference and must outlive `handle`; it drops here
    // with the other locals after the loop, which is after the last inference.
    let _plugin = loaded;
    let scored = qwen_reasoning::score_completions(
        rows.iter()
            .zip(items.iter())
            .map(|((_, completion, _, _), item)| (completion.as_str(), item.answer.as_str())),
    );
    Ok((BenchOutcome::Scored(scored), rows))
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
        let err = score_reasoning_file("gsm8k_think", &gsm8k_slice(), adapter.path())
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
        let err = score_reasoning_file("gsm8k_think", &gsm8k_slice(), &adapter)
            .expect_err("a missing adapter cannot be scored");
        assert!(format!("{err:#}").contains("scored 0 of 32"));
        assert!(!adapter.exists(), "the adapter path must not be created");
    }
}
