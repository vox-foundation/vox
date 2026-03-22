//! Training progress and GPU visibility via [`tracing`].
//!
//! **GPU visibility:** `RUST_LOG=vox_populi_gpu=info` (or `warn`) surfaces structured
//! [`tracing`] events with `event = "gpu_fallback_to_cpu"` / `"gpu_intentional_cpu"` / `"gpu_selected"`.
//!
//! General training lines use `target = "vox_populi_train"` so they appear when the CLI subscriber
//! is initialized (e.g. `vox` with default `RUST_LOG=info`).

/// Format a scalar loss for **human-readable** progress logs.
///
/// Fixed decimals like `{:.6}` explode into dozens of digits when CE spikes; subnormal values
/// look like `0.000000`. This uses scientific notation outside a sane band.
#[must_use]
pub fn format_loss_for_log(loss: f64) -> String {
    if loss.is_nan() {
        return "nan".to_string();
    }
    if loss.is_infinite() {
        return if loss.is_sign_positive() {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    let ax = loss.abs();
    if ax == 0.0 {
        return "0".to_string();
    }
    if !(1e-3..1e3).contains(&ax) {
        format!("{loss:.4e}")
    } else {
        format!("{loss:.6}")
    }
}

/// Informational training line (epoch progress, device selection, etc.).
pub fn info(msg: &str) {
    tracing::info!(target: "vox_populi_train", "{}", msg);
}

/// User-visible warning for unsupported knobs and training caveats.
pub fn warn(msg: &str) {
    tracing::warn!(target: "vox_populi_train", "{}", msg);
}

/// When the user expected GPU acceleration but execution is on CPU (CUDA/Metal init failed, wrong build, etc.).
/// Emits `tracing::warn!` with stable `target = "vox_populi_gpu"` for log aggregation.
pub fn gpu_fallback(component: &str, summary: &str) {
    tracing::warn!(
        target: "vox_populi_gpu",
        component,
        summary = %summary,
        event = "gpu_fallback_to_cpu",
        "GPU→CPU fallback ({component}): {summary}",
    );
}

#[cfg(test)]
mod tests {
    use super::format_loss_for_log;

    #[test]
    fn format_loss_scientific_for_large() {
        let s = format_loss_for_log(4.3e19);
        assert!(s.contains('e') || s.contains('E'), "{s}");
    }

    #[test]
    fn format_loss_fixed_for_typical_ce() {
        let s = format_loss_for_log(2.3456789);
        assert!(s.starts_with("2.345"), "{s}");
    }

    #[test]
    fn format_loss_zero() {
        assert_eq!(format_loss_for_log(0.0), "0");
    }
}
