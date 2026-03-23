//! Resolve canonical `train.jsonl` (or contract override) before native training.

use std::path::{Path, PathBuf};

use anyhow::Context;

/// Primary training filename inside a data directory.
pub const PRIMARY_TRAIN_FILE: &str = "train.jsonl";
/// Fallback corpus file from extract/validate pipelines.
pub const FALLBACK_TRAIN_FILE: &str = "validated.jsonl";
/// Optional YAML contract under workspace `populi/config/`.
pub const CONTRACT_PATH: &str = "populi/config/training_contract.yaml";

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

fn count_nonempty_lines(path: &Path) -> std::io::Result<usize> {
    let data = std::fs::read_to_string(path)?;
    Ok(data.lines().filter(|l| !l.trim().is_empty()).count())
}

/// Load optional training contract: `train_path` relative to workspace or absolute.
#[derive(Debug, serde::Deserialize)]
struct TrainContract {
    train_path: Option<String>,
}

/// Parse `populi/config/training_contract.yaml` when present; returns override train path if set.
pub fn load_contract(workspace: &Path) -> anyhow::Result<Option<PathBuf>> {
    let p = workspace.join(CONTRACT_PATH);
    if !p.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&p)
        .with_context(|| format!("read training contract {}", p.display()))?;
    let c: TrainContract = serde_yaml::from_str(&raw)
        .with_context(|| format!("parse YAML contract {}", p.display()))?;
    let Some(rel) = c.train_path.filter(|s| !s.trim().is_empty()) else {
        return Ok(None);
    };
    let path = PathBuf::from(&rel);
    let full = if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    };
    Ok(Some(full))
}

/// Pick training JSONL: explicit **`training_contract.yaml`** `train_path` wins over a stale
/// `data_dir/train.jsonl` when both exist, then primary, then fallback.
pub fn resolve_train_input(
    data_dir: &Path,
    workspace: Option<&Path>,
) -> anyhow::Result<ResolvedTrainInput> {
    if let Some(ws) = workspace
        && let Some(contract_path) = load_contract(ws)?
    {
        if !contract_path.is_file() {
            let contract_file = ws.join(CONTRACT_PATH);
            anyhow::bail!(
                "Training contract `{}` sets `train_path` -> `{}`, but that file does not exist.\n\
                 Fix the path or edit the YAML so training does not silently ignore the contract and use `{}` under the data directory.",
                contract_file.display(),
                contract_path.display(),
                PRIMARY_TRAIN_FILE
            );
        }
        let n = count_nonempty_lines(&contract_path).ok();
        return Ok(ResolvedTrainInput {
            path: contract_path,
            source: ResolveSource::Contract,
            sample_count: n,
        });
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
        "No training JSONL found. Expected {} or {} under {} (or {} via {}).",
        PRIMARY_TRAIN_FILE,
        FALLBACK_TRAIN_FILE,
        data_dir.display(),
        CONTRACT_PATH,
        "populi/config/training_contract.yaml"
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
            anyhow::bail!("Training contract override `{}` does not exist.", override_path.display());
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

        let cfg_dir = ws.join("populi/config");
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

        let cfg_dir = ws.join("populi/config");
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
}
