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

/// Refuse inputs the generic arithmetic would quietly pass: a non-finite score
/// (`f64::max` ignores NaN), a baseline of zero or less (rate forced to 0.0), or
/// a fraction outside `[0, 1]`.
fn guard(slice: &str, metric: &str, pre: f64, post: f64) -> Result<(), String> {
    if !pre.is_finite() || !post.is_finite() {
        return Err(format!(
            "{slice} {metric}: non-finite score (pre={pre}, post={post}); refusing"
        ));
    }
    if pre <= 0.0 {
        return Err(format!(
            "{slice} {metric}: baseline {pre} gives no relative bar; re-baseline or choose a \
             different slice"
        ));
    }
    if !(0.0..=1.0).contains(&pre) || !(0.0..=1.0).contains(&post) {
        return Err(format!(
            "{slice} {metric}: score outside [0, 1] (pre={pre}, post={post}); refusing"
        ));
    }
    Ok(())
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
    guard(slice, "accuracy", accuracy.0, accuracy.1)?;
    guard(
        slice,
        "think_initiation",
        think_initiation.0,
        think_initiation.1,
    )?;
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

/// One stored number of one slice, read defensively. serde_json writes a
/// non-finite float as `null`, so a missing or non-numeric value covers it.
fn stored_f64(bench: &serde_json::Value, slice: &str, pointer: &str) -> Result<f64, String> {
    bench
        .pointer(pointer)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| format!("{slice}: stored {pointer} is missing or not a finite number"))
}

