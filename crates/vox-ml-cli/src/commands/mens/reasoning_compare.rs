//! `vox mens eval-reasoning-compare`.
//!
//! Writes only `--out`. Detection never loads a model. The report is not a serve-gate input.

mod detect;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::Result;
use vox_eval::reasoning_bench::{AbsenceReason, BaseLabel, BenchOutcome, ReasoningItem};

/// Directory under `mens/runs/` that holds one report per base.
pub(crate) const REASONING_COMPARE_SUBDIR: &str = "reasoning_compare";

/// Which weights a side scorer is about to score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    /// The pinned base snapshot.
    Base,
    /// The adapter directory.
    Adapter,
}

/// Result of one comparison attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunStatus {
    /// Every slice was scored on both sides.
    Compared,
    /// An absence record was written and nothing was scored.
    NotCompared(AbsenceReason),
}

/// Parsed `eval-reasoning-compare` arguments.
pub(crate) struct CompareArgs {
    /// Size name or `hf_id@sha`. `None` is the pinned default base.
    pub base: Option<String>,
    /// Adapter run directory. `None` is recorded as absent.
    pub adapter: Option<PathBuf>,
    /// Hugging Face hub cache root. `None` resolves from the environment.
    pub hub_root: Option<PathBuf>,
    /// Root that holds `<slice>/reasoning.jsonl`.
    pub bench_root: PathBuf,
    /// Report path. `None` is `mens/runs/reasoning_compare/<size>-<rev12>.json`.
    pub out: Option<PathBuf>,
}

/// Refuse paths `vox mens serve` would treat as a gate input, plus hub and adapter dirs.
pub(crate) fn check_out_path(out: &Path, hub_root: &Path, adapter: Option<&Path>) -> Result<()> {
    let name = out.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if name == "collateral_damage_report.json" || name == "reasoning_report.json" {
        anyhow::bail!("refusing to write serve-gate file {name}");
    }
    if out.components().any(|c| c.as_os_str() == "collateral_eval") {
        anyhow::bail!("refusing to write under collateral_eval");
    }
    let out_abs = std::path::absolute(out)?;
    let hub_abs = std::path::absolute(hub_root)?;
    if out_abs.starts_with(&hub_abs) {
        anyhow::bail!(
            "refusing to write under the hub root {}",
            hub_root.display()
        );
    }
    if let Some(adapter) = adapter {
        let adapter_abs = std::path::absolute(adapter)?;
        if out_abs.starts_with(&adapter_abs) {
            anyhow::bail!(
                "refusing to write under the adapter dir {}",
                adapter.display()
            );
        }
    }
    Ok(())
}

