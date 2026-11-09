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
#[derive(Debug, Clone)]
pub(crate) struct ReasoningReport {
    /// Fraction of items whose final answer matched the gold answer.
    pub(crate) accuracy: f64,
    /// Fraction of completions that opened a non-empty `<think>` block.
    pub(crate) think_initiation_rate: f64,
    /// Items actually generated. The collateral writer reads it and refuses any
    /// count other than `SLICE_SIZE`.
    pub(crate) n: usize,
    /// `(id, correct, opened_think)` per item, in slice order.
    pub(crate) items: Vec<(String, bool, bool)>,
}

/// `(id, completion, correct, opened_think)` for one generated item.
type Row = (String, String, bool, bool);

/// Score one reasoning bench. `Ok` only for a full, error-free run. This is the
/// only place a [`ReasoningReport`] is built, and only from `BenchOutcome::Scored`.
pub(crate) fn score_reasoning_file(
    bench: &str,
    items_path: &Path,
    adapter: &Path,
    force_restart: bool,
) -> Result<ReasoningReport> {
    score_reasoning_file_with(
        bench,
        items_path,
        adapter,
        max_tokens_env().as_deref(),
        Path::new(super::item_checkpoint::REASONING_CHECKPOINT_ROOT),
        force_restart,
    )
}

/// [`score_reasoning_file`] with the raw token-cap value passed in, so callers
/// and tests do not depend on the process environment.
fn score_reasoning_file_with(
    bench: &str,
    items_path: &Path,
    adapter: &Path,
    raw_max_tokens: Option<&str>,
    ck_root: &Path,
    force_restart: bool,
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
    let settings = match decode_settings(max_tokens) {
        Ok(s) => s,
        Err(e) => {
            return Err(fail_closed(bench, adapter, &format!("{e:#}"), SLICE_SIZE));
        }
    };
    score_reasoning_file_with_settings(
        bench,
        items_path,
        adapter,
        &settings,
        ck_root,
        force_restart,
    )
}

pub(crate) fn score_reasoning_file_with_settings(
    bench: &str,
    items_path: &Path,
    adapter: &Path,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    ck_root: &Path,
    force_restart: bool,
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
    let (outcome, rows) =
        match run_backend(bench, &items, adapter, settings, ck_root, force_restart) {
            Ok(done) => done,
            Err(e) => {
                return Err(fail_closed(bench, adapter, &format!("{e:#}"), items.len()));
            }
        };
    write_outcome(bench, adapter, &outcome, &rows)?;
    report_of(bench, outcome, rows)
}

/// The only place a [`ReasoningReport`] is built, and only from `Scored`.
fn report_of(bench: &str, outcome: BenchOutcome, rows: Vec<Row>) -> Result<ReasoningReport> {
    match outcome {
        BenchOutcome::Scored(scored) => Ok(ReasoningReport {
            accuracy: scored.accuracy,
            think_initiation_rate: scored.think_initiation_rate,
            n: scored.n,
            items: rows
                .into_iter()
                .map(|(id, _, correct, think)| (id, correct, think))
                .collect(),
        }),
        BenchOutcome::NotScored { reason, .. } => {
            anyhow::bail!("reasoning bench '{bench}' not scored: {reason}")
        }
    }
}

/// Score the base side of a reasoning bench through the same checkpoint, token
/// cap and grader as [`score_reasoning_file`], writing nothing into `base` (a
/// Hugging Face snapshot). Fails closed the same way: any per-item failure is `Err`.
pub(crate) fn score_base_file(
    bench: &str,
    items_path: &Path,
    base: &Path,
    force_restart: bool,
) -> Result<ReasoningReport> {
    let max_tokens = parse_max_tokens(max_tokens_env().as_deref())?;
    let settings = decode_settings(max_tokens)?;
    score_base_file_with_settings(bench, items_path, base, &settings, force_restart)
}

