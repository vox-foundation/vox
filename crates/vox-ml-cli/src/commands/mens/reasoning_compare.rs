//! `vox mens eval-reasoning-compare`.
//!
//! Writes only `--out`. Detection never loads a model. The report is not a serve-gate input.

mod detect;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vox_eval::reasoning_bench::{
    AbsenceReason, BaseLabel, BenchOutcome, ReasoningItem, SLICE_NAMES, SLICE_SIZE,
    compared_report_json, load_slice,
};

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

/// Collapse `.` and `..` without resolving symlinks or requiring the path to exist.
fn normalized_prefix(path: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(path)?;
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

/// Path used for the hub and adapter prefix check.
///
/// `std::path::absolute` leaves `..` and symlinks intact, and `starts_with`
/// compares raw components, so `other/../hub/file` and a symlinked parent both
/// miss a lexical prefix. Collapse `.` and `..`, then canonicalize the nearest
/// existing ancestor. That is the path `remove_file` and `write` will hit.
fn containment_path(path: &Path) -> Result<PathBuf> {
    let mut tail = PathBuf::new();
    let mut cursor = normalized_prefix(path)?;
    loop {
        match cursor.try_exists() {
            Ok(true) => {
                let mut resolved = cursor
                    .canonicalize()
                    .with_context(|| format!("canonicalizing {}", cursor.display()))?;
                if !tail.as_os_str().is_empty() {
                    resolved.push(&tail);
                }
                return normalized_prefix(&resolved);
            }
            Ok(false) => {
                let Some(name) = cursor.file_name().map(|name| name.to_os_string()) else {
                    return Ok(cursor);
                };
                tail = PathBuf::from(name).join(&tail);
                if !cursor.pop() {
                    return normalized_prefix(&tail);
                }
            }
            Err(err) => {
                anyhow::bail!("checking {}: {err}", cursor.display());
            }
        }
    }
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
    let out_abs = containment_path(out)?;
    let hub_abs = containment_path(hub_root)?;
    if out_abs.starts_with(&hub_abs) {
        anyhow::bail!(
            "refusing to write under the hub root {}",
            hub_root.display()
        );
    }
    if let Some(adapter) = adapter {
        let adapter_abs = containment_path(adapter)?;
        if out_abs.starts_with(&adapter_abs) {
            anyhow::bail!(
                "refusing to write under the adapter dir {}",
                adapter.display()
            );
        }
    }
    Ok(())
}

/// Named outcomes for one side. Callers match them back to slice names.
pub(crate) type SideScorer<'a> = &'a mut dyn FnMut(
    Side,
    &Path,
    &[(&'static str, Vec<ReasoningItem>)],
) -> Vec<(&'static str, BenchOutcome)>;

/// Score already-loaded slices. Writes nothing.
pub(crate) fn score_slices_loaded(
    slices: &[(&'static str, Vec<ReasoningItem>)],
    max_tokens: usize,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> Vec<(&'static str, BenchOutcome)> {
    slices
        .iter()
        .map(|(name, items)| {
            let (outcome, _rows) =
                super::reasoning_eval::score_loaded(name, items, max_tokens, infer);
            (*name, outcome)
        })
        .collect()
}

/// Load one model directory, score every passed slice, then unload.
fn score_side_with_backend(
    model_dir: &Path,
    slices: &[(&'static str, Vec<ReasoningItem>)],
    max_tokens: usize,
) -> Vec<(&'static str, BenchOutcome)> {
    let unscored = |reason: String| -> Vec<(&'static str, BenchOutcome)> {
        slices
            .iter()
            .map(|(name, _items)| {
                (
                    *name,
                    BenchOutcome::NotScored {
                        reason: reason.clone(),
                        n_requested: SLICE_SIZE,
                    },
                )
            })
            .collect()
    };
    let plugin_id = vox_populi::mens::select_mens_backend(
        vox_populi::mens::DeviceKind::Best,
        &vox_plugin_host::probe(),
    );
    let loaded = match vox_plugin_host::cached_code_plugin(plugin_id) {
        Ok(loaded) => loaded,
        Err(err) => return unscored(format!("{plugin_id} plugin not found: {err}")),
    };
    let Some(backend) = loaded.plugin.as_ml_backend().into_option() else {
        return unscored(format!("{plugin_id} plugin does not provide MlBackend"));
    };
    let handle = match backend
        .load_model(model_dir.to_string_lossy().as_ref().into())
        .into_result()
    {
        Ok(handle) => handle,
        Err(err) => {
            return unscored(format!("load_model({}): {err}", model_dir.display()));
        }
    };
    let mut infer = |prompt_json: &str| -> Result<String> {
        backend
            .run_inference(&handle, prompt_json.into())
            .into_result()
            .map(|resp| resp.as_str().to_string())
            .map_err(|err| anyhow::anyhow!("{err}"))
    };
    let done = score_slices_loaded(slices, max_tokens, &mut infer);
    let _ = backend.unload_model(&handle);
    // `loaded` is a cache reference and must outlive `handle`; it drops here,
    // with the other locals, which is after the last inference.
    let _plugin = loaded;
    done
}

/// Write the absence record, or score both sides and write the comparison.
pub(crate) fn run_before_after_with(
    detected: detect::Detected,
    bench_root: &Path,
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
            write_pretty(out, &record.to_json())?;
            Ok(RunStatus::NotCompared(reason))
        }
        detect::Detected::Ready {
            snapshot,
            adapter,
            base,
        } => {
            let mut loaded: Vec<(&'static str, Vec<ReasoningItem>)> = Vec::new();
            let mut failed: Vec<(&'static str, String)> = Vec::new();
            for name in SLICE_NAMES {
                let path = bench_root.join(name).join("reasoning.jsonl");
                match load_slice(&path, SLICE_SIZE) {
                    Ok(items) => loaded.push((name, items)),
                    Err(err) => failed.push((name, err)),
                }
            }
            let base_outcomes = score_side(Side::Base, &snapshot, &loaded);
            let adapter_outcomes = score_side(Side::Adapter, &adapter, &loaded);
            let mut rows: Vec<(&str, BenchOutcome, BenchOutcome)> = Vec::new();
            for name in SLICE_NAMES {
                if let Some((_, err)) = failed.iter().find(|(got, _)| *got == name) {
                    let outcome = BenchOutcome::NotScored {
                        reason: err.clone(),
                        n_requested: SLICE_SIZE,
                    };
                    rows.push((name, outcome.clone(), outcome));
                } else {
                    rows.push((
                        name,
                        named_outcome(&base_outcomes, name),
                        named_outcome(&adapter_outcomes, name),
                    ));
                }
            }
            let (report, compared) = compared_report_json(&base, &rows);
            write_pretty(out, &report)?;
            if compared {
                Ok(RunStatus::Compared)
            } else {
                Err(anyhow::anyhow!("{}", first_unscored(&rows)))
            }
        }
    }
}

fn named_outcome(outcomes: &[(&'static str, BenchOutcome)], name: &str) -> BenchOutcome {
    outcomes
        .iter()
        .find(|(got, _)| *got == name)
        .map(|(_, outcome)| outcome.clone())
        .unwrap_or_else(|| BenchOutcome::NotScored {
            reason: "scorer returned no outcome".to_string(),
            n_requested: SLICE_SIZE,
        })
}

fn first_unscored(rows: &[(&str, BenchOutcome, BenchOutcome)]) -> String {
    for (name, base_out, adapter_out) in rows {
        if matches!(base_out, BenchOutcome::NotScored { .. }) {
            return format!("{name} base not_scored");
        }
        if matches!(adapter_out, BenchOutcome::NotScored { .. }) {
            return format!("{name} adapter not_scored");
        }
    }
    "incomplete comparison".to_string()
}

fn write_pretty(out: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(value)?;
    std::fs::write(out, body).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}

/// Single entry every caller and every entry-level test uses.
pub(crate) fn run_eval_reasoning_compare_with(
    args: CompareArgs,
    env_hub: (Option<OsString>, Option<OsString>, Option<PathBuf>),
    runs_root: &Path,
    overlay_root: &Path,
    score_side: SideScorer<'_>,
) -> Result<RunStatus> {
    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(overlay_root)
        .map_err(|err| anyhow::anyhow!("load train_bases overlay: {err}"))?;
    let pin = detect::resolve_base_pin(args.base.as_deref(), &overlay)?;
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

/// CLI wrapper. Parses the token cap once; the cap is not stored in the report.
pub(crate) fn run_eval_reasoning_compare(args: CompareArgs) -> Result<()> {
    let max_tokens = super::reasoning_eval::parse_max_tokens(
        super::reasoning_eval::max_tokens_env().as_deref(),
    )?;
    let _status = run_eval_reasoning_compare_with(
        args,
        (
            std::env::var_os("HF_HUB_CACHE"),
            std::env::var_os("HF_HOME"),
            dirs::home_dir(),
        ),
        Path::new("mens/runs"),
        &std::env::current_dir()?,
        &mut |_side, model_dir, slices| score_side_with_backend(model_dir, slices, max_tokens),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_eval::qwen_reasoning::ReasoningScore;

    const PIN_8B: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn data_root() -> PathBuf {
        repo_root().join("mens/data")
    }

    struct Fixture {
        hub: tempfile::TempDir,
        adapter: tempfile::TempDir,
        snapshot: PathBuf,
    }

    fn present_fixture() -> Fixture {
        let hub = tempfile::tempdir().unwrap();
        let label = BaseLabel::from_pin(PIN_8B);
        let snapshot = hub
            .path()
            .join("models--Qwen--Qwen3-8B")
            .join("snapshots")
            .join(label.revision.expect("pin"));
        std::fs::create_dir_all(&snapshot).unwrap();
        for name in ["config.json", "tokenizer.json", "model.safetensors"] {
            std::fs::write(snapshot.join(name), b"").unwrap();
        }
        let adapter = tempfile::tempdir().unwrap();
        std::fs::write(adapter.path().join("candle_qlora_adapter.safetensors"), b"").unwrap();
        std::fs::write(adapter.path().join("tokenizer.json"), b"").unwrap();
        std::fs::write(adapter.path().join("config.json"), b"").unwrap();
        let manifest = serde_json::json!({ "base_model": snapshot.display().to_string() });
        std::fs::write(
            adapter.path().join("adapter_manifest.json"),
            manifest.to_string(),
        )
        .unwrap();
        Fixture {
            hub,
            adapter,
            snapshot,
        }
    }

    fn run_at(
        hub: &Path,
        runs: &Path,
        adapter: Option<PathBuf>,
        out: Option<PathBuf>,
        bench_root: &Path,
        score: &mut dyn FnMut(
            Side,
            &Path,
            &[(&'static str, Vec<ReasoningItem>)],
        ) -> Vec<(&'static str, BenchOutcome)>,
    ) -> Result<RunStatus> {
        run_eval_reasoning_compare_with(
            CompareArgs {
                base: None,
                adapter,
                hub_root: Some(hub.to_path_buf()),
                bench_root: bench_root.to_path_buf(),
                out,
            },
            (None, None, None),
            runs,
            &repo_root(),
            score,
        )
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
        let data = data_root();
        run_at(hub, runs, adapter, out, &data, score)
    }

    fn full_score(n: usize, accuracy: f64, think: f64) -> BenchOutcome {
        BenchOutcome::Scored(ReasoningScore {
            accuracy,
            think_initiation_rate: think,
            n,
        })
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
    fn out_via_parent_dir_into_hub_or_adapter_is_refused() {
        let parent = tempfile::tempdir().unwrap();
        let hub = parent.path().join("hub");
        let adapter = parent.path().join("adapter");
        let decoy = parent.path().join("other");
        std::fs::create_dir(&hub).unwrap();
        std::fs::create_dir(&adapter).unwrap();
        std::fs::create_dir(&decoy).unwrap();
        let runs = tempfile::tempdir().unwrap();

        let in_hub = decoy.join("..").join("hub").join("report.json");
        let err = run(
            &hub,
            runs.path(),
            Some(adapter.clone()),
            Some(in_hub.clone()),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect_err("dotdot hub");
        assert!(!hub.join("report.json").exists(), "{err}");

        let in_adapter = decoy.join("..").join("adapter").join("report.json");
        let err = run(
            &hub,
            runs.path(),
            Some(adapter.clone()),
            Some(in_adapter.clone()),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect_err("dotdot adapter");
        assert!(!adapter.join("report.json").exists(), "{err}");

        let escaped = hub.join("..").join("outside.json");
        run(
            &hub,
            runs.path(),
            Some(adapter),
            Some(escaped),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect("a .. that leaves the hub is outside it");
        assert!(parent.path().join("outside.json").is_file());
        assert!(!hub.join("outside.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn out_through_symlink_into_hub_or_adapter_is_refused() {
        let hub = tempfile::tempdir().unwrap();
        let adapter = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let links = tempfile::tempdir().unwrap();
        let hub_link = links.path().join("hub");
        std::os::unix::fs::symlink(hub.path(), &hub_link).unwrap();
        let via_hub = hub_link.join("report.json");
        let err = run(
            hub.path(),
            runs.path(),
            Some(adapter.path().to_path_buf()),
            Some(via_hub),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect_err("symlinked hub");
        assert!(!hub.path().join("report.json").exists(), "{err}");

        let adapter_link = links.path().join("adapter");
        std::os::unix::fs::symlink(adapter.path(), &adapter_link).unwrap();
        let via_adapter = adapter_link.join("report.json");
        let err = run(
            hub.path(),
            runs.path(),
            Some(adapter.path().to_path_buf()),
            Some(via_adapter),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect_err("symlinked adapter");
        assert!(!adapter.path().join("report.json").exists(), "{err}");
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

    #[test]
    fn both_sides_get_identical_checked_in_items() {
        let fixture = present_fixture();
        let runs = tempfile::tempdir().unwrap();
        let mut seen: Vec<(Side, PathBuf, Vec<(&'static str, Vec<ReasoningItem>)>)> = Vec::new();
        let status = run(
            fixture.hub.path(),
            runs.path(),
            Some(fixture.adapter.path().to_path_buf()),
            Some(runs.path().join("out.json")),
            &mut |side, path, slices| {
                seen.push((
                    side,
                    path.to_path_buf(),
                    slices
                        .iter()
                        .map(|(name, items)| (*name, items.clone()))
                        .collect(),
                ));
                slices
                    .iter()
                    .map(|(name, items)| (*name, full_score(items.len(), 1.0, 1.0)))
                    .collect()
            },
        )
        .expect("present");
        assert!(matches!(status, RunStatus::Compared));
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].0, Side::Base);
        assert_eq!(seen[0].1, fixture.snapshot);
        assert_eq!(seen[1].0, Side::Adapter);
        assert_eq!(seen[1].1, fixture.adapter.path());
        assert_eq!(seen[0].2, seen[1].2);
        assert_eq!(
            seen[0].2.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            SLICE_NAMES
        );
        for (name, items) in &seen[0].2 {
            let expected =
                load_slice(&data_root().join(name).join("reasoning.jsonl"), SLICE_SIZE).unwrap();
            assert_eq!(items, &expected, "{name}");
        }
    }

    #[test]
    fn present_branch_writes_all_six_entries() {
        let fixture = present_fixture();
        let runs = tempfile::tempdir().unwrap();
        let out = runs.path().join("out.json");
        let status = run(
            fixture.hub.path(),
            runs.path(),
            Some(fixture.adapter.path().to_path_buf()),
            Some(out.clone()),
            &mut |_side, _path, slices| {
                slices
                    .iter()
                    .map(|(name, items)| (*name, full_score(items.len(), 0.5, 0.25)))
                    .collect()
            },
        )
        .expect("compared");
        assert!(matches!(status, RunStatus::Compared));
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(body["status"], "compared");
        for name in SLICE_NAMES {
            for side in ["base", "adapter"] {
                assert!(body["slices"][name][side].get("accuracy").is_some());
                assert!(
                    body["slices"][name][side]
                        .get("think_initiation_rate")
                        .is_some()
                );
            }
        }
    }

    #[test]
    fn one_not_scored_side_returns_err_and_writes_incomplete() {
        let fixture = present_fixture();
        let runs = tempfile::tempdir().unwrap();
        let out = runs.path().join("out.json");
        let err = run(
            fixture.hub.path(),
            runs.path(),
            Some(fixture.adapter.path().to_path_buf()),
            Some(out.clone()),
            &mut |side, _path, slices| {
                slices
                    .iter()
                    .map(|(name, items)| {
                        let outcome = if side == Side::Adapter && *name == "math500_think" {
                            BenchOutcome::NotScored {
                                reason: "adapter failed".to_string(),
                                n_requested: SLICE_SIZE,
                            }
                        } else {
                            full_score(items.len(), 1.0, 1.0)
                        };
                        (*name, outcome)
                    })
                    .collect()
            },
        )
        .expect_err("one unscored side");
        assert!(err.to_string().contains("math500_think"));
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(body["status"], "incomplete");
        let entry = &body["slices"]["math500_think"]["adapter"];
        assert_eq!(entry["status"], "not_scored");
        assert!(entry.get("accuracy").is_none());
        assert!(entry.get("think_initiation_rate").is_none());
    }

    #[test]
    fn slice_load_failure_is_not_scored_on_both_sides() {
        let fixture = present_fixture();
        let bench = tempfile::tempdir().unwrap();
        for name in ["gsm8k_think", "vox_think"] {
            let src = data_root().join(name).join("reasoning.jsonl");
            let dst = bench.path().join(name);
            std::fs::create_dir_all(&dst).unwrap();
            std::fs::copy(src, dst.join("reasoning.jsonl")).unwrap();
        }
        let runs = tempfile::tempdir().unwrap();
        let out = runs.path().join("out.json");
        let mut base_names: Vec<&str> = Vec::new();
        let err = run_at(
            fixture.hub.path(),
            runs.path(),
            Some(fixture.adapter.path().to_path_buf()),
            Some(out.clone()),
            bench.path(),
            &mut |side, _path, slices| {
                if side == Side::Base {
                    base_names = slices.iter().map(|(name, _)| *name).collect();
                }
                slices
                    .iter()
                    .map(|(name, items)| {
                        let accuracy = if *name == "gsm8k_think" { 0.5 } else { 0.25 };
                        (*name, full_score(items.len(), accuracy, 1.0))
                    })
                    .collect()
            },
        )
        .expect_err("missing slice");
        assert!(err.to_string().contains("math500_think"));
        assert_eq!(base_names, ["gsm8k_think", "vox_think"]);
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(body["status"], "incomplete");
        for side in ["base", "adapter"] {
            let missing = &body["slices"]["math500_think"][side];
            assert_eq!(missing["status"], "not_scored");
            assert!(missing.get("accuracy").is_none());
            assert_eq!(body["slices"]["gsm8k_think"][side]["accuracy"], 0.5);
            assert_eq!(body["slices"]["vox_think"][side]["accuracy"], 0.25);
        }
    }

    #[test]
    fn scorer_outcomes_are_matched_by_name() {
        let fixture = present_fixture();
        let runs = tempfile::tempdir().unwrap();
        let reversed = runs.path().join("reversed.json");
        run(
            fixture.hub.path(),
            runs.path(),
            Some(fixture.adapter.path().to_path_buf()),
            Some(reversed.clone()),
            &mut |_side, _path, slices| {
                let mut outcomes: Vec<_> = slices
                    .iter()
                    .map(|(name, items)| {
                        let accuracy = match *name {
                            "gsm8k_think" => 0.25,
                            "math500_think" => 0.5,
                            "vox_think" => 0.75,
                            _ => 0.0,
                        };
                        (*name, full_score(items.len(), accuracy, 1.0))
                    })
                    .collect();
                outcomes.reverse();
                outcomes
            },
        )
        .expect("reversed still compared");
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&reversed).unwrap()).unwrap();
        assert_eq!(body["slices"]["gsm8k_think"]["base"]["accuracy"], 0.25);
        assert_eq!(body["slices"]["math500_think"]["adapter"]["accuracy"], 0.5);
        assert_eq!(body["slices"]["vox_think"]["base"]["accuracy"], 0.75);

        let omitted = runs.path().join("omitted.json");
        let err = run(
            fixture.hub.path(),
            runs.path(),
            Some(fixture.adapter.path().to_path_buf()),
            Some(omitted.clone()),
            &mut |_side, _path, slices| {
                slices
                    .iter()
                    .filter(|(name, _)| *name != "math500_think")
                    .map(|(name, items)| (*name, full_score(items.len(), 1.0, 1.0)))
                    .collect()
            },
        )
        .expect_err("omitted name");
        assert!(err.to_string().contains("math500_think"));
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&omitted).unwrap()).unwrap();
        let entry = &body["slices"]["math500_think"]["base"];
        assert_eq!(entry["status"], "not_scored");
        assert_eq!(entry["reason"], "scorer returned no outcome");
        assert_eq!(body["slices"]["gsm8k_think"]["base"]["accuracy"], 1.0);
    }

    #[test]
    fn no_gate_files_are_created() {
        let fixture = present_fixture();
        let runs = tempfile::tempdir().unwrap();
        let out = runs.path().join("present.json");
        run(
            fixture.hub.path(),
            runs.path(),
            Some(fixture.adapter.path().to_path_buf()),
            Some(out.clone()),
            &mut |_side, _path, slices| {
                slices
                    .iter()
                    .map(|(name, items)| (*name, full_score(items.len(), 1.0, 1.0)))
                    .collect()
            },
        )
        .expect("present");
        let empty = tempfile::tempdir().unwrap();
        let absent = runs.path().join("absent.json");
        run(
            empty.path(),
            runs.path(),
            None,
            Some(absent.clone()),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect("absent");
        for dir in [
            fixture.snapshot.as_path(),
            fixture.adapter.path(),
            runs.path(),
        ] {
            assert!(!contains_gate(dir), "{}", dir.display());
        }
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
        let slices = [("gsm8k_think", items)];
        let mut infer = |prompt_json: &str| -> Result<String> {
            let value: serde_json::Value = serde_json::from_str(prompt_json)?;
            let prompt = value["prompt"].as_str().unwrap_or("");
            let after = prompt.split("what is ").nth(1).unwrap_or("");
            let gold = after.split('?').next().unwrap_or("");
            Ok(serde_json::json!({
                "generated_text": format!("<think>\nx\n</think>\n\n\\boxed{{{gold}}}")
            })
            .to_string())
        };
        let scratch = tempfile::tempdir().unwrap();
        let before = std::fs::read_dir(scratch.path()).unwrap().count();
        let outcomes = score_slices_loaded(&slices, 2048, &mut infer);
        let after = std::fs::read_dir(scratch.path()).unwrap().count();
        assert_eq!(before, after);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].0, "gsm8k_think");
        match &outcomes[0].1 {
            BenchOutcome::Scored(score) => {
                assert_eq!(score.n, 32);
                assert_eq!(score.accuracy, 1.0);
                assert_eq!(score.think_initiation_rate, 1.0);
            }
            other => panic!("expected Scored, got {other:?}"),
        }
    }

    fn dir_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|ent| ent.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn contains_gate(dir: &Path) -> bool {
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name == "collateral_damage_report.json"
                    || name == "reasoning_report.json"
                    || name == "collateral_eval"
                {
                    return true;
                }
                if entry.path().is_dir() {
                    stack.push(entry.path());
                }
            }
        }
        false
    }

    #[test]
    fn cli_entry_records_the_resolved_pin() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let status = run_eval_reasoning_compare_with(
            CompareArgs {
                base: Some("Qwen3-0.6B".to_string()),
                adapter: None,
                hub_root: Some(hub.path().to_path_buf()),
                bench_root: data_root(),
                out: None,
            },
            (None, None, None),
            runs.path(),
            &repo_root(),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect("resolved pin");
        assert!(matches!(status, RunStatus::NotCompared(_)));
        let path = runs
            .path()
            .join(REASONING_COMPARE_SUBDIR)
            .join("Qwen3-0.6B-c1899de289a0.json");
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(body["base_model"], "Qwen/Qwen3-0.6B");
        assert_eq!(
            body["base_revision"],
            "c1899de289a04d12100db370d81485cdf75e47ca"
        );
    }

    #[test]
    fn size_name_1_7b_records_the_pinned_unmeasured_base() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let status = run_eval_reasoning_compare_with(
            CompareArgs {
                base: Some("1.7B".to_string()),
                adapter: None,
                hub_root: Some(hub.path().to_path_buf()),
                bench_root: data_root(),
                out: None,
            },
            (None, None, None),
            runs.path(),
            &repo_root(),
            &mut |_, _, _| panic!("scorer must not run"),
        )
        .expect("resolved pin");
        assert!(matches!(status, RunStatus::NotCompared(_)));
        let path = runs
            .path()
            .join(REASONING_COMPARE_SUBDIR)
            .join("Qwen3-1.7B-70d244cc86cc.json");
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(body["base_model"], "Qwen/Qwen3-1.7B");
        assert_eq!(
            body["base_revision"],
            "70d244cc86ccca08cf5af4e1e306ecf908b1ad5e"
        );
    }
}
