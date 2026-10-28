//! Corpus mix for Mens training (single prep path for CLI / schola / pipeline).
//!
//! Relative `--data-dir` / `--output-dir` / resume paths are anchored in
//! [`crate::training::contract::normalize_workspace_relative_path`] before mix and validation run.

use std::path::{Path, PathBuf};

use crate::corpus::{self, MixConfigSchema, MixRunOptions};

/// Relative path from workspace root to mix configuration.
pub const MIX_CONFIG_REL: &str = "mens/config/mix.yaml";

/// Whether two paths name the same location (canonicalized when both exist, lexical otherwise).
fn same_location(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Whether `data_dir` is the canonical corpus directory (`<workspace>/target/dogfood`).
///
/// The corpus mix only ever runs for the canonical directory; an explicit `--data-dir`
/// elsewhere is the user's data and is trained on as-is.
#[must_use]
pub fn is_canonical_data_dir(workspace_root: Option<&Path>, data_dir: &Path) -> bool {
    let base = workspace_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    same_location(
        &base.join(data_dir),
        &base.join(super::CANONICAL_TRAIN_DATA_DIR),
    )
}

/// `VOX_TRAIN_SKIP_CORPUS_MIX=1|true` skips mix entirely (operators / tests).
#[must_use]
pub fn corpus_mix_skip_from_env() -> bool {
    std::env::var("VOX_TRAIN_SKIP_CORPUS_MIX")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Mix-skip decision: the `--fast-corpus` flag OR a user-set `VOX_TRAIN_SKIP_CORPUS_MIX`.
/// Callers must never clear the env var — a user's opt-out always holds.
#[must_use]
pub fn corpus_mix_skipped(fast_corpus_flag: bool) -> bool {
    fast_corpus_flag || corpus_mix_skip_from_env()
}

/// Resolve mix YAML path: prefer workspace root; fall back to `cwd/mens/config/mix.yaml` (pipeline legacy).
pub fn resolve_mix_config_path(workspace_root: Option<&Path>) -> PathBuf {
    if let Some(root) = workspace_root {
        root.join(MIX_CONFIG_REL)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(MIX_CONFIG_REL)
    }
}

/// Run corpus mix (when not skipped) and return the mix **output** path for
/// [`super::preflight::validate_train_preflight`].
///
/// File roles: `data_dir/train.jsonl` is the pairs file (or the user's own data) and is
/// never written here; the mix writes only its configured `output:`. The mix runs only
/// for the canonical data dir ([`is_canonical_data_dir`]); for any other `--data-dir`
/// this returns `None` and training reads that directory's `train.jsonl` untouched.
pub fn refresh_train_contract_override_from_mix(
    workspace_root: Option<&Path>,
    data_dir: &Path,
    skip_mix: bool,
    explicit_mix_yaml: Option<&Path>,
) -> anyhow::Result<Option<PathBuf>> {
    if skip_mix {
        return Ok(None);
    }
    if !is_canonical_data_dir(workspace_root, data_dir) {
        eprintln!(
            "  ⏭ --data-dir {} is not the canonical {}: training on its train.jsonl as-is (corpus mix not run). Omit --data-dir to train on the corpus mix.",
            data_dir.display(),
            super::CANONICAL_TRAIN_DATA_DIR
        );
        return Ok(None);
    }
    let mix_yaml = explicit_mix_yaml
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolve_mix_config_path(workspace_root));
    if !mix_yaml.is_file() {
        return Ok(None);
    }
    let path_base_for_mix = workspace_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if let Err(e) = corpus::run_mix_with_options(
        &mix_yaml,
        Some(path_base_for_mix.as_path()),
        MixRunOptions::default(),
    ) {
        tracing::warn!(error = %e, mix_yaml = %mix_yaml.display(), "corpus mix failed; continuing with existing train files");
        return Ok(None);
    }
    let Ok(mix_cfg) = MixConfigSchema::load(&mix_yaml) else {
        return Ok(None);
    };
    let mix_output = path_base_for_mix.join(&mix_cfg.output);
    Ok(mix_output.is_file().then_some(mix_output))
}

/// Re-materialize training JSONL if it disappeared after preflight (e.g. long HF download while `cargo clean`
/// removed `target/`, or the mixed file lived only under `target/`).
pub fn recover_train_input_path_after_prefetch(
    workspace_root: Option<&Path>,
    data_dir: &Path,
    mix_yaml: &Path,
    skip_mix: bool,
    previously_resolved: &Path,
) -> anyhow::Result<PathBuf> {
    if previously_resolved.is_file() {
        return Ok(previously_resolved.to_path_buf());
    }
    tracing::warn!(
        path = %previously_resolved.display(),
        "training JSONL missing before kernel load; attempting recovery from mix output or re-mix"
    );
    let primary = data_dir.join(super::preflight::PRIMARY_TRAIN_FILE);

    if !skip_mix && mix_yaml.is_file() && is_canonical_data_dir(workspace_root, data_dir) {
        if let Ok(cfg) = crate::corpus::mix::MixConfigSchema::load(mix_yaml) {
            let path_base = workspace_root
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            for src in &cfg.sources {
                let p = path_base.join(&src.path);
                if !p.exists() {
                    if src.path == "mens/data/mix_sources/docs.jsonl" {
                        eprintln!("  🔄 Generating missing components: docs.jsonl");
                        let c = crate::corpus::extract_docs::ExtractDocsConfig {
                            root: path_base.join("docs"),
                            ..Default::default()
                        };
                        if let Ok(pairs) = crate::corpus::extract_docs::walk_and_extract_docs(&c) {
                            let _ = crate::corpus::extract_docs::write_docs_to_jsonl(&pairs, &p);
                        }
                    } else if src.path == "mens/data/mix_sources/rust_source.jsonl" {
                        eprintln!("  🔄 Generating missing components: rust_source.jsonl");
                        let c = crate::corpus::extract_rs::ExtractRsConfig {
                            root: path_base.join("crates"),
                            ..Default::default()
                        };
                        if let Ok(pairs) = crate::corpus::extract_rs::walk_and_extract(&c) {
                            let _ = crate::corpus::extract_rs::write_to_jsonl(&pairs, &p);
                        }
                    }
                }
            }
        }
        match refresh_train_contract_override_from_mix(
            workspace_root,
            data_dir,
            false,
            Some(mix_yaml),
        ) {
            Ok(Some(p)) if p.is_file() => return Ok(p),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "recovery: corpus mix re-run failed"),
        }
    }

    if primary.is_file() {
        return Ok(primary);
    }

    anyhow::bail!(
        "Training data `{}` is missing before training. If `--data-dir` is under `target/`, avoid `cargo clean` between preflight and load, or use a data directory outside `target/`.",
        previously_resolved.display()
    )
}

