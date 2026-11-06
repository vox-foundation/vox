//! Phase 321 (`HARN-01`, `BENCH-04`): Chosen-size serve target resolution behind the
//! collateral gate, and multi-size before/after reasoning + intelligence report writer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vox_eval::reasoning_bench::{
    BaseLabel, BenchOutcome, ReasoningItem, SLICE_NAMES, SLICE_SIZE, load_slice,
};
use vox_populi::mens::tensor::spoke_base_resolver::{
    TrainBase, load_overlay, read_accelerator_free_mb,
};

use super::populi::{assert_collateral_gate, assert_serve_preconditions};
use super::reasoning_compare::{Side, SideScorer, check_out_path, detect};
use super::size_train::SIZE_TRAIN_SUBDIR;

/// Schema version for the multi-size harness before/after report (`BENCH-04`).
pub(crate) const HARNESS_BEFORE_AFTER_SCHEMA: &str = "harness_before_after.v1";

/// Default filename under `mens/runs/` for the multi-size before/after report.
pub(crate) const HARNESS_BEFORE_AFTER_FILE: &str = "harness_before_after.json";

const QWEN3_SIZE_ORDER: &[&str] = &["0.6B", "1.7B", "4B", "8B", "14B", "32B"];

/// Intelligence score on a codegen or agent benchmark (`BENCH-04`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IntelligenceScore {
    /// Benchmark name (e.g., `"heldout_bench"`).
    pub benchmark: String,
    /// Unbiased `pass_rate_at_1` in `[0.0, 1.0]`.
    pub pass_rate_at_1: f64,
    /// Number of benchmark items evaluated (`n >= 1`).
    pub n: usize,
}

/// Scorer for the intelligence benchmark on a base snapshot or adapter directory.
pub(crate) type IntelligenceScorer<'a> =
    &'a mut dyn FnMut(Side, &Path, &str) -> Result<IntelligenceScore>;

/// Resolved serve target for a chosen Qwen3 size (`HARN-01`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedServeTarget {
    /// Pinned `hf_id@sha` from the `qwen3_code` overlay.
    pub hf_id: String,
    /// Short Qwen3 size token (e.g., `"0.6B"`, `"8B"`).
    pub size_token: String,
    /// Which weights `model_path` points to (`Side::Base` or `Side::Adapter`).
    pub side: Side,
    /// Verified directory to pass to `ServeConfig.model_path`.
    pub model_path: PathBuf,
}

/// CLI arguments for `vox mens harness-before-after`.
pub(crate) struct HarnessBeforeAfterArgs {
    /// Hugging Face hub cache root.
    pub hub_root: Option<PathBuf>,
    /// Root holding `<slice>/reasoning.jsonl`.
    pub bench_root: PathBuf,
    /// Output runs root (default: `mens/runs`).
    pub runs_root: Option<PathBuf>,
    /// Explicit report output path (default: `<runs_root>/harness_before_after.json`).
    pub out: Option<PathBuf>,
}

fn size_token_for_label(label: &BaseLabel) -> &str {
    label
        .repo_id
        .strip_prefix("Qwen/Qwen3-")
        .unwrap_or(&label.repo_id)
}

fn qwen3_size_rank(size_token: &str) -> usize {
    QWEN3_SIZE_ORDER
        .iter()
        .position(|t| *t == size_token)
        .unwrap_or(usize::MAX)
}

fn resolve_manifest_path(adapter_or_manifest: &Path) -> PathBuf {
    if adapter_or_manifest
        .file_name()
        .is_some_and(|name| name == "adapter_manifest.json")
    {
        adapter_or_manifest.to_path_buf()
    } else {
        adapter_or_manifest.join("adapter_manifest.json")
    }
}

fn resolve_adapter_dir(adapter_or_manifest: &Path) -> PathBuf {
    if adapter_or_manifest
        .file_name()
        .is_some_and(|name| name == "adapter_manifest.json")
    {
        adapter_or_manifest
            .parent()
            .unwrap_or(adapter_or_manifest)
            .to_path_buf()
    } else {
        adapter_or_manifest.to_path_buf()
    }
}

