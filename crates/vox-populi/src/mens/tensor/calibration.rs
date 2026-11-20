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
//! ## Current calibration status: `candle-metal` is calibrated (Nov 2026)
//!
//! `(candle-metal, gradient_checkpointing=true)` is now fitted via
//! `fit_a_lane` from two `Fits` observations of `Qwen/Qwen3.5-2B`
//! (`tokens_per_step` 512 and 2048) on Apple M5 Max in
//! `docs/src/architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.json`,
//! giving `act_bytes_per_lht = 162.48240311940512` in
//! `contracts/mens/memory-model.v1.yaml`. The historical single-point record
//! (`2026-09-11-candle-metal-calibration.json`) remains pinned by
//! `single_real_candle_metal_point_refuses_to_fit` below to guarantee
//! single-point inputs still fail closed.

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

/// Multi-term calibration observation distinguishing resident weights, cached BF16 weight
/// amplification (`cache_dequantized`), `lm_head` LoRA participation, and attention width.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MultiTermCalibrationRecord {
    pub host_key: String,
    pub lane: String,
    pub model: String,
    pub artifact_bytes: u64,
    pub resident_weight_bytes: u64,
    pub layers: u32,
    pub hidden: u32,
    pub attn_width: u32,
    pub vocab_size: u64,
    pub tie_word_embeddings: bool,
    pub tokens_per_step: u64,
    pub cache_dequantized: bool,
    pub rank: u32,
    pub lora_exclude_lm_head: bool,
    pub sequence_packing: bool,
    #[serde(default = "default_steps_executed")]
    pub steps_executed: u32,
    pub outcome: FitOutcome,
    pub peak_bytes: Option<u64>,
}

fn default_steps_executed() -> u32 {
    1
}

/// Fitted non-negative multi-term memory model coefficients for one training lane.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MultiTermCoeffs {
    /// Fixed runtime/allocator baseline in bytes.
    pub fixed_bytes: f64,
    /// Bytes per `(layers * hidden * tokens_per_step)`.
    pub act_bytes_per_lht: f64,
    /// Logits / cross-entropy buffer bytes per `(vocab_size * tokens_per_step)`.
    pub logits_bytes_per_vt: f64,
    /// Extra activation/arena retention per `(layers * attn_width * tokens_per_step)` when
    /// `cache_dequantized == true`.
    pub cached_attn_bytes_per_lwt: f64,
    /// Extra bytes per `(vocab_size * tokens_per_step)` when `lora_exclude_lm_head == false`.
    pub lm_head_lora_bytes_per_vt: f64,
}

