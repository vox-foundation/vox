//! Candle (pure Rust) Whisper backend: HF Hub weights + symphonia decode + Candle inference.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use byteorder::{ByteOrder, LittleEndian};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config, audio};
use hf_hub::{Repo, RepoType, api::sync::Api};
use tokenizers::Tokenizer;

use super::audio_io::pcm_decode_to_16k_mono;
use super::candle_engine::{DecodeTask, Decoder, WhisperModel, token_id};
use super::multilingual;

/// Environment variable: Hugging Face model id (default `openai/whisper-tiny.en`).
pub const ENV_MODEL: &str = "VOX_ORATIO_MODEL";
/// Environment variable: HF revision (see `default_revision_for_model` in this module).
pub const ENV_REVISION: &str = "VOX_ORATIO_REVISION";
/// Set to `1` to use CUDA device 0 (requires `vox-oratio` `cuda` feature).
pub const ENV_CUDA: &str = "VOX_ORATIO_CUDA";

struct Session {
    model_id: String,
    revision: String,
    config: Config,
    tokenizer: Tokenizer,
    mel_filters: Vec<f32>,
    whisper: Option<WhisperModel>,
    device: Device,
}

fn cache() -> &'static Mutex<Option<Session>> {
    static C: OnceLock<Mutex<Option<Session>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

fn pick_device() -> Result<Device> {
    if std::env::var(ENV_CUDA).ok().as_deref() == Some("1") {
        #[cfg(feature = "cuda")]
        {
            return Device::new_cuda(0).map_err(|e| anyhow::anyhow!("CUDA: {e}"));
        }
        #[cfg(not(feature = "cuda"))]
        {
            anyhow::bail!("{ENV_CUDA}=1 but vox-oratio was built without `cuda` feature");
        }
    }
    Ok(Device::Cpu)
}

fn default_revision_for_model(model_id: &str) -> &'static str {
    let m = model_id.to_ascii_lowercase();
    if m.contains("tiny.en") {
        "refs/pr/15"
    } else if m.contains("base.en") {
        "refs/pr/13"
    } else if m.contains("small.en") {
        "refs/pr/10"
    } else if m.contains("base") && !m.contains(".en") {
        "refs/pr/22"
    } else if m.contains("large-v2") {
        "refs/pr/57"
    } else if m.contains("openai/whisper-large")
        && !m.contains("v2")
        && !m.contains("v3")
        && !m.contains("turbo")
    {
        "refs/pr/36"
    } else {
        "main"
    }
}

fn model_is_multilingual(model_id: &str) -> bool {
    let l = model_id.to_ascii_lowercase();
    !(l.contains(".en") || l.contains("distil-medium.en"))
}

fn load_mel_filters(config: &Config) -> Result<Vec<f32>> {
    let mel_bytes: &[u8] = match config.num_mel_bins {
        80 => include_bytes!("melfilters.bytes").as_slice(),
        128 => include_bytes!("melfilters128.bytes").as_slice(),
        nmel => anyhow::bail!("unexpected num_mel_bins {nmel}"),
    };
    let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
    LittleEndian::read_f32_into(mel_bytes, &mut mel_filters);
    Ok(mel_filters)
}

fn download_artifacts(
    model_id: &str,
    revision: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let api = Api::new().context("hf-hub Api::new")?;
    let repo = api.repo(Repo::with_revision(
        model_id.to_string(),
        RepoType::Model,
        revision.to_string(),
    ));
    let config = repo.get("config.json").context("fetch config.json")?;
    let tokenizer = repo.get("tokenizer.json").context("fetch tokenizer.json")?;
    let weights = repo
        .get("model.safetensors")
        .context("fetch model.safetensors")?;
    Ok((config, tokenizer, weights))
}

fn load_session(model_id: &str, revision: &str) -> Result<Session> {
    let device = pick_device()?;
    #[cfg(feature = "cuda")]
    {
        if matches!(&device, Device::Cpu) {
            tracing::info!(
                target: "vox_oratio_gpu",
                event = "oratio_inference_cpu_default",
                cuda_env = ENV_CUDA,
                "Oratio Whisper: CPU inference (this build supports CUDA; set {}=1 for GPU device 0)",
                ENV_CUDA,
            );
        } else {
            tracing::info!(
                target: "vox_oratio_gpu",
                event = "oratio_inference_gpu",
                device = ?device,
                "Oratio Whisper: GPU inference",
            );
        }
    }
    let (config_path, tokenizer_path, weights_path) = download_artifacts(model_id, revision)?;
    let config: Config = serde_json::from_str(
        &std::fs::read_to_string(&config_path)
            .with_context(|| config_path.display().to_string())?,
    )
    .context("parse whisper config.json")?;
    let tokenizer =
        Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow::anyhow!("tokenizer: {e}"))?;
    let mel_filters = load_mel_filters(&config)?;
    #[allow(unsafe_code)]
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[weights_path], m::DTYPE, &device)
            .context("mmap safetensors")?
    };
    let inner = m::model::Whisper::load(&vb, config.clone()).context("Whisper::load")?;
    let whisper = WhisperModel::Normal(inner);
    Ok(Session {
        model_id: model_id.to_string(),
        revision: revision.to_string(),
        config,
        tokenizer,
        mel_filters,
        whisper: Some(whisper),
        device,
    })
}

