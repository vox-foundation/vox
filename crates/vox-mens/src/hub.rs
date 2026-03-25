//! Hugging Face Hub downloads for `vox schola train --model <repo_id>`.

use std::path::PathBuf;

use hf_hub::api::tokio::Api;

fn normalize_hf_token_env() {
    let hf_token = std::env::var("HF_TOKEN").ok().filter(|v| !v.trim().is_empty());
    let hub_token = std::env::var("HUGGING_FACE_HUB_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty());
    // Keep both vars aligned so hf-hub auth works regardless of which one operators set.
    if let (Some(token), None) = (hf_token.as_deref(), hub_token.as_deref()) {
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("HUGGING_FACE_HUB_TOKEN", token);
        }
    } else if let (None, Some(token)) = (hf_token.as_deref(), hub_token.as_deref()) {
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("HF_TOKEN", token);
        }
    }
}

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
    normalize_hf_token_env();
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

#[cfg(all(test, feature = "hf-hub"))]
#[allow(unsafe_code)] // Serialized env mutation for token sync tests (Rust 2024 `set_var` safety).
mod tests {
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn hf_token_propagates_to_hugging_face_hub_token() {
        let _g = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::remove_var("HF_TOKEN");
            std::env::remove_var("HUGGING_FACE_HUB_TOKEN");
            std::env::set_var("HF_TOKEN", "from-hf-only");
        }
        super::normalize_hf_token_env();
        assert_eq!(
            std::env::var("HUGGING_FACE_HUB_TOKEN").expect("hub token"),
            "from-hf-only"
        );
        unsafe {
            std::env::remove_var("HF_TOKEN");
            std::env::remove_var("HUGGING_FACE_HUB_TOKEN");
        }
    }

    #[test]
    fn hugging_face_hub_token_propagates_to_hf_token() {
        let _g = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::remove_var("HF_TOKEN");
            std::env::remove_var("HUGGING_FACE_HUB_TOKEN");
            std::env::set_var("HUGGING_FACE_HUB_TOKEN", "from-hub-only");
        }
        super::normalize_hf_token_env();
        assert_eq!(std::env::var("HF_TOKEN").expect("hf token"), "from-hub-only");
        unsafe {
            std::env::remove_var("HF_TOKEN");
            std::env::remove_var("HUGGING_FACE_HUB_TOKEN");
        }
    }
}
