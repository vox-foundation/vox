//! `vox mens size-train` — write one `size_train.v1` record per `qwen3_code` QLoRA size
//! from one free-memory reading without downloading weights or running training.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vox_eval::reasoning_bench::BaseLabel;
use vox_populi::mens::tensor::spoke_base_resolver::{
    TrainBase, load_overlay, read_accelerator_free_mb,
};

use super::reasoning_compare::check_out_path;
use super::reasoning_compare::detect;

/// Subdirectory under `mens/runs/` holding one `size_train.v1` JSON file per QLoRA size.
pub(crate) const SIZE_TRAIN_SUBDIR: &str = "size_train";

/// Schema identifier written on every `size_train` JSON record.
pub(crate) const SIZE_TRAIN_SCHEMA: &str = "size_train.v1";

const QWEN3_SIZE_ORDER: &[&str] = &["0.6B", "1.7B", "4B", "8B", "14B", "32B"];

/// Parsed `vox mens size-train` arguments.
pub(crate) struct SizeTrainArgs {
    /// Hugging Face hub cache root. `None` resolves from the environment.
    pub hub_root: Option<PathBuf>,
    /// Output runs root. `None` defaults to `mens/runs`.
    pub runs_root: Option<PathBuf>,
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

/// Write one `size_train.v1` record for `row`.
pub(crate) fn write_one_size_train(
    runs_root: &Path,
    hub_root: &Path,
    row: &TrainBase,
    free_mb: u32,
    adapter: Option<&Path>,
) -> Result<PathBuf> {
    let label = BaseLabel::from_pin(&row.hf_id);
    let out = runs_root
        .join(SIZE_TRAIN_SUBDIR)
        .join(label.report_file_name());
    let adapter_dir = adapter.map(|p| {
        if p.file_name()
            .is_some_and(|name| name == "adapter_manifest.json")
        {
            p.parent().unwrap_or(p)
        } else {
            p
        }
    });
    check_out_path(&out, hub_root, adapter_dir)?;

    let size_token = size_token_for_label(&label);

    let record = if !row.floor_measured {
        serde_json::json!({
            "schema": SIZE_TRAIN_SCHEMA,
            "status": "not_trained",
            "reason": "floor_unmeasured",
            "floor_measured": false,
            "hf_id": row.hf_id,
            "size_token": size_token,
            "free_mb": free_mb,
        })
    } else {
        let floor_mb = row
            .floor_mb
            .ok_or_else(|| anyhow::anyhow!("measured row {} missing floor_mb", row.hf_id))?;
        if floor_mb > free_mb {
            serde_json::json!({
                "schema": SIZE_TRAIN_SCHEMA,
                "status": "not_trained",
                "reason": "does_not_fit",
                "hf_id": row.hf_id,
                "size_token": size_token,
                "free_mb": free_mb,
                "floor_mb": floor_mb,
            })
        } else {
            let ready_snapshot = detect::snapshot_dir(hub_root, &label)
                .filter(|dir| dir.is_dir() && detect::snapshot_complete(dir));
            match ready_snapshot {
                None => serde_json::json!({
                    "schema": SIZE_TRAIN_SCHEMA,
                    "status": "not_trained",
                    "reason": "base_snapshot_absent",
                    "hf_id": row.hf_id,
                    "size_token": size_token,
                    "free_mb": free_mb,
                    "floor_mb": floor_mb,
                }),
                Some(snapshot) => {
                    let matching_manifest = adapter
                        .map(resolve_manifest_path)
                        .filter(|m| detect::adapter_manifest_names_snapshot(m, &snapshot));
                    match matching_manifest {
                        Some(manifest_path) => serde_json::json!({
                            "schema": SIZE_TRAIN_SCHEMA,
                            "status": "trained",
                            "hf_id": row.hf_id,
                            "size_token": size_token,
                            "free_mb": free_mb,
                            "floor_mb": floor_mb,
                            "adapter_manifest": manifest_path.display().to_string(),
                        }),
                        None => serde_json::json!({
                            "schema": SIZE_TRAIN_SCHEMA,
                            "status": "not_trained",
                            "reason": "train_not_run",
                            "hf_id": row.hf_id,
                            "size_token": size_token,
                            "free_mb": free_mb,
                            "floor_mb": floor_mb,
                        }),
                    }
                }
            }
        }
    };

    write_pretty(&out, &record)?;
    Ok(out)
}

/// Write one `size_train.v1` record per `qwen3_code` QLoRA size, smallest measured `floor_mb` first,
/// followed by unmeasured rows in `QWEN3_SIZE_TOKENS` order.
pub(crate) fn write_size_train_records(
    overlay: &HashMap<String, Vec<TrainBase>>,
    free_mb: u32,
    hub_root: &Path,
    runs_root: &Path,
    adapters: &HashMap<String, PathBuf>,
) -> Result<Vec<PathBuf>> {
    let rows = overlay
        .get("qwen3_code")
        .ok_or_else(|| anyhow::anyhow!("overlay missing qwen3_codespoke"))?;

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

    let mut written = Vec::with_capacity(measured.len() + unmeasured.len());
    for row in measured.into_iter().chain(unmeasured) {
        let adapter = adapters.get(&row.hf_id).map(PathBuf::as_path);
        let path = write_one_size_train(runs_root, hub_root, row, free_mb, adapter)?;
        written.push(path);
    }
    Ok(written)
}

/// Take one free-memory reading via `read_free_mb`, load the overlay from `overlay_root`,
/// and write every `qwen3_code` QLoRA size record.
pub(crate) fn run_size_train_reading<F>(
    read_free_mb: F,
    hub_root: &Path,
    overlay_root: &Path,
    runs_root: &Path,
    adapters: &HashMap<String, PathBuf>,
) -> Result<Vec<PathBuf>>
where
    F: FnOnce() -> Result<u32>,
{
    let free_mb = read_free_mb()?;
    let overlay = load_overlay(overlay_root)
        .map_err(|err| anyhow::anyhow!("load train_bases overlay: {err}"))?;
    write_size_train_records(&overlay, free_mb, hub_root, runs_root, adapters)
}

/// CLI entry point for `vox mens size-train`.
pub(crate) fn run_size_train(args: SizeTrainArgs) -> Result<()> {
    let hub_root = detect::resolve_hub_root(
        args.hub_root.as_deref(),
        std::env::var_os("HF_HUB_CACHE"),
        std::env::var_os("HF_HOME"),
        dirs::home_dir(),
    )
    .ok_or_else(|| anyhow::anyhow!("no hub root resolvable; pass --hub-root"))?;
    let runs_root = args.runs_root.unwrap_or_else(|| PathBuf::from("mens/runs"));
    let overlay_root = std::env::current_dir()?;
    let adapters = HashMap::new();
    let _paths = run_size_train_reading(
        || read_accelerator_free_mb().map_err(|err| anyhow::anyhow!("{err}")),
        &hub_root,
        &overlay_root,
        &runs_root,
        &adapters,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN_0_6B: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca";
    const PIN_1_7B: &str = "Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e";
    const PIN_4B: &str = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";
    const PIN_8B: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";
    const PIN_14B: &str = "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18";
    const PIN_32B: &str = "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137";

    const ALLOWED_KEYS: &[&str] = &[
        "schema",
        "status",
        "hf_id",
        "size_token",
        "free_mb",
        "floor_mb",
        "floor_measured",
        "reason",
        "adapter_manifest",
    ];

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn read_json(path: &Path) -> serde_json::Value {
        let text = std::fs::read_to_string(path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&text).unwrap();
        let obj = val.as_object().expect("record must be a JSON object");
        for key in obj.keys() {
            assert!(
                ALLOWED_KEYS.contains(&key.as_str()),
                "unexpected key {key} in {}",
                path.display()
            );
        }
        assert!(!obj.contains_key("score"));
        val
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

    #[test]
    fn size_train_absent_0_6b_record() {
        let overlay = load_overlay(&repo_root()).unwrap();
        let row = overlay["qwen3_code"]
            .iter()
            .find(|r| r.hf_id == PIN_0_6B && r.methods.iter().any(|m| m == "qlora"))
            .expect("0.6B qlora row");

        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let out = write_one_size_train(runs.path(), hub.path(), row, 2500, None).unwrap();
        assert!(
            out.ends_with("size_train/Qwen3-0.6B-c1899de289a0.json"),
            "unexpected path {}",
            out.display()
        );

        let val = read_json(&out);
        assert_eq!(val["schema"], "size_train.v1");
        assert_eq!(val["status"], "not_trained");
        assert_eq!(val["reason"], "base_snapshot_absent");
        assert_eq!(val["hf_id"], PIN_0_6B);
        assert_eq!(val["size_token"], "0.6B");
        assert_eq!(val["free_mb"], 2500);
        assert_eq!(val["floor_mb"], 2000);
        assert_eq!(val["floor_mb"], serde_json::json!(row.floor_mb.unwrap()));
        assert!(val.get("adapter_manifest").is_none());

        let inside_hub = hub.path().join("runs");
        let res = write_one_size_train(&inside_hub, hub.path(), row, 2500, None);
        assert!(res.is_err(), "runs_root inside hub must fail");
        assert!(
            !inside_hub.exists(),
            "no JSON or directory may be created inside hub"
        );
    }

    #[test]
    fn size_train_records_every_qlora_size_smallest_floor_first() {
        let overlay = load_overlay(&repo_root()).unwrap();
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let adapters = HashMap::new();

        let paths =
            write_size_train_records(&overlay, 1500, hub.path(), runs.path(), &adapters).unwrap();
        let names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "Qwen3-0.6B-c1899de289a0.json",
                "Qwen3-8B-b968826d9c46.json",
                "Qwen3-14B-40c069824f42.json",
                "Qwen3-32B-9216db5781bf.json",
                "Qwen3-1.7B-70d244cc86cc.json",
                "Qwen3-4B-1cfa9a720891.json",
            ]
        );

        let expected_measured = [
            (0usize, PIN_0_6B, 2000u64),
            (1, PIN_8B, 12000),
            (2, PIN_14B, 20000),
            (3, PIN_32B, 60000),
        ];
        for (idx, pin, floor) in expected_measured {
            let v = read_json(&paths[idx]);
            assert_eq!(v["status"], "not_trained");
            assert_eq!(v["reason"], "does_not_fit");
            assert_eq!(v["hf_id"], pin);
            assert_eq!(v["free_mb"], 1500);
            assert_eq!(v["floor_mb"], floor);
            assert!(v.get("adapter_manifest").is_none());
        }

        for (idx, pin) in [(4usize, PIN_1_7B), (5, PIN_4B)] {
            let v = read_json(&paths[idx]);
            assert_eq!(v["status"], "not_trained");
            assert_eq!(v["reason"], "floor_unmeasured");
            assert_eq!(v["floor_measured"], false);
            assert_eq!(v["hf_id"], pin);
            assert_eq!(v["free_mb"], 1500);
            assert!(v.get("floor_mb").is_none());
            assert!(v.get("adapter_manifest").is_none());
        }

        let dir_count = std::fs::read_dir(runs.path().join(SIZE_TRAIN_SUBDIR))
            .unwrap()
            .count();
        assert_eq!(dir_count, 6);

        // Second call: free_mb 70000 and 8B snapshot containing only config.json
        let snap_8b = detect::snapshot_dir(hub.path(), &BaseLabel::from_pin(PIN_8B)).unwrap();
        std::fs::create_dir_all(&snap_8b).unwrap();
        std::fs::write(snap_8b.join("config.json"), "{}").unwrap();

        let paths2 =
            write_size_train_records(&overlay, 70000, hub.path(), runs.path(), &adapters).unwrap();
        let v_8b = read_json(&paths2[1]);
        assert_eq!(v_8b["status"], "not_trained");
        assert_eq!(v_8b["reason"], "base_snapshot_absent");
        assert_eq!(v_8b["floor_mb"], 12000);
        assert_eq!(v_8b["free_mb"], 70000);
    }

    #[test]
    fn size_train_ready_fixture_is_trained_when_manifest_names_the_pin() {
        let overlay = load_overlay(&repo_root()).unwrap();
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let adapter_dir = tempfile::tempdir().unwrap();

        let snap_0_6b = write_ready_snapshot(hub.path(), PIN_0_6B);
        let manifest_path = adapter_dir.path().join("adapter_manifest.json");
        std::fs::write(
            &manifest_path,
            serde_json::to_string(&serde_json::json!({
                "base_model": snap_0_6b.display().to_string(),
            }))
            .unwrap(),
        )
        .unwrap();

        let mut adapters = HashMap::new();
        adapters.insert(PIN_0_6B.to_string(), manifest_path.clone());

        let paths =
            write_size_train_records(&overlay, 70000, hub.path(), runs.path(), &adapters).unwrap();
        let v0 = read_json(&paths[0]);
        assert_eq!(v0["status"], "trained");
        assert_eq!(v0["hf_id"], PIN_0_6B);
        assert_eq!(v0["floor_mb"], 2000);
        assert_eq!(v0["free_mb"], 70000);
        assert_eq!(v0["adapter_manifest"], manifest_path.display().to_string());
        assert!(v0.get("reason").is_none());

        for idx in [1usize, 2, 3] {
            let v = read_json(&paths[idx]);
            assert_eq!(v["status"], "not_trained");
            assert_eq!(v["reason"], "base_snapshot_absent");
        }
        for idx in [4usize, 5] {
            let v = read_json(&paths[idx]);
            assert_eq!(v["status"], "not_trained");
            assert_eq!(v["reason"], "floor_unmeasured");
            assert_eq!(v["floor_measured"], false);
        }
    }

    #[test]
    fn size_train_ready_without_adapter_is_train_not_run() {
        let overlay = load_overlay(&repo_root()).unwrap();
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let _snap_0_6b = write_ready_snapshot(hub.path(), PIN_0_6B);

        let empty = HashMap::new();
        let paths =
            write_size_train_records(&overlay, 70000, hub.path(), runs.path(), &empty).unwrap();
        let v0 = read_json(&paths[0]);
        assert_eq!(v0["status"], "not_trained");
        assert_eq!(v0["reason"], "train_not_run");
        assert_eq!(v0["floor_mb"], 2000);
        assert_eq!(v0["free_mb"], 70000);
        assert!(v0.get("adapter_manifest").is_none());

        // Manifest pointing to a different directory is also train_not_run
        let other_dir = tempfile::tempdir().unwrap();
        let bad_adapter = tempfile::tempdir().unwrap();
        let bad_manifest = bad_adapter.path().join("adapter_manifest.json");
        std::fs::write(
            &bad_manifest,
            serde_json::to_string(&serde_json::json!({
                "base_model": other_dir.path().display().to_string(),
            }))
            .unwrap(),
        )
        .unwrap();
        let mut wrong_map = HashMap::new();
        wrong_map.insert(PIN_0_6B.to_string(), bad_manifest);

        let paths2 =
            write_size_train_records(&overlay, 70000, hub.path(), runs.path(), &wrong_map).unwrap();
        let v0_wrong = read_json(&paths2[0]);
        assert_eq!(v0_wrong["status"], "not_trained");
        assert_eq!(v0_wrong["reason"], "train_not_run");
        assert!(v0_wrong.get("adapter_manifest").is_none());
    }

    #[test]
    fn size_train_command_reads_free_memory_once() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let adapters = HashMap::new();
        let mut calls = 0usize;

        let paths = run_size_train_reading(
            || {
                calls += 1;
                Ok(2500)
            },
            hub.path(),
            &repo_root(),
            runs.path(),
            &adapters,
        )
        .unwrap();

        assert_eq!(calls, 1);
        assert_eq!(paths.len(), 6);
        let v0 = read_json(&paths[0]);
        assert_eq!(v0["status"], "not_trained");
        assert_eq!(v0["reason"], "base_snapshot_absent");
        assert_eq!(v0["free_mb"], 2500);
        assert_eq!(v0["floor_mb"], 2000);
    }

