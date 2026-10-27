//! Strict collateral comparison for the three held-out reasoning slices.
//!
//! Each slice (`gsm8k_think`, `math500_think`, `vox_think`) is scored on two
//! metrics, accuracy (`<slice>`) and think-initiation (`<slice>_init`), compared
//! separately at the generic 5% bar. The arithmetic stays in
//! [`crate::eval_collateral_damage`]; this module refuses the inputs that
//! function would quietly accept (a non-finite score, a zero baseline, a wrong
//! item count) so none of them can become a passing report.

use crate::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};
use crate::{CollateralDamageConfig, CollateralDamageReport, eval_collateral_damage};

/// Suffix that turns a slice name into its think-initiation key.
pub const THINK_INIT_SUFFIX: &str = "_init";

/// Report text that keeps the 5% bar from being read as a full-benchmark result.
pub const REASONING_SLICE_SCOPE_NOTE: &str = "The reasoning benches gsm8k_think, math500_think and vox_think are fixed 32-item slices, not full GSM8K or MATH-500 results. The 5% relative bar applies to these 32 items only.";

/// Which of a slice's two scores a baseline key names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningMetric {
    Accuracy,
    ThinkInitiation,
}

/// Classify a baseline key by name: `gsm8k_think` is accuracy for that slice,
/// `gsm8k_think_init` its think-initiation. Anything else is not a reasoning key.
/// Never consults the filesystem, so the result does not depend on the cwd.
pub fn reasoning_slice_of(key: &str) -> Option<(&'static str, ReasoningMetric)> {
    let (base, metric) = match key.strip_suffix(THINK_INIT_SUFFIX) {
        Some(base) => (base, ReasoningMetric::ThinkInitiation),
        None => (key, ReasoningMetric::Accuracy),
    };
    SLICE_NAMES
        .iter()
        .find(|name| **name == base)
        .map(|name| (*name, metric))
}

/// Require `<slice>` and `<slice>_init` for every slice. The error lists every
/// missing key in `SLICE_NAMES` order, accuracy key first.
pub fn require_reasoning_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let have: Vec<&str> = keys.into_iter().collect();
    let mut missing: Vec<String> = Vec::new();
    for name in SLICE_NAMES {
        let init = format!("{name}{THINK_INIT_SUFFIX}");
        for key in [name.to_string(), init] {
            if !have.contains(&key.as_str()) {
                missing.push(key);
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "baseline is missing {}; it must name accuracy and think-initiation for all three \
         32-item slices ({})",
        missing.join(", "),
        SLICE_NAMES.join(", ")
    ))
}

/// Both comparisons for one slice.
#[derive(Debug, Clone)]
pub struct ReasoningSliceVerdict {
    pub slice: &'static str,
    pub n: usize,
    pub accuracy: CollateralDamageReport,
    pub think_initiation: CollateralDamageReport,
}

impl ReasoningSliceVerdict {
    /// True when accuracy or think-initiation dropped past the bar.
    pub fn exceeds_threshold(&self) -> bool {
        self.accuracy.exceeds_threshold || self.think_initiation.exceeds_threshold
    }

    /// `<slice>` if accuracy failed, else `<slice>_init` if think-initiation failed.
    pub fn failed_key(&self) -> Option<String> {
        if self.accuracy.exceeds_threshold {
            Some(self.slice.to_string())
        } else if self.think_initiation.exceeds_threshold {
            Some(format!("{}{THINK_INIT_SUFFIX}", self.slice))
        } else {
            None
        }
    }
}

/// Compare accuracy and think-initiation `(pre, post)` for one slice.
/// `n` is the number of items the scorer actually generated.
pub fn eval_reasoning_slice(
    slice: &str,
    accuracy: (f64, f64),
    think_initiation: (f64, f64),
    n: usize,
    config: &CollateralDamageConfig,
) -> Result<ReasoningSliceVerdict, String> {
    let slice = SLICE_NAMES
        .iter()
        .find(|name| **name == slice)
        .copied()
        .ok_or_else(|| {
            format!(
                "{slice}: not a reasoning slice ({})",
                SLICE_NAMES.join(", ")
            )
        })?;
    if n != SLICE_SIZE {
        return Err(format!("{slice}: n={n}, expected {SLICE_SIZE}"));
    }
    let init_key = format!("{slice}{THINK_INIT_SUFFIX}");
    Ok(ReasoningSliceVerdict {
        slice,
        n,
        accuracy: eval_collateral_damage(slice, accuracy.0, accuracy.1, config),
        think_initiation: eval_collateral_damage(
            &init_key,
            think_initiation.0,
            think_initiation.1,
            config,
        ),
    })
}

