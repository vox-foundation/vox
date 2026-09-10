//! Hugging Face Hub downloads for `vox mens train --model <repo_id>`.
//!
//! [`download_model`] is the actual downloader: it prints a size disclosure
//! before the first weight byte is fetched and refuses outright when
//! [`NO_DOWNLOAD_ENV`] is set. Unlike `vox-speech`'s sherpa model resolution
//! (`sherpa_model_config`), there is no local-directory alternative for an
//! arbitrary training `repo_id` today, so the opt-out error says so honestly
//! instead of inventing one.

use std::path::PathBuf;

use hf_hub::{HFClient, split_id};

/// Env var name: when set to a truthy value, [`download_model`] refuses to
/// download and returns an error instead, so a user or CI job can guarantee
/// nothing large is fetched. Mirrors the naming and truthy/falsy parsing of
/// `vox_speech::backends::sherpa_model_config::VOX_ORATIO_SHERPA_NO_DOWNLOAD`,
/// under a name proper to this downloader.
pub const NO_DOWNLOAD_ENV: &str = "VOX_MENS_NO_DOWNLOAD";

/// True iff `raw` (the raw [`NO_DOWNLOAD_ENV`] value, if any) asks for
/// downloads to be refused. Pure function of the env value so tests never
/// need to mutate real process env for the parsing rules themselves. Empty,
/// `"0"`, and (case-insensitively) `"false"` are treated as "not set";
/// anything else is truthy. Matches
/// `sherpa_model_config::no_download_requested`'s parsing exactly.
fn no_download_requested(raw: Option<&str>) -> bool {
    matches!(raw.map(str::trim), Some(v) if !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"))
}

/// Refuse the download when [`NO_DOWNLOAD_ENV`] is set. Called before
/// `HFClient::new()` so an opted-out caller never touches the network at all.
fn ensure_download_allowed(repo_id: &str) -> anyhow::Result<()> {
    if no_download_requested(std::env::var(NO_DOWNLOAD_ENV).ok().as_deref()) {
        anyhow::bail!(
            "{NO_DOWNLOAD_ENV} is set: refusing to download training model `{repo_id}`. \
             There is no local-directory alternative for this downloader today."
        );
    }
    Ok(())
}

/// `repo_id` with any trailing `@<revision>` suffix removed.
fn repo_name_without_revision(repo_id: &str) -> &str {
    repo_id.split('@').next().unwrap_or(repo_id)
}

/// True iff `repo_id` names the same repo as [`super::DEFAULT_MODEL_ID`],
/// ignoring any `@<revision>` suffix on either side.
///
/// A prefix-before-`@` comparison is used rather than exact string equality:
/// `DEFAULT_MODEL_ID` is pinned to a specific commit SHA
/// (`Qwen/Qwen3-8B@<sha>`), but a caller may reasonably pass just
/// `"Qwen/Qwen3-8B"` (no revision) and still mean the documented default
/// model. `DEFAULT_MODEL_APPROX_BYTES`'s arithmetic is revision-independent
/// (same architecture, same bf16 safetensors format), so treating those two
/// spellings as equivalent for size-disclosure purposes is honest; it would
/// not be honest to do the same for a different repo entirely.
fn is_default_model_repo(repo_id: &str) -> bool {
    repo_name_without_revision(repo_id) == repo_name_without_revision(super::DEFAULT_MODEL_ID)
}

/// Compose the pre-download disclosure line printed before the first weight
/// byte is fetched (i.e. before `repo.download_file("config.json")` in
/// [`download_model`]). Pure function of already-known inputs, so it is
/// testable without any `hf_hub` types:
///
/// - `is_default`: whether `repo_id` resolves to [`super::DEFAULT_MODEL_ID`]
///   (see [`is_default_model_repo`]) — when true, the documented approximate
///   size is stated.
/// - `safetensors_count`: for any other repo, the size is not knowable
///   without a network call this code must not make just to report a
///   number; instead this states the number of `*.safetensors` entries
///   already present in `info.siblings` (from the `repo.info()` call that
///   already ran), which is honest, checkable information.
fn download_size_notice(repo_id: &str, is_default: bool, safetensors_count: usize) -> String {
    if is_default {
        format!(
            "vox-populi: downloading training model `{repo_id}` — this is the default VoxMens \
             base model; approximate size: {}",
            super::default_model_approx_size_human()
        )
    } else {
        format!(
            "vox-populi: downloading training model `{repo_id}`; exact size is not known ahead \
             of time (not the documented default model). The repo manifest lists \
             {safetensors_count} *.safetensors file(s) that will be fetched."
        )
    }
}

/// Shape of `model.safetensors.index.json` we care about — only
/// `weight_map` (tensor key -> shard filename) is used; `metadata` is
/// ignored.
#[derive(serde::Deserialize)]
struct SafetensorsIndex {
    weight_map: std::collections::HashMap<String, String>,
}

/// Given the parsed `weight_map` from `model.safetensors.index.json`
/// (tensor key -> shard filename), return the set of shard filenames that
/// carry at least one text-tower tensor. Vision/MTP-only shards are
/// excluded. Pure, no I/O — classification is delegated to
/// [`vox_hf_layout::is_vision_or_mtp_key`] rather than reimplemented here.
fn required_shard_filenames(
    weight_map: &std::collections::HashMap<String, String>,
) -> std::collections::HashSet<String> {
    weight_map
        .iter()
        .filter(|(key, _)| !vox_hf_layout::is_vision_or_mtp_key(key))
        .map(|(_, shard)| shard.clone())
        .collect()
}

fn normalize_hf_token_env() {
    let token_resolved = vox_secrets::resolve_secret(vox_secrets::SecretId::HuggingFaceToken);
    let token = token_resolved.expose();

    if let Some(token) = token {
        // hf-hub defaults to HF_TOKEN then HUGGING_FACE_HUB_TOKEN.
        // We set both to ensure the crate finds it regardless of its internal priority.
        #[allow(unsafe_code)]
        // SAFETY: Called sequentially before spawning HF requests.
        unsafe {
            std::env::set_var("HF_TOKEN", token);
            std::env::set_var("HUGGING_FACE_HUB_TOKEN", token);
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
    pub tokenizer_config: Option<PathBuf>,
    pub chat_template: Option<PathBuf>,
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
    ensure_download_allowed(repo_id)?;
    normalize_hf_token_env();
    let client = HFClient::new().map_err(|e| anyhow::anyhow!("hf-hub HFClient::new: {e}"))?;
    let (owner, name) = split_id(repo_id);
    let repo = client.model(owner, name);
    let info = repo
        .info()
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("hf-hub repo info for {repo_id}: {e}"))?;
    // 1.0 makes `siblings` optional; an absent listing is indistinguishable from
    // an empty one at the type level, so name the difference here rather than
    // silently reporting "no safetensors" for a repo we simply failed to list.
    let siblings = info
        .siblings
        .ok_or_else(|| anyhow::anyhow!("repo {repo_id} info returned no file listing"))?;

    // Disclosure before the first weight byte moves: `repo.info()` above
    // already fetched the manifest, so counting `*.safetensors` siblings
    // here adds no extra network call. Uses the already-unwrapped `siblings`
    // (not `info.siblings`, which `ok_or_else` above moved out of `info`).
    let safetensors_count = siblings
        .iter()
        .filter(|s| s.rfilename.ends_with(".safetensors"))
        .count();
    eprintln!(
        "{}",
        download_size_notice(repo_id, is_default_model_repo(repo_id), safetensors_count)
    );

    let config = repo
        .download_file()
        .filename("config.json")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("download config.json: {e}"))?;

    let cache_dir = config
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut tokenizer = None::<PathBuf>;
    for name in ["tokenizer.json", "tokenizer.model"] {
        if siblings.iter().any(|s| s.rfilename == name) {
            tokenizer = Some(
                repo.download_file()
                    .filename(name)
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("download {name}: {e}"))?,
            );
            break;
        }
    }

    let mut tokenizer_config = None::<PathBuf>;
    if siblings
        .iter()
        .any(|s| s.rfilename == "tokenizer_config.json")
    {
        tokenizer_config = Some(
            repo.download_file()
                .filename("tokenizer_config.json")
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("download tokenizer_config.json: {e}"))?,
        );
    }

    let mut chat_template = None::<PathBuf>;
    if siblings
        .iter()
        .any(|s| s.rfilename == "chat_template.jinja")
    {
        chat_template = Some(
            repo.download_file()
                .filename("chat_template.jinja")
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("download chat_template.jinja: {e}"))?,
        );
    }

    let mut weight_names: Vec<&str> = siblings
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

    // If the shard index is present, use it to select only the shards
    // carrying text-tower tensors (see `required_shard_filenames`), rather
    // than downloading every `*.safetensors` sibling (which also pulls
    // vision-tower and MTP-head shards a text-only trainer never needs).
    // Intersected with the extension-based `weight_names` above as a safety
    // net: a filename the index names is never downloaded unless it is also
    // a confirmed real `*.safetensors` sibling. Repos without an index
    // (e.g. unsharded single-file models) fall back to the existing,
    // unchanged extension-based enumeration.
    const SAFETENSORS_INDEX_FILENAME: &str = "model.safetensors.index.json";
    if siblings
        .iter()
        .any(|s| s.rfilename == SAFETENSORS_INDEX_FILENAME)
    {
        let index_path = repo
            .download_file()
            .filename(SAFETENSORS_INDEX_FILENAME)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("download {SAFETENSORS_INDEX_FILENAME}: {e}"))?;
        let index_json = std::fs::read_to_string(&index_path)
            .map_err(|e| anyhow::anyhow!("read {SAFETENSORS_INDEX_FILENAME}: {e}"))?;
        let index: SafetensorsIndex = serde_json::from_str(&index_json)
            .map_err(|e| anyhow::anyhow!("parse {SAFETENSORS_INDEX_FILENAME}: {e}"))?;
        let required = required_shard_filenames(&index.weight_map);
        weight_names.retain(|n| required.contains(*n));
        if weight_names.is_empty() {
            anyhow::bail!(
                "repo {repo_id} index {SAFETENSORS_INDEX_FILENAME} names no shard that is \
                 also a *.safetensors sibling"
            );
        }
    }

    let mut weights = Vec::with_capacity(weight_names.len());
    for w in weight_names {
        let p = repo
            .download_file()
            .filename(w)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("download {w}: {e}"))?;
        weights.push(p);
    }

    Ok(DownloadedModelFiles {
        cache_dir,
        config,
        weights,
        tokenizer,
        tokenizer_config,
        chat_template,
    })
}