/// Named outcomes for one side. Later tasks match them back to slice names.
pub(crate) type SideScorer<'a> = &'a mut dyn FnMut(
    Side,
    &Path,
    &[(&'static str, Vec<ReasoningItem>)],
) -> Vec<(&'static str, BenchOutcome)>;

/// Write the absence record, or fail closed when both inputs are present.
pub(crate) fn run_before_after_with(
    detected: detect::Detected,
    _bench_root: &Path,
    out: &Path,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    match std::fs::remove_file(out) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(anyhow::anyhow!(
                "could not remove stale report {}: {e}",
                out.display()
            ));
        }
    }
    match detected {
        detect::Detected::NotCompared(record) => {
            let reason = record.reason;
            if let Some(parent) = out.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(out, serde_json::to_string_pretty(&record.to_json())?)?;
            Ok(RunStatus::NotCompared(reason))
        }
        detect::Detected::Ready { .. } => {
            let _ = score_side;
            Err(anyhow::anyhow!(
                "both inputs present but scoring is not wired in this build"
            ))
        }
    }
}

/// Single entry every caller and every entry-level test uses.
pub(crate) fn run_eval_reasoning_compare_with(
    args: CompareArgs,
    env_hub: (Option<OsString>, Option<OsString>, Option<PathBuf>),
    runs_root: &Path,
    overlay_root: &Path,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    // Size-name resolution through train_bases is wired in a later task.
    let _ = overlay_root;
    let pin = args
        .base
        .clone()
        .unwrap_or_else(|| vox_populi::mens::DEFAULT_MODEL_ID.to_string());
    let label = BaseLabel::from_pin(&pin);
    let hub_root =
        detect::resolve_hub_root(args.hub_root.as_deref(), env_hub.0, env_hub.1, env_hub.2)
            .ok_or_else(|| anyhow::anyhow!("no hub root resolvable; pass --hub-root"))?;
    let out = args.out.clone().unwrap_or_else(|| {
        runs_root
            .join(REASONING_COMPARE_SUBDIR)
            .join(label.report_file_name())
    });
    check_out_path(&out, &hub_root, args.adapter.as_deref())?;
    let detected = detect::detect_inputs(&hub_root, &label, args.adapter.as_deref());
    let status = run_before_after_with(detected, &args.bench_root, &out, score_side)?;
    match &status {
        RunStatus::NotCompared(reason) => {
            eprintln!("NOT COMPARED: {} ({})", reason.as_str(), out.display());
        }
        RunStatus::Compared => {
            println!("{}", out.display());
        }
    }
    Ok(status)
}

/// CLI wrapper. The real scorer is unreachable on the absent branch in this build.
pub(crate) fn run_eval_reasoning_compare(args: CompareArgs) -> Result<()> {
    let _status = run_eval_reasoning_compare_with(
        args,
        (
            std::env::var_os("HF_HUB_CACHE"),
            std::env::var_os("HF_HOME"),
            dirs::home_dir(),
        ),
        Path::new("mens/runs"),
        &std::env::current_dir()?,
        &mut |_side, _model_dir, slices| {
            slices
                .iter()
                .map(|(name, _items)| {
                    (
                        *name,
                        BenchOutcome::NotScored {
                            reason: "scorer not wired".to_string(),
                            n_requested: vox_eval::reasoning_bench::SLICE_SIZE,
                        },
                    )
                })
                .collect()
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn args(hub: &Path, adapter: Option<PathBuf>, out: Option<PathBuf>) -> CompareArgs {
        CompareArgs {
            base: None,
            adapter,
            hub_root: Some(hub.to_path_buf()),
            bench_root: repo_root().join("mens/data"),
            out,
        }
    }

    fn run(
        hub: &Path,
        runs: &Path,
        adapter: Option<PathBuf>,
        out: Option<PathBuf>,
        score: &mut dyn FnMut(
            Side,
            &Path,
            &[(&'static str, Vec<ReasoningItem>)],
        ) -> Vec<(&'static str, BenchOutcome)>,
    ) -> Result<RunStatus> {
        run_eval_reasoning_compare_with(
            args(hub, adapter, out),
            (None, None, None),
            runs,
            &repo_root(),
            score,
        )
    }

    #[test]
    fn absent_branch_never_calls_the_scorer() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let status = run(
            hub.path(),
            runs.path(),
            None,
            None,
            &mut |_side, _path, _slices| panic!("scorer must not run"),
        )
        .expect("absent branch returns Ok");
        assert!(matches!(
            status,
            RunStatus::NotCompared(AbsenceReason::BaseSnapshotAbsent)
        ));
        let path = runs
            .path()
            .join(REASONING_COMPARE_SUBDIR)
            .join("Qwen3-8B-b968826d9c46.json");
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(body["schema"], "reasoning_before_after.v1");
        assert_eq!(body["status"], "not_compared");
        assert_eq!(body["reason"], "base_snapshot_absent");
        assert_eq!(body["base_model"], "Qwen/Qwen3-8B");
        assert!(body.get("accuracy").is_none());
        assert!(body.get("think_initiation_rate").is_none());
    }

    #[test]
    fn out_named_like_a_gate_file_is_refused() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        for name in ["collateral_damage_report.json", "reasoning_report.json"] {
            let out = scratch.path().join(name);
            let err = run(
                hub.path(),
                runs.path(),
                None,
                Some(out.clone()),
                &mut |_, _, _| panic!("scorer must not run"),
            )
            .expect_err(name);
            assert!(!out.exists(), "{name} must not be created: {err}");
        }
        let nested = scratch.path().join("collateral_eval").join("report.json");
        let err = run(
            hub.path(),
            runs.path(),
            None,
            Some(nested.clone()),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect_err("collateral_eval");
        assert!(!nested.exists(), "{err}");
    }

    #[test]
    fn out_inside_hub_root_or_adapter_is_refused() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let adapter = tempfile::tempdir().unwrap();
        let in_hub = hub.path().join("report.json");
        let err = run(
            hub.path(),
            runs.path(),
            Some(adapter.path().to_path_buf()),
            Some(in_hub.clone()),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect_err("hub");
        assert!(!in_hub.exists(), "{err}");
        let in_adapter = adapter.path().join("report.json");
        let err = run(
            hub.path(),
            runs.path(),
            Some(adapter.path().to_path_buf()),
            Some(in_adapter.clone()),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect_err("adapter");
        assert!(!in_adapter.exists(), "{err}");
    }

    #[test]
    fn stale_compared_report_is_replaced() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let out = runs.path().join("stale.json");
        std::fs::write(&out, r#"{"status":"compared"}"#).unwrap();
        run(
            hub.path(),
            runs.path(),
            None,
            Some(out.clone()),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect("replaces the stale file");
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(body["status"], "not_compared");
        assert_eq!(body["schema"], "reasoning_before_after.v1");
    }

    #[test]
    fn default_out_is_outside_a_runs_v1_adapter() {
        let root = tempfile::tempdir().unwrap();
        let runs = root.path().join("mens/runs");
        let adapter = runs.join("v1");
        std::fs::create_dir_all(&adapter).unwrap();
        std::fs::write(adapter.join("candle_qlora_adapter.safetensors"), b"").unwrap();
        std::fs::write(adapter.join("adapter_manifest.json"), b"{}").unwrap();
        let before = dir_names(&adapter);
        let hub = tempfile::tempdir().unwrap();
        run(
            hub.path(),
            &runs,
            Some(adapter.clone()),
            None,
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect("absence record");
        let written = runs
            .join(REASONING_COMPARE_SUBDIR)
            .join("Qwen3-8B-b968826d9c46.json");
        assert!(written.is_file(), "{}", written.display());
        assert_eq!(dir_names(&adapter), before);
    }

    fn dir_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|ent| ent.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}