fn ensure_session(model_id: &str, revision: &str) -> Result<()> {
    let mut g = cache()
        .lock()
        .map_err(|_| anyhow::anyhow!("oratio session mutex poisoned"))?;
    let need_new = g
        .as_ref()
        .map(|s| s.model_id != model_id || s.revision != revision)
        .unwrap_or(true);
    if need_new {
        *g = Some(load_session(model_id, revision)?);
    }
    Ok(())
}

/// JSON-friendly status for CLI / tools (weights path is HF cache, not printed).
pub fn candle_backend_status_json() -> serde_json::Value {
    let model = std::env::var(ENV_MODEL).unwrap_or_else(|_| "openai/whisper-tiny.en".to_string());
    let rev = std::env::var(ENV_REVISION)
        .unwrap_or_else(|_| default_revision_for_model(&model).to_string());
    let cuda_requested = std::env::var(ENV_CUDA).ok().as_deref() == Some("1");
    let cuda_feature = cfg!(feature = "cuda");
    let inference_note = if cuda_feature && !cuda_requested {
        Some(format!(
            "Default inference device is CPU; set {ENV_CUDA}=1 to use CUDA device 0 when available."
        ))
    } else {
        None
    };
    serde_json::json!({
        "backend": "candle-whisper",
        "model_env": ENV_MODEL,
        "revision_env": ENV_REVISION,
        "default_model": model,
        "default_revision": rev,
        "multilingual": model_is_multilingual(&model),
        "cuda_env": ENV_CUDA,
        "cuda_feature_enabled": cuda_feature,
        "cuda_requested_via_env": cuda_requested,
        "inference_note": inference_note,
    })
}

/// Transcribe an audio file using Candle Whisper (downloads weights on first use).
pub fn transcribe_audio_file(path: &Path) -> Result<String> {
    let model_id =
        std::env::var(ENV_MODEL).unwrap_or_else(|_| "openai/whisper-tiny.en".to_string());
    let revision = std::env::var(ENV_REVISION)
        .unwrap_or_else(|_| default_revision_for_model(&model_id).to_string());

    let pcm = pcm_decode_to_16k_mono(path)?;
    if pcm.is_empty() {
        anyhow::bail!("no audio samples decoded from {}", path.display());
    }

    ensure_session(&model_id, &revision)?;

    let mut g = cache()
        .lock()
        .map_err(|_| anyhow::anyhow!("oratio session mutex poisoned"))?;
    let sess = g.as_mut().expect("session");

    let mel = audio::pcm_to_mel(&sess.config, &pcm, &sess.mel_filters);
    let mel_len = mel.len();
    let mel_tensor = Tensor::from_vec(
        mel,
        (
            1usize,
            sess.config.num_mel_bins,
            mel_len / sess.config.num_mel_bins,
        ),
        &sess.device,
    )
    .context("mel tensor")?;

    let multilingual = model_is_multilingual(&model_id);
    let lang = std::env::var("VOX_ORATIO_LANGUAGE").ok();
    let mut whisper = sess
        .whisper
        .take()
        .ok_or_else(|| anyhow::anyhow!("Whisper model is busy; retry"))?;

    let language_token = match (multilingual, lang.as_deref()) {
        (true, None) => {
            let t = match multilingual::detect_language(&mut whisper, &sess.tokenizer, &mel_tensor)
            {
                Ok(t) => t,
                Err(e) => {
                    sess.whisper = Some(whisper);
                    return Err(anyhow::anyhow!("language detect: {e}"));
                }
            };
            Some(t)
        }
        (false, None) => None,
        (true, Some(language)) => match token_id(&sess.tokenizer, &format!("<|{language}|>")) {
            Ok(t) => Some(t),
            Err(_) => {
                sess.whisper = Some(whisper);
                anyhow::bail!("unsupported language code {language:?}");
            }
        },
        (false, Some(_)) => {
            sess.whisper = Some(whisper);
            anyhow::bail!("language override not allowed for English-only models");
        }
    };

    let seed: u64 = std::env::var("VOX_ORATIO_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(299_792_458);
    let verbose = std::env::var("VOX_ORATIO_VERBOSE").ok().as_deref() == Some("1");

    let task = match std::env::var("VOX_ORATIO_TASK").ok().as_deref() {
        Some("translate") => Some(DecodeTask::Translate),
        Some("transcribe") | None => Some(DecodeTask::Transcribe),
        Some(other) => {
            sess.whisper = Some(whisper);
            anyhow::bail!("VOX_ORATIO_TASK must be transcribe or translate, got {other}");
        }
    };

    let tokenizer_clone = sess.tokenizer.clone();

    let mut decoder = match Decoder::new(
        whisper,
        tokenizer_clone,
        seed,
        &sess.device,
        language_token,
        task,
        false,
        None,
        verbose,
    ) {
        Ok(d) => d,
        Err(e) => {
            *g = None;
            return Err(e.context("Whisper decoder init failed; session cleared — retry"));
        }
    };

    let text = match decoder.run_collected(&mel_tensor) {
        Ok(t) => t,
        Err(e) => {
            sess.whisper = Some(decoder.into_whisper_model());
            return Err(e.context("Whisper inference"));
        }
    };
    sess.whisper = Some(decoder.into_whisper_model());
    Ok(text)
}
