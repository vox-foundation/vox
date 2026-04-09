//! Sherpa-ONNX offline ASR backend — high-accuracy Whisper via C ONNX runtime.

#![cfg(feature = "stt-sherpa")]

use std::sync::Mutex;
use anyhow::{Context, Result};
use sherpa_rs::offline_recognizer::{OfflineRecognizer, OfflineRecognizerConfig};
use super::asr_backend::{AsrBackend, AsrOutput};
use super::sherpa_model_config::resolve_sherpa_model_paths;

/// Oratio Sherpa-ONNX backend. Thread-safe via Mutex; one session per process.
pub struct SherpaOnnxBackend {
    inner: Mutex<OfflineRecognizer>,
}

impl SherpaOnnxBackend {
    /// Initialize the backend (downloads model if needed).
    pub fn new() -> Result<Self> {
        let paths = resolve_sherpa_model_paths()?;
        let cfg = OfflineRecognizerConfig::new(
            paths.model_onnx.to_str().context("model path UTF-8")?,
            paths.tokens_txt.to_str().context("tokens path UTF-8")?,
        );
        let recognizer = OfflineRecognizer::new(cfg)
            .map_err(|e| anyhow::anyhow!("Sherpa-ONNX init: {e}"))?;
        tracing::info!(
            target: "vox_oratio_sherpa",
            event = "sherpa_backend_init",
            "Sherpa-ONNX backend initialized"
        );
        Ok(Self { inner: Mutex::new(recognizer) })
    }
}

impl AsrBackend for SherpaOnnxBackend {
    fn name(&self) -> &'static str { "sherpa-onnx" }

    fn transcribe_pcm(
        &self,
        pcm: &[f32],
        sample_rate: u32,
        _language: Option<&str>,
    ) -> Result<AsrOutput> {
        let mut rec = self.inner.lock()
            .map_err(|_| anyhow::anyhow!("SherpaOnnxBackend mutex poisoned"))?;

        // sherpa-rs expects f32 mono PCM at 16 kHz; resample if needed.
        let pcm_16k: std::borrow::Cow<[f32]> = if sample_rate == 16_000 {
            std::borrow::Cow::Borrowed(pcm)
        } else {
            std::borrow::Cow::Owned(resample_to_16k(pcm, sample_rate)?)
        };

        let stream = rec.create_stream()
            .map_err(|e| anyhow::anyhow!("create_stream: {e}"))?;
        stream.accept_waveform(16_000, &pcm_16k);
        rec.decode_stream(&stream)
            .map_err(|e| anyhow::anyhow!("decode_stream: {e}"))?;
        let result = stream.get_result();
        Ok(AsrOutput {
            raw_text: result.text.trim().to_string(),
            confidence: 0.85, // sherpa-rs offline does not expose per-token log-prob yet
            n_best: Vec::new(),
        })
    }
}

fn resample_to_16k(pcm: &[f32], from_hz: u32) -> Result<Vec<f32>> {
    use rubato::{FftFixedInOut, Resampler};
    let ratio = 16_000.0 / from_hz as f64;
    let chunk = 1024usize;
    let mut resampler = FftFixedInOut::<f32>::new(from_hz as usize, 16_000, chunk, 1)
        .map_err(|e| anyhow::anyhow!("rubato init: {e}"))?;
    let mut out = Vec::with_capacity((pcm.len() as f64 * ratio) as usize + chunk);
    let mut pos = 0usize;
    let in_chunk = resampler.input_frames_next();
    while pos + in_chunk <= pcm.len() {
        let frames = resampler.process(&[&pcm[pos..pos + in_chunk]], None)
            .map_err(|e| anyhow::anyhow!("rubato process: {e}"))?;
        out.extend_from_slice(&frames[0]);
        pos += in_chunk;
    }
    // Handle tail
    if pos < pcm.len() {
        let mut tail = pcm[pos..].to_vec();
        tail.resize(in_chunk, 0.0);
        let frames = resampler.process(&[&tail], None)
            .map_err(|e| anyhow::anyhow!("rubato tail: {e}"))?;
        let useful = ((pcm.len() - pos) as f64 * ratio) as usize;
        out.extend_from_slice(&frames[0][..useful.min(frames[0].len())]);
    }
    Ok(out)
}
