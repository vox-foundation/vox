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
use vox_eval::reasoning_bench::{AbsenceReason, BaseLabel, BenchOutcome, ReasoningItem};

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
        Detected::Ready { .. } => {
            let _ = (bench_root, score_side);
            anyhow::bail!("both inputs present but scoring is not wired in this build")
        }
    }
}

/// The single entry: resolve the pin and hub root, guard `--out`, detect, write.
pub(crate) fn run_eval_reasoning_compare_with(
    args: CompareArgs,
    env_hub: (Option<OsString>, Option<OsString>, Option<PathBuf>),
    runs_root: &Path,
    overlay_root: &Path,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    let _ = overlay_root;
    let pin = args
        .base
        .clone()
        .unwrap_or_else(|| vox_populi::mens::DEFAULT_MODEL_ID.to_string());
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
    let mut score = |_side: Side, _dir: &Path, slices: &[(&'static str, Vec<ReasoningItem>)]| {
        slices
            .iter()
            .map(|(name, _)| {
                (
                    *name,
                    BenchOutcome::NotScored {
                        reason: "scorer not wired".to_string(),
                        n_requested: vox_eval::reasoning_bench::SLICE_SIZE,
                    },
                )
            })
            .collect()
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
}
