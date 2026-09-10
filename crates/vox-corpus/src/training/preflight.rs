//! Resolve canonical `train.jsonl` (or contract override) before native training.

use std::path::{Path, PathBuf};

use anyhow::Context;

use vox_bounded_fs::read_utf8_path_capped;

/// Primary training filename inside a data directory.
pub const PRIMARY_TRAIN_FILE: &str = "train.jsonl";
/// Fallback corpus file from extract/validate pipelines.
pub const FALLBACK_TRAIN_FILE: &str = "validated.jsonl";
/// Optional YAML contract under workspace `mens/config/`.
pub const CONTRACT_PATH: &str = "mens/config/training_contract.yaml";
/// Optional per-run contract inside a training `--data-dir`.
pub const DATA_DIR_CONTRACT_FILE: &str = "training_contract.yaml";

/// Where the resolved training JSONL came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveSource {
    /// `data_dir/train.jsonl`
    Primary,
    /// `data_dir/validated.jsonl`
    Fallback,
    /// Path from contract file
    Contract,
}

/// Resolved training input for a run.
#[derive(Debug, Clone)]
pub struct ResolvedTrainInput {
    /// Absolute or logical path to the JSONL used for training.
    pub path: PathBuf,
    /// Which resolution rule produced `path`.
    pub source: ResolveSource,
    /// Line count of non-empty JSONL rows (None if unreadable).
    pub sample_count: Option<usize>,
}

fn count_nonempty_lines(path: &Path) -> anyhow::Result<usize> {
    use std::io::BufRead;
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::with_capacity(128 * 1024, file);
    let mut count = 0;
    for line in reader.lines() {
        if let Ok(l) = line
            && !l.trim().is_empty()
        {
            count += 1;
        }
    }
    Ok(count)
}

/// Load optional training contract: `train_path` relative to workspace or absolute.
#[derive(Debug, serde::Deserialize)]
struct TrainContract {
    train_path: Option<String>,
}

/// Parse a training contract YAML; returns override train path if set.
///
/// Relative `train_path` values are resolved against `path_base` (workspace root for
/// the workspace contract, or the data directory for a per-run contract).
fn load_contract_file(contract: &Path, path_base: &Path) -> anyhow::Result<Option<PathBuf>> {
    if !contract.is_file() {
        return Ok(None);
    }
    let raw = read_utf8_path_capped(contract)
        .with_context(|| format!("read training contract {}", contract.display()))?;
    let c: TrainContract = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse YAML contract {}", contract.display()))?;
    let Some(rel) = c.train_path.filter(|s| !s.trim().is_empty()) else {
        return Ok(None);
    };
    let path = PathBuf::from(&rel);
    let full = if path.is_absolute() {
        path
    } else {
        path_base.join(path)
    };
    Ok(Some(full))
}

/// Parse `mens/config/training_contract.yaml` when present; returns override train path if set.
pub fn load_contract(workspace: &Path) -> anyhow::Result<Option<PathBuf>> {
    load_contract_file(&workspace.join(CONTRACT_PATH), workspace)
}

fn resolve_from_contract(
    contract_file: &Path,
    train_path: PathBuf,
) -> anyhow::Result<ResolvedTrainInput> {
    if !train_path.is_file() {
        anyhow::bail!(
            "Training contract `{}` sets `train_path` -> `{}`, but that file does not exist.\n\
             Fix the path or edit the YAML so training does not silently ignore the contract and use `{}` under the data directory.",
            contract_file.display(),
            train_path.display(),
            PRIMARY_TRAIN_FILE
        );
    }
    let n = count_nonempty_lines(&train_path).ok();
    Ok(ResolvedTrainInput {
        path: train_path,
        source: ResolveSource::Contract,
        sample_count: n,
    })
}

/// Pick training JSONL: per-run **`data_dir/training_contract.yaml`** wins over the
/// workspace contract, then either contract's `train_path` wins over a stale
/// `data_dir/train.jsonl`, then primary, then fallback.
pub fn resolve_train_input(
    data_dir: &Path,
    workspace: Option<&Path>,
) -> anyhow::Result<ResolvedTrainInput> {
    let data_dir_contract = data_dir.join(DATA_DIR_CONTRACT_FILE);
    if let Some(contract_path) = load_contract_file(&data_dir_contract, data_dir)? {
        return resolve_from_contract(&data_dir_contract, contract_path);
    }

    if let Some(ws) = workspace
        && let Some(contract_path) = load_contract(ws)?
    {
        return resolve_from_contract(&ws.join(CONTRACT_PATH), contract_path);
    }

    let primary = data_dir.join(PRIMARY_TRAIN_FILE);
    if primary.is_file() {
        let n = count_nonempty_lines(&primary).ok();
        return Ok(ResolvedTrainInput {
            path: primary,
            source: ResolveSource::Primary,
            sample_count: n,
        });
    }

    let fallback = data_dir.join(FALLBACK_TRAIN_FILE);
    if fallback.is_file() {
        let n = count_nonempty_lines(&fallback).ok();
        return Ok(ResolvedTrainInput {
            path: fallback,
            source: ResolveSource::Fallback,
            sample_count: n,
        });
    }

    anyhow::bail!(
        "No training JSONL found. Expected {} or {} under {} (or {} / {}).",
        PRIMARY_TRAIN_FILE,
        FALLBACK_TRAIN_FILE,
        data_dir.display(),
        data_dir.join(DATA_DIR_CONTRACT_FILE).display(),
        CONTRACT_PATH
    )
}