fn metric_json(r: &CollateralDamageReport) -> serde_json::Value {
    serde_json::json!({
        "pre": r.pre_training_score,
        "post": r.post_training_score,
        "degradation_rate": r.degradation_rate,
        "exceeds_threshold": r.exceeds_threshold,
    })
}

/// The `reasoning_slices` block of the collateral report. Hand projection: the
/// report types are not `Serialize`.
pub fn reasoning_slices_json(verdicts: &[ReasoningSliceVerdict]) -> serde_json::Value {
    let mut benches = serde_json::Map::new();
    for v in verdicts {
        benches.insert(
            v.slice.to_string(),
            serde_json::json!({
                "n": v.n,
                "accuracy": metric_json(&v.accuracy),
                "think_initiation": metric_json(&v.think_initiation),
            }),
        );
    }
    serde_json::json!({
        "scope_note": REASONING_SLICE_SCOPE_NOTE,
        "slice_size": SLICE_SIZE,
        "benches": benches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CollateralDamageConfig {
        CollateralDamageConfig::default()
    }

    #[test]
    fn slice_of_classifies_by_name_only() {
        assert_eq!(
            reasoning_slice_of("gsm8k_think"),
            Some(("gsm8k_think", ReasoningMetric::Accuracy))
        );
        assert_eq!(
            reasoning_slice_of("vox_think_init"),
            Some(("vox_think", ReasoningMetric::ThinkInitiation))
        );
        assert_eq!(reasoning_slice_of("code_bench"), None);
        assert_eq!(reasoning_slice_of("foo_init"), None);
    }

    #[test]
    fn five_keys_are_refused_and_six_are_accepted() {
        let five = [
            "gsm8k_think",
            "math500_think",
            "math500_think_init",
            "vox_think",
            "vox_think_init",
        ];
        let err = require_reasoning_keys(five).unwrap_err();
        assert!(err.contains("gsm8k_think_init"), "{err}");
        let six = [
            "gsm8k_think",
            "gsm8k_think_init",
            "math500_think",
            "math500_think_init",
            "vox_think",
            "vox_think_init",
        ];
        assert!(require_reasoning_keys(six).is_ok());
    }

    #[test]
    fn an_accuracy_drop_past_five_percent_fails_on_the_slice_key() {
        let v = eval_reasoning_slice("gsm8k_think", (0.625, 0.5625), (0.625, 0.625), 32, &cfg())
            .unwrap();
        assert!(v.exceeds_threshold());
        assert_eq!(v.failed_key().as_deref(), Some("gsm8k_think"));
    }

    #[test]
    fn exactly_five_percent_passes_on_both_metrics() {
        let v = eval_reasoning_slice(
            "math500_think",
            (0.625, 0.59375),
            (0.625, 0.59375),
            32,
            &cfg(),
        )
        .unwrap();
        assert!(!v.exceeds_threshold());
        assert_eq!(v.failed_key(), None);
    }

    #[test]
    fn a_think_initiation_drop_alone_fails_on_the_init_key() {
        let v =
            eval_reasoning_slice("vox_think", (0.625, 0.625), (0.5, 0.46875), 32, &cfg()).unwrap();
        assert!(v.exceeds_threshold());
        assert_eq!(v.failed_key().as_deref(), Some("vox_think_init"));
    }

    #[test]
    fn the_report_block_labels_the_benches_as_32_item_slices() {
        let verdicts: Vec<_> = SLICE_NAMES
            .iter()
            .map(|s| eval_reasoning_slice(s, (0.625, 0.625), (0.5, 0.5), 32, &cfg()).unwrap())
            .collect();
        let j = reasoning_slices_json(&verdicts);
        let note = j["scope_note"].as_str().unwrap();
        assert!(
            note.contains("32-item") && note.contains("not full"),
            "{note}"
        );
        assert_eq!(j["slice_size"], 32);
        for name in SLICE_NAMES {
            assert_eq!(j["benches"][name]["n"], 32, "{name}");
        }
    }
}
