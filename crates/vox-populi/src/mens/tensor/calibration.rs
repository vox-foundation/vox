//! Calibration record store: what actually happened when a shape was trained,
//! keyed by the accelerator that ran it (see [`super::accel_budget`]).
//!
//! A record is either a `Fits` observation (we know the real peak) or an
//! `Oom` observation (we know a lower bound: it died trying to allocate
//! more than the peak we saw). Only `Fits` rows are trustworthy evidence of
//! "this shape fits and here's how much it costs" — an `Oom` row's peak is
//! real memory pressure, but selecting it as the best-known-good point would
//! recommend the shape that just failed.
//!
//! ## Current calibration status: `candle-metal` is NOT fit
//!
//! As of 2026-09-11 exactly one real `Fits` record exists for the
//! `candle-metal` lane (Qwen3-0.6B, batch=2×seq=512, on this repo's Apple M5
//! Max dev host) — see
//! `docs/src/architecture/measurements/2026-09-11-candle-metal-calibration.json`
//! and its companion `.md` writeup for the full story, including a real
//! `preset_schema.rs` bug that silently changed the requested shape. One
//! point cannot determine a slope (`fit_a_lane` below refuses on purpose —
//! see `single_real_candle_metal_point_refuses_to_fit`), so `a_candle_metal`
//! has no fitted value yet. A second measurement (same host/lane, a
//! different `tokens_per_step` or model) is a follow-up, not a blocker:
//! callers needing a memory-budget coefficient for `candle-metal` today
//! should treat it as `Seeded`/`Uncalibrated`, the same honest-absence
//! pattern `accel_budget::query_accel_budget()` already uses for the CUDA
//! lane on non-macOS (returns `None` rather than a fabricated number).

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FitOutcome {
    Fits,
    Oom,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalibrationRecord {
    pub host_key: String,
    pub lane: String,
    pub model: String,
    pub artifact_bytes: u64,
    pub layers: u32,
    pub hidden: u32,
    pub tokens_per_step: u64,
    pub outcome: FitOutcome,
    pub peak_bytes: Option<u64>,
}

/// The best (highest observed peak, i.e. most memory-hungry successful run)
/// record for a given host and lane. `Oom` rows are never candidates — a
/// near-OOM run peaks highest right before it dies, so a filter-blind
/// `max_by_key` would pick exactly the wrong row.
pub fn best_record<'a>(
    records: &'a [CalibrationRecord],
    host_key: &str,
    lane: &str,
) -> Option<&'a CalibrationRecord> {
    records
        .iter()
        .filter(|r| r.host_key == host_key && r.lane == lane && r.outcome == FitOutcome::Fits)
        .max_by_key(|r| r.peak_bytes.unwrap_or(0))
}

