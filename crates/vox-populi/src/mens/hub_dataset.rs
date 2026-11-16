//! Staging and dry-run/push publishing for Hugging Face dataset repos (`338-09`).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Dataset category controlling Hub repo naming (`eval` vs `data`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetKind {
    Eval,
    Data,
}

/// One local file included inside a staged dataset repository bundle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatasetSource {
    pub local_path: PathBuf,
    pub repo_rel_path: String,
    #[serde(default)]
    pub license: Option<String>,
}

/// Specification for staging or publishing a Hub dataset repo bundle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatasetBundleSpec {
    pub kind: DatasetKind,
    pub slug: String,
    pub namespace: String,
    pub description: String,
    pub sources: Vec<DatasetSource>,
}

/// Staged bundle output with per-file BLAKE3 digests.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StagedDatasetBundle {
    pub repo_id: String,
    pub stage_dir: PathBuf,
    pub files: Vec<(String, String, u64)>,
}

const FORBIDDEN_SECRET_PATTERNS: &[&str] = &["hf_", "sk-", "ghp_", "AIza", "-----BEGIN "];

fn check_no_secrets_or_heldout_leak(path: &Path, kind: DatasetKind) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading dataset source {}", path.display()))?;
    for pat in FORBIDDEN_SECRET_PATTERNS {
        if text.contains(pat) {
            bail!(
                "refusing dataset source {}: matched secret pattern `{pat}`",
                path.display()
            );
        }
    }
    if kind == DatasetKind::Data
        && (text.contains("\"training_eligible\":false")
            || text.contains("\"training_eligible\": false"))
    {
        bail!(
            "refusing training dataset source {}: contains held-out (`training_eligible: false`) rows",
            path.display()
        );
    }
    Ok(())
}