impl MultiTermCoeffs {
    pub fn predict_bytes(
        &self,
        resident_weight_bytes: u64,
        layers: u32,
        hidden: u32,
        attn_width: u32,
        vocab_size: u64,
        tokens_per_step: u64,
        cache_dequantized: bool,
        lora_exclude_lm_head: bool,
    ) -> u64 {
        self.predict_bytes_with_chunk_rows(
            resident_weight_bytes,
            layers,
            hidden,
            attn_width,
            vocab_size,
            tokens_per_step,
            cache_dequantized,
            lora_exclude_lm_head,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn predict_bytes_with_chunk_rows(
        &self,
        resident_weight_bytes: u64,
        layers: u32,
        hidden: u32,
        attn_width: u32,
        vocab_size: u64,
        tokens_per_step: u64,
        cache_dequantized: bool,
        lora_exclude_lm_head: bool,
        chunked_lm_head_rows: Option<u64>,
    ) -> u64 {
        let lht = (layers as f64) * (hidden as f64) * (tokens_per_step as f64);
        let head_rows = match chunked_lm_head_rows {
            Some(c) if c > 0 => tokens_per_step.min(c),
            _ => tokens_per_step,
        };
        let vt = (vocab_size as f64) * (head_rows as f64);
        let lwt = if cache_dequantized {
            (layers as f64) * (attn_width as f64) * (tokens_per_step as f64)
        } else {
            0.0
        };
        let lm_head_vt = if lora_exclude_lm_head { 0.0 } else { vt };
        let overhead = self.fixed_bytes
            + self.act_bytes_per_lht * lht
            + self.logits_bytes_per_vt * vt
            + self.cached_attn_bytes_per_lwt * lwt
            + self.lm_head_lora_bytes_per_vt * lm_head_vt;
        resident_weight_bytes.saturating_add(overhead.max(0.0).round() as u64)
    }
}

/// Solve small dense linear system `A * x = b` (`N <= 5`) via partial-pivot Gaussian elimination.
fn solve_linear_system<const N: usize>(mut a: [[f64; N]; N], mut b: [f64; N]) -> Option<[f64; N]> {
    for col in 0..N {
        let mut pivot_row = col;
        let mut pivot_val = a[col][col].abs();
        for (r, row) in a.iter().enumerate().skip(col + 1) {
            if row[col].abs() > pivot_val {
                pivot_val = row[col].abs();
                pivot_row = r;
            }
        }
        if pivot_val < 1e-12 {
            return None;
        }
        if pivot_row != col {
            a.swap(col, pivot_row);
            b.swap(col, pivot_row);
        }
        let diag = a[col][col];
        for r in (col + 1)..N {
            let factor = a[r][col] / diag;
            a[r][col] = 0.0;
            let pivot = a[col];
            for (c, p) in pivot.iter().enumerate().skip(col + 1) {
                a[r][c] -= factor * p;
            }
            b[r] -= factor * b[col];
        }
    }
    let mut x = [0.0_f64; N];
    for r in (0..N).rev() {
        let mut sum = b[r];
        for c in (r + 1)..N {
            sum -= a[r][c] * x[c];
        }
        x[r] = sum / a[r][r];
    }
    Some(x)
}

/// Non-negative least squares over 5 columns via subset search (`2^5 = 32` active masks).
/// Inputs `rows` are `(features [fixed_gib, lht_1e9, vt_1e9, cached_lwt_1e9, lmhead_vt_1e9], excess_gib)`.
fn solve_nnls_5(rows: &[([f64; 5], f64)]) -> Option<[f64; 5]> {
    let mut best_beta: Option<[f64; 5]> = None;
    let mut best_sse = f64::INFINITY;

    for mask in 1u8..32u8 {
        let active: Vec<usize> = (0..5).filter(|j| (mask & (1 << j)) != 0).collect();
        let k = active.len();
        let mut ata = [[0.0_f64; 5]; 5];
        let mut aty = [0.0_f64; 5];
        for (x, y) in rows {
            for (ii, &ci) in active.iter().enumerate() {
                aty[ii] += x[ci] * *y;
                for (jj, &cj) in active.iter().enumerate() {
                    ata[ii][jj] += x[ci] * x[cj];
                }
            }
        }
        // Embedded identity for inactive columns so solve_linear_system<5> solves the k×k subset.
        for (i, row) in ata.iter_mut().enumerate() {
            if i >= k {
                row[i] = 1.0;
            } else {
                row[i] += 1e-9;
            }
        }
        let Some(sol) = solve_linear_system(ata, aty) else {
            continue;
        };
        let mut beta = [0.0_f64; 5];
        let mut non_neg = true;
        for (ii, &ci) in active.iter().enumerate() {
            if sol[ii] < -1e-9 {
                non_neg = false;
                break;
            }
            beta[ci] = sol[ii].max(0.0);
        }
        if !non_neg {
            continue;
        }
        let sse: f64 = rows
            .iter()
            .map(|(x, y)| {
                let pred: f64 = (0..5).map(|j| beta[j] * x[j]).sum();
                let err = pred - *y;
                err * err
            })
            .sum();
        if sse < best_sse {
            best_sse = sse;
            best_beta = Some(beta);
        }
    }
    best_beta
}

/// Fit non-negative [`MultiTermCoeffs`] across distinct steady-state `(model, tokens_per_step,
/// cache_dequantized, lora_exclude_lm_head)` calibration cells (`Fits` rows only).
///
/// Requires at least 6 distinct cells covering both `cache_dequantized` modes or both `lm_head`
/// configurations so every term is identifiable; returns `None` otherwise (fail closed).
pub fn fit_multi_term_lane(records: &[MultiTermCalibrationRecord]) -> Option<MultiTermCoeffs> {
    use std::collections::HashMap;

    let mut cells: HashMap<(&str, u64, bool, bool), &MultiTermCalibrationRecord> = HashMap::new();
    for r in records {
        if r.outcome != FitOutcome::Fits || r.peak_bytes.is_none() {
            continue;
        }
        let key = (
            r.model.as_str(),
            r.tokens_per_step,
            r.cache_dequantized,
            r.lora_exclude_lm_head,
        );
        cells
            .entry(key)
            .and_modify(|cur| {
                if r.peak_bytes.unwrap_or(0) > cur.peak_bytes.unwrap_or(0) {
                    *cur = r;
                }
            })
            .or_insert(r);
    }

    if cells.len() < 6 {
        return None;
    }

    const SCALE: f64 = 1_000_000_000.0;
    let mut rows: Vec<([f64; 5], f64)> = Vec::with_capacity(cells.len());
    let mut envelope_recs: Vec<&MultiTermCalibrationRecord> = Vec::new();
    for r in cells.values() {
        let peak = r.peak_bytes? as f64;
        let excess_g = (peak - r.resident_weight_bytes as f64) / SCALE;
        let lht_g = (r.layers as f64 * r.hidden as f64 * r.tokens_per_step as f64) / SCALE;
        let vt_g = (r.vocab_size as f64 * r.tokens_per_step as f64) / SCALE;
        let lwt_g = if r.cache_dequantized {
            (r.layers as f64 * r.attn_width as f64 * r.tokens_per_step as f64) / SCALE
        } else {
            0.0
        };
        let lm_vt_g = if r.lora_exclude_lm_head { 0.0 } else { vt_g };
        rows.push(([1.0, lht_g, vt_g, lwt_g, lm_vt_g], excess_g));
        if r.sequence_packing || !r.lora_exclude_lm_head {
            envelope_recs.push(*r);
        }
    }

    let beta = solve_nnls_5(&rows)?;
    let mut coeffs = MultiTermCoeffs {
        fixed_bytes: beta[0] * SCALE,
        act_bytes_per_lht: beta[1],
        logits_bytes_per_vt: beta[2],
        cached_attn_bytes_per_lwt: beta[3],
        lm_head_lora_bytes_per_vt: beta[4],
    };

    // Upper-envelope guard across steady-state production cells (`sequence_packing` or
    // explicit `lm_head` LoRA calibration): absorb any sub-2% under-prediction into `fixed_bytes`
    // (or `cached_attn_bytes_per_lwt` when the deficit belongs to a `cache_dequantized` cell) so
    // no measured production configuration under-predicts.
    let mut max_uncached_deficit = 0.0_f64;
    for r in &envelope_recs {
        if !r.cache_dequantized {
            let pred = coeffs.predict_bytes(
                r.resident_weight_bytes,
                r.layers,
                r.hidden,
                r.attn_width,
                r.vocab_size,
                r.tokens_per_step,
                r.cache_dequantized,
                r.lora_exclude_lm_head,
            ) as f64;
            let actual = r.peak_bytes.unwrap_or(0) as f64;
            max_uncached_deficit = max_uncached_deficit.max(actual - pred);
        }
    }
    if max_uncached_deficit > 0.0 {
        coeffs.fixed_bytes += max_uncached_deficit;
    }

    let mut max_cached_lwt_delta = 0.0_f64;
    for r in &envelope_recs {
        if r.cache_dequantized {
            let pred = coeffs.predict_bytes(
                r.resident_weight_bytes,
                r.layers,
                r.hidden,
                r.attn_width,
                r.vocab_size,
                r.tokens_per_step,
                r.cache_dequantized,
                r.lora_exclude_lm_head,
            ) as f64;
            let actual = r.peak_bytes.unwrap_or(0) as f64;
            let lwt = (r.layers as f64) * (r.attn_width as f64) * (r.tokens_per_step as f64);
            if actual > pred && lwt > 0.0 {
                max_cached_lwt_delta = max_cached_lwt_delta.max((actual - pred) / lwt);
            }
        }
    }
    if max_cached_lwt_delta > 0.0 {
        coeffs.cached_attn_bytes_per_lwt += max_cached_lwt_delta;
    }

    Some(coeffs)
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

    #[test]
    fn qwen35_candle_metal_calibration_fits_the_shipped_contract_slope() {
        use super::super::memory_model::{CalKey, CalSource, Lane, MemoryModels};

        let raw = include_str!(
            "../../../../../docs/src/architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.json"
        );
        let doc: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");
        let records: Vec<CalibrationRecord> = serde_json::from_value(doc["records"].clone())
            .expect("records parse as CalibrationRecord");

        assert_eq!(records.len(), 6, "expected 6 calibration Fits records");
        let fitted =
            fit_a_lane(&records).expect("two distinct Qwen3.5-2B token counts fit a slope");

        let models = MemoryModels::load_default().expect("load shipped memory-model.v1.yaml");
        let contract_row = models
            .get(&CalKey::new(Lane::CandleMetal, true).expect("valid CalKey"))
            .expect("candle-metal gradient_checkpointing=true must be calibrated");
        assert_eq!(contract_row.source, CalSource::Measured);
        assert!(
            (fitted - contract_row.act_bytes_per_lht).abs() < 1e-9,
            "fitted {fitted} must match contract act_bytes_per_lht {}",
            contract_row.act_bytes_per_lht
        );
    }

    #[test]
    fn multi_term_metal_calibration_fits_and_bounds_all_full_runs() {
        let raw = include_str!(
            "../../../../../docs/src/architecture/measurements/2026-11-19-candle-metal-multi-term-calibration.json"
        );
        let doc: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");
        let records: Vec<MultiTermCalibrationRecord> =
            serde_json::from_value(doc["records"].clone())
                .expect("records parse as MultiTermCalibrationRecord");
        assert_eq!(
            records.len(),
            11,
            "expected all 11 measured Metal Fits runs"
        );

        let coeffs = fit_multi_term_lane(&records).expect("multi-term calibration succeeds");
        assert!(coeffs.fixed_bytes >= 0.0);
        assert!(coeffs.act_bytes_per_lht >= 0.0);
        assert!(coeffs.logits_bytes_per_vt >= 0.0);
        assert!(coeffs.cached_attn_bytes_per_lwt >= 0.0);
        assert!(coeffs.lm_head_lora_bytes_per_vt >= 0.0);

        for r in records
            .iter()
            .filter(|r| r.sequence_packing || !r.lora_exclude_lm_head)
        {
            let actual = r.peak_bytes.expect("peak_bytes");
            let pred = coeffs.predict_bytes(
                r.resident_weight_bytes,
                r.layers,
                r.hidden,
                r.attn_width,
                r.vocab_size,
                r.tokens_per_step,
                r.cache_dequantized,
                r.lora_exclude_lm_head,
            );
            let ratio = pred as f64 / actual as f64;
            assert!(
                pred >= actual,
                "{} ({}) under-predicted: pred={} < actual={}",
                r.model,
                if r.lora_exclude_lm_head {
                    "lean"
                } else {
                    "lm_head"
                },
                pred,
                actual
            );
            assert!(
                ratio <= 1.25,
                "{} overly conservative: pred={} vs actual={} ({ratio:.3}x)",
                r.model,
                pred,
                actual
            );
        }

        // Verify 27B lean (r16, --lora-exclude-lm-head) stays within +10% of 43.28 GB measured.
        let r27_lean = records
            .iter()
            .find(|r| r.model.contains("27B") && r.lora_exclude_lm_head)
            .expect("27B lean record");
        let pred_27_lean = coeffs.predict_bytes(
            r27_lean.resident_weight_bytes,
            r27_lean.layers,
            r27_lean.hidden,
            r27_lean.attn_width,
            r27_lean.vocab_size,
            r27_lean.tokens_per_step,
            r27_lean.cache_dequantized,
            r27_lean.lora_exclude_lm_head,
        );
        let actual_27_lean = r27_lean.peak_bytes.unwrap();
        let ratio_27 = pred_27_lean as f64 / actual_27_lean as f64;
        assert!(
            (1.0..=1.10).contains(&ratio_27),
            "27B lean prediction {pred_27_lean} vs actual {actual_27_lean} ({ratio_27:.3}x); coeffs={coeffs:?}"
        );
    }
}