/// Validate that a training file exists and is non-empty; returns resolved path + counts.
pub fn validate_train_preflight(
    data_dir: &Path,
    contract_override: Option<&Path>,
    workspace_root: Option<&Path>,
) -> anyhow::Result<ResolvedTrainInput> {
    let resolved = if let Some(override_path) = contract_override {
        if !override_path.is_file() {
            anyhow::bail!(
                "Training contract override `{}` does not exist.",
                override_path.display()
            );
        }
        let n = count_nonempty_lines(override_path).ok();
        ResolvedTrainInput {
            path: override_path.to_path_buf(),
            source: ResolveSource::Contract,
            sample_count: n,
        }
    } else {
        resolve_train_input(data_dir, workspace_root)?
    };

    let count = resolved.sample_count.unwrap_or(0);
    if count == 0 {
        anyhow::bail!(
            "Training file {} is missing or has no JSONL rows",
            resolved.path.display()
        );
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn contract_train_path_missing_errors_instead_of_fallback() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("dogfood");
        std::fs::create_dir_all(&data).unwrap();
        let ws: &std::path::Path = tmp.path();

        let primary = data.join(PRIMARY_TRAIN_FILE);
        std::fs::write(&primary, "{\"prompt\":\"stale\",\"response\":\"x\"}\n").unwrap();

        let cfg_dir = ws.join("mens/config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        let mut f = std::fs::File::create(cfg_dir.join("training_contract.yaml")).unwrap();
        writeln!(f, "train_path: this_file_does_not_exist.jsonl").unwrap();

        let err = resolve_train_input(&data, Some(ws)).unwrap_err();
        let s = err.to_string();
        assert!(
            s.contains("this_file_does_not_exist") || s.contains("does not exist"),
            "{s}"
        );
    }

    #[test]
    fn contract_train_path_wins_over_primary_train_jsonl() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("dogfood");
        std::fs::create_dir_all(&data).unwrap();
        let ws: &std::path::Path = tmp.path();

        let primary = data.join(PRIMARY_TRAIN_FILE);
        std::fs::write(&primary, "{\"prompt\":\"old\",\"response\":\"x\"}\n").unwrap();

        let contract_target = ws.join("custom_train.jsonl");
        std::fs::write(
            &contract_target,
            "{\"prompt\":\"from_contract\",\"response\":\"y\"}\n",
        )
        .unwrap();

        let cfg_dir = ws.join("mens/config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        let mut f = std::fs::File::create(cfg_dir.join("training_contract.yaml")).unwrap();
        writeln!(
            f,
            "train_path: {}",
            contract_target.file_name().unwrap().to_string_lossy()
        )
        .unwrap();

        let r = resolve_train_input(&data, Some(ws)).expect("resolve");
        assert_eq!(r.source, ResolveSource::Contract);
        assert_eq!(r.path, contract_target);
    }

    #[test]
    fn data_dir_contract_wins_over_workspace_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data = tmp.path().join("metal-e2e");
        std::fs::create_dir_all(&data).unwrap();
        let ws: &std::path::Path = tmp.path();

        let workspace_target = ws.join("workspace_train.jsonl");
        std::fs::write(
            &workspace_target,
            "{\"prompt\":\"workspace\",\"response\":\"x\"}\n",
        )
        .unwrap();
        let cfg_dir = ws.join("mens/config");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        let mut f = std::fs::File::create(cfg_dir.join("training_contract.yaml")).unwrap();
        writeln!(f, "train_path: workspace_train.jsonl").unwrap();

        let local_target = data.join("dogfood-metal-e2e.jsonl");
        std::fs::write(&local_target, "{\"prompt\":\"local\",\"response\":\"y\"}\n").unwrap();
        let mut local = std::fs::File::create(data.join(DATA_DIR_CONTRACT_FILE)).unwrap();
        writeln!(local, "train_path: dogfood-metal-e2e.jsonl").unwrap();

        let r = resolve_train_input(&data, Some(ws)).expect("resolve");
        assert_eq!(r.source, ResolveSource::Contract);
        assert_eq!(r.path, local_target);
    }
}
