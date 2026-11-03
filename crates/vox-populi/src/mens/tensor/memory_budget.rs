//! Model-hint parsing shared by the training-sizing code paths.
//!
//! The params_b-based VRAM-budget heuristics that used to live here (fixed
//! per-family "resident GiB/B" constants, the Qwen3/Qwen3.5/Qwen2.5-Coder
//! retreat ladders, `plan`/`plan_with_resident*`) were deleted: they were
//! superseded by the measured [`super::memory_model`] SSOT (`MemoryModels`,
//! `plan_for`, `sweep`), which fits a request against a real calibrated
//! activation model and a real on-disk [`super::memory_model::ModelShape`]
//! instead of a params_b-only guess. One surviving bug from the old ladder:
//! `vram_autodetect::auto_preset_for`'s `METAL_*_GIB` constants had no 27B
//! rung and silently handed a 27B model the 14B preset — a class of bug the
//! measured model can't reproduce, since it fails closed on an uncalibrated
//! lane instead of guessing.
//!
//! [`params_b_from_model_hint`] survives because it is not a heuristic budget
//! — it is a plain parser used wherever a human-readable model id needs to
//! report an approximate size (logs, gradient-checkpointing auto-enable),
//! independent of any VRAM-fit decision.

/// Parse a model's parameter count (in billions) from a model id / hint.
///
/// Recognizes patterns like `Qwen/Qwen3.5-4B`, `qwen2.5-0.8b`, `llama-9B`. Returns
/// `None` when no size token is present (caller falls back to a default).
#[must_use]
pub fn params_b_from_model_hint(hint: &str) -> Option<f64> {
    let lower = hint.to_ascii_lowercase();
    // Scan for a `<number>b` token (optionally decimal), preceded by a non-alnum or
    // a digit boundary so we don't match the `b` in arbitrary words.
    let bytes = lower.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'b' {
            // Walk backward over digits and a single dot.
            let mut j = i;
            let mut seen_digit = false;
            let mut seen_dot = false;
            while j > 0 {
                let c = bytes[j - 1];
                if c.is_ascii_digit() {
                    seen_digit = true;
                    j -= 1;
                } else if c == b'.' && !seen_dot {
                    seen_dot = true;
                    j -= 1;
                } else {
                    break;
                }
            }
            if seen_digit
                && let Ok(v) = lower[j..i].parse::<f64>()
                && v > 0.0
                && v < 2000.0
            {
                return Some(v);
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_param_counts() {
        assert_eq!(params_b_from_model_hint("Qwen/Qwen3.5-4B"), Some(4.0));
        assert_eq!(params_b_from_model_hint("qwen2.5-0.8b"), Some(0.8));
        assert_eq!(
            params_b_from_model_hint("meta-llama/Llama-3-70B"),
            Some(70.0)
        );
        assert_eq!(params_b_from_model_hint("some-model"), None);
    }

    #[test]
    fn empty_hint_returns_none() {
        // Catches: panic or index-out-of-bounds when the input is the empty string.
        assert_eq!(params_b_from_model_hint(""), None);
    }

    #[test]
    fn b_in_non_numeric_context_is_not_parsed() {
        // Catches: false positive where "b" in "backend" or "qwen2.5b-instruct" (the
        // "b" suffix on the full string without a numeric prefix) is incorrectly treated
        // as a parameter count.
        assert_eq!(params_b_from_model_hint("backend"), None);
        assert_eq!(params_b_from_model_hint("bert-base"), None);
    }

    #[test]
    fn decimal_param_count_parsed_correctly() {
        // Catches: parser dropping the fractional part when a dot is present (e.g.
        // parsing "0.5b" as 0.0 or 5.0 instead of 0.5).
        assert_eq!(
            params_b_from_model_hint("Qwen/Qwen2.5-Coder-0.5B-Instruct"),
            Some(0.5)
        );
        assert_eq!(params_b_from_model_hint("tiny-1.5b-model"), Some(1.5));
    }
}