/// Fit the per-(layer, hidden-unit, token) memory coefficient `a` from two
/// `Fits` observations of the same model shape at different `tokens_per_step`.
///
/// This is a difference quotient between the two points, not a least-squares
/// slope forced through the origin: `artifact_bytes` (the fixed weight cost)
/// cancels out of `peak2 - peak1`, which is exactly why the difference is the
/// right estimator here and why no intercept is ever fitted. A single point
/// cannot determine a slope, so at least two distinct `tokens_per_step`
/// values are required.
pub fn fit_a_lane(records: &[CalibrationRecord]) -> Option<f64> {
    use std::collections::HashMap;

    let mut groups: HashMap<(&str, u32, u32), Vec<&CalibrationRecord>> = HashMap::new();
    for r in records {
        if r.outcome == FitOutcome::Fits && r.peak_bytes.is_some() {
            groups
                .entry((r.model.as_str(), r.layers, r.hidden))
                .or_default()
                .push(r);
        }
    }

    for group in groups.values() {
        let lo = group.iter().min_by_key(|r| r.tokens_per_step)?;
        let hi = group.iter().max_by_key(|r| r.tokens_per_step)?;
        if lo.tokens_per_step == hi.tokens_per_step {
            continue;
        }
        let denom =
            (hi.tokens_per_step - lo.tokens_per_step) as f64 * lo.layers as f64 * lo.hidden as f64;
        if denom == 0.0 {
            continue;
        }
        let p1 = lo.peak_bytes.unwrap() as f64;
        let p2 = hi.peak_bytes.unwrap() as f64;
        return Some((p2 - p1) / denom);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(
        host_key: &str,
        lane: &str,
        outcome: FitOutcome,
        tokens_per_step: u64,
        peak_bytes: Option<u64>,
    ) -> CalibrationRecord {
        CalibrationRecord {
            host_key: host_key.to_string(),
            lane: lane.to_string(),
            model: "qwen3.5-27b".to_string(),
            artifact_bytes: 27_000_000_000,
            layers: 64,
            hidden: 5120,
            tokens_per_step,
            outcome,
            peak_bytes,
        }
    }

    /// Same as `rec`, fixed to the Qwen3.5-27B shape (layers=64, hidden=5120)
    /// used by the difference-quotient fit tests below.
    fn rec27(
        host_key: &str,
        lane: &str,
        outcome: FitOutcome,
        tokens_per_step: u64,
        peak_bytes: Option<u64>,
    ) -> CalibrationRecord {
        rec(host_key, lane, outcome, tokens_per_step, peak_bytes)
    }

    #[test]
    fn best_record_ignores_oom_rows_even_when_they_peaked_higher() {
        // A near-OOM run really does peak highest just before it dies, so the OOM
        // fixture carries the LARGEST peak. A max_by_key that forgets the outcome
        // filter will pick it.
        let rows = vec![
            rec(
                "AppleM5Max-115448725504",
                "candle-metal",
                FitOutcome::Fits,
                512,
                Some(41_765_000_000),
            ),
            rec(
                "AppleM5Max-115448725504",
                "candle-metal",
                FitOutcome::Oom,
                4096,
                Some(119_000_000_000),
            ),
        ];
        let best = best_record(&rows, "AppleM5Max-115448725504", "candle-metal").unwrap();
        assert_eq!(
            best.tokens_per_step, 512,
            "an OOM row must never be selected as best"
        );
    }

    #[test]
    fn a_record_from_another_host_is_not_reused() {
        let rows = vec![rec(
            "OtherMac-57724362752",
            "candle-metal",
            FitOutcome::Fits,
            512,
            Some(1_000),
        )];
        assert!(best_record(&rows, "AppleM5Max-115448725504", "candle-metal").is_none());
    }

    #[test]
    fn a_record_from_another_lane_is_not_reused() {
        let rows = vec![rec(
            "AppleM5Max-115448725504",
            "mlx",
            FitOutcome::Fits,
            512,
            Some(41_765_000_000),
        )];
        assert!(
            best_record(&rows, "AppleM5Max-115448725504", "candle-metal").is_none(),
            "an MLX measurement must not stand in for a candle-metal one"
        );
    }

    #[test]
    fn fitting_uses_the_difference_quotient_between_two_token_counts() {
        // Two points at the same shape: a = (peak2 - peak1) / ((T2 - T1) * L * H).
        // NOT a least-squares slope forced through the origin, which would give
        // 74.126 for these points and silently disagree with the model.
        let rows = vec![
            rec27(
                "AppleM5Max-115448725504",
                "mlx",
                FitOutcome::Fits,
                512,
                Some(41_765_000_000),
            ),
            rec27(
                "AppleM5Max-115448725504",
                "mlx",
                FitOutcome::Fits,
                1024,
                Some(54_459_000_000),
            ),
        ];
        let a = fit_a_lane(&rows).expect("two distinct token counts are enough");
        assert!((a - 75.662).abs() < 0.01, "fitted a = {a}, expected 75.662");
    }

    #[test]
    fn fitting_refuses_a_single_point() {
        let rows = vec![rec27(
            "AppleM5Max-115448725504",
            "mlx",
            FitOutcome::Fits,
            512,
            Some(41_765_000_000),
        )];
        assert!(
            fit_a_lane(&rows).is_none(),
            "one point cannot determine a slope; fitting it invents a degree of freedom"
        );
    }

    /// The single real `candle-metal` measurement on disk (see the module
    /// doc comment) must parse into a `CalibrationRecord`, and `fit_a_lane`
    /// must honestly refuse to fit a slope from it alone — this documents
    /// the current calibration state rather than hiding it.
    #[test]
    fn single_real_candle_metal_point_refuses_to_fit() {
        let raw = include_str!(
            "../../../../../docs/src/architecture/measurements/2026-09-11-candle-metal-calibration.json"
        );
        let doc: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");
        let records: Vec<CalibrationRecord> = serde_json::from_value(doc["records"].clone())
            .expect("records parse as CalibrationRecord");

        assert_eq!(
            records.len(),
            1,
            "exactly one real candle-metal point exists so far"
        );
        let r = &records[0];
        assert_eq!(r.host_key, "AppleM5Max-115448725504");
        assert_eq!(r.lane, "candle-metal");
        assert_eq!(r.model, "Qwen/Qwen3-0.6B");
        assert_eq!(r.layers, 28);
        assert_eq!(r.hidden, 1024);
        assert_eq!(r.tokens_per_step, 1024);
        assert_eq!(r.outcome, FitOutcome::Fits);
        assert_eq!(r.peak_bytes, Some(53_439_004_672));

        assert!(
            fit_a_lane(&records).is_none(),
            "a_candle_metal must not be fit from a single point; it is honestly Uncalibrated \
             until a second measurement lands"
        );
    }
}