/// Canonical DPO preference pair (`(chosen, rejected)`) with oracle receipts (`TRAIN-01`, `AUDIT-02`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChosenRejectedPairV2 {
    /// Schema discriminator (`"vox_dpo_pair_v2"`).
    pub schema_version: String,
    /// Target spoke.
    pub spoke: crate::training::contract::SpokeKind,
    /// Shared user prompt.
    pub prompt: String,
    /// Shared system prompt.
    pub system_prompt: String,
    /// Verified winning completion (`chosen_receipt.passed == true`).
    pub chosen: String,
    /// Verified failing or hard-negative completion (`rejected_receipt.passed == false`).
    pub rejected: String,
    /// Oracle receipt for `chosen`.
    pub chosen_receipt: crate::training::contract::OracleReceipt,
    /// Oracle receipt for `rejected`.
    pub rejected_receipt: crate::training::contract::OracleReceipt,
    /// Provenance metadata.
    pub provenance: crate::training::contract::RecordProvenance,
    /// 32-hex `xxh3_128` digest of `spoke|prompt|chosen|rejected`.
    pub content_digest: String,
}

impl ChosenRejectedPairV2 {
    /// Canonical schema discriminator.
    pub const SCHEMA_VERSION: &'static str = "vox_dpo_pair_v2";