/// Re-validate a stored collateral report without trusting its verdict fields.
///
/// Requires the report threshold to equal `config`, the slice label (`scope_note`,
/// `slice_size`, per-slice `n`), and every name in [`SLICE_NAMES`] with all four
/// stored numbers. Each slice is recomputed with [`eval_reasoning_slice`] from the
/// stored `pre`/`post` values; the stored `degradation_rate`, `exceeds_threshold`
/// and top-level `status` are ignored. A drop past the bar on either metric is an `Err`.
pub fn verify_reasoning_slices(
    report: &serde_json::Value,
    config: &CollateralDamageConfig,
) -> Result<Vec<ReasoningSliceVerdict>, String> {
    let threshold = report
        .get("threshold")
        .and_then(serde_json::Value::as_f64)
        .ok_or("report has no numeric threshold")?;
    if threshold != config.max_degradation_rate {
        return Err(format!(
            "stored threshold {threshold} is not the required {}",
            config.max_degradation_rate
        ));
    }
    let block = report
        .get("reasoning_slices")
        .filter(|b| b.is_object())
        .ok_or("report has no reasoning_slices block (written by an older or edited run)")?;
    if block.get("scope_note").and_then(serde_json::Value::as_str)
        != Some(REASONING_SLICE_SCOPE_NOTE)
    {
        return Err("reasoning_slices.scope_note is missing or not the expected label".into());
    }
    if block.get("slice_size").and_then(serde_json::Value::as_u64) != Some(SLICE_SIZE as u64) {
        return Err(format!(
            "reasoning_slices.slice_size is missing or not {SLICE_SIZE}"
        ));
    }
    let benches = block
        .get("benches")
        .filter(|b| b.is_object())
        .ok_or("reasoning_slices.benches is missing")?;
    let mut verdicts = Vec::with_capacity(SLICE_NAMES.len());
    for name in SLICE_NAMES {
        let bench = benches
            .get(name)
            .ok_or_else(|| format!("reasoning_slices.benches is missing {name}"))?;
        let n = bench
            .get("n")
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| format!("{name}: stored n is missing or not a count"))?;
        let accuracy = (
            stored_f64(bench, name, "/accuracy/pre")?,
            stored_f64(bench, name, "/accuracy/post")?,
        );
        let think = (
            stored_f64(bench, name, "/think_initiation/pre")?,
            stored_f64(bench, name, "/think_initiation/post")?,
        );
        let verdict = eval_reasoning_slice(name, accuracy, think, n, config)?;
        if let Some(key) = verdict.failed_key() {
            let r = if verdict.accuracy.exceeds_threshold {
                &verdict.accuracy
            } else {
                &verdict.think_initiation
            };
            return Err(format!(
                "{key} dropped {:.4} (pre {} post {}), past the {} bar",
                r.degradation_rate,
                r.pre_training_score,
                r.post_training_score,
                config.max_degradation_rate
            ));
        }
        verdicts.push(verdict);
    }
    Ok(verdicts)
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

    fn refused(slice: &str, acc: (f64, f64), think: (f64, f64), n: usize) -> String {
        eval_reasoning_slice(slice, acc, think, n, &cfg()).unwrap_err()
    }

    #[test]
    fn a_nan_accuracy_post_score_is_refused() {
        let e = refused("gsm8k_think", (0.625, f64::NAN), (0.625, 0.625), 32);
        assert!(e.contains("non-finite") && e.contains("gsm8k_think"), "{e}");
    }

    #[test]
    fn an_infinite_think_initiation_post_score_is_refused() {
        let e = refused("gsm8k_think", (0.625, 0.625), (0.625, f64::INFINITY), 32);
        assert!(e.contains("non-finite"), "{e}");
    }

    #[test]
    fn a_nan_baseline_is_refused() {
        refused("vox_think", (f64::NAN, 0.5), (0.5, 0.5), 32);
    }

    #[test]
    fn a_zero_accuracy_baseline_is_refused() {
        let e = refused("math500_think", (0.0, 0.0), (0.625, 0.625), 32);
        assert!(e.contains("no relative bar"), "{e}");
    }

    #[test]
    fn a_zero_think_initiation_baseline_is_refused() {
        let e = refused("math500_think", (0.625, 0.625), (0.0, 0.0), 32);
        assert!(e.contains("no relative bar"), "{e}");
    }

    #[test]
    fn an_accuracy_above_one_is_refused() {
        refused("vox_think", (0.625, 1.5), (0.5, 0.5), 32);
    }

    #[test]
    fn a_negative_think_initiation_is_refused() {
        refused("vox_think", (0.625, 0.625), (0.5, -0.03125), 32);
    }

    #[test]
    fn an_item_count_other_than_32_is_refused() {
        for n in [0, 31, 33] {
            let e = refused("gsm8k_think", (0.625, 0.625), (0.625, 0.625), n);
            assert!(e.contains("expected 32"), "n={n}: {e}");
        }
    }

    #[test]
    fn an_unknown_slice_name_is_refused() {
        refused("gsm8k", (0.625, 0.625), (0.625, 0.625), 32);
    }

    /// A report as the writer emits it: three slices at 0.625 on both metrics.
    fn good_report() -> serde_json::Value {
        let verdicts: Vec<_> = SLICE_NAMES
            .iter()
            .map(|s| eval_reasoning_slice(s, (0.625, 0.625), (0.625, 0.625), 32, &cfg()).unwrap())
            .collect();
        serde_json::json!({
            "status": "pass",
            "failed_on": null,
            "threshold": 0.05,
            "reports": [],
            "reasoning_slices": reasoning_slices_json(&verdicts),
        })
    }

    #[test]
    fn a_stored_report_in_the_writer_shape_is_accepted() {
        let verdicts = verify_reasoning_slices(&good_report(), &cfg()).unwrap();
        assert_eq!(verdicts.len(), 3);
        assert!(verdicts.iter().all(|v| v.n == 32));
    }

    #[test]
    fn a_report_without_a_reasoning_slices_block_is_refused() {
        let mut r = good_report();
        r.as_object_mut().unwrap().remove("reasoning_slices");
        let e = verify_reasoning_slices(&r, &cfg()).unwrap_err();
        assert!(e.contains("reasoning_slices"), "{e}");
    }

    #[test]
    fn a_stored_pass_over_a_degraded_accuracy_is_recomputed_and_refused() {
        let mut r = good_report();
        let base = "/reasoning_slices/benches/gsm8k_think/accuracy";
        *r.pointer_mut(&format!("{base}/pre")).unwrap() = serde_json::json!(0.5);
        *r.pointer_mut(&format!("{base}/post")).unwrap() = serde_json::json!(0.46875);
        assert_eq!(
            r.pointer(&format!("{base}/exceeds_threshold")),
            Some(&serde_json::json!(false))
        );
        let e = verify_reasoning_slices(&r, &cfg()).unwrap_err();
        assert!(e.contains("gsm8k_think"), "{e}");
    }

    fn edited(pointer: &str, value: serde_json::Value) -> String {
        let mut r = good_report();
        *r.pointer_mut(pointer).unwrap() = value;
        verify_reasoning_slices(&r, &cfg()).unwrap_err()
    }

    #[test]
    fn a_widened_stored_threshold_is_refused() {
        let e = edited("/threshold", serde_json::json!(0.5));
        assert!(e.contains("threshold"), "{e}");
    }

    #[test]
    fn a_stored_slice_size_other_than_32_is_refused() {
        let e = edited("/reasoning_slices/slice_size", serde_json::json!(31));
        assert!(e.contains("slice_size"), "{e}");
    }

    #[test]
    fn a_missing_scope_note_is_refused() {
        let mut r = good_report();
        r.pointer_mut("/reasoning_slices")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("scope_note");
        let e = verify_reasoning_slices(&r, &cfg()).unwrap_err();
        assert!(e.contains("scope_note"), "{e}");
    }

    #[test]
    fn a_missing_slice_is_refused_by_name() {
        let mut r = good_report();
        r.pointer_mut("/reasoning_slices/benches")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("math500_think");
        let e = verify_reasoning_slices(&r, &cfg()).unwrap_err();
        assert!(e.contains("math500_think"), "{e}");
    }

    #[test]
    fn a_null_stored_score_is_refused() {
        let e = edited(
            "/reasoning_slices/benches/vox_think/think_initiation/post",
            serde_json::Value::Null,
        );
        assert!(
            e.contains("vox_think") && e.contains("think_initiation/post"),
            "{e}"
        );
    }

    #[test]
    fn a_stored_slice_count_of_31_is_refused() {
        let e = edited(
            "/reasoning_slices/benches/gsm8k_think/n",
            serde_json::json!(31),
        );
        assert!(
            e.contains("gsm8k_think") && e.contains("expected 32"),
            "{e}"
        );
    }

    #[test]
    fn a_zero_stored_baseline_is_refused() {
        let e = edited(
            "/reasoning_slices/benches/math500_think/accuracy/pre",
            serde_json::json!(0.0),
        );
        assert!(e.contains("no relative bar"), "{e}");
    }

    #[test]
    fn a_stored_pass_over_a_degraded_think_initiation_is_refused_on_the_init_key() {
        let mut r = good_report();
        let base = "/reasoning_slices/benches/vox_think/think_initiation";
        *r.pointer_mut(&format!("{base}/pre")).unwrap() = serde_json::json!(0.5);
        *r.pointer_mut(&format!("{base}/post")).unwrap() = serde_json::json!(0.46875);
        let e = verify_reasoning_slices(&r, &cfg()).unwrap_err();
        assert!(e.contains("vox_think_init"), "{e}");
    }
}