/// Resolve a chosen Qwen3 size (or pinned `hf_id@sha`) to a serveable base snapshot
/// or adapter directory (`HARN-01`).
///
/// - Base serve (`adapter == None`): requires a complete pinned snapshot on disk.
/// - Adapter serve (`adapter == Some(dir)`): requires a complete pinned snapshot on disk,
///   an `adapter_manifest.json` whose `base_model` canonicalizes to that pinned snapshot,
///   and a passing `assert_collateral_gate` + `assert_serve_preconditions` check.
pub(crate) fn resolve_harness_serve_target(
    requested_size: &str,
    overlay: &HashMap<String, Vec<TrainBase>>,
    hub_root: &Path,
    adapter: Option<&Path>,
) -> Result<ResolvedServeTarget> {
    let pin = detect::resolve_base_pin(Some(requested_size), overlay)?;
    let label = BaseLabel::from_pin(&pin);
    let snapshot = detect::snapshot_dir(hub_root, &label).ok_or_else(|| {
        anyhow::anyhow!("pin '{pin}' has no 40-hex revision; cannot resolve snapshot")
    })?;
    if !snapshot.is_dir() || !detect::snapshot_complete(&snapshot) {
        anyhow::bail!(
            "pinned base snapshot for '{pin}' is absent or incomplete at {}",
            snapshot.display()
        );
    }
    let size_token = size_token_for_label(&label).to_string();
    match adapter {
        None => Ok(ResolvedServeTarget {
            hf_id: pin,
            size_token,
            side: Side::Base,
            model_path: snapshot,
        }),
        Some(adapter_input) => {
            let adapter_dir = resolve_adapter_dir(adapter_input);
            let manifest_path = resolve_manifest_path(&adapter_dir);
            if !detect::adapter_manifest_names_snapshot(&manifest_path, &snapshot) {
                anyhow::bail!(
                    "adapter manifest {} does not name pinned snapshot {}",
                    manifest_path.display(),
                    snapshot.display()
                );
            }
            assert_collateral_gate(&adapter_dir)?;
            assert_serve_preconditions(&adapter_dir)?;
            Ok(ResolvedServeTarget {
                hf_id: pin,
                size_token,
                side: Side::Adapter,
                model_path: adapter_dir,
            })
        }
    }
}