    /// Construct a new DPO preference pair with deterministic `xxh3_128` digest.
    #[must_use]
    pub fn new(
        spoke: crate::training::contract::SpokeKind,
        prompt: impl Into<String>,
        system_prompt: impl Into<String>,
        chosen: impl Into<String>,
        rejected: impl Into<String>,
        chosen_receipt: crate::training::contract::OracleReceipt,
        rejected_receipt: crate::training::contract::OracleReceipt,
        provenance: crate::training::contract::RecordProvenance,
    ) -> Self {
        let prompt = prompt.into();
        let system_prompt = system_prompt.into();
        let chosen = chosen.into();
        let rejected = rejected.into();
        let payload = format!(
            "{}\n---\n{}\n---\n{}\n---\n{}",
            spoke.as_str(),
            prompt.trim(),
            chosen.trim(),
            rejected.trim()
        );
        let content_digest = format!("{:032x}", xxhash_rust::xxh3::xxh3_128(payload.as_bytes()));
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            spoke,
            prompt,
            system_prompt,
            chosen,
            rejected,
            chosen_receipt,
            rejected_receipt,
            provenance,
            content_digest,
        }
    }

    /// Verify that `chosen` passed its spoke oracle, `rejected` failed its spoke oracle,
    /// and the digest is intact.
    #[must_use]
    pub fn verify_pair_integrity(&self) -> bool {
        let payload = format!(
            "{}\n---\n{}\n---\n{}\n---\n{}",
            self.spoke.as_str(),
            self.prompt.trim(),
            self.chosen.trim(),
            self.rejected.trim()
        );
        let expected = format!("{:032x}", xxhash_rust::xxh3::xxh3_128(payload.as_bytes()));
        self.schema_version == Self::SCHEMA_VERSION
            && self.chosen_receipt.passed
            && !self.rejected_receipt.passed
            && self.chosen.trim() != self.rejected.trim()
            && self.content_digest == expected
    }
}

/// Token-bounded training micro-batch bridging a `TrustyDatasetSnapshot` into SFT + DPO steps (`TRAIN-01`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SftDpoBatch {
    /// Zero-based batch index.
    pub batch_index: usize,
    /// Source dataset snapshot Trusty URI (`trusty://vox-dataset/...`).
    pub snapshot_uri: String,
    /// SFT positive records in this batch.
    pub sft_records: Vec<crate::training::contract::TrainingRecordV2>,
    /// DPO preference pairs in this batch.
    pub dpo_pairs: Vec<ChosenRejectedPairV2>,
    /// Exact accounted token budget for this batch.
    pub total_tokens: usize,
}

/// Deterministic token estimator (~4 UTF-8 bytes per subword token, minimum 1 for non-empty strings) (`TRAIN-01`).
#[must_use]
pub fn estimate_token_count(text: &str) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        0
    } else {
        trimmed.len().div_ceil(4).max(1)
    }
}