    #[test]
    fn size_train_command_ready_snapshot_is_train_not_run() {
        let hub = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        let _snap_0_6b = write_ready_snapshot(hub.path(), PIN_0_6B);
        let adapters = HashMap::new();
        let mut calls = 0usize;

        let paths = run_size_train_reading(
            || {
                calls += 1;
                Ok(2500)
            },
            hub.path(),
            &repo_root(),
            runs.path(),
            &adapters,
        )
        .unwrap();

        assert_eq!(calls, 1);
        let v0 = read_json(&paths[0]);
        assert_eq!(v0["status"], "not_trained");
        assert_eq!(v0["reason"], "train_not_run");
        assert_eq!(v0["floor_mb"], 2000);
        assert_eq!(v0["free_mb"], 2500);
        assert!(v0.get("adapter_manifest").is_none());

        for (idx, floor) in [(1usize, 12000u64), (2, 20000), (3, 60000)] {
            let v = read_json(&paths[idx]);
            assert_eq!(v["status"], "not_trained");
            assert_eq!(v["reason"], "does_not_fit");
            assert_eq!(v["free_mb"], 2500);
            assert_eq!(v["floor_mb"], floor);
        }

        for idx in [4usize, 5] {
            let v = read_json(&paths[idx]);
            assert_eq!(v["status"], "not_trained");
            assert_eq!(v["reason"], "floor_unmeasured");
            assert_eq!(v["floor_measured"], false);
        }
    }
}
