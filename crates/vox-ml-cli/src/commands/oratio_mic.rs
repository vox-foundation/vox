//! Default-input microphone capture to 16-bit mono WAV (edge capture for Oratio).
//!
//! Enabled with Cargo feature **`oratio-mic`** (`cpal` + `hound`).

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Record from the default input device for `seconds`, writing mono PCM WAV at the device sample rate.
///
/// Oratio decodes arbitrary-rate WAV and resamples to 16 kHz internally.
pub fn record_default_input_wav(out: &Path, seconds: f32) -> Result<()> {
    anyhow::ensure!(
        seconds >= 0.25 && seconds <= 300.0,
        "seconds must be between 0.25 and 300"
    );
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("no default input device"))?;
    let supported = device
        .default_input_config()
        .context("default_input_config")?;
    let sr = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let max_frames = ((seconds * sr as f32).ceil() as usize).saturating_add(sr as usize / 4);
    let max_samples = max_frames.saturating_mul(channels);

    let buf = Arc::new(Mutex::new(Vec::<f32>::with_capacity(
        max_samples.min(48_000 * 16),
    )));

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let b = buf.clone();
            device.build_input_stream(
                &supported.into(),
                move |data: &[f32], _| {
                    let mut g = b.lock().expect("mic buffer lock");
                    if g.len() < max_samples {
                        g.extend_from_slice(data);
                    }
                },
                |e| {
                    tracing::warn!(target: "vox_oratio_mic", "input stream error: {e}");
                },
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let b = buf.clone();
            device.build_input_stream(
                &supported.into(),
                move |data: &[i16], _| {
                    let mut g = b.lock().expect("mic buffer lock");
                    if g.len() >= max_samples {
                        return;
                    }
                    for &s in data {
                        if g.len() >= max_samples {
                            break;
                        }
                        g.push(s as f32 / i16::MAX as f32);
                    }
                },
                |e| {
                    tracing::warn!(target: "vox_oratio_mic", "input stream error: {e}");
                },
                None,
            )
        }
        other => anyhow::bail!("unsupported microphone sample format: {other:?}"),
    }
    .context("build_input_stream")?;

    stream.play().context("play input stream")?;
    std::thread::sleep(Duration::from_secs_f32(seconds));
    drop(stream);

    let mut pcm = buf.lock().expect("mic buffer lock").clone();
    pcm.truncate(max_samples.min(pcm.len()));

    let mono: Vec<f32> = if channels <= 1 {
        pcm
    } else {
        pcm.chunks(channels)
            .filter_map(|ch| {
                if ch.is_empty() {
                    return None;
                }
                let sum: f32 = ch.iter().copied().sum();
                Some(sum / channels as f32)
            })
            .collect()
    };

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).with_context(|| parent.display().to_string())?;
    }
    let mut writer = hound::WavWriter::create(out, spec)
        .with_context(|| format!("create wav {}", out.display()))?;
    for s in mono {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v)?;
    }
    writer.finalize()?;
    Ok(())
}