fn ordered_qlora_rows(overlay: &HashMap<String, Vec<TrainBase>>) -> Result<Vec<&TrainBase>> {
    let rows = overlay
        .get("qwen3_code")
        .ok_or_else(|| anyhow::anyhow!("overlay missing qwen3_code spoke"))?;
    let mut measured: Vec<&TrainBase> = Vec::new();
    let mut unmeasured: Vec<&TrainBase> = Vec::new();
    for row in rows {
        if !row.methods.iter().any(|m| m == "qlora") {
            continue;
        }
        if row.floor_measured {
            measured.push(row);
        } else {
            unmeasured.push(row);
        }
    }
    measured.sort_by_key(|row| {
        let label = BaseLabel::from_pin(&row.hf_id);
        (
            row.floor_mb.unwrap_or(u32::MAX),
            qwen3_size_rank(size_token_for_label(&label)),
        )
    });
    unmeasured.sort_by_key(|row| {
        let label = BaseLabel::from_pin(&row.hf_id);
        qwen3_size_rank(size_token_for_label(&label))
    });
    Ok(measured.into_iter().chain(unmeasured).collect())
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

/// Write the consolidated `harness_before_after.v1` report across all `qwen3_code` QLoRA sizes (`BENCH-04`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_harness_before_after_report(
    overlay: &HashMap<String, Vec<TrainBase>>,
    free_mb: u32,
    hub_root: &Path,
    bench_root: &Path,
    runs_root: &Path,
    out_override: Option<&Path>,
    adapters: &HashMap<String, PathBuf>,
    score_reasoning: SideScorer<'_>,
    score_intelligence: IntelligenceScorer<'_>,
) -> Result<PathBuf> {
    let out = out_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| runs_root.join(HARNESS_BEFORE_AFTER_FILE));
    check_out_path(&out, hub_root, None)?;

    let rows = ordered_qlora_rows(overlay)?;
    let mut loaded_slices: Vec<(&'static str, Vec<ReasoningItem>)> = Vec::new();
    for name in SLICE_NAMES {
        let slice_path = bench_root.join(name).join("reasoning.jsonl");
        let items = load_slice(&slice_path, SLICE_SIZE)
            .map_err(|err| anyhow::anyhow!("loading slice {name}: {err}"))?;
        loaded_slices.push((name, items));
    }

    let mut size_entries: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    let mut trained_scored_count = 0usize;

    for row in rows {
        let label = BaseLabel::from_pin(&row.hf_id);
        let size_token = size_token_for_label(&label);

        if !row.floor_measured {
            size_entries.push(serde_json::json!({
                "size_token": size_token,
                "hf_id": row.hf_id,
                "status": "not_trained",
                "reason": "floor_unmeasured",
                "floor_measured": false,
                "free_mb": free_mb,
            }));
            continue;
        }

        let floor_mb = row
            .floor_mb
            .ok_or_else(|| anyhow::anyhow!("measured row {} missing floor_mb", row.hf_id))?;

        if floor_mb > free_mb {
            size_entries.push(serde_json::json!({
                "size_token": size_token,
                "hf_id": row.hf_id,
                "status": "not_trained",
                "reason": "does_not_fit",
                "floor_mb": floor_mb,
                "free_mb": free_mb,
            }));
            continue;
        }

        let ready_snapshot = detect::snapshot_dir(hub_root, &label)
            .filter(|dir| dir.is_dir() && detect::snapshot_complete(dir));
        let Some(snapshot) = ready_snapshot else {
            size_entries.push(serde_json::json!({
                "size_token": size_token,
                "hf_id": row.hf_id,
                "status": "not_trained",
                "reason": "base_snapshot_absent",
                "floor_mb": floor_mb,
                "free_mb": free_mb,
            }));
            continue;
        };

        // Resolve adapter from explicit map or prior size_train record on disk
        let candidate_adapter = adapters.get(&row.hf_id).cloned().or_else(|| {
            let record_path = runs_root
                .join(SIZE_TRAIN_SUBDIR)
                .join(label.report_file_name());
            let text = std::fs::read_to_string(record_path).ok()?;
            let val: serde_json::Value = serde_json::from_str(&text).ok()?;
            if val.get("status").and_then(|s| s.as_str()) == Some("trained") {
                val.get("adapter_manifest")
                    .and_then(|m| m.as_str())
                    .map(PathBuf::from)
            } else {
                None
            }
        });

        let matching_adapter = candidate_adapter.and_then(|p| {
            let manifest = resolve_manifest_path(&p);
            let adapter_dir = resolve_adapter_dir(&p);
            detect::adapter_manifest_names_snapshot(&manifest, &snapshot)
                .then_some((adapter_dir, manifest))
        });

        let Some((adapter_dir, manifest_path)) = matching_adapter else {
            size_entries.push(serde_json::json!({
                "size_token": size_token,
                "hf_id": row.hf_id,
                "status": "not_trained",
                "reason": "train_not_run",
                "floor_mb": floor_mb,
                "free_mb": free_mb,
            }));
            continue;
        };

        check_out_path(&out, hub_root, Some(&adapter_dir))?;

        let base_outcomes = score_reasoning(Side::Base, &snapshot, &loaded_slices);
        let adapter_outcomes = score_reasoning(Side::Adapter, &adapter_dir, &loaded_slices);

        let mut slices_json = serde_json::Map::new();
        for name in SLICE_NAMES {
            let b_out = named_outcome(&base_outcomes, name);
            let a_out = named_outcome(&adapter_outcomes, name);
            if !matches!(b_out, BenchOutcome::Scored(_))
                || !matches!(a_out, BenchOutcome::Scored(_))
            {
                anyhow::bail!("trained size {size_token} failed to score reasoning slice {name}");
            }
            slices_json.insert(
                name.to_string(),
                serde_json::json!({
                    "base": b_out.to_json(),
                    "adapter": a_out.to_json(),
                }),
            );
        }

        let base_intel = score_intelligence(Side::Base, &snapshot, size_token)?;
        let adapter_intel = score_intelligence(Side::Adapter, &adapter_dir, size_token)?;
        if base_intel.n == 0 || adapter_intel.n == 0 {
            anyhow::bail!("intelligence benchmark n must be >= 1 for trained size {size_token}");
        }
        if base_intel.benchmark != adapter_intel.benchmark {
            anyhow::bail!(
                "intelligence benchmark mismatch for {size_token}: {} vs {}",
                base_intel.benchmark,
                adapter_intel.benchmark
            );
        }

        trained_scored_count += 1;
        size_entries.push(serde_json::json!({
            "size_token": size_token,
            "hf_id": row.hf_id,
            "status": "trained",
            "floor_mb": floor_mb,
            "free_mb": free_mb,
            "adapter_manifest": manifest_path.display().to_string(),
            "slices": slices_json,
            "intelligence": {
                "benchmark": base_intel.benchmark,
                "base": {
                    "status": "scored",
                    "pass_rate_at_1": base_intel.pass_rate_at_1,
                    "n": base_intel.n,
                },
                "adapter": {
                    "status": "scored",
                    "pass_rate_at_1": adapter_intel.pass_rate_at_1,
                    "n": adapter_intel.n,
                },
            }
        }));
    }

    let report = serde_json::json!({
        "schema": HARNESS_BEFORE_AFTER_SCHEMA,
        "scope_note": vox_eval::reasoning_slice::REASONING_SLICE_SCOPE_NOTE,
        "slice_size": SLICE_SIZE,
        "free_mb": free_mb,
        "trained_scored_count": trained_scored_count,
        "milestone_ready": trained_scored_count >= 2,
        "gate_input": false,
        "sizes": size_entries,
    });

    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(&report)?;
    std::fs::write(&out, body).with_context(|| format!("writing {}", out.display()))?;
    Ok(out)
}

