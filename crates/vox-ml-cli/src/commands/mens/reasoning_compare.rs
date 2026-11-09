//! `vox mens eval-reasoning-compare` (BENCH-01): score a pinned Qwen3 base and an
//! adapter trained on it on the three checked-in, hash-pinned reasoning slices.
//!
//! The command writes exactly one file, `--out` (default
//! `mens/runs/reasoning_compare/<size>-<rev12>.json`), and nothing else. Detection
//! never loads a model. When the pinned snapshot or a matching adapter is not on
//! disk it writes a `not_compared` record with no score keys. The record also
//! names the base's ladder rungs (`base_rungs`) and whether each floor was
//! measured, and stderr says `floor unmeasured` for an estimate. The report is never
//! a serve-gate input: the out guard refuses every file name and directory that
//! `vox mens serve` reads.

mod detect;

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use vox_eval::reasoning_bench::{
    AbsenceReason, BEFORE_AFTER_SCHEMA, BaseLabel, BenchOutcome, ReasoningItem, SLICE_NAMES,
    SLICE_SIZE, compared_report_json, load_slice,
};

use super::reasoning_eval;
use detect::Detected;

/// Subdirectory of the runs root that holds one comparison file per base.
pub(crate) const REASONING_COMPARE_SUBDIR: &str = "reasoning_compare";

/// File names the serve gate reads; `--out` may never be one of them.
const GATE_FILE_NAMES: [&str; 2] = ["collateral_damage_report.json", "reasoning_report.json"];

/// Which model a scorer call is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    /// The pinned base snapshot ("before").
    Base,
    /// The adapter run directory ("after").
    Adapter,
}

/// How a run ended when it returned `Ok`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunStatus {
    /// All six entries were scored.
    Compared,
    /// `--slices` named a strict subset and both sides of each were scored.
    Partial,
    /// An absence record was written.
    NotCompared(AbsenceReason),
}

/// Parsed CLI arguments.
#[derive(Debug, Clone)]
pub(crate) struct CompareArgs {
    pub base: Option<String>,
    pub adapter: Option<PathBuf>,
    pub hub_root: Option<PathBuf>,
    pub bench_root: PathBuf,
    pub out: Option<PathBuf>,
    /// Score only these slices (a `partial` record unless every slice is named).
    pub slices: Option<Vec<String>>,
    /// Build the report from the per-item checkpoint only; never generate.
    pub merge_only: bool,
    /// Discard existing per-item checkpoints instead of reusing them.
    pub force_restart: bool,
    /// Discard existing base per-item checkpoints instead of reusing them.
    pub force_restart_base: bool,
}

/// Inference error for an item `--merge-only` found no checkpointed generation for.
pub(crate) const MERGE_ONLY_REFUSAL: &str =
    "not in the per-item checkpoint; --merge-only never generates";

/// Scores one side on the loaded slices and returns one named outcome per slice.
pub(crate) type SideScorer<'a> = &'a mut dyn FnMut(
    Side,
    &Path,
    &[(&'static str, Vec<ReasoningItem>)],
) -> Vec<(&'static str, BenchOutcome)>;

/// `p` where the filesystem will put it: absolute, each existing prefix
/// canonicalized (symlinks, `..`, on-disk letter case), and `..` after a missing
/// directory popped, as `create_dir_all` then `write` would resolve it. A path
/// entry that exists but does not canonicalize (a dangling symlink) is refused,
/// since writing through it would create its target.
fn resolved(p: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(p).with_context(|| format!("resolving {}", p.display()))?;
    let mut cur = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                cur.pop();
            }
            other => {
                cur.push(other);
                match cur.canonicalize() {
                    Ok(real) => cur = real,
                    Err(_) if std::fs::symlink_metadata(&cur).is_ok() => anyhow::bail!(
                        "{} exists but does not resolve (dangling symlink?); refusing",
                        cur.display()
                    ),
                    Err(_) => {}
                }
            }
        }
    }
    Ok(cur)
}

/// Refuse an `--out` that serve could read, that is a symlink, or that resolves
/// into the hub root, the adapter dir, or the bench root.
pub(crate) fn check_out_path(
    out: &Path,
    hub_root: &Path,
    adapter: Option<&Path>,
    bench_root: &Path,
) -> Result<()> {
    if std::fs::symlink_metadata(out).is_ok_and(|m| m.file_type().is_symlink()) {
        anyhow::bail!("--out {} is a symlink; refusing", out.display());
    }
    let abs = resolved(out)?;
    for p in [out, abs.as_path()] {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if GATE_FILE_NAMES.iter().any(|g| g.eq_ignore_ascii_case(name)) {
            anyhow::bail!(
                "--out {} is a serve-gate file name; refusing",
                out.display()
            );
        }
        if p.components()
            .any(|c| c.as_os_str().eq_ignore_ascii_case("collateral_eval"))
        {
            anyhow::bail!(
                "--out {} is inside a collateral_eval dir; refusing",
                out.display()
            );
        }
    }
    for (what, root) in [("hub root", hub_root), ("bench root", bench_root)]
        .into_iter()
        .chain(adapter.map(|a| ("adapter dir", a)))
    {
        if abs.starts_with(resolved(root)?) {
            anyhow::bail!(
                "--out {} is inside the {what} {}; refusing",
                out.display(),
                root.display()
            );
        }
    }
    Ok(())
}

/// Write `value` pretty-printed to `out`, creating the parent directory.
fn write_report(out: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(out, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("writing {}", out.display()))
}

/// Remove a prior `reasoning_before_after.v1` record at `out`. Anything else
/// already there (another file, a directory, unparseable JSON) is refused and
/// left as it is: a mistyped `--out` must not delete data.
fn remove_stale_record(out: &Path) -> Result<()> {
    let meta = match std::fs::symlink_metadata(out) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("inspecting {}", out.display())),
    };
    let is_record = meta.file_type().is_file()
        && vox_bounded_fs::read_utf8_path_capped(out)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .is_some_and(|v| v["schema"] == BEFORE_AFTER_SCHEMA);
    if !is_record {
        anyhow::bail!(
            "--out {} exists and is not a {BEFORE_AFTER_SCHEMA} record; refusing to replace it",
            out.display()
        );
    }
    std::fs::remove_file(out).with_context(|| format!("removing stale {}", out.display()))
}

