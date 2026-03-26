//! Lightweight preprocessing before STT: peak normalization with a strict latency budget.
//!
//! Set **`VOX_ORATIO_ACOUSTIC_PREPROCESS`** to `none` (default), `peak_normalize`, or `1` (alias for peak).

use std::time::Instant;

use serde::Serialize;

/// What ran in [`preprocess_audio_pcm_f32_reported`].
#[derive(Debug, Clone, Serialize)]
pub struct AcousticPreprocessDiagnostics {
    /// Effective mode (`none`, `peak_normalize`, or `skipped_budget`).
    pub mode: String,
    /// Wall time spent in preprocessing (milliseconds).
    pub wall_ms: f64,
    /// Absolute peak before processing.
    pub peak_before: f32,
    /// Absolute peak after processing.
    pub peak_after: f32,
    /// True when work exceeded `max_extra_ms` and the original buffer was returned.
    pub skipped_due_to_budget: bool,
}

fn peak_abs(samples: &[f32]) -> f32 {
    samples.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

fn effective_mode() -> &'static str {
    match std::env::var("VOX_ORATIO_ACOUSTIC_PREPROCESS") {
        Ok(s) => {
            let t = s.trim();
            if t.is_empty() || t == "0" || t.eq_ignore_ascii_case("none") {
                return "none";
            }
            if t == "1" || t.eq_ignore_ascii_case("peak") || t.eq_ignore_ascii_case("peak_normalize")
            {
                return "peak_normalize";
            }
            "none"
        }
        Err(_) => "none",
    }
}

/// Preprocess PCM f32 mono (typ. -1..=1) with diagnostics; respects `max_extra_ms` wall budget.
#[must_use]
pub fn preprocess_audio_pcm_f32_reported(
    samples: &[f32],
    max_extra_ms: u64,
) -> (Vec<f32>, AcousticPreprocessDiagnostics) {
    let start = Instant::now();
    let mode = effective_mode();
    let peak_before = peak_abs(samples);
    if samples.is_empty() || mode == "none" {
        return (
            samples.to_vec(),
            AcousticPreprocessDiagnostics {
                mode: mode.to_string(),
                wall_ms: elapsed_ms(&start),
                peak_before,
                peak_after: peak_before,
                skipped_due_to_budget: false,
            },
        );
    }

    // Peak normalize toward 0.95 full scale (leave headroom).
    const TARGET: f32 = 0.95;
    let out: Vec<f32> = if peak_before > 0.0 {
        let scale = TARGET / peak_before;
        samples
            .iter()
            .map(|x| (x * scale).clamp(-1.0, 1.0))
            .collect()
    } else {
        samples.to_vec()
    };

    let wall_ms = elapsed_ms(&start);
    if wall_ms > max_extra_ms as f64 && max_extra_ms > 0 {
        return (
            samples.to_vec(),
            AcousticPreprocessDiagnostics {
                mode: "skipped_budget".to_string(),
                wall_ms,
                peak_before,
                peak_after: peak_before,
                skipped_due_to_budget: true,
            },
        );
    }

    let peak_after = peak_abs(&out);
    (
        out,
        AcousticPreprocessDiagnostics {
            mode: "peak_normalize".to_string(),
            wall_ms,
            peak_before,
            peak_after,
            skipped_due_to_budget: false,
        },
    )
}

fn elapsed_ms(start: &Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// When local denoise is unavailable or mode is `none`, returns `samples` unchanged (or scaled for peak mode).
///
/// `max_extra_ms` caps wall time: if exceeded, returns the **original** buffer.
#[must_use]
pub fn preprocess_audio_pcm_f32(samples: &[f32], max_extra_ms: u64) -> Vec<f32> {
    preprocess_audio_pcm_f32_reported(samples, max_extra_ms).0
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn peak_normalize_scales_quiet_signal() {
        // SAFETY: tests are single-threaded here; env is restored after the assertion.
        unsafe {
            std::env::set_var("VOX_ORATIO_ACOUSTIC_PREPROCESS", "peak_normalize");
        }
        let quiet: Vec<f32> = vec![0.01, -0.01, 0.005];
        let (out, d) = preprocess_audio_pcm_f32_reported(&quiet, 100);
        assert!(d.peak_after > d.peak_before);
        assert!((out[0].abs() - 0.95).abs() < 0.01, "got {:?}", out);
        unsafe {
            std::env::remove_var("VOX_ORATIO_ACOUSTIC_PREPROCESS");
        }
    }
}