/// Resolve repo id, validate sources, and stage files + README.md + dataset_manifest.json into `stage_dir`.
pub fn stage_dataset_bundle(
    spec: &DatasetBundleSpec,
    stage_dir: &Path,
) -> Result<StagedDatasetBundle> {
    if spec.sources.is_empty() {
        bail!("dataset bundle `{}` has zero sources", spec.slug);
    }
    let repo_id = match spec.kind {
        DatasetKind::Eval => {
            crate::mens::hub_repo::eval_dataset_repo_id(&spec.namespace, &spec.slug)?
        }
        DatasetKind::Data => {
            crate::mens::hub_repo::data_dataset_repo_id(&spec.namespace, &spec.slug)?
        }
    };

    std::fs::create_dir_all(stage_dir)
        .with_context(|| format!("creating stage dir {}", stage_dir.display()))?;

    let mut staged_files = Vec::new();
    for src in &spec.sources {
        if !src.local_path.is_file() {
            bail!("missing dataset source {}", src.local_path.display());
        }
        check_no_secrets_or_heldout_leak(&src.local_path, spec.kind)?;
        let dst = stage_dir.join(&src.repo_rel_path);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src.local_path, &dst)?;
        let bytes = std::fs::read(&dst)?;
        let digest = vox_crypto::hash_fast_hex(&bytes);
        staged_files.push((src.repo_rel_path.clone(), digest, bytes.len() as u64));
    }

    let readme = format!(
        "---\nlicense: apache-2.0\ntags:\n- vox\n- vox-mens\n- {}\n---\n# {}\n\n{}\n",
        match spec.kind {
            DatasetKind::Eval => "eval",
            DatasetKind::Data => "training-data",
        },
        repo_id,
        spec.description
    );
    let readme_path = stage_dir.join("README.md");
    std::fs::write(&readme_path, readme.as_bytes())?;
    staged_files.push((
        "README.md".to_string(),
        vox_crypto::hash_fast_hex(readme.as_bytes()),
        readme.len() as u64,
    ));

    let manifest = serde_json::json!({
        "schema": "vox_mens_dataset_bundle_v1",
        "repo_id": repo_id,
        "kind": spec.kind,
        "slug": spec.slug,
        "files": staged_files.iter().map(|(p, h, n)| serde_json::json!({
            "path": p,
            "blake3": h,
            "size_bytes": n,
        })).collect::<Vec<_>>(),
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    std::fs::write(stage_dir.join("dataset_manifest.json"), &manifest_bytes)?;
    staged_files.push((
        "dataset_manifest.json".to_string(),
        vox_crypto::hash_fast_hex(&manifest_bytes),
        manifest_bytes.len() as u64,
    ));

    Ok(StagedDatasetBundle {
        repo_id,
        stage_dir: stage_dir.to_path_buf(),
        files: staged_files,
    })
}

/// Stage `spec` and push to HF Hub only when `push` is `true` (`VOX_MENS_HF_SYNC` respected).
pub async fn publish_dataset_bundle(
    spec: &DatasetBundleSpec,
    stage_dir: &Path,
    push: bool,
    private: bool,
) -> Result<StagedDatasetBundle> {
    let staged = stage_dataset_bundle(spec, stage_dir)?;
    if !push {
        return Ok(staged);
    }
    let _token = crate::mens::hub_sync::require_clavis_hf_token()?;
    let _ = private;
    // Upload intentionally delegated only when caller passes --push with Clavis token present.
    crate::mens::hub::upload_model_folder(
        &staged.repo_id,
        stage_dir,
        private,
        Some("Publish Mens dataset bundle"),
    )
    .await?;
    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_dataset_bundle_stages_files_and_refuses_secrets_or_heldout() {
        let dir = tempfile::tempdir().unwrap();
        let stage = dir.path().join("stage");
        let good = dir.path().join("good.jsonl");
        std::fs::write(
            &good,
            "{\"id\":\"1\",\"prompt\":\"ok\",\"training_eligible\":true}\n",
        )
        .unwrap();

        let spec = DatasetBundleSpec {
            kind: DatasetKind::Data,
            slug: "vox-lang".to_string(),
            namespace: "me".to_string(),
            description: "Curated Vox language examples".to_string(),
            sources: vec![DatasetSource {
                local_path: good,
                repo_rel_path: "data/train.jsonl".to_string(),
                license: Some("apache-2.0".to_string()),
            }],
        };
        let staged = stage_dataset_bundle(&spec, &stage).expect("stage good dataset");
        assert_eq!(staged.repo_id, "me/vox-mens-data-vox-lang");
        assert!(stage.join("README.md").is_file());
        assert!(stage.join("dataset_manifest.json").is_file());
        assert!(stage.join("data/train.jsonl").is_file());

        // Held-out rows in a Data bundle must be refused.
        let heldout = dir.path().join("heldout.jsonl");
        std::fs::write(
            &heldout,
            "{\"id\":\"2\",\"prompt\":\"leak\",\"training_eligible\":false}\n",
        )
        .unwrap();
        let bad_heldout = DatasetBundleSpec {
            kind: DatasetKind::Data,
            slug: "vox-lang".to_string(),
            namespace: "me".to_string(),
            description: "bad".to_string(),
            sources: vec![DatasetSource {
                local_path: heldout,
                repo_rel_path: "data/heldout.jsonl".to_string(),
                license: None,
            }],
        };
        let err = stage_dataset_bundle(&bad_heldout, &dir.path().join("s2"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("training_eligible"), "{err}");

        // Secret tokens must be refused in any dataset kind.
        let secret_file = dir.path().join("secret.jsonl");
        std::fs::write(&secret_file, "{\"token\":\"hf_1234567890\"}\n").unwrap();
        let bad_secret = DatasetBundleSpec {
            kind: DatasetKind::Eval,
            slug: "agent_episodes".to_string(),
            namespace: "me".to_string(),
            description: "bad secret".to_string(),
            sources: vec![DatasetSource {
                local_path: secret_file,
                repo_rel_path: "episodes.jsonl".to_string(),
                license: None,
            }],
        };
        let err2 = stage_dataset_bundle(&bad_secret, &dir.path().join("s3"))
            .unwrap_err()
            .to_string();
        assert!(err2.contains("secret"), "{err2}");
    }
}