/// Stream a `TrustyDatasetSnapshot`'s SFT records and DPO preference pairs into deterministic,
/// token-budgeted `SftDpoBatch`es (`TRAIN-01`).
#[must_use]
pub fn stream_snapshot_batches(
    snapshot_uri: &str,
    sft_records: &[crate::training::contract::TrainingRecordV2],
    dpo_pairs: &[ChosenRejectedPairV2],
    max_batch_tokens: usize,
) -> Vec<SftDpoBatch> {
    let cap = max_batch_tokens.max(64);
    let mut batches = Vec::new();
    let mut cur_sft = Vec::new();
    let mut cur_dpo = Vec::new();
    let mut cur_tokens = 0usize;

    for rec in sft_records {
        if !rec.verify_integrity() {
            continue;
        }
        let rec_tokens = estimate_token_count(&rec.system_context)
            + estimate_token_count(&rec.prompt)
            + estimate_token_count(&rec.response);
        if (!cur_sft.is_empty() || !cur_dpo.is_empty()) && cur_tokens + rec_tokens > cap {
            batches.push(SftDpoBatch {
                batch_index: batches.len(),
                snapshot_uri: snapshot_uri.to_string(),
                sft_records: std::mem::take(&mut cur_sft),
                dpo_pairs: std::mem::take(&mut cur_dpo),
                total_tokens: cur_tokens,
            });
            cur_tokens = 0;
        }
        cur_tokens += rec_tokens;
        cur_sft.push(rec.clone());
    }

    for pair in dpo_pairs {
        if !pair.verify_pair_integrity() {
            continue;
        }
        let pair_tokens = estimate_token_count(&pair.system_prompt)
            + estimate_token_count(&pair.prompt)
            + estimate_token_count(&pair.chosen)
            + estimate_token_count(&pair.rejected);
        if (!cur_sft.is_empty() || !cur_dpo.is_empty()) && cur_tokens + pair_tokens > cap {
            batches.push(SftDpoBatch {
                batch_index: batches.len(),
                snapshot_uri: snapshot_uri.to_string(),
                sft_records: std::mem::take(&mut cur_sft),
                dpo_pairs: std::mem::take(&mut cur_dpo),
                total_tokens: cur_tokens,
            });
            cur_tokens = 0;
        }
        cur_tokens += pair_tokens;
        cur_dpo.push(pair.clone());
    }

    if !cur_sft.is_empty() || !cur_dpo.is_empty() {
        batches.push(SftDpoBatch {
            batch_index: batches.len(),
            snapshot_uri: snapshot_uri.to_string(),
            sft_records: cur_sft,
            dpo_pairs: cur_dpo,
            total_tokens: cur_tokens,
        });
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Temp workspace with a canonical `target/dogfood/train.jsonl` pairs file and a
    /// `mix.yaml` whose primary source is that pairs file.
    fn temp_workspace() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ws = tmp.path();
        std::fs::create_dir_all(ws.join("target/dogfood")).expect("dirs");
        std::fs::create_dir_all(ws.join("mens/config")).expect("mix dir");
        std::fs::write(
            ws.join("target/dogfood/train.jsonl"),
            "{\"prompt\":\"p1\",\"response\":\"r1\"}\n{\"prompt\":\"p2\",\"response\":\"r2\"}\n",
        )
        .expect("pairs");
        let mut f = std::fs::File::create(ws.join(MIX_CONFIG_REL)).expect("mix");
        writeln!(
            f,
            "output: target/dogfood/train_mixed.jsonl\nsources:\n  - path: target/dogfood/train.jsonl\n    weight: 2.0"
        )
        .expect("write");
        tmp
    }

    #[test]
    fn explicit_data_dir_is_never_modified_and_wins_over_canonical_train() {
        let tmp = temp_workspace();
        let ws = tmp.path();
        let user = ws.join("my_data");
        std::fs::create_dir_all(&user).expect("user dir");
        let user_train = user.join("train.jsonl");
        let original = "{\"prompt\":\"mine\",\"response\":\"only\"}\n";
        std::fs::write(&user_train, original).expect("user train");

        let over = refresh_train_contract_override_from_mix(Some(ws), &user, false, None)
            .expect("refresh");
        assert_eq!(
            over, None,
            "explicit data dir must not be routed to a mix output"
        );
        assert!(
            !ws.join("target/dogfood/train_mixed.jsonl").exists(),
            "mix must not run for an explicit data dir"
        );
        let resolved =
            crate::training::preflight::validate_train_preflight(&user, over.as_deref(), Some(ws))
                .expect("preflight");
        assert_eq!(resolved.path, user_train, "explicit --data-dir must win");
        assert_eq!(std::fs::read_to_string(&user_train).unwrap(), original);

        // Same with the skip flag (the `--fast-corpus` / env opt-out path).
        let over =
            refresh_train_contract_override_from_mix(Some(ws), &user, true, None).expect("refresh");
        assert_eq!(over, None);
        assert_eq!(std::fs::read_to_string(&user_train).unwrap(), original);
    }

    #[test]
    fn mix_output_never_becomes_mix_input() {
        let tmp = temp_workspace();
        let ws = tmp.path();
        let data = ws.join("target/dogfood");
        let pairs = data.join("train.jsonl");
        let pairs_before = std::fs::read(&pairs).unwrap();

        let first = refresh_train_contract_override_from_mix(Some(ws), &data, false, None)
            .expect("refresh")
            .expect("mix output");
        assert_eq!(first, ws.join("target/dogfood/train_mixed.jsonl"));
        let mixed_first = std::fs::read(&first).unwrap();
        assert_eq!(
            std::fs::read(&pairs).unwrap(),
            pairs_before,
            "pairs file untouched"
        );

        // A second run must mix the same pairs again, not the previous output.
        std::fs::remove_file(ws.join("target/dogfood/train_mixed.mix_report.json")).unwrap();
        let second = refresh_train_contract_override_from_mix(Some(ws), &data, false, None)
            .expect("refresh")
            .expect("mix output");
        assert_eq!(std::fs::read(&second).unwrap(), mixed_first);
        assert_eq!(
            std::fs::read(&pairs).unwrap(),
            pairs_before,
            "pairs file untouched"
        );

        let resolved =
            crate::training::preflight::validate_train_preflight(&data, Some(&second), Some(ws))
                .expect("preflight");
        assert_eq!(
            resolved.path, second,
            "trainer reads the mix output when the mix ran"
        );
    }

    #[test]
    fn canonical_data_dir_detection() {
        let tmp = temp_workspace();
        let ws = tmp.path();
        assert!(is_canonical_data_dir(Some(ws), Path::new("target/dogfood")));
        assert!(is_canonical_data_dir(Some(ws), &ws.join("target/dogfood")));
        assert!(!is_canonical_data_dir(Some(ws), &ws.join("other")));
        assert!(!is_canonical_data_dir(Some(ws), Path::new("/tmp/X")));
    }

    #[test]
    fn user_set_skip_env_is_honored_without_the_flag() {
        // SAFETY: test-only; no other test in this crate reads this variable concurrently.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_TRAIN_SKIP_CORPUS_MIX", "1");
        }
        let with_env = corpus_mix_skipped(false);
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_TRAIN_SKIP_CORPUS_MIX");
        }
        assert!(with_env, "env opt-out must hold even without --fast-corpus");
        assert!(corpus_mix_skipped(true));
        assert!(!corpus_mix_skipped(false));
    }

    #[test]
    fn shipped_workspace_contract_does_not_override_data_dir() {
        let ws = crate::training::contract::find_workspace_root().unwrap();
        assert_eq!(
            crate::training::preflight::load_contract(&ws).unwrap(),
            None,
            "a workspace train_path would silently replace every --data-dir/train.jsonl"
        );
    }

    #[test]
    fn test_mix_config_paths_valid() {
        let ws = crate::training::contract::find_workspace_root().unwrap();
        let mix_yaml = resolve_mix_config_path(Some(&ws));
        let config = crate::corpus::MixConfigSchema::load(&mix_yaml).unwrap();
        for source in config.sources {
            if !source.optional {
                assert!(
                    ws.join(&source.path).is_file()
                        // Pairs output, generated by `vox mens corpus pairs`.
                        || source.path == "target/dogfood/train.jsonl",
                    "Source path {} must exist if not optional",
                    source.path
                );
            }
        }
    }

    #[test]
    fn stream_snapshot_batches_partitions_sft_and_dpo_within_token_budget() {
        use crate::training::contract::{
            OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2,
        };

        let sft = vec![TrainingRecordV2::new(
            SpokeKind::VoxLang,
            "Write a Vox function `id`",
            "sys",
            "fn id(x: int) to int { return x }",
            OracleReceipt {
                spoke: SpokeKind::VoxLang,
                passed: true,
                stage: "ok".to_string(),
                diagnostics: Vec::new(),
                exit_code: Some(0),
            },
            RecordProvenance {
                origin: "test".to_string(),
                source_id: "sft1".to_string(),
                task_family: "vox_fn".to_string(),
                difficulty: 2,
            },
        )];
        let dpo = vec![ChosenRejectedPairV2::new(
            SpokeKind::VoxLang,
            "Write a Vox function `id`",
            "sys",
            "fn id(x: int) to int { return x }",
            "@table fn id(x: int) -> int { x }",
            OracleReceipt {
                spoke: SpokeKind::VoxLang,
                passed: true,
                stage: "ok".to_string(),
                diagnostics: Vec::new(),
                exit_code: Some(0),
            },
            OracleReceipt {
                spoke: SpokeKind::VoxLang,
                passed: false,
                stage: "parse".to_string(),
                diagnostics: vec!["retired syntax".to_string()],
                exit_code: Some(1),
            },
            RecordProvenance {
                origin: "test".to_string(),
                source_id: "dpo1".to_string(),
                task_family: "vox_fn".to_string(),
                difficulty: 4,
            },
        )];
        assert!(dpo[0].verify_pair_integrity());

        let batches = stream_snapshot_batches(
            "trusty://vox-dataset/0123456789abcdef0123456789abcdef",
            &sft,
            &dpo,
            128,
        );
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].sft_records.len(), 1);
        assert_eq!(batches[0].dpo_pairs.len(), 1);
        assert!(batches[0].total_tokens > 0 && batches[0].total_tokens <= 128);
    }
}
