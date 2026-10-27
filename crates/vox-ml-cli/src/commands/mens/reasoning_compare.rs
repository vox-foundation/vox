//! `vox mens eval-reasoning-compare` (BENCH-01): score a pinned Qwen3 base and an
//! adapter trained on it on the three checked-in 32-item reasoning slices.
//!
//! The command writes exactly one file, `--out` (default
//! `mens/runs/reasoning_compare/<size>-<rev12>.json`), and nothing else. Detection
//! never loads a model. When the pinned snapshot or a matching adapter is not on
//! disk it writes a `not_compared` record with no score keys. The report is never
//! a serve-gate input: the out guard refuses every file name and directory that
//! `vox mens serve` reads.

mod detect;

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use vox_eval::reasoning_bench::{
    AbsenceReason, BaseLabel, BenchOutcome, ReasoningItem, SLICE_NAMES, SLICE_SIZE,
    compared_report_json, load_slice,
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
}

/// Scores one side on the loaded slices and returns one named outcome per slice.
pub(crate) type SideScorer<'a> = &'a mut dyn FnMut(
    Side,
    &Path,
    &[(&'static str, Vec<ReasoningItem>)],
) -> Vec<(&'static str, BenchOutcome)>;

/// `p` made absolute with `.` and `..` removed lexically.
fn normalized(p: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(p).with_context(|| format!("resolving {}", p.display()))?;
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

/// Refuse an `--out` that serve could read or that sits in a model directory.
pub(crate) fn check_out_path(out: &Path, hub_root: &Path, adapter: Option<&Path>) -> Result<()> {
    let name = out.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if GATE_FILE_NAMES.iter().any(|g| g.eq_ignore_ascii_case(name)) {
        anyhow::bail!(
            "--out {} is a serve-gate file name; refusing",
            out.display()
        );
    }
    if out
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("collateral_eval"))
    {
        anyhow::bail!(
            "--out {} is inside a collateral_eval dir; refusing",
            out.display()
        );
    }
    let abs = normalized(out)?;
    for (what, root) in
        std::iter::once(("hub root", hub_root)).chain(adapter.map(|a| ("adapter dir", a)))
    {
        if abs.starts_with(normalized(root)?) {
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

/// Remove any stale file at `out`, then write either the absence record or the
/// comparison. The absent branch never calls `score_side`.
pub(crate) fn run_before_after_with(
    detected: Detected,
    bench_root: &Path,
    out: &Path,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    match std::fs::remove_file(out) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).with_context(|| format!("removing stale {}", out.display())),
    }
    match detected {
        Detected::NotCompared(record) => {
            write_report(out, &record.to_json())?;
            Ok(RunStatus::NotCompared(record.reason))
        }
        Detected::Ready {
            base,
            snapshot,
            adapter,
        } => {
            let mut load_errors: Vec<(&'static str, String)> = Vec::new();
            let mut loaded: Vec<(&'static str, Vec<ReasoningItem>)> = Vec::new();
            for name in SLICE_NAMES {
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
            let rows: Vec<(&str, BenchOutcome, BenchOutcome)> = SLICE_NAMES
                .iter()
                .map(|n| (*n, pick(&base_out, n), pick(&adapter_out, n)))
                .collect();
            let (report, compared) = compared_report_json(&base, &rows);
            write_report(out, &report)?;
            if compared {
                return Ok(RunStatus::Compared);
            }
            let is_scored = |o: &BenchOutcome| matches!(o, BenchOutcome::Scored(_));
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

/// Score every loaded slice through `infer` with the unchanged grader. Writes nothing.
pub(crate) fn score_slices_loaded(
    slices: &[(&'static str, Vec<ReasoningItem>)],
    max_tokens: usize,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Vec<(&'static str, BenchOutcome)> {
    slices
        .iter()
        .map(|(name, items)| {
            let (outcome, _rows) = reasoning_eval::score_loaded(name, items, max_tokens, infer);
            (*name, outcome)
        })
        .collect()
}

/// Load `model_dir` once in the selected backend, score, and unload before
/// returning so two models are never resident. Any plugin or load error makes
/// every passed slice `NotScored` (fail closed, no number).
fn score_side_with_backend(
    model_dir: &Path,
    slices: &[(&'static str, Vec<ReasoningItem>)],
    max_tokens: usize,
) -> Vec<(&'static str, BenchOutcome)> {
    match backend_scores(model_dir, slices, max_tokens) {
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

fn backend_scores(
    model_dir: &Path,
    slices: &[(&'static str, Vec<ReasoningItem>)],
    max_tokens: usize,
) -> Result<Vec<(&'static str, BenchOutcome)>> {
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
        .load_model(model_dir.to_string_lossy().as_ref().into())
        .into_result()
        .map_err(|e| anyhow::anyhow!("load_model({}): {e}", model_dir.display()))?;
    let mut infer = |prompt_json: &str| -> Result<String> {
        backend
            .run_inference(&handle, prompt_json.into())
            .into_result()
            .map(|resp| resp.as_str().to_string())
            .map_err(|e| anyhow::anyhow!("{e}"))
    };
    let outcomes = score_slices_loaded(slices, max_tokens, &mut infer);
    if let Err(e) = backend.unload_model(&handle).into_result() {
        eprintln!("warning: unload_model({}): {e}", model_dir.display());
    }
    // `loaded` is a cache reference and must outlive `handle`; it drops here
    // with the other locals, which is after the unload.
    let _plugin = loaded;
    Ok(outcomes)
}

/// The single entry: resolve the pin and hub root, guard `--out`, detect, write.
pub(crate) fn run_eval_reasoning_compare_with(
    args: CompareArgs,
    env_hub: (Option<OsString>, Option<OsString>, Option<PathBuf>),
    runs_root: &Path,
    overlay_root: &Path,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(overlay_root)
        .context("loading train_bases from mens/config/gpu-specs.yaml")?;
    let pin = detect::resolve_base_pin(args.base.as_deref(), &overlay);
    let label = BaseLabel::from_pin(&pin);
    let (hf_hub_cache, hf_home, home) = env_hub;
    let hub_root = detect::resolve_hub_root(args.hub_root.as_deref(), hf_hub_cache, hf_home, home)
        .ok_or_else(|| anyhow::anyhow!("no Hugging Face hub root found; pass --hub-root <dir>"))?;
    let out = args.out.clone().unwrap_or_else(|| {
        runs_root
            .join(REASONING_COMPARE_SUBDIR)
            .join(label.report_file_name())
    });
    check_out_path(&out, &hub_root, args.adapter.as_deref())?;
    let detected = detect::detect_inputs(&hub_root, &label, args.adapter.as_deref());
    let status = run_before_after_with(detected, &args.bench_root, &out, score_side)?;
    match status {
        RunStatus::NotCompared(reason) => {
            eprintln!("NOT COMPARED: {} ({})", reason.as_str(), out.display());
        }
        RunStatus::Compared => println!("{}", out.display()),
    }
    Ok(status)
}

/// CLI wrapper: real environment, `mens/runs` as the runs root, the real scorer.
pub(crate) fn run_eval_reasoning_compare(args: CompareArgs) -> Result<()> {
    let env_hub = (
        std::env::var_os("HF_HUB_CACHE"),
        std::env::var_os("HF_HOME"),
        dirs::home_dir(),
    );
    let overlay_root = std::env::current_dir()?;
    let max_tokens = reasoning_eval::parse_max_tokens(reasoning_eval::max_tokens_env().as_deref())?;
    let mut score = |_side: Side, dir: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
        score_side_with_backend(dir, slices, max_tokens)
    };
    run_eval_reasoning_compare_with(
        args,
        env_hub,
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
        }
    }

    fn run(a: CompareArgs, runs_root: &Path, score: SideScorer<'_>) -> Result<RunStatus> {
        run_eval_reasoning_compare_with(a, (None, None, None), runs_root, &repo_root(), score)
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
        std::fs::write(&out, r#"{"status":"compared"}"#).expect("seed stale");
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
        assert_eq!(v["status"], "compared");
        assert_eq!(v["gate_input"], false);
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
        let outs = score_slices_loaded(&slices, 64, &mut infer);
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
}
