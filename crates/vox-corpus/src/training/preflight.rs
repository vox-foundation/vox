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

/// Result of [`check_heldout_contamination`] (D-03).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HeldoutCheck {
    /// `clean`, or `not_checked` when no workspace root is known.
    pub status: &'static str,
    /// `mens/data/*_think/reasoning.jsonl` files read.
    pub slices: usize,
    /// Questions searched for (those that normalize to nothing are dropped).
    pub heldout_questions: usize,
    /// Non-blank rows of the train file searched.
    pub rows_checked: usize,
}

impl std::fmt::Display for HeldoutCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({} slices, {} held-out questions, {} rows)",
            self.status, self.slices, self.heldout_questions, self.rows_checked
        )
    }
}

/// Lowercase; every non-alphanumeric character becomes a space; whitespace
/// collapsed; trimmed.
fn normalize(s: &str) -> String {
    let spaced: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Appends every string value in `v` (objects and arrays walked) to `out`.
fn push_strings(v: &serde_json::Value, out: &mut String) {
    match v {
        serde_json::Value::String(s) => {
            out.push(' ');
            out.push_str(s);
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| push_strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| push_strings(x, out)),
        _ => {}
    }
}

/// Calls `f(line_number, row)` for each non-blank JSONL row; a read or parse
/// error names the file and line.
fn for_each_row(
    path: &Path,
    mut f: impl FnMut(usize, serde_json::Value) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    for (i, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| format!("read {} line {}", path.display(), i + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let row = serde_json::from_str(&line)
            .with_context(|| format!("{} line {}: invalid JSON", path.display(), i + 1))?;
        f(i + 1, row)?;
    }
    Ok(())
}

/// Refuses a train file in which any row contains a held-out reasoning question
/// (D-03): every `mens/data/*_think/reasoning.jsonl` question, normalized, is
/// searched for (whole words) in the normalized text of every string field of
/// every row. An unreadable slice fails closed; no workspace root is
/// `not_checked`.
pub fn check_heldout_contamination(
    train_path: &Path,
    workspace_root: Option<&Path>,
) -> anyhow::Result<HeldoutCheck> {
    let Some(root) = workspace_root else {
        return Ok(HeldoutCheck {
            status: "not_checked",
            slices: 0,
            heldout_questions: 0,
            rows_checked: 0,
        });
    };
    let data = root.join("mens/data");
    let mut slices: Vec<PathBuf> = std::fs::read_dir(&data)
        .with_context(|| format!("read held-out slices under {}", data.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("_think"))
        })
        .map(|p| p.join("reasoning.jsonl"))
        .filter(|p| p.is_file())
        .collect();
    slices.sort();
    // (" <normalized question> ", "<slice> line <n>")
    let mut questions: Vec<(String, String)> = Vec::new();
    for slice in &slices {
        for_each_row(slice, |n, row| {
            let q = row
                .get("question")
                .and_then(|q| q.as_str())
                .with_context(|| format!("{} line {n}: no string `question`", slice.display()))?;
            questions.push((
                format!(" {} ", normalize(q)),
                format!("{} line {n}", slice.display()),
            ));
            Ok(())
        })?;
    }
    questions.retain(|(q, _)| !q.trim().is_empty());
    let mut rows_checked = 0;
    let mut hits: Vec<(usize, &str)> = Vec::new();
    for_each_row(train_path, |n, row| {
        rows_checked += 1;
        let mut text = String::new();
        push_strings(&row, &mut text);
        let text = format!(" {} ", normalize(&text));
        if let Some((_, origin)) = questions.iter().find(|(q, _)| text.contains(q.as_str())) {
            hits.push((n, origin));
        }
        Ok(())
    })?;
    if !hits.is_empty() {
        let lines: Vec<String> = hits.iter().take(20).map(|(n, _)| n.to_string()).collect();
        anyhow::bail!(
            "held-out contamination: {} row(s) of {} contain a question from mens/data/*_think \
             (lines {}; line {} matches {}); remove them before training",
            hits.len(),
            train_path.display(),
            lines.join(", "),
            hits[0].0,
            hits[0].1
        );
    }
    Ok(HeldoutCheck {
        status: "clean",
        slices: slices.len(),
        heldout_questions: questions.len(),
        rows_checked,
    })
}

/// Pre-flight training safety gate (`TRAIN-05`).
///
/// Blocks training if:
/// 1. `decontamination.contaminated_removed > 0` and caller did not pass the decontaminated slice,
///    or `records` is empty;
/// 2. Any of the 5 `SpokeKind::ALL` spokes has `0` rows in `snapshot` / `records`;
/// 3. Any record in `records` fails `TrainingRecordV2::verify_integrity()`.
pub fn validate_snapshot_safety_gate(
    snapshot: &crate::dataset_snapshot::TrustyDatasetSnapshot,
    records: &[crate::training::contract::TrainingRecordV2],
    unfiltered_holdout_leakage_count: usize,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if unfiltered_holdout_leakage_count > 0 {
        errors.push(format!(
            "holdout contamination gate failed: {unfiltered_holdout_leakage_count} benchmark-leaking rows present"
        ));
    }

    if records.is_empty() || snapshot.total_rows == 0 {
        errors.push("snapshot contains zero training records".to_string());
    }

    for spoke in crate::training::contract::SpokeKind::ALL {
        let count = records.iter().filter(|r| r.spoke == spoke).count();
        if count == 0 {
            errors.push(format!(
                "spoke coverage gate failed: spoke `{}` has 0 records",
                spoke.as_str()
            ));
        }
    }

    for (idx, rec) in records.iter().enumerate() {
        if !rec.verify_integrity() {
            errors.push(format!(
                "record {idx} (`{}`) failed oracle or content_digest integrity verification",
                rec.provenance.source_id
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
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

    #[test]
    fn validate_snapshot_safety_gate_enforces_contamination_and_spoke_coverage() {
        use crate::dataset_snapshot::{SplitPolicy, build_trusty_snapshot};
        use crate::training::contract::{
            OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2,
        };

        let recs: Vec<TrainingRecordV2> = SpokeKind::ALL
            .into_iter()
            .map(|spoke| {
                TrainingRecordV2::new(
                    spoke,
                    format!("prompt {}", spoke.as_str()),
                    "sys",
                    "resp",
                    OracleReceipt {
                        spoke,
                        passed: true,
                        stage: "ok".to_string(),
                        diagnostics: Vec::new(),
                        exit_code: Some(0),
                    },
                    RecordProvenance {
                        origin: "test".to_string(),
                        source_id: spoke.as_str().to_string(),
                        task_family: "test".to_string(),
                        difficulty: 3,
                    },
                )
            })
            .collect();
        let split = SplitPolicy {
            train_fraction: 0.9,
            eval_fraction: 0.1,
            seed: 42,
        };
        let (snap, _) = build_trusty_snapshot("snap-safe", &recs, split.clone());
        assert!(validate_snapshot_safety_gate(&snap, &recs, 0).is_ok());

        // Contaminate or drop a spoke -> must fail closed.
        assert!(validate_snapshot_safety_gate(&snap, &recs, 1).is_err());
        let partial = &recs[..4];
        let (snap_partial, _) = build_trusty_snapshot("snap-partial", partial, split);
        assert!(validate_snapshot_safety_gate(&snap_partial, partial, 0).is_err());
    }

    /// Workspace with `mens/data/a_think/reasoning.jsonl` holding `questions`,
    /// and a train file holding `rows`.
    fn heldout_fixture(questions: &[&str], rows: &[&str]) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let slice = tmp.path().join("mens/data/a_think");
        std::fs::create_dir_all(&slice).unwrap();
        let body: String = questions
            .iter()
            .map(|q| {
                format!(
                    "{}\n",
                    serde_json::json!({"id": "x", "question": q, "answer": "1"})
                )
            })
            .collect();
        std::fs::write(slice.join("reasoning.jsonl"), body).unwrap();
        let train = tmp.path().join("train.jsonl");
        std::fs::write(&train, rows.join("\n") + "\n").unwrap();
        (tmp, train)
    }

    const Q1: &str = "How many eggs does Janet sell each day?";
    const Q2: &str = "What is 2 + 2?";

    #[test]
    fn heldout_question_in_a_row_is_refused_with_its_line() {
        let (tmp, train) = heldout_fixture(
            &[Q1, Q2],
            &[
                r#"{"prompt":"unrelated","response":"a"}"#,
                r#"{"prompt":"Solve: how many EGGS does janet sell, each day??","response":"b"}"#,
                r#"{"prompt":"other","response":"c"}"#,
            ],
        );
        let err = check_heldout_contamination(&train, Some(tmp.path()))
            .expect_err("contaminated corpus must be refused")
            .to_string();
        assert!(err.contains("lines 2"), "{err}");
        assert!(err.contains("a_think/reasoning.jsonl"), "{err}");
    }

    #[test]
    fn question_inside_messages_is_found() {
        let (tmp, train) = heldout_fixture(
            &[Q1, Q2],
            &[
                r#"{"messages":[{"role":"user","content":"Please: How many eggs does Janet sell each day?"}]}"#,
                r#"{"prompt":"other","response":"c"}"#,
            ],
        );
        let err = check_heldout_contamination(&train, Some(tmp.path()))
            .expect_err("question in messages must be found")
            .to_string();
        assert!(err.contains("lines 1"), "{err}");
    }

    #[test]
    fn clean_corpus_reports_slices_questions_and_rows() {
        let (tmp, train) = heldout_fixture(
            &[Q1, Q2],
            &[
                r#"{"prompt":"a","response":"x"}"#,
                r#"{"prompt":"b","response":"y"}"#,
                r#"{"prompt":"what is 2 + 22","response":"z"}"#,
            ],
        );
        let got = check_heldout_contamination(&train, Some(tmp.path())).expect("clean");
        assert_eq!(
            got,
            HeldoutCheck {
                status: "clean",
                slices: 1,
                heldout_questions: 2,
                rows_checked: 3,
            }
        );
    }

    #[test]
    fn punctuation_only_question_matches_nothing() {
        let (tmp, train) = heldout_fixture(
            &[Q1, "???"],
            &[r#"{"prompt":"a","response":"x"}"#, r#"{"tags":[]}"#],
        );
        let got = check_heldout_contamination(&train, Some(tmp.path())).expect("clean");
        assert_eq!(got.heldout_questions, 1);
        assert_eq!(got.rows_checked, 2);
    }

    #[test]
    fn unreadable_slice_fails_closed() {
        for bad in [r#"{"id":"x","answer":"1"}"#, "not json"] {
            let (tmp, train) = heldout_fixture(&[Q1], &[r#"{"prompt":"a"}"#]);
            let slice = tmp.path().join("mens/data/a_think/reasoning.jsonl");
            std::fs::write(
                &slice,
                format!("{}\n{bad}\n", serde_json::json!({"question": Q2})),
            )
            .unwrap();
            let err = check_heldout_contamination(&train, Some(tmp.path()))
                .expect_err("unreadable slice must fail closed")
                .to_string();
            assert!(err.contains("a_think/reasoning.jsonl"), "{err}");
            assert!(err.contains("line 2"), "{err}");
        }
    }

    #[test]
    fn no_workspace_is_not_checked() {
        let (_tmp, train) = heldout_fixture(&[Q1], &[r#"{"prompt":"a"}"#]);
        let got = check_heldout_contamination(&train, None).expect("not checked");
        assert_eq!(
            got,
            HeldoutCheck {
                status: "not_checked",
                slices: 0,
                heldout_questions: 0,
                rows_checked: 0,
            }
        );
    }
}