pub(crate) fn score_base_file_with_settings(
    bench: &str,
    items_path: &Path,
    base: &Path,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart: bool,
) -> Result<ReasoningReport> {
    let items = load_slice(items_path, SLICE_SIZE)
        .map_err(|e| anyhow::anyhow!("reasoning bench '{bench}': loading slice: {e}"))?;
    let ck_root = Path::new(super::item_checkpoint::REASONING_CHECKPOINT_ROOT);
    let (outcome, rows) = run_backend(bench, &items, base, settings, ck_root, force_restart)?;
    report_of(bench, outcome, rows).with_context(|| format!("base {}", base.display()))
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
pub(crate) fn parse_max_tokens(raw: Option<&str>) -> Result<usize> {
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
pub(crate) fn max_tokens_env() -> Option<String> {
    std::env::var_os("VOX_REASONING_EVAL_MAX_TOKENS").map(|v| v.to_string_lossy().into_owned())
}

/// Decode settings for reasoning eval (protocol from env, greedy temp 0, seed 0, thinking true).
pub(crate) fn decode_settings(
    max_tokens: usize,
) -> Result<vox_eval::decode_protocol::DecodeSettings> {
    let protocol = vox_eval::decode_protocol::DecodeProtocol::from_env()
        .map_err(|e| anyhow::anyhow!("{}: {e}", vox_eval::decode_protocol::PROTOCOL_ENV))?;
    Ok(vox_eval::decode_protocol::DecodeSettings {
        protocol,
        max_tokens,
        temperature: 0.0,
        seed: 0,
        thinking: true,
    })
}

/// Score `items` on `adapter` through the shared per-item checkpoint, loading
/// the model only if an item needs generating. `Err` means the plugin or the
/// checkpoint could not be opened; a model that does not load fails the slice.
fn run_backend(
    bench: &str,
    items: &[ReasoningItem],
    adapter: &Path,
    settings: &vox_eval::decode_protocol::DecodeSettings,
    ck_root: &Path,
    force_restart: bool,
) -> Result<(BenchOutcome, Vec<Row>)> {
    with_backend(adapter, false, |infer| {
        score_slice_checkpointed(
            ck_root,
            adapter,
            bench,
            items,
            settings,
            force_restart,
            infer,
        )
    })?
}

/// Run `f` with an inference function for `model_dir` on the selected backend.
/// The plugin loads up front. With `eager` the model loads before `f` and a load
/// failure is this function's `Err`; otherwise it loads on the first call (so a
/// run whose items are all checkpointed never loads it) and a load failure is
/// that call's `Err`. The model is unloaded before returning.
pub(crate) fn with_backend<T>(
    model_dir: &Path,
    eager: bool,
    f: impl FnOnce(&mut dyn FnMut(&str) -> Result<String>) -> T,
) -> Result<T> {
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
    let load = || {
        backend
            .load_model(model_dir.to_string_lossy().as_ref().into())
            .into_result()
            .map_err(|e| anyhow::anyhow!("load_model({}): {e}", model_dir.display()))
    };
    let mut handle = if eager { Some(load()?) } else { None };
    let out = {
        let mut infer = |prompt_json: &str| -> Result<String> {
            if handle.is_none() {
                handle = Some(load()?);
            }
            let h = handle.as_ref().expect("loaded above");
            backend
                .run_inference(h, prompt_json.into())
                .into_result()
                .map(|resp| resp.as_str().to_string())
                .map_err(|e| anyhow::anyhow!("{e}"))
        };
        f(&mut infer)
    };
    if let Some(h) = handle {
        if let Err(e) = backend.unload_model(&h).into_result() {
            eprintln!("warning: unload_model({}): {e}", model_dir.display());
        }
    }
    Ok(out)
}

/// [`score_loaded`] through the shared per-item checkpoint for this model,
/// bench, item set and decode settings: recorded generations are reused and
/// `infer` runs only for the rest. `Err` only when the checkpoint is refused.
pub(crate) fn score_slice_checkpointed(
    ck_root: &Path,
    model_dir: &Path,
    bench: &str,
    items: &[ReasoningItem],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart: bool,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<(BenchOutcome, Vec<Row>)> {
    let mut ck = super::item_checkpoint::reasoning_checkpoint(
        ck_root,
        model_dir,
        bench,
        items,
        settings,
        force_restart,
    )?;
    if ck.len() > 0 {
        eprintln!(
            "  {bench}: {} of {} items already in the checkpoint for {}",
            ck.len(),
            items.len(),
            model_dir.display()
        );
    }
    let pending: Vec<String> = items
        .iter()
        .map(|i| prompt_request(i, settings.max_tokens))
        .filter(|p| !ck.has(p))
        .collect();
    let size = parse_batch_size(batch_size_env().as_deref())?;
    let max_width = verified_batch_width(Path::new(DECODE_IDENTITY), &settings.protocol)?;
    decode_batched(bench, &pending, size, max_width, &mut ck, infer)?;
    let mut via_ck = |prompt_json: &str| ck.infer(prompt_json, infer);
    Ok(score_loaded(bench, items, settings.max_tokens, &mut via_ck))
}

/// Controls whether [`decode_stream`] continues consuming batch responses or cancels the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamControl {
    Continue,
    Cancel,
}

pub const DECODE_IDENTITY: &str = "mens/config/decode-identity.yaml";

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DecodeIdentityFile {
    f32: F32Identity,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct F32Identity {
    verified_batch_width: usize,
    evidence: String,
}

pub fn verified_batch_width(
    path: &Path,
    protocol: &vox_eval::decode_protocol::DecodeProtocol,
) -> Result<Option<usize>> {
    match protocol {
        vox_eval::decode_protocol::DecodeProtocol::F32 => {
            let real_path = if path.is_file() {
                path.to_path_buf()
            } else {
                let manifest_candidate = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(path);
                if manifest_candidate.is_file() {
                    manifest_candidate
                } else {
                    path.to_path_buf()
                }
            };
            let content = std::fs::read_to_string(&real_path).map_err(|e| {
                anyhow::anyhow!("{} not found: run from the repo root ({e})", path.display())
            })?;
            let doc: DecodeIdentityFile = serde_yaml::from_str(&content)
                .map_err(|e| anyhow::anyhow!("{}: parse error: {e}", path.display()))?;
            anyhow::ensure!(
                doc.f32.verified_batch_width > 0,
                "{}: verified_batch_width must be greater than zero",
                path.display()
            );
            Ok(Some(doc.f32.verified_batch_width))
        }
        vox_eval::decode_protocol::DecodeProtocol::Bf16 { .. } => Ok(None),
    }
}

/// Decode `pending` as a single stream through the backend's batch protocol,
/// calling `on_done(index, response)` for each sequence that finishes.
///
/// If `on_done` returns [`StreamControl::Cancel`], sends `{"batch_cancel": true}`
/// to the backend and returns `Ok(())` without consuming further sequences.
/// If the backend does not support batching or fails mid-stream, returns `Ok(())`
/// so callers can fall back to the per-item path.
pub(crate) fn decode_stream(
    label: &str,
    pending: &[String],
    size: Option<usize>,
    max_width: Option<usize>,
    on_done: &mut dyn FnMut(usize, &str) -> Result<StreamControl>,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }
    if pending.len() < 2 || size == Some(1) {
        return Ok(());
    }
    let reqs: Vec<serde_json::Value> = pending
        .iter()
        .map(|p| serde_json::from_str(p))
        .collect::<std::result::Result<_, _>>()?;
    let mut body = serde_json::json!({ "batch": reqs });

    if let Some(mw) = max_width {
        body["max_batch_size"] = mw.into();
    }
    let effective_size = match (size, max_width) {
        (Some(s), Some(mw)) if s > mw => {
            eprintln!(
                "warning: VOX_REASONING_EVAL_BATCH_SIZE={s} exceeds verified batch width {mw}; clamping to {mw}"
            );
            Some(mw)
        }
        (Some(s), _) => Some(s),
        (None, _) => None,
    };

    match effective_size {
        Some(n) => body["batch_size"] = n.into(),
        None => {
            let mut sys = sysinfo::System::new();
            sys.refresh_memory();
            body["free_bytes"] = sys.available_memory().into();
        }
    }
    let started = infer(&body.to_string());
    let width = match started {
        Ok(ref r) => match serde_json::from_str::<serde_json::Value>(r) {
            Ok(v) => v.get("batch_size").and_then(|x| x.as_u64()),
            Err(e) => {
                eprintln!("  {label}: the backend returned invalid JSON ({e:#}): {r}");
                None
            }
        },
        Err(ref e) => {
            eprintln!("  {label}: the backend did not start a batch ({e:#}); one item at a time");
            None
        }
    };
    let width = match width {
        Some(w) => w,
        None => {
            return Ok(());
        }
    };
    eprintln!(
        "  {label}: decoding {} items, batch size {width}",
        pending.len()
    );
    loop {
        let next = match infer(r#"{"batch_next":true}"#) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  {label}: batched decode stopped ({e:#}); the rest one at a time");
                return Ok(());
            }
        };
        let v: serde_json::Value = serde_json::from_str(&next)?;
        if v["done"] == true {
            return Ok(());
        }
        let (Some(i), Some(resp)) = (v["index"].as_u64(), v["response"].as_str()) else {
            anyhow::bail!("{label}: malformed batch_next response: {next}");
        };
        let idx = usize::try_from(i).unwrap_or(usize::MAX);
        if idx >= pending.len() {
            anyhow::bail!("{label}: batch_next index {i} out of range");
        }
        match on_done(idx, resp)? {
            StreamControl::Continue => {}
            StreamControl::Cancel => {
                if let Err(e) = infer(r#"{"batch_cancel":true}"#) {
                    eprintln!("warning: batch_cancel failed: {e:#}");
                }
                return Ok(());
            }
        }
    }
}

/// Generate `pending` (prompt requests with no recorded generation) as one
/// batch, recording each in `ck` the moment its sequence finishes. Items it does
/// not record are left to [`score_loaded`], which generates them one at a time
/// and fails closed on their errors as before; so a plugin without batch support,
/// or an error mid-batch, only costs speed. `Err` only when recording fails.
///
/// The checkpoint key does not name the batch size: a batched sequence's text is
/// token-identical to its batch=1 text up to the verified batch width in
/// `mens/config/decode-identity.yaml`.
fn decode_batched(
    bench: &str,
    pending: &[String],
    size: Option<usize>,
    max_width: Option<usize>,
    ck: &mut super::item_checkpoint::ItemCheckpoint,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<()> {
    decode_stream(
        bench,
        pending,
        size,
        max_width,
        &mut |idx, resp| {
            let prompt = &pending[idx];
            ck.record(prompt, resp)?;
            Ok(StreamControl::Continue)
        },
        infer,
    )
}

/// `VOX_REASONING_EVAL_BATCH_SIZE`: unset means size the batch from free memory.
fn parse_batch_size(raw: Option<&str>) -> Result<Option<usize>> {
    let Some(raw) = raw else { return Ok(None) };
    let n: usize = raw.trim().parse().map_err(|_| {
        anyhow::anyhow!("VOX_REASONING_EVAL_BATCH_SIZE={raw:?} is not a positive integer")
    })?;
    anyhow::ensure!(n >= 1, "VOX_REASONING_EVAL_BATCH_SIZE must be at least 1");
    Ok(Some(n))
}

fn batch_size_env() -> Option<String> {
    std::env::var_os("VOX_REASONING_EVAL_BATCH_SIZE").map(|v| v.to_string_lossy().into_owned())
}

/// The exact prompt request [`score_loaded`] sends for `item`, so a batched
/// generation is recorded under the same line key a batch=1 one is.
pub(crate) fn prompt_request(item: &ReasoningItem, max_tokens: usize) -> String {
    serde_json::json!({
        "prompt": qwen_reasoning::thinking_mode_prompt(&item.question),
        "max_tokens": max_tokens,
        "temperature": 0.0,
        "seed": 0,
    })
    .to_string()
}

/// Prefill reasoning items for multiple benchmarks as one continuous decode stream.
///
/// Opens each bench's checkpoint with `reasoning_checkpoint(..., settings, force_restart)`,
/// collects prompts each checkpoint does not have, calls `on_record(bench, true)` for
/// each bench with nothing pending first (a `Cancel` there returns before streaming),
/// then runs one `decode_stream` over all pending prompts. For each recorded item,
/// calls `on_record(bench, complete)` (complete = that bench has nothing pending now)
/// and respects the returned `StreamControl`.
pub(crate) fn prefill_benches(
    ck_root: &Path,
    model_dir: &Path,
    benches: &[(&str, &[ReasoningItem])],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart: bool,
    on_record: &mut dyn FnMut(&str, bool) -> Result<StreamControl>,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<()> {
    if benches.is_empty() {
        return Ok(());
    }

    struct BenchState<'a> {
        name: &'a str,
        ck: super::item_checkpoint::ItemCheckpoint,
        pending_prompts: Vec<String>,
        remaining: usize,
    }

    let mut states: Vec<BenchState> = Vec::with_capacity(benches.len());
    for &(bench_name, items) in benches {
        let ck = super::item_checkpoint::reasoning_checkpoint(
            ck_root,
            model_dir,
            bench_name,
            items,
            settings,
            force_restart,
        )?;
        let bench_pending: Vec<String> = items
            .iter()
            .map(|i| prompt_request(i, settings.max_tokens))
            .filter(|p| !ck.has(p))
            .collect();
        let remaining = bench_pending.len();
        states.push(BenchState {
            name: bench_name,
            ck,
            pending_prompts: bench_pending,
            remaining,
        });
    }

    // Call on_record(bench, true) for each bench with nothing pending first, in bench order.
    for state in &states {
        if state.remaining == 0 {
            match on_record(state.name, true)? {
                StreamControl::Continue => {}
                StreamControl::Cancel => return Ok(()),
            }
        }
    }

    let mut flat_pending = Vec::new();
    let mut prompt_bench_idx = Vec::new();
    for (idx, state) in states.iter().enumerate() {
        for p in &state.pending_prompts {
            flat_pending.push(p.clone());
            prompt_bench_idx.push(idx);
        }
    }

    if flat_pending.is_empty() {
        return Ok(());
    }

    let size = parse_batch_size(batch_size_env().as_deref())?;
    let max_width = verified_batch_width(Path::new(DECODE_IDENTITY), &settings.protocol)?;

    decode_stream(
        "prefill",
        &flat_pending,
        size,
        max_width,
        &mut |idx, resp| {
            let b_idx = prompt_bench_idx[idx];
            let prompt = &flat_pending[idx];
            states[b_idx].ck.record(prompt, resp)?;
            states[b_idx].remaining = states[b_idx].remaining.saturating_sub(1);
            let complete = states[b_idx].remaining == 0;
            on_record(states[b_idx].name, complete)
        },
        infer,
    )?;

    Ok(())
}

/// `generated_text` of a response, only when it parses and is non-empty.
pub(crate) fn usable_text(resp: &str) -> Option<String> {
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
pub(crate) fn score_loaded(
    bench: &str,
    items: &[ReasoningItem],
    max_tokens: usize,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> (BenchOutcome, Vec<Row>) {
    let n = items.len();
    let mut rows: Vec<Row> = Vec::with_capacity(n);
    for (i, item) in items.iter().enumerate() {
        let prompt_json = prompt_request(item, max_tokens);
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

    /// A checkpoint root no test shares.
    fn ck_root() -> std::path::PathBuf {
        tempfile::tempdir().expect("ck").keep()
    }

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
        let err = score_reasoning_file_with(
            "gsm8k_think",
            &gsm8k_slice(),
            adapter.path(),
            None,
            &ck_root(),
            false,
        )
        .expect_err("an empty adapter dir cannot be scored");
        let msg = format!("{err:#}");
        assert!(msg.contains(&none_of_slice()), "message was: {msg}");
        let v = read_report(adapter.path(), "gsm8k_think");
        assert_eq!(v["status"], "not_scored");
        assert_eq!(v["n"], 0);
        let obj = v.as_object().expect("object");
        assert!(!obj.contains_key("accuracy"));
        assert!(!obj.contains_key("think_initiation_rate"));
    }

    fn none_of_slice() -> String {
        format!("scored 0 of {SLICE_SIZE}")
    }

    /// A backend speaking the batch protocol: sequences finish in `order`, and
    /// `batch_next` fails after `fail_after` of them. Counts single requests.
    fn batch_backend(
        order: Vec<usize>,
        fail_after: usize,
        singles: &mut usize,
    ) -> impl FnMut(&str) -> Result<String> + '_ {
        let mut reqs: Vec<serde_json::Value> = Vec::new();
        let mut handed = 0usize;
        move |p: &str| {
            let v: serde_json::Value = serde_json::from_str(p)?;
            if let Some(r) = v.get("batch") {
                reqs = r.as_array().unwrap().clone();
                return Ok(
                    serde_json::json!({"batch_started": reqs.len(), "batch_size": 2}).to_string(),
                );
            }
            if v.get("batch_next").is_some() {
                anyhow::ensure!(handed < fail_after, "device lost");
                let Some(&i) = order.get(handed) else {
                    return Ok(r#"{"done":true}"#.to_string());
                };
                handed += 1;
                let gold = gold_from_prompt(&reqs[i].to_string());
                return Ok(
                    serde_json::json!({"index": i, "response": boxed_response(&gold)}).to_string(),
                );
            }
            if v.get("batch_cancel").is_some() {
                return Ok(
                    serde_json::json!({"cancelled": reqs.len().saturating_sub(handed)}).to_string(),
                );
            }
            *singles += 1;
            Ok(boxed_response(&gold_from_prompt(p)))
        }
    }

    /// The item keys of a checkpoint's generation lines, in file order.
    fn recorded_items(ck_root: &Path) -> Vec<String> {
        let model = std::fs::read_dir(ck_root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let file = std::fs::read_dir(model)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        std::fs::read_to_string(file)
            .unwrap()
            .lines()
            .skip(1)
            .map(|l| {
                serde_json::from_str::<serde_json::Value>(l).unwrap()["item"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    }

    fn item_key(item: &ReasoningItem) -> String {
        vox_crypto::hex_encode(&vox_crypto::secure_hash(
            prompt_request(item, 2048).as_bytes(),
        ))
    }

    fn test_settings(max_tokens: usize) -> vox_eval::decode_protocol::DecodeSettings {
        vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            max_tokens,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        }
    }

    #[test]
    fn a_batch_records_each_item_as_its_sequence_finishes() {
        let items = synthetic_items(4);
        let (root, model) = (ck_root(), tempfile::tempdir().unwrap());
        let mut singles = 0;
        let mut infer = batch_backend(vec![2, 0, 3, 1], usize::MAX, &mut singles);
        let (outcome, _) = score_slice_checkpointed(
            &root,
            model.path(),
            "syn",
            &items,
            &test_settings(2048),
            false,
            &mut infer,
        )
        .unwrap();
        drop(infer);
        assert!(matches!(outcome, BenchOutcome::Scored(s) if s.accuracy == 1.0));
        assert_eq!(singles, 0, "every item came from the batch");
        let want: Vec<String> = [2, 0, 3, 1].iter().map(|&i| item_key(&items[i])).collect();
        assert_eq!(
            recorded_items(&root),
            want,
            "one line per item, in finishing order"
        );
    }

    #[test]
    fn items_a_failed_batch_did_not_finish_are_generated_one_at_a_time() {
        let items = synthetic_items(4);
        let (root, model) = (ck_root(), tempfile::tempdir().unwrap());
        let mut singles = 0;
        let mut infer = batch_backend(vec![3, 1, 0, 2], 2, &mut singles);
        let (outcome, _) = score_slice_checkpointed(
            &root,
            model.path(),
            "syn",
            &items,
            &test_settings(2048),
            false,
            &mut infer,
        )
        .unwrap();
        drop(infer);
        assert!(matches!(outcome, BenchOutcome::Scored(s) if s.accuracy == 1.0));
        assert_eq!(
            singles, 2,
            "only the two unfinished items are generated again"
        );
        let want: Vec<String> = [3, 1, 0, 2].iter().map(|&i| item_key(&items[i])).collect();
        assert_eq!(
            recorded_items(&root),
            want,
            "the finished two were already recorded"
        );
    }

    #[test]
    fn batch_size_parse_rules() {
        assert_eq!(parse_batch_size(None).unwrap(), None);
        assert_eq!(parse_batch_size(Some(" 8 ")).unwrap(), Some(8));
        assert_eq!(parse_batch_size(Some("1")).unwrap(), Some(1));
        for bad in ["0", "-2", "auto", ""] {
            assert!(
                parse_batch_size(Some(bad)).is_err(),
                "{bad:?} must not parse"
            );
        }
    }

    #[test]
    fn a_missing_adapter_path_is_err_and_not_created() {
        let root = tempfile::tempdir().expect("tempdir");
        let adapter = root.path().join("no-such-adapter");
        let err = score_reasoning_file_with(
            "gsm8k_think",
            &gsm8k_slice(),
            &adapter,
            None,
            &ck_root(),
            false,
        )
        .expect_err("a missing adapter cannot be scored");
        assert!(format!("{err:#}").contains(&none_of_slice()));
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
        let err = score_reasoning_file_with(
            "gsm8k_think",
            &slice,
            adapter.path(),
            None,
            &ck_root(),
            false,
        )
        .expect_err("a 31-row slice must not be scored");
        let msg = format!("{err:#}");
        assert!(msg.contains(&none_of_slice()), "message was: {msg}");
        let v = read_report(adapter.path(), "gsm8k_think");
        assert_eq!(v["status"], "not_scored");
        assert_eq!(v["n"], 0);
        assert_eq!(v["n_requested"], SLICE_SIZE);
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
        let err = score_reasoning_file_with(
            "gsm8k_think",
            &gsm8k_slice(),
            adapter.path(),
            Some("4k"),
            &ck_root(),
            false,
        )
        .expect_err("a non-integer cap must not be scored");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("VOX_REASONING_EVAL_MAX_TOKENS"),
            "message was: {msg}"
        );
        assert!(msg.contains(&none_of_slice()), "message was: {msg}");
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

    #[test]
    fn a_cancel_stops_consumption() {
        let prompts: Vec<String> = (0..5)
            .map(|i| format!(r#"{{"prompt":"q{i}","max_tokens":10,"temperature":0.0,"seed":0}}"#))
            .collect();
        let mut calls: Vec<String> = Vec::new();
        let mut handed = 0usize;
        let mut infer = |p: &str| -> Result<String> {
            let v: serde_json::Value = serde_json::from_str(p)?;
            if v.get("batch").is_some() {
                calls.push("batch".into());
                return Ok(serde_json::json!({"batch_started": 5, "batch_size": 2}).to_string());
            }
            if v.get("batch_next").is_some() {
                calls.push("batch_next".into());
                let i = handed;
                handed += 1;
                return Ok(
                    serde_json::json!({"index": i, "response": format!("resp{i}")}).to_string(),
                );
            }
            if v.get("batch_cancel").is_some() {
                calls.push("batch_cancel".into());
                return Ok(serde_json::json!({"cancelled": 5 - handed}).to_string());
            }
            calls.push("single".into());
            Ok("single".into())
        };

        let mut on_done_count = 0;
        decode_stream(
            "test_bench",
            &prompts,
            Some(2),
            None,
            &mut |_idx, _resp| {
                on_done_count += 1;
                if on_done_count == 2 {
                    Ok(StreamControl::Cancel)
                } else {
                    Ok(StreamControl::Continue)
                }
            },
            &mut infer,
        )
        .unwrap();

        assert_eq!(on_done_count, 2);
        let cancel_calls: Vec<&str> = calls
            .iter()
            .filter(|c| *c == "batch_cancel")
            .map(String::as_str)
            .collect();
        assert_eq!(
            cancel_calls.len(),
            1,
            "backend receives exactly one batch_cancel"
        );
        let last_call = calls.last().unwrap();
        assert_eq!(
            last_call, "batch_cancel",
            "no batch_next after batch_cancel"
        );
    }

    #[test]
    fn a_stream_reports_every_item() {
        let prompts: Vec<String> = (0..5)
            .map(|i| format!(r#"{{"prompt":"q{i}","max_tokens":10,"temperature":0.0,"seed":0}}"#))
            .collect();
        let mut batch_calls = 0;
        let mut handed = 0usize;
        let mut infer = |p: &str| -> Result<String> {
            let v: serde_json::Value = serde_json::from_str(p)?;
            if v.get("batch").is_some() {
                batch_calls += 1;
                return Ok(serde_json::json!({"batch_started": 5, "batch_size": 2}).to_string());
            }
            if v.get("batch_next").is_some() {
                if handed >= 5 {
                    return Ok(r#"{"done":true}"#.into());
                }
                let i = handed;
                handed += 1;
                return Ok(
                    serde_json::json!({"index": i, "response": format!("resp{i}")}).to_string(),
                );
            }
            Ok("single".into())
        };

        let mut seen_indices = Vec::new();
        decode_stream(
            "test_bench",
            &prompts,
            Some(2),
            None,
            &mut |idx, _resp| {
                seen_indices.push(idx);
                Ok(StreamControl::Continue)
            },
            &mut infer,
        )
        .unwrap();

        assert_eq!(batch_calls, 1, "backend received one batch request");
        assert_eq!(
            seen_indices,
            vec![0, 1, 2, 3, 4],
            "sees each index 0..5 exactly once"
        );
    }

    #[test]
    fn the_committed_decode_identity_file_loads() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/config/decode-identity.yaml");
        let f32_width =
            verified_batch_width(&path, &vox_eval::decode_protocol::DecodeProtocol::F32).unwrap();
        assert_eq!(f32_width, Some(64));

        let bf16_width = verified_batch_width(
            &path,
            &vox_eval::decode_protocol::DecodeProtocol::Bf16 { gemm_rows: 32 },
        )
        .unwrap();
        assert_eq!(bf16_width, None);
    }

    #[test]
    fn decode_identity_refuses_zero_width_or_unknown_key() {
        let dir = tempfile::tempdir().unwrap();
        let zero_path = dir.path().join("zero.yaml");
        std::fs::write(
            &zero_path,
            "f32:\n  verified_batch_width: 0\n  evidence: none\n",
        )
        .unwrap();
        let err_zero =
            verified_batch_width(&zero_path, &vox_eval::decode_protocol::DecodeProtocol::F32)
                .unwrap_err();
        assert!(format!("{err_zero:#}").contains("verified_batch_width must be greater than zero"));

        let unknown_path = dir.path().join("unknown.yaml");
        std::fs::write(
            &unknown_path,
            "f32:\n  verified_batch_width: 16\n  evidence: none\n  extra_field: bad\n",
        )
        .unwrap();
        let err_unknown = verified_batch_width(
            &unknown_path,
            &vox_eval::decode_protocol::DecodeProtocol::F32,
        )
        .unwrap_err();
        assert!(format!("{err_unknown:#}").contains("unknown.yaml"));
    }

    #[test]
    fn a_request_carries_the_verified_width() {
        let prompts: Vec<String> = (0..2)
            .map(|i| format!(r#"{{"prompt":"q{i}","max_tokens":10,"temperature":0.0,"seed":0}}"#))
            .collect();
        let recorded_bodies = std::sync::Mutex::new(Vec::new());
        let mut infer = |p: &str| -> Result<String> {
            let v: serde_json::Value = serde_json::from_str(p)?;
            if v.get("batch").is_some() {
                recorded_bodies.lock().unwrap().push(v);
                return Ok(serde_json::json!({"batch_started": 2, "batch_size": 2}).to_string());
            }
            if v.get("batch_next").is_some() {
                return Ok(r#"{"done":true}"#.into());
            }
            Ok("single".into())
        };

        // free_bytes case (size: None)
        decode_stream(
            "bench",
            &prompts,
            None,
            Some(64),
            &mut |_, _| Ok(StreamControl::Continue),
            &mut infer,
        )
        .unwrap();
        assert_eq!(
            recorded_bodies.lock().unwrap().last().unwrap()["max_batch_size"],
            64
        );

        // explicit batch_size case (size: Some(16))
        decode_stream(
            "bench",
            &prompts,
            Some(16),
            Some(64),
            &mut |_, _| Ok(StreamControl::Continue),
            &mut infer,
        )
        .unwrap();
        assert_eq!(
            recorded_bodies.lock().unwrap().last().unwrap()["max_batch_size"],
            64
        );
    }

    #[test]
    fn an_explicit_width_above_the_verified_width_is_clamped() {
        let prompts: Vec<String> = (0..2)
            .map(|i| format!(r#"{{"prompt":"q{i}","max_tokens":10,"temperature":0.0,"seed":0}}"#))
            .collect();
        let recorded_body = std::sync::Mutex::new(None);
        let mut infer = |p: &str| -> Result<String> {
            let v: serde_json::Value = serde_json::from_str(p)?;
            if v.get("batch").is_some() {
                *recorded_body.lock().unwrap() = Some(v);
                return Ok(serde_json::json!({"batch_started": 2, "batch_size": 16}).to_string());
            }
            if v.get("batch_next").is_some() {
                return Ok(r#"{"done":true}"#.into());
            }
            Ok("single".into())
        };

        decode_stream(
            "bench",
            &prompts,
            Some(64),
            Some(16),
            &mut |_, _| Ok(StreamControl::Continue),
            &mut infer,
        )
        .unwrap();
        let guard = recorded_body.lock().unwrap();
        let body = guard.as_ref().unwrap();
        assert_eq!(body["batch_size"], 16);
        assert_eq!(body["max_batch_size"], 16);
    }
}
