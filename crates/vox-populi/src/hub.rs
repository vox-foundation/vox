//! Hugging Face Hub downloads for `vox populi train --model <repo_id>`.

use std::path::PathBuf;

use hf_hub::api::tokio::Api;

/// Resolved local paths after downloading a model repo snapshot.
#[derive(Debug, Clone)]
pub struct DownloadedModelFiles {
    /// Directory containing the resolved snapshot (parent of `config.json` when present).
    pub cache_dir: PathBuf,
    pub config: PathBuf,
    pub weights: Vec<PathBuf>,
    pub tokenizer: Option<PathBuf>,
}

impl DownloadedModelFiles {
    /// True if at least one weight file uses the SafeTensors format.
    #[must_use]
    pub fn is_safetensors(&self) -> bool {
        self.weights.iter().any(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("safetensors"))
        })
    }
}

/// Download `config.json`, tokenizer files (if listed), and all `*.safetensors` shards.
pub async fn download_model(repo_id: &str) -> anyhow::Result<DownloadedModelFiles> {
    let api = Api::new().map_err(|e| anyhow::anyhow!("hf-hub Api::new: {e}"))?;
    let repo = api.model(repo_id.to_string());
    let info = repo
        .info()
        .await
        .map_err(|e| anyhow::anyhow!("hf-hub repo info for {repo_id}: {e}"))?;

    let config = repo
        .get("config.json")
        .await
        .map_err(|e| anyhow::anyhow!("download config.json: {e}"))?;

    let cache_dir = config
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut tokenizer = None::<PathBuf>;
    for name in ["tokenizer.json", "tokenizer.model"] {
        if info.siblings.iter().any(|s| s.rfilename == name) {
            tokenizer = Some(
                repo.get(name)
                    .await
                    .map_err(|e| anyhow::anyhow!("download {name}: {e}"))?,
            );
            break;
        }
    }

    let mut weight_names: Vec<&str> = info
        .siblings
        .iter()
        .map(|s| s.rfilename.as_str())
        .filter(|n| n.ends_with(".safetensors"))
        .collect();
    weight_names.sort_unstable();
    if weight_names.is_empty() {
        anyhow::bail!(
            "repo {repo_id} has no *.safetensors files in the Hub manifest; need a safetensors-based model"
        );
    }

    let mut weights = Vec::with_capacity(weight_names.len());
    for w in weight_names {
        let p = repo
            .get(w)
            .await
            .map_err(|e| anyhow::anyhow!("download {w}: {e}"))?;
        weights.push(p);
    }

    Ok(DownloadedModelFiles {
        cache_dir,
        config,
        weights,
        tokenizer,
    })
}