/// Remove a stale record at `out`, then write either the absence record or the
/// comparison. The absent branch never calls `score_side`. Both carry
/// `base_rungs`, the base's ladder rungs (empty off the ladder).
pub(crate) fn run_before_after_with(
    detected: Detected,
    bench_root: &Path,
    out: &Path,
    base_rungs: &serde_json::Value,
    selected: &[&'static str],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    remove_stale_record(out)?;
    match detected {
        Detected::NotCompared(record) => {
            let mut value = record.to_json();
            value["base_rungs"] = base_rungs.clone();
            value["decode"] = settings.report_object();
            write_report(out, &value)?;
            Ok(RunStatus::NotCompared(record.reason))
        }
        Detected::Ready {
            base,
            snapshot,
            adapter,
        } => {
            let mut load_errors: Vec<(&'static str, String)> = Vec::new();
            let mut loaded: Vec<(&'static str, Vec<ReasoningItem>)> = Vec::new();
            for name in selected.iter().copied() {
                match load_slice(&bench_root.join(name).join("reasoning.jsonl"), SLICE_SIZE) {
                    Ok(items) => loaded.push((name, items)),
                    Err(e) => load_errors.push((name, format!("loading slice: {e}"))),
                }
            }
            // No slice loaded means nothing to score; do not load a model for it.
            let (base_out, adapter_out) = if loaded.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                (
                    score_side(Side::Base, &snapshot, &loaded),
                    score_side(Side::Adapter, &adapter, &loaded),
                )
            };
            let pick = |outs: &[(&'static str, BenchOutcome)], name: &str| -> BenchOutcome {
                let not_scored = |reason: &str| BenchOutcome::NotScored {
                    reason: reason.to_string(),
                    n_requested: SLICE_SIZE,
                };
                if let Some((_, reason)) = load_errors.iter().find(|(n, _)| *n == name) {
                    return not_scored(reason);
                }
                outs.iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, o)| o.clone())
                    .unwrap_or_else(|| not_scored("scorer returned no outcome"))
            };
            let rows: Vec<(&str, BenchOutcome, BenchOutcome)> = selected
                .iter()
                .map(|n| (*n, pick(&base_out, n), pick(&adapter_out, n)))
                .collect();
            let adapter_dir = adapter.canonicalize().unwrap_or(adapter);
            let (mut report, compared) = compared_report_json(&base, &adapter_dir, &rows);
            let is_scored = |o: &BenchOutcome| matches!(o, BenchOutcome::Scored(_));
            let partial = selected.len() < SLICE_NAMES.len()
                && rows.iter().all(|(_, b, a)| is_scored(b) && is_scored(a));
            if partial {
                report["status"] = "partial".into();
                report["slices_requested"] = serde_json::json!(selected);
            }
            report["base_rungs"] = base_rungs.clone();
            report["decode"] = settings.report_object();
            write_report(out, &report)?;
            if compared {
                return Ok(RunStatus::Compared);
            }
            if partial {
                return Ok(RunStatus::Partial);
            }
            let first = rows.iter().find_map(|(n, b, a)| {
                (!is_scored(b))
                    .then_some((*n, "base"))
                    .or((!is_scored(a)).then_some((*n, "adapter")))
            });
            match first {
                Some((name, side)) => anyhow::bail!(
                    "comparison incomplete: {name} not scored on the {side} side ({})",
                    out.display()
                ),
                None => anyhow::bail!("comparison incomplete ({})", out.display()),
            }
        }
    }
}

/// Score every loaded slice of one model through the shared per-item
/// checkpoint under `ck_root`, calling `infer` only for items not yet recorded.
/// `Err` only when a checkpoint cannot be opened (a refused mismatch).
pub(crate) fn score_slices_checkpointed(
    ck_root: &Path,
    model_dir: &Path,
    slices: &[(&'static str, Vec<ReasoningItem>)],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart: bool,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Vec<(&'static str, BenchOutcome)>> {
    slices
        .iter()
        .map(|(name, items)| {
            let (outcome, _rows) = reasoning_eval::score_slice_checkpointed(
                ck_root,
                model_dir,
                name,
                items,
                settings,
                force_restart,
                infer,
            )?;
            Ok((*name, outcome))
        })
        .collect()
}

/// Score one side through the checkpoint, loading `model_dir` in the selected
/// backend only if an item needs generating and unloading it before returning,
/// so two models are never resident. `merge_only` never loads a model. Any
/// plugin or checkpoint error makes every passed slice `NotScored` (fail
/// closed, no number).
/// Score one side through `prefill_benches` and `score_slices_checkpointed`.
/// The prefill runs once over all pending items with `force_restart`.
/// The scoring pass then runs with `force_restart = false`, reading the records
/// just written without discarding them again.
pub(crate) fn score_side_with(
    ck_root: &Path,
    model_dir: &Path,
    slices: &[(&'static str, Vec<ReasoningItem>)],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart: bool,
    merge_only: bool,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Result<Vec<(&'static str, BenchOutcome)>> {
    if !merge_only {
        let bench_refs: Vec<(&str, &[ReasoningItem])> = slices
            .iter()
            .map(|(n, items)| (*n, items.as_slice()))
            .collect();
        reasoning_eval::prefill_benches(
            ck_root,
            model_dir,
            &bench_refs,
            settings,
            force_restart,
            &mut |_, _| Ok(reasoning_eval::StreamControl::Continue),
            infer,
        )?;
    }
    let mut refuse = |_: &str| -> Result<String> { anyhow::bail!("{MERGE_ONLY_REFUSAL}") };
    let inner: &mut dyn FnMut(&str) -> Result<String> =
        if merge_only { &mut refuse } else { infer };
    score_slices_checkpointed(ck_root, model_dir, slices, settings, false, inner)
}

/// Score one side through the checkpoint, loading `model_dir` in the selected
/// backend only if an item needs generating and unloading it before returning,
/// so two models are never resident. `merge_only` never loads a model. Any
/// plugin or checkpoint error makes every passed slice `NotScored` (fail
/// closed, no number).
fn score_side_with_backend(
    model_dir: &Path,
    slices: &[(&'static str, Vec<ReasoningItem>)],
    settings: &vox_eval::decode_protocol::DecodeSettings,
    force_restart: bool,
    merge_only: bool,
) -> Vec<(&'static str, BenchOutcome)> {
    let ck_root = Path::new(crate::commands::mens::item_checkpoint::REASONING_CHECKPOINT_ROOT);
    let score = |infer: &mut dyn FnMut(&str) -> Result<String>| {
        score_side_with(
            ck_root,
            model_dir,
            slices,
            settings,
            force_restart,
            merge_only,
            infer,
        )
    };
    let result = if merge_only {
        let mut refuse = |_: &str| -> Result<String> { anyhow::bail!("{MERGE_ONLY_REFUSAL}") };
        score(&mut refuse)
    } else {
        reasoning_eval::with_backend(model_dir, false, score).and_then(|r| r)
    };
    match result {
        Ok(outcomes) => outcomes,
        Err(e) => slices
            .iter()
            .map(|(name, _)| {
                (
                    *name,
                    BenchOutcome::NotScored {
                        reason: format!("scored 0 of {SLICE_SIZE} items: {e:#}"),
                        n_requested: SLICE_SIZE,
                    },
                )
            })
            .collect(),
    }
}

/// The `--slices` selection in canonical slice order, or every slice. An
/// unknown name is refused before anything is detected or written.
fn selected_slices(requested: Option<&[String]>) -> Result<Vec<&'static str>> {
    let Some(requested) = requested else {
        return Ok(SLICE_NAMES.to_vec());
    };
    if let Some(bad) = requested
        .iter()
        .find(|r| !SLICE_NAMES.contains(&r.as_str()))
    {
        anyhow::bail!(
            "--slices names unknown slice {bad:?}; known slices: {}",
            SLICE_NAMES.join(", ")
        );
    }
    if requested.is_empty() {
        anyhow::bail!("--slices names no slice");
    }
    Ok(SLICE_NAMES
        .iter()
        .copied()
        .filter(|n| requested.iter().any(|r| r == n))
        .collect())
}

/// The single entry: resolve the pin and hub root, guard `--out`, detect, write.
pub(crate) fn run_eval_reasoning_compare_with(
    args: CompareArgs,
    env_hub: detect::HubEnv,
    runs_root: &Path,
    overlay_root: &Path,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    let selected = selected_slices(args.slices.as_deref())?;
    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(overlay_root)
        .context("loading train_bases from mens/config/gpu-specs.yaml")?;
    // No --base means the 8B default; the size selector itself has no default size.
    let requested = args
        .base
        .as_deref()
        .unwrap_or(vox_populi::mens::DEFAULT_MODEL_ID);
    let pin = detect::resolve_base_pin(requested, &overlay)?;
    let rungs = vox_populi::mens::tensor::spoke_base_resolver::rungs_for_hf_id(&overlay, &pin);
    for rung in &rungs {
        eprintln!("base {pin}: {}", rung.floor_label());
    }
    let base_rungs = serde_json::to_value(&rungs).context("serializing the base's ladder rungs")?;
    let label = BaseLabel::from_pin(&pin);
    let hub_root = detect::resolve_hub_root(args.hub_root.as_deref(), env_hub)
        .ok_or_else(|| anyhow::anyhow!("no Hugging Face hub root found; pass --hub-root <dir>"))?;
    let out = args.out.clone().unwrap_or_else(|| {
        runs_root
            .join(REASONING_COMPARE_SUBDIR)
            .join(label.report_file_name())
    });
    check_out_path(&out, &hub_root, args.adapter.as_deref(), &args.bench_root)?;
    let detected = detect::detect_inputs(&hub_root, &label, args.adapter.as_deref());
    let max_tokens = reasoning_eval::parse_max_tokens(reasoning_eval::max_tokens_env().as_deref())
        .unwrap_or(2048);
    let settings = reasoning_eval::decode_settings(max_tokens).unwrap_or_default();
    let status = run_before_after_with(
        detected,
        &args.bench_root,
        &out,
        &base_rungs,
        &selected,
        &settings,
        score_side,
    )?;
    match status {
        RunStatus::NotCompared(reason) => {
            eprintln!("NOT COMPARED: {} ({})", reason.as_str(), out.display());
        }
        RunStatus::Compared | RunStatus::Partial => println!("{}", out.display()),
    }
    Ok(status)
}

/// CLI wrapper: real environment, `mens/runs` as the runs root, the real scorer.
pub(crate) fn run_eval_reasoning_compare(args: CompareArgs) -> Result<()> {
    let overlay_root = std::env::current_dir()?;
    let max_tokens = reasoning_eval::parse_max_tokens(reasoning_eval::max_tokens_env().as_deref())?;
    let settings = reasoning_eval::decode_settings(max_tokens)?;
    let force = crate::commands::mens::item_checkpoint::ForceRestart {
        adapter: args.force_restart,
        base: args.force_restart_base,
    };
    let merge_only = args.merge_only;
    let mut score = |side: Side, dir: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
        let side_force = match side {
            Side::Base => force.for_base(),
            Side::Adapter => force.for_adapter(),
        };
        score_side_with_backend(dir, slices, &settings, side_force, merge_only)
    };
    run_eval_reasoning_compare_with(
        args,
        detect::HubEnv::from_process(),
        Path::new("mens/runs"),
        &overlay_root,
        &mut score,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::detect::tests::{PIN_8B, fake_adapter};
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn args(hub: &Path, out: Option<PathBuf>, adapter: Option<PathBuf>) -> CompareArgs {
        CompareArgs {
            base: None,
            adapter,
            hub_root: Some(hub.to_path_buf()),
            bench_root: repo_root().join("mens/data"),
            out,
            slices: None,
            merge_only: false,
            force_restart: false,
            force_restart_base: false,
        }
    }

    fn run(a: CompareArgs, runs_root: &Path, score: SideScorer<'_>) -> Result<RunStatus> {
        run_eval_reasoning_compare_with(a, Default::default(), runs_root, &repo_root(), score)
    }

    fn panicking_scorer() -> impl FnMut(
        Side,
        &Path,
        &[(&'static str, Vec<ReasoningItem>)],
    ) -> Vec<(&'static str, BenchOutcome)> {
        |_, _, _| panic!("the scorer must not be called")
    }

    fn read_json(p: &Path) -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(p).expect("report exists")).expect("json")
    }

    #[test]
    fn absent_branch_never_calls_the_scorer() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        let out = runs.path().join("rec.json");
        let mut score = panicking_scorer();
        let status = run(
            args(hub.path(), Some(out.clone()), None),
            runs.path(),
            &mut score,
        )
        .expect("absent branch is Ok");
        assert_eq!(
            status,
            RunStatus::NotCompared(AbsenceReason::BaseSnapshotAbsent)
        );
        let v = read_json(&out);
        assert_eq!(v["status"], "not_compared");
        assert_eq!(v["schema"], "reasoning_before_after.v1");
        assert_eq!(v["base_requested"], PIN_8B);
    }

    #[test]
    fn out_named_like_a_gate_file_is_refused() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        for out in [
            runs.path().join("collateral_damage_report.json"),
            runs.path().join("x").join("reasoning_report.json"),
            runs.path().join("collateral_eval").join("cmp.json"),
        ] {
            let mut score = panicking_scorer();
            let r = run(
                args(hub.path(), Some(out.clone()), None),
                runs.path(),
                &mut score,
            );
            assert!(r.is_err(), "{} must be refused", out.display());
            assert!(!out.exists(), "{} was created", out.display());
        }
    }

    #[test]
    fn out_inside_hub_root_or_adapter_is_refused() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        let adapter = runs.path().join("adapter");
        fake_adapter(&adapter, hub.path());
        for out in [
            hub.path().join("cmp.json"),
            adapter.join("sub").join("cmp.json"),
            runs.path()
                .join("elsewhere")
                .join("..")
                .join("adapter")
                .join("cmp.json"),
        ] {
            let mut score = panicking_scorer();
            let a = args(hub.path(), Some(out.clone()), Some(adapter.clone()));
            assert!(
                run(a, runs.path(), &mut score).is_err(),
                "{} must be refused",
                out.display()
            );
            assert!(!out.exists(), "{} was created", out.display());
        }
    }

    #[test]
    fn stale_compared_report_is_replaced() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        let out = runs.path().join("rec.json");
        std::fs::write(
            &out,
            r#"{"schema":"reasoning_before_after.v1","status":"compared"}"#,
        )
        .expect("seed stale");
        let mut score = panicking_scorer();
        run(
            args(hub.path(), Some(out.clone()), None),
            runs.path(),
            &mut score,
        )
        .expect("ok");
        let v = read_json(&out);
        assert_eq!(v["status"], "not_compared");
        assert_eq!(v["reason"], "base_snapshot_absent");
    }

    #[test]
    fn existing_file_that_is_not_a_prior_record_is_refused_and_kept() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        for (name, body) in [
            ("notes.txt", "precious"),
            ("other.json", r#"{"schema":"something_else.v1"}"#),
        ] {
            let out = runs.path().join(name);
            std::fs::write(&out, body).expect("seed");
            let mut score = panicking_scorer();
            assert!(
                run(
                    args(hub.path(), Some(out.clone()), None),
                    runs.path(),
                    &mut score
                )
                .is_err(),
                "{name} must be refused"
            );
            assert_eq!(std::fs::read_to_string(&out).expect("kept"), body);
        }
    }

    #[test]
    fn out_under_the_bench_root_is_refused() {
        let (hub, runs, bench) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
            tempfile::tempdir().expect("bench"),
        );
        let out = bench.path().join("gsm8k_think").join("cmp.json");
        let mut a = args(hub.path(), Some(out.clone()), None);
        a.bench_root = bench.path().to_path_buf();
        let mut score = panicking_scorer();
        assert!(run(a, runs.path(), &mut score).is_err());
        assert!(!out.exists(), "{} was created", out.display());
    }

    #[test]
    fn default_out_is_outside_a_runs_v1_adapter() {
        let (hub, tmp) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("tmp"),
        );
        let runs_root = tmp.path().join("mens/runs");
        let adapter = runs_root.join("v1");
        fake_adapter(&adapter, hub.path());
        let before: Vec<_> = std::fs::read_dir(&adapter)
            .expect("ls")
            .map(|e| e.expect("e").path())
            .collect();
        let mut score = panicking_scorer();
        run(
            args(hub.path(), None, Some(adapter.clone())),
            &runs_root,
            &mut score,
        )
        .expect("ok");
        let want = runs_root.join("reasoning_compare/Qwen3-8B-b968826d9c46.json");
        assert_eq!(read_json(&want)["status"], "not_compared");
        let after: Vec<_> = std::fs::read_dir(&adapter)
            .expect("ls")
            .map(|e| e.expect("e").path())
            .collect();
        assert_eq!(before.len(), after.len(), "a file was added under runs/v1");
    }

    // ---- Task 2: the present branch ----

    use super::detect::tests::fake_snapshot;
    use vox_eval::qwen_reasoning::ReasoningScore;

    type Slices = Vec<(&'static str, Vec<ReasoningItem>)>;

    /// Hub with the pinned 8B snapshot, an adapter trained on it, and a runs dir.
    struct Present {
        hub: tempfile::TempDir,
        runs: tempfile::TempDir,
        snapshot: PathBuf,
        adapter: PathBuf,
    }

    fn present() -> Present {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        let snapshot = fake_snapshot(hub.path(), PIN_8B);
        let adapter = runs.path().join("v1");
        fake_adapter(&adapter, &snapshot);
        Present {
            hub,
            runs,
            snapshot,
            adapter,
        }
    }

    impl Present {
        fn out(&self) -> PathBuf {
            self.runs.path().join("cmp.json")
        }
        fn args(&self, bench_root: Option<&Path>) -> CompareArgs {
            let mut a = args(
                self.hub.path(),
                Some(self.out()),
                Some(self.adapter.clone()),
            );
            if let Some(b) = bench_root {
                a.bench_root = b.to_path_buf();
            }
            a
        }
    }

    /// A distinct accuracy per slice and side, so a mismatch shows up.
    fn accuracy_for(side: Side, name: &str) -> f64 {
        let i = SLICE_NAMES
            .iter()
            .position(|n| *n == name)
            .expect("known slice") as f64;
        let s = if side == Side::Base { 0.0 } else { 0.5 };
        s + (i + 1.0) / 8.0
    }

    fn scored(side: Side, name: &str) -> BenchOutcome {
        BenchOutcome::Scored(ReasoningScore {
            accuracy: accuracy_for(side, name),
            think_initiation_rate: 0.5,
            n: 32,
        })
    }

    fn named_scores(
        side: Side,
        slices: &[(&'static str, Vec<ReasoningItem>)],
    ) -> Vec<(&'static str, BenchOutcome)> {
        slices.iter().map(|(n, _)| (*n, scored(side, n))).collect()
    }

    fn checked_in() -> Slices {
        SLICE_NAMES
            .iter()
            .map(|n| {
                let p = repo_root()
                    .join("mens/data")
                    .join(n)
                    .join("reasoning.jsonl");
                (*n, load_slice(&p, SLICE_SIZE).expect("checked-in slice"))
            })
            .collect()
    }

    #[test]
    fn both_sides_get_identical_checked_in_items() {
        let p = present();
        let mut calls: Vec<(Side, PathBuf, Slices)> = Vec::new();
        let mut score = |side: Side, dir: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
            calls.push((side, dir.to_path_buf(), slices.to_vec()));
            named_scores(side, slices)
        };
        run(p.args(None), p.runs.path(), &mut score).expect("compared");
        assert_eq!(calls.len(), 2);
        assert_eq!((calls[0].0, &calls[0].1), (Side::Base, &p.snapshot));
        assert_eq!((calls[1].0, &calls[1].1), (Side::Adapter, &p.adapter));
        let want = checked_in();
        assert_eq!(calls[0].2, want);
        assert_eq!(calls[1].2, want);
    }

    #[test]
    fn present_branch_writes_all_six_entries() {
        let p = present();
        let mut score = |side: Side, _: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
            named_scores(side, slices)
        };
        let status = run(p.args(None), p.runs.path(), &mut score).expect("compared");
        assert_eq!(status, RunStatus::Compared);
        let v = read_json(&p.out());
        let canonical = p.adapter.canonicalize().expect("adapter");
        assert_eq!(v["adapter"]["dir"], canonical.display().to_string());
        assert_eq!(v["status"], "compared");
        assert_eq!(v["gate_input"], false);
        assert!(
            v["base_rungs"].as_array().is_some_and(|r| !r.is_empty()),
            "compared report must carry base_rungs: {}",
            v["base_rungs"]
        );
        for name in SLICE_NAMES {
            for (key, side) in [("base", Side::Base), ("adapter", Side::Adapter)] {
                let e = &v["slices"][name][key];
                assert_eq!(e["accuracy"].as_f64(), Some(accuracy_for(side, name)));
                assert_eq!(e["think_initiation_rate"].as_f64(), Some(0.5));
            }
        }
    }

    #[test]
    fn one_not_scored_side_returns_err_and_writes_incomplete() {
        let p = present();
        let mut score = |side: Side, _: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
            let mut outs = named_scores(side, slices);
            if side == Side::Adapter {
                outs[1].1 = BenchOutcome::NotScored {
                    reason: "item x: inference error".to_string(),
                    n_requested: 32,
                };
            }
            outs
        };
        let err = run(p.args(None), p.runs.path(), &mut score).expect_err("incomplete is Err");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("math500_think") && msg.contains("adapter"),
            "{msg}"
        );
        let v = read_json(&p.out());
        assert_eq!(v["status"], "incomplete");
        assert_eq!(
            v["slices"]["math500_think"]["adapter"]["status"],
            "not_scored"
        );
    }

    #[test]
    fn slice_load_failure_is_not_scored_on_both_sides() {
        let p = present();
        let bench = tempfile::tempdir().expect("bench");
        for name in ["gsm8k_think", "vox_think"] {
            let dir = bench.path().join(name);
            std::fs::create_dir_all(&dir).expect("mkdir");
            std::fs::copy(
                repo_root()
                    .join("mens/data")
                    .join(name)
                    .join("reasoning.jsonl"),
                dir.join("reasoning.jsonl"),
            )
            .expect("copy slice");
        }
        let mut seen: Vec<Vec<&'static str>> = Vec::new();
        let mut score = |side: Side, _: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
            seen.push(slices.iter().map(|(n, _)| *n).collect());
            named_scores(side, slices)
        };
        assert!(run(p.args(Some(bench.path())), p.runs.path(), &mut score).is_err());
        assert_eq!(seen, vec![vec!["gsm8k_think", "vox_think"]; 2]);
        let v = read_json(&p.out());
        assert_eq!(v["status"], "incomplete");
        for key in ["base", "adapter"] {
            assert_eq!(v["slices"]["math500_think"][key]["status"], "not_scored");
        }
        for name in ["gsm8k_think", "vox_think"] {
            assert_eq!(
                v["slices"][name]["base"]["accuracy"].as_f64(),
                Some(accuracy_for(Side::Base, name))
            );
            assert_eq!(
                v["slices"][name]["adapter"]["accuracy"].as_f64(),
                Some(accuracy_for(Side::Adapter, name))
            );
        }
    }

    #[test]
    fn scorer_outcomes_are_matched_by_name() {
        let p = present();
        let mut reversed = |side: Side, _: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
            let mut outs = named_scores(side, slices);
            outs.reverse();
            outs
        };
        assert_eq!(
            run(p.args(None), p.runs.path(), &mut reversed).expect("compared"),
            RunStatus::Compared
        );
        let v = read_json(&p.out());
        for name in SLICE_NAMES {
            assert_eq!(
                v["slices"][name]["base"]["accuracy"].as_f64(),
                Some(accuracy_for(Side::Base, name))
            );
        }
        let mut omits = |side: Side, _: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
            named_scores(side, slices)
                .into_iter()
                .filter(|(n, _)| *n != "vox_think")
                .collect()
        };
        assert!(run(p.args(None), p.runs.path(), &mut omits).is_err());
        let v = read_json(&p.out());
        let e = &v["slices"]["vox_think"]["base"];
        assert_eq!(e["status"], "not_scored");
        assert_eq!(e["reason"], "scorer returned no outcome");
        assert_eq!(
            v["slices"]["gsm8k_think"]["adapter"]["accuracy"].as_f64(),
            Some(accuracy_for(Side::Adapter, "gsm8k_think"))
        );
    }

    /// Each `out` must be refused and leave every snapshot file byte-identical.
    fn assert_refused_and_untouched(p: &Present, outs: &[PathBuf]) {
        std::fs::write(p.snapshot.join("config.json"), b"original").expect("seed config");
        for out in outs {
            let mut score =
                |side: Side, _: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
                    named_scores(side, slices)
                };
            let a = args(p.hub.path(), Some(out.clone()), Some(p.adapter.clone()));
            assert!(
                run(a, p.runs.path(), &mut score).is_err(),
                "{} must be refused",
                out.display()
            );
            for (f, want) in [("config.json", &b"original"[..]), ("tokenizer.json", b"")] {
                assert_eq!(
                    std::fs::read(p.snapshot.join(f)).expect("snapshot file kept"),
                    want,
                    "{f} changed via {}",
                    out.display()
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn out_through_a_symlink_into_the_snapshot_is_refused() {
        let p = present();
        let links = p.runs.path().join("linkparent");
        std::fs::create_dir_all(&links).expect("mkdir links");
        let snap_link = links.join("snap");
        std::os::unix::fs::symlink(&p.snapshot, &snap_link).expect("dir link");
        let file_link = p.runs.path().join("file-link.json");
        std::os::unix::fs::symlink(p.snapshot.join("tokenizer.json"), &file_link)
            .expect("file link");
        let sha = p.snapshot.file_name().expect("sha").to_owned();
        assert_refused_and_untouched(
            &p,
            &[
                // A directory symlink as a parent.
                snap_link.join("config.json"),
                // `..` after a symlink: lexically under runs, really under snapshots.
                snap_link.join("..").join(&sha).join("config.json"),
                // `..` after a missing directory below the symlink.
                snap_link.join("new").join("..").join("config.json"),
                // `--out` itself is a symlink.
                file_link,
            ],
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn out_with_a_case_variant_of_the_hub_root_is_refused() {
        let p = present();
        let hub = p.hub.path();
        let name = hub
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_uppercase();
        let upper = hub.parent().expect("parent").join(name);
        if !upper.is_dir() {
            return; // case-sensitive volume: no alias to test
        }
        let rel = p.snapshot.strip_prefix(hub).expect("snapshot under hub");
        assert_refused_and_untouched(&p, &[upper.join(rel).join("tokenizer.json")]);
    }

    fn gate_files_under(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("entry").path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if GATE_FILE_NAMES.contains(&name) || name == "collateral_eval" {
                found.push(path.clone());
            }
            if path.is_dir() {
                gate_files_under(&path, found);
            }
        }
    }

    #[test]
    fn no_gate_files_are_created() {
        let p = present();
        let mut score = |side: Side, _: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
            named_scores(side, slices)
        };
        run(p.args(None), p.runs.path(), &mut score).expect("compared");
        std::fs::remove_file(p.snapshot.join("tokenizer.json")).expect("make incomplete");
        let status = run(p.args(None), p.runs.path(), &mut score).expect("absent is Ok");
        assert_eq!(
            status,
            RunStatus::NotCompared(AbsenceReason::BaseSnapshotIncomplete)
        );
        let mut found = Vec::new();
        for dir in [&p.adapter, &p.snapshot, &p.runs.path().to_path_buf()] {
            gate_files_under(dir, &mut found);
        }
        assert!(found.is_empty(), "gate files created: {found:?}");
    }

    // ---- Task 3: the base is an input ----

    #[test]
    fn cli_entry_records_the_resolved_pin() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        let mut a = args(hub.path(), None, None);
        a.base = Some("Qwen3-0.6B".to_string());
        let mut score = panicking_scorer();
        let status = run(a, runs.path(), &mut score).expect("absent branch is Ok");
        assert_eq!(
            status,
            RunStatus::NotCompared(AbsenceReason::BaseSnapshotAbsent)
        );
        let v = read_json(
            &runs
                .path()
                .join("reasoning_compare/Qwen3-0.6B-c1899de289a0.json"),
        );
        assert_eq!(v["base_model"], "Qwen/Qwen3-0.6B");
        assert_eq!(
            v["base_revision"],
            "c1899de289a04d12100db370d81485cdf75e47ca"
        );
        assert_eq!(v["base_requested"], super::detect::tests::PIN_0_6B);
    }

    #[test]
    fn record_reports_the_base_rung_floor_status() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        let mut a = args(hub.path(), None, None);
        a.base = Some("Qwen3-4B".to_string());
        let mut score = panicking_scorer();
        let status = run(a, runs.path(), &mut score).expect("absent branch is Ok");
        assert_eq!(
            status,
            RunStatus::NotCompared(AbsenceReason::BaseSnapshotAbsent)
        );
        let v = read_json(
            &runs
                .path()
                .join("reasoning_compare/Qwen3-4B-1cfa9a720891.json"),
        );
        assert_eq!(v["base_model"], "Qwen/Qwen3-4B");
        let rungs = v["base_rungs"].as_array().expect("base_rungs array");
        assert_eq!(rungs.len(), 1, "{rungs:?}");
        assert_eq!(rungs[0]["hf_id"], super::detect::tests::PIN_4B);
        assert_eq!(rungs[0]["floor_mb"], 34460);
        assert_eq!(rungs[0]["floor_measured"], true);
        assert_eq!(rungs[0]["license"], "apache-2.0");

        let out = runs.path().join("off-ladder.json");
        let mut a = args(hub.path(), Some(out.clone()), None);
        a.base = Some("someone/Other-Model@0123456789abcdef0123456789abcdef01234567".to_string());
        let mut score = panicking_scorer();
        run(a, runs.path(), &mut score).expect("absent branch is Ok");
        assert_eq!(read_json(&out)["base_rungs"], serde_json::json!([]));
    }

    #[test]
    fn unresolvable_base_is_an_error_and_writes_nothing() {
        let (hub, runs) = (
            tempfile::tempdir().expect("hub"),
            tempfile::tempdir().expect("runs"),
        );
        for base in ["Qwen3-8b", "Qwen3-1234B", "Qwen/Qwen3-8B@main"] {
            let mut a = args(hub.path(), None, None);
            a.base = Some(base.to_string());
            let mut score = panicking_scorer();
            assert!(run(a, runs.path(), &mut score).is_err(), "{base}");
        }
        assert!(
            !runs.path().join(REASONING_COMPARE_SUBDIR).exists(),
            "a record was written for an unresolvable base"
        );
    }

    #[test]
    fn score_slices_loaded_uses_the_unchanged_grader() {
        let items: Vec<ReasoningItem> = (0..32)
            .map(|i| ReasoningItem {
                id: format!("syn-{i:04}"),
                question: format!("what is {i}?"),
                answer: i.to_string(),
            })
            .collect();
        let slices: Slices = vec![("gsm8k_think", items.clone()), ("vox_think", items)];
        let mut infer = |prompt_json: &str| -> Result<String> {
            let v: serde_json::Value = serde_json::from_str(prompt_json).expect("prompt json");
            anyhow::ensure!(
                v.get("batch").is_none(),
                "this backend decodes one item at a time"
            );
            assert_eq!(v["max_tokens"], 64);
            let prompt = v["prompt"].as_str().expect("prompt");
            let gold = prompt
                .split("what is ")
                .nth(1)
                .and_then(|s| s.split('?').next())
                .expect("gold");
            // Right answer only on even items: 16 of 32.
            let answer = if gold.parse::<u32>().expect("n") % 2 == 0 {
                gold
            } else {
                "999999"
            };
            Ok(serde_json::json!({
                "generated_text": format!("<think>\nx\n</think>\n\n\\boxed{{{answer}}}")
            })
            .to_string())
        };
        let ck = tempfile::tempdir().expect("ck");
        let settings = vox_eval::decode_protocol::DecodeSettings {
            max_tokens: 64,
            ..Default::default()
        };
        let outs =
            score_slices_checkpointed(ck.path(), ck.path(), &slices, &settings, false, &mut infer)
                .expect("checkpoint opens");
        assert_eq!(
            outs.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            ["gsm8k_think", "vox_think"]
        );
        for (_, o) in &outs {
            match o {
                BenchOutcome::Scored(s) => {
                    assert_eq!(s.n, 32);
                    assert_eq!(s.accuracy, 0.5);
                    assert_eq!(s.think_initiation_rate, 1.0);
                }
                other => panic!("expected Scored, got {other:?}"),
            }
        }
    }

    // ---- --slices (fake side scorer) ----

    type Calls = Vec<(Side, Vec<&'static str>)>;

    fn sliced_args(p: &Present, out: &Path, slices: Option<&[&str]>) -> CompareArgs {
        let mut a = p.args(None);
        a.out = Some(out.to_path_buf());
        a.slices = slices.map(|s| s.iter().map(|n| n.to_string()).collect());
        a
    }

    fn run_recording(p: &Present, a: CompareArgs, calls: &mut Calls) -> Result<RunStatus> {
        let mut score = |side: Side, _: &Path, s: &[(&'static str, Vec<ReasoningItem>)]| {
            calls.push((side, s.iter().map(|(n, _)| *n).collect()));
            named_scores(side, s)
        };
        run(a, p.runs.path(), &mut score)
    }

    #[test]
    fn a_sliced_run_scores_only_its_slices_and_is_marked_partial() {
        let p = present();
        let mut calls = Calls::new();
        let status = run_recording(
            &p,
            sliced_args(&p, &p.out(), Some(&["vox_think"])),
            &mut calls,
        )
        .expect("partial is Ok");
        assert_eq!(status, RunStatus::Partial);
        assert_eq!(
            calls,
            vec![
                (Side::Base, vec!["vox_think"]),
                (Side::Adapter, vec!["vox_think"])
            ]
        );
        let v = read_json(&p.out());
        assert_eq!(v["status"], "partial");
        assert_eq!(v["slices_requested"], serde_json::json!(["vox_think"]));
        let keys: Vec<&String> = v["slices"].as_object().expect("slices").keys().collect();
        assert_eq!(keys, ["vox_think"]);
    }

    #[test]
    fn a_sliced_run_naming_every_slice_is_a_full_compare() {
        let p = present();
        let mut calls = Calls::new();
        let all = ["math500_think", "vox_think", "gsm8k_think"];
        let status =
            run_recording(&p, sliced_args(&p, &p.out(), Some(&all)), &mut calls).expect("compared");
        assert_eq!(status, RunStatus::Compared);
        assert_eq!(read_json(&p.out())["status"], "compared");
    }

    #[test]
    fn an_unknown_slice_name_is_refused_before_any_scoring() {
        let p = present();
        let mut calls = Calls::new();
        let err = run_recording(
            &p,
            sliced_args(&p, &p.out(), Some(&["nope_think"])),
            &mut calls,
        )
        .expect_err("unknown slice");
        assert!(format!("{err:#}").contains("nope_think"), "{err:#}");
        assert!(calls.is_empty());
        assert!(!p.out().exists());
    }

    // ---- shared per-item checkpoint (fake inference) ----

    /// Deterministic fake generation: right answer on items whose question has
    /// an even length. Counts calls.
    fn fake_infer(calls: &mut usize) -> impl FnMut(&str) -> Result<String> + '_ {
        move |prompt_json: &str| {
            let v: serde_json::Value = serde_json::from_str(prompt_json).expect("prompt json");
            anyhow::ensure!(
                v.get("batch").is_none(),
                "this backend decodes one item at a time"
            );
            *calls += 1;
            let n = v["prompt"].as_str().expect("prompt").len();
            Ok(serde_json::json!({
                "generated_text": format!("<think>\n{n}\n</think>\n\n\\boxed{{{}}}", n % 7)
            })
            .to_string())
        }
    }

    /// One checkpointed pass over both sides; returns the report bytes.
    fn ck_report(
        p: &Present,
        root: &Path,
        slices: &[(&'static str, Vec<ReasoningItem>)],
        merge_only: bool,
        calls: &mut usize,
    ) -> String {
        let mut infer = fake_infer(calls);
        let mut refuse = |_: &str| -> Result<String> { anyhow::bail!("{MERGE_ONLY_REFUSAL}") };
        let inner: &mut dyn FnMut(&str) -> Result<String> =
            if merge_only { &mut refuse } else { &mut infer };
        let settings = vox_eval::decode_protocol::DecodeSettings::default();
        let base = score_slices_checkpointed(root, &p.snapshot, slices, &settings, false, inner)
            .expect("base");
        let adapter = score_slices_checkpointed(root, &p.adapter, slices, &settings, false, inner)
            .expect("adapter");
        let rows: Vec<(&str, BenchOutcome, BenchOutcome)> = base
            .into_iter()
            .zip(adapter)
            .map(|((n, b), (_, a))| (n, b, a))
            .collect();
        let label = BaseLabel::from_pin(PIN_8B);
        serde_json::to_string_pretty(&compared_report_json(&label, &p.adapter, &rows).0)
            .expect("json")
    }

    fn ck_files(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for d in std::fs::read_dir(root).expect("root") {
            for f in std::fs::read_dir(d.expect("entry").path()).expect("model dir") {
                out.push(f.expect("file").path());
            }
        }
        out.sort();
        out
    }

    #[test]
    fn a_rerun_generates_nothing_already_checkpointed() {
        let p = present();
        let root = p.runs.path().join("ck");
        let (mut first, mut second) = (0, 0);
        let a = ck_report(&p, &root, &checked_in(), false, &mut first);
        assert_eq!(first, 2 * 3 * SLICE_SIZE);
        let b = ck_report(&p, &root, &checked_in(), false, &mut second);
        assert_eq!(second, 0, "every item was already scored");
        assert_eq!(a, b);
    }

    #[test]
    fn a_retrained_adapter_does_not_reuse_old_generations() {
        let p = present();
        let root = p.runs.path().join("ck");
        let vox: Slices = checked_in()
            .into_iter()
            .filter(|(n, _)| *n == "vox_think")
            .collect();
        let mut calls = 0;
        ck_report(&p, &root, &vox, false, &mut calls);
        std::fs::write(p.adapter.join(detect::ADAPTER_WEIGHTS), b"retrained").expect("retrain");
        calls = 0;
        ck_report(&p, &root, &vox, false, &mut calls);
        assert_eq!(
            calls, SLICE_SIZE,
            "only the adapter side is generated again"
        );
    }

    #[test]
    fn resuming_after_a_kill_mid_file_equals_an_uninterrupted_run() {
        let p = present();
        let mut calls = 0;
        let whole = ck_report(
            &p,
            &p.runs.path().join("whole"),
            &checked_in(),
            false,
            &mut calls,
        );

        let root = p.runs.path().join("killed");
        ck_report(&p, &root, &checked_in(), false, &mut calls);
        // Simulate a kill: every file loses its tail, the last kept line torn.
        for f in ck_files(&root) {
            let text = std::fs::read_to_string(&f).expect("ck");
            let keep: usize = text.lines().take(11).map(|l| l.len() + 1).sum();
            std::fs::write(&f, &text.as_bytes()[..keep + 20]).expect("truncate");
        }
        calls = 0;
        let resumed = ck_report(&p, &root, &checked_in(), false, &mut calls);
        assert_eq!(
            calls,
            6 * (SLICE_SIZE - 10),
            "the torn item and the rest are generated"
        );
        assert_eq!(resumed, whole);
    }

    #[test]
    fn slices_scored_separately_merge_into_the_full_report_without_generating() {
        let p = present();
        let mut calls = 0;
        let whole = ck_report(
            &p,
            &p.runs.path().join("whole"),
            &checked_in(),
            false,
            &mut calls,
        );

        let root = p.runs.path().join("sliced");
        for name in ["vox_think", "gsm8k_think", "math500_think"] {
            let one: Slices = checked_in()
                .into_iter()
                .filter(|(n, _)| *n == name)
                .collect();
            ck_report(&p, &root, &one, false, &mut calls);
        }
        calls = 0;
        let merged = ck_report(&p, &root, &checked_in(), true, &mut calls);
        assert_eq!(calls, 0);
        assert_eq!(merged, whole);
    }

    #[test]
    fn merge_only_never_generates_a_missing_item() {
        let p = present();
        let root = p.runs.path().join("ck");
        let vox: Slices = checked_in()
            .into_iter()
            .filter(|(n, _)| *n == "vox_think")
            .collect();
        let mut calls = 0;
        ck_report(&p, &root, &vox, false, &mut calls);
        calls = 0;
        let report = ck_report(&p, &root, &checked_in(), true, &mut calls);
        assert_eq!(calls, 0);
        let v: serde_json::Value = serde_json::from_str(&report).expect("json");
        assert_eq!(v["status"], "incomplete");
        let reason = v["slices"]["gsm8k_think"]["base"]["reason"]
            .as_str()
            .unwrap_or_default();
        assert!(reason.contains("--merge-only"), "{v}");
        assert!(
            v["slices"]["vox_think"]["adapter"]["accuracy"].is_number(),
            "{v}"
        );
    }

    #[test]
    fn compare_report_carries_decode_object() {
        let p = present();
        let out = p.runs.path().join("compare.json");
        let settings = vox_eval::decode_protocol::DecodeSettings {
            protocol: vox_eval::decode_protocol::DecodeProtocol::F32,
            max_tokens: 1024,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let detected = Detected::NotCompared(vox_eval::reasoning_bench::AbsenceRecord {
            base: vox_eval::reasoning_bench::BaseLabel::from_pin("0.6B"),
            reason: vox_eval::reasoning_bench::AbsenceReason::AdapterAbsent,
            searched: None,
            adapter_given: false,
            adapter_on_disk: false,
        });
        run_before_after_with(
            detected,
            Path::new("mens/data"),
            &out,
            &serde_json::json!({}),
            &[],
            &settings,
            &mut |_, _, _| vec![],
        )
        .unwrap();
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(report["decode"], settings.report_object());
    }

    fn batch_fake_infer<'a>(
        batch_requests: &'a std::sync::atomic::AtomicUsize,
        prompts_received: &'a std::sync::Mutex<Vec<String>>,
        single_calls: &'a std::sync::atomic::AtomicUsize,
    ) -> impl FnMut(&str) -> Result<String> + 'a {
        let queue = std::sync::Mutex::new(Vec::new());
        move |p: &str| {
            let v: serde_json::Value = serde_json::from_str(p)?;
            if let Some(arr) = v.get("batch").and_then(|b| b.as_array()) {
                batch_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let mut pr = prompts_received.lock().unwrap();
                for item in arr {
                    pr.push(item["prompt"].as_str().unwrap_or_default().to_string());
                }
                let len = arr.len();
                let mut q = queue.lock().unwrap();
                q.clear();
                for i in 0..len {
                    q.push(i);
                }
                return Ok(serde_json::json!({
                    "batch_started": len,
                    "batch_size": len,
                })
                .to_string());
            }
            if v.get("batch_next").is_some() {
                let mut q = queue.lock().unwrap();
                if q.is_empty() {
                    return Ok(r#"{"done":true}"#.to_string());
                }
                let i = q.remove(0);
                return Ok(serde_json::json!({
                    "index": i,
                    "response": serde_json::json!({
                        "generated_text": "<think>\n42\n</think>\n\n\\boxed{42}"
                    }).to_string()
                })
                .to_string());
            }
            single_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(serde_json::json!({
                "generated_text": "<think>\n42\n</think>\n\n\\boxed{42}"
            })
            .to_string())
        }
    }

    #[test]
    fn compare_decodes_one_stream_per_side() {
        let (root, model) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let s1: Vec<ReasoningItem> = (0..3)
            .map(|i| ReasoningItem {
                id: format!("s1_{i}"),
                question: format!("q1_{i}"),
                answer: "42".into(),
            })
            .collect();
        let s2: Vec<ReasoningItem> = (0..3)
            .map(|i| ReasoningItem {
                id: format!("s2_{i}"),
                question: format!("q2_{i}"),
                answer: "42".into(),
            })
            .collect();
        let slices: Vec<(&'static str, Vec<ReasoningItem>)> = vec![("slice1", s1), ("slice2", s2)];

        let batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let prompts_received = std::sync::Mutex::new(Vec::new());
        let single_calls = std::sync::atomic::AtomicUsize::new(0);
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        let mut infer1 = batch_fake_infer(&batch_requests, &prompts_received, &single_calls);
        let outs1 = score_side_with(
            root.path(),
            model.path(),
            &slices,
            &settings,
            false,
            false,
            &mut infer1,
        )
        .expect("scoring passes");

        assert_eq!(batch_requests.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(prompts_received.lock().unwrap().len(), 6);
        assert_eq!(single_calls.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(outs1.len(), 2);
        for (_, o) in &outs1 {
            match o {
                BenchOutcome::Scored(s) => {
                    assert_eq!(s.n, 3);
                    assert_eq!(s.accuracy, 1.0);
                }
                _ => panic!("expected scored outcome, got {o:?}"),
            }
        }

        // A second scoring of the same side sends no request at all (everything recorded)
        let mut infer2 = batch_fake_infer(&batch_requests, &prompts_received, &single_calls);
        let outs2 = score_side_with(
            root.path(),
            model.path(),
            &slices,
            &settings,
            false,
            false,
            &mut infer2,
        )
        .expect("second scoring passes");

        assert_eq!(batch_requests.load(std::sync::atomic::Ordering::Relaxed), 1); // still 1
        assert_eq!(single_calls.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(outs1, outs2);
    }

    #[test]
    fn a_restart_discards_once() {
        let (root, model) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let s1: Vec<ReasoningItem> = (0..3)
            .map(|i| ReasoningItem {
                id: format!("s1_{i}"),
                question: format!("q1_{i}"),
                answer: "42".into(),
            })
            .collect();
        let s2: Vec<ReasoningItem> = (0..3)
            .map(|i| ReasoningItem {
                id: format!("s2_{i}"),
                question: format!("q2_{i}"),
                answer: "42".into(),
            })
            .collect();
        let slices: Vec<(&'static str, Vec<ReasoningItem>)> = vec![("slice1", s1), ("slice2", s2)];

        let batch_requests = std::sync::atomic::AtomicUsize::new(0);
        let prompts_received = std::sync::Mutex::new(Vec::new());
        let single_calls = std::sync::atomic::AtomicUsize::new(0);
        let settings = vox_eval::decode_protocol::DecodeSettings::default();

        // Populate initial checkpoints
        let mut infer = batch_fake_infer(&batch_requests, &prompts_received, &single_calls);
        score_side_with(
            root.path(),
            model.path(),
            &slices,
            &settings,
            false,
            false,
            &mut infer,
        )
        .expect("initial scoring passes");
        assert_eq!(batch_requests.load(std::sync::atomic::Ordering::Relaxed), 1);

        // Run with force_restart = true:
        // Prefill discards once and sends one batch request for the 6 items.
        // The scoring pass after it runs with force_restart = false, reading the 6 records
        // and sending zero further requests.
        let mut restart_infer = batch_fake_infer(&batch_requests, &prompts_received, &single_calls);
        let outs = score_side_with(
            root.path(),
            model.path(),
            &slices,
            &settings,
            true,
            false,
            &mut restart_infer,
        )
        .expect("restart scoring passes");

        assert_eq!(batch_requests.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert_eq!(single_calls.load(std::sync::atomic::Ordering::Relaxed), 0);
        for (_, o) in &outs {
            match o {
                BenchOutcome::Scored(s) => assert_eq!(s.n, 3),
                _ => panic!("expected scored outcome, got {o:?}"),
            }
        }
    }
}