/// CLI entry point for `vox mens harness-before-after`.
pub(crate) fn run_harness_before_after(args: HarnessBeforeAfterArgs) -> Result<()> {
    let hub_root = detect::resolve_hub_root(
        args.hub_root.as_deref(),
        std::env::var_os("HF_HUB_CACHE"),
        std::env::var_os("HF_HOME"),
        dirs::home_dir(),
    )
    .ok_or_else(|| anyhow::anyhow!("no hub root resolvable; pass --hub-root"))?;
    let runs_root = args.runs_root.unwrap_or_else(|| PathBuf::from("mens/runs"));
    let overlay_root = std::env::current_dir()?;
    let overlay = load_overlay(&overlay_root)
        .map_err(|err| anyhow::anyhow!("load train_bases overlay: {err}"))?;
    let free_mb = read_accelerator_free_mb().map_err(|err| anyhow::anyhow!("{err}"))?;
    let adapters = HashMap::new();
    let _out = write_harness_before_after_report(
        &overlay,
        free_mb,
        &hub_root,
        &args.bench_root,
        &runs_root,
        args.out.as_deref(),
        &adapters,
        &mut |_side, _dir, slices| {
            slices
                .iter()
                .map(|(name, _)| {
                    (
                        *name,
                        BenchOutcome::NotScored {
                            reason: "no live scorer attached".to_string(),
                            n_requested: SLICE_SIZE,
                        },
                    )
                })
                .collect()
        },
        &mut |_side, _dir, size| anyhow::bail!("no live intelligence scorer attached for {size}"),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::mens::eval_collateral::{BenchScore, run_collateral_damage_with};
    use vox_eval::qwen_reasoning::ReasoningScore;

    const PIN_0_6B: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca";
    const PIN_8B: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";

    const SIX: &str = r#""gsm8k_think":0.625,"gsm8k_think_init":0.625,"math500_think":0.625,"math500_think_init":0.625,"vox_think":0.625,"vox_think_init":0.625"#;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn data_root() -> PathBuf {
        repo_root().join("mens/data")
    }

    fn write_ready_snapshot(hub_root: &Path, pin: &str) -> PathBuf {
        let label = BaseLabel::from_pin(pin);
        let snap = detect::snapshot_dir(hub_root, &label).unwrap();
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::write(snap.join("config.json"), "{}").unwrap();
        std::fs::write(snap.join("tokenizer.json"), "{}").unwrap();
        std::fs::write(snap.join("model.safetensors"), b"weights").unwrap();
        snap
    }

    fn write_gated_adapter(snapshot: &Path, pass_collateral: bool) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("adapter_manifest.json"),
            serde_json::to_string(&serde_json::json!({
                "base_model": snapshot.display().to_string(),
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(dir.path().join("candle_qlora_adapter.safetensors"), b"w").unwrap();
        std::fs::write(dir.path().join("config.json"), "{}").unwrap();
        std::fs::write(dir.path().join("tokenizer.json"), "{}").unwrap();

        if pass_collateral {
            let pre = dir.path().join("pre.json");
            std::fs::write(&pre, format!(r#"{{"benchmarks":{{{SIX}}}}}"#)).unwrap();
            let mut scorer = |_: &str| {
                Ok(BenchScore {
                    value: 0.625,
                    n: Some(32),
                })
            };
            let code = run_collateral_damage_with(&pre, dir.path(), &mut scorer).unwrap();
            assert_eq!(code, 0);
            for name in SLICE_NAMES {
                let slice_path = dir
                    .path()
                    .join("collateral_eval")
                    .join(name)
                    .join("reasoning_report.json");
                std::fs::create_dir_all(slice_path.parent().unwrap()).unwrap();
                std::fs::write(
                    slice_path,
                    format!(
                        r#"{{"benchmark":"{name}","status":"scored","n":32,"accuracy":0.625,"think_initiation_rate":0.625}}"#
                    ),
                )
                .unwrap();
            }
            std::fs::write(
                dir.path().join("gate_receipt.json"),
                r#"{
                  "overall_passed": true,
                  "gates": [
                    {"name":"throughput","passed":true,"message":"142 tok/s >= 100"},
                    {"name":"supervised_ratio","passed":true,"message":"0.71 >= 0.60"}
                  ]}"#,
            )
            .unwrap();
        }
        dir
    }

    #[test]
    fn harn_01_serves_chosen_size_base_and_gated_adapter() {
        let overlay = load_overlay(&repo_root()).unwrap();
        let hub = tempfile::tempdir().unwrap();
        let snap_0_6b = write_ready_snapshot(hub.path(), PIN_0_6B);

        // 1. Base serve resolves to the pinned snapshot
        let base_target = resolve_harness_serve_target("0.6B", &overlay, hub.path(), None).unwrap();
        assert_eq!(base_target.hf_id, PIN_0_6B);
        assert_eq!(base_target.size_token, "0.6B");
        assert_eq!(base_target.side, Side::Base);
        assert_eq!(base_target.model_path, snap_0_6b);

        // 2. Adapter without collateral report is refused by the collateral gate
        let ungated = write_gated_adapter(&snap_0_6b, false);
        let err = resolve_harness_serve_target("0.6B", &overlay, hub.path(), Some(ungated.path()))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("eval_collateral_damage check not found"),
            "unexpected error: {err}"
        );

        // 3. Adapter with passing collateral gate + substantive gate_receipt is accepted
        let gated = write_gated_adapter(&snap_0_6b, true);
        let adapter_target =
            resolve_harness_serve_target("0.6B", &overlay, hub.path(), Some(gated.path())).unwrap();
        assert_eq!(adapter_target.hf_id, PIN_0_6B);
        assert_eq!(adapter_target.size_token, "0.6B");
        assert_eq!(adapter_target.side, Side::Adapter);
        assert_eq!(adapter_target.model_path, gated.path());
    }

    #[test]
    fn bench_04_two_trained_sizes_report_reasoning_and_intelligence_and_untrained_have_no_scores() {
        let overlay = load_overlay(&repo_root()).unwrap();
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();

        let snap_0_6b = write_ready_snapshot(hub.path(), PIN_0_6B);
        let snap_8b = write_ready_snapshot(hub.path(), PIN_8B);

        let adapter_0_6b = write_gated_adapter(&snap_0_6b, true);
        let adapter_8b = write_gated_adapter(&snap_8b, true);

        let mut adapters = HashMap::new();
        adapters.insert(PIN_0_6B.to_string(), adapter_0_6b.path().to_path_buf());
        adapters.insert(PIN_8B.to_string(), adapter_8b.path().to_path_buf());

        let out = write_harness_before_after_report(
            &overlay,
            16000,
            hub.path(),
            &data_root(),
            runs.path(),
            None,
            &adapters,
            &mut |side, _path, slices| {
                slices
                    .iter()
                    .map(|(name, items)| {
                        let (acc, think) = match side {
                            Side::Base => (0.50, 0.75),
                            Side::Adapter => (0.625, 0.875),
                        };
                        (
                            *name,
                            BenchOutcome::Scored(ReasoningScore {
                                accuracy: acc,
                                think_initiation_rate: think,
                                n: items.len(),
                            }),
                        )
                    })
                    .collect()
            },
            &mut |side, _path, size_token| {
                let base_rate = if size_token == "0.6B" { 0.35 } else { 0.55 };
                let rate = match side {
                    Side::Base => base_rate,
                    Side::Adapter => base_rate + 0.15,
                };
                Ok(IntelligenceScore {
                    benchmark: "heldout_bench".to_string(),
                    pass_rate_at_1: rate,
                    n: 25,
                })
            },
        )
        .unwrap();

        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(body["schema"], "harness_before_after.v1");
        assert_eq!(body["slice_size"], 32);
        assert_eq!(body["trained_scored_count"], 2);
        assert_eq!(body["milestone_ready"], true);

        let sizes = body["sizes"].as_array().unwrap();
        assert_eq!(sizes.len(), 6);

        // 0.6B and 8B are trained and carry both reasoning slices and intelligence with n
        for idx in [0usize, 1] {
            let entry = &sizes[idx];
            assert_eq!(entry["status"], "trained");
            for slice in SLICE_NAMES {
                assert_eq!(entry["slices"][slice]["base"]["n"], 32);
                assert_eq!(entry["slices"][slice]["base"]["accuracy"], 0.50);
                assert_eq!(
                    entry["slices"][slice]["base"]["think_initiation_rate"],
                    0.75
                );
                assert_eq!(entry["slices"][slice]["adapter"]["n"], 32);
                assert_eq!(entry["slices"][slice]["adapter"]["accuracy"], 0.625);
                assert_eq!(
                    entry["slices"][slice]["adapter"]["think_initiation_rate"],
                    0.875
                );
            }
            assert_eq!(entry["intelligence"]["benchmark"], "heldout_bench");
            assert_eq!(entry["intelligence"]["base"]["n"], 25);
            assert_eq!(entry["intelligence"]["adapter"]["n"], 25);
            assert!(
                entry["intelligence"]["adapter"]["pass_rate_at_1"]
                    .as_f64()
                    .unwrap()
                    > entry["intelligence"]["base"]["pass_rate_at_1"]
                        .as_f64()
                        .unwrap()
            );
        }

        // 14B and 32B do not fit free_mb=16000; 1.7B and 4B are floor_unmeasured.
        // None of them may contain slices or intelligence keys.
        for idx in [2usize, 3, 4, 5] {
            let entry = &sizes[idx];
            assert_eq!(entry["status"], "not_trained");
            assert!(entry.get("slices").is_none());
            assert!(entry.get("intelligence").is_none());
            let serialized = serde_json::to_string(entry).unwrap();
            assert!(!serialized.contains("accuracy"));
            assert!(!serialized.contains("think_initiation_rate"));
            assert!(!serialized.contains("pass_rate_at_1"));
        }
        assert_eq!(sizes[2]["reason"], "does_not_fit");
        assert_eq!(sizes[3]["reason"], "does_not_fit");
        assert_eq!(sizes[4]["reason"], "floor_unmeasured");
        assert_eq!(sizes[5]["reason"], "floor_unmeasured");
    }
}