/// Block on [`download_model`] using a dedicated runtime (sync training / CLI entrypoints).
pub fn download_model_blocking(repo_id: &str) -> anyhow::Result<DownloadedModelFiles> {
    let repo_id = repo_id.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("tokio runtime init failed: {e}"))
            .and_then(|rt| rt.block_on(download_model(&repo_id)));
        let _ = tx.send(result);
    });
    rx.recv()
        .map_err(|_| anyhow::anyhow!("HF download thread exited without sending result"))?
}

#[cfg(all(test, feature = "mens-hf-hub"))]
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
            vox_secrets::resolve_secret(vox_secrets::SecretId::HuggingFaceToken)
                .expose()
                .expect("hub token"),
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
        assert_eq!(
            vox_secrets::resolve_secret(vox_secrets::SecretId::HuggingFaceToken)
                .expose()
                .expect("hf token"),
            "from-hub-only"
        );
        unsafe {
            std::env::remove_var("HF_TOKEN");
            std::env::remove_var("HUGGING_FACE_HUB_TOKEN");
        }
    }

    #[test]
    fn no_download_requested_treats_blank_zero_and_false_as_unset() {
        assert!(!super::no_download_requested(None));
        assert!(!super::no_download_requested(Some("")));
        assert!(!super::no_download_requested(Some("   ")));
        assert!(!super::no_download_requested(Some("0")));
        assert!(!super::no_download_requested(Some("false")));
        assert!(!super::no_download_requested(Some("FALSE")));
        assert!(super::no_download_requested(Some("1")));
        assert!(super::no_download_requested(Some("true")));
        assert!(super::no_download_requested(Some("yes")));
    }

    #[test]
    fn is_default_model_repo_matches_with_and_without_revision() {
        // Exact match, including the pinned revision.
        assert!(super::is_default_model_repo(super::super::DEFAULT_MODEL_ID));
        // Bare repo name, no revision — still the default model.
        assert!(super::is_default_model_repo("Qwen/Qwen3-8B"));
        // A different revision of the same repo is still the default model
        // for size-disclosure purposes (see the doc comment on
        // `is_default_model_repo`).
        assert!(super::is_default_model_repo(
            "Qwen/Qwen3-8B@0000000000000000000000000000000000000000"
        ));
        // A different repo entirely is never the default model.
        assert!(!super::is_default_model_repo("Qwen/Qwen2.5-Coder-7B"));
        assert!(!super::is_default_model_repo(
            "some-org/unrelated-repo@deadbeef"
        ));
    }

    #[test]
    fn download_size_notice_states_documented_size_for_default_model() {
        let msg = super::download_size_notice("Qwen/Qwen3-8B", true, 0);
        assert!(
            msg.contains("Qwen/Qwen3-8B"),
            "must name the repo id: {msg}"
        );
        assert!(
            msg.contains(&super::super::default_model_approx_size_human()),
            "must state the documented approximate size: {msg}"
        );
    }

    #[test]
    fn download_size_notice_states_file_count_for_other_repos() {
        let msg = super::download_size_notice("some-org/some-model", false, 3);
        assert!(
            msg.contains("some-org/some-model"),
            "must name the repo id: {msg}"
        );
        assert!(
            msg.to_lowercase().contains("not known"),
            "must be honest that the size is unknown: {msg}"
        );
        assert!(
            msg.contains('3') && msg.contains("safetensors"),
            "must state the checkable safetensors file count: {msg}"
        );
    }

    #[test]
    fn download_selects_only_shards_named_in_index() {
        let weight_map: std::collections::HashMap<String, String> = [
            (
                "model.language_model.layers.0.self_attn.q_proj.weight".to_string(),
                "model-00001-of-00002.safetensors".to_string(),
            ),
            (
                "model.visual.blocks.0.attn.qkv.weight".to_string(),
                "model-00002-of-00002.safetensors".to_string(),
            ),
            (
                "mtp.fc.weight".to_string(),
                "model-00002-of-00002.safetensors".to_string(),
            ),
        ]
        .into_iter()
        .collect();

        let required = super::required_shard_filenames(&weight_map);

        assert!(
            required.contains("model-00001-of-00002.safetensors"),
            "must include the shard carrying a text-tower tensor: {required:?}"
        );
        assert!(
            !required.contains("model-00002-of-00002.safetensors"),
            "must exclude the vision/mtp-only shard: {required:?}"
        );
    }

    #[test]
    fn download_model_refuses_when_opted_out() {
        // `ensure_download_allowed` is the exact guard `download_model` runs
        // before `Api::new()`/`repo.get()` — calling it directly proves the
        // refusal path without needing to await `download_model` itself (and
        // so without holding this lock across an await point). Guards the
        // same env var the token-propagation tests above touch are not
        // shared with this one, but `set_var`/`remove_var` require `unsafe`
        // under the module-level allow and any concurrent test mutating
        // process env must be serialized — reuse `ENV_LOCK` rather than
        // introduce a second, unguarded lock.
        let _g = ENV_LOCK.lock().expect("env lock");
        unsafe {
            std::env::set_var(super::NO_DOWNLOAD_ENV, "1");
        }
        let err = super::ensure_download_allowed("Qwen/Qwen3-8B")
            .expect_err("must refuse, not attempt a download");
        let msg = err.to_string();
        assert!(
            msg.contains(super::NO_DOWNLOAD_ENV),
            "error must name the opt-out: {msg}"
        );
        assert!(
            msg.contains("Qwen/Qwen3-8B"),
            "error must name the refused repo id: {msg}"
        );
        unsafe {
            std::env::remove_var(super::NO_DOWNLOAD_ENV);
        }
    }
}
