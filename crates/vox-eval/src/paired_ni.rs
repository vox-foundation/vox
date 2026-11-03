//! Paired non-inferiority: an interval for the difference of two paired proportions and the
//! pre-registered verdict mapping.
//!
//! Both arms answer the SAME tasks, so the outcome is a paired 2x2 table: `a` both succeed, `b` only
//! arm 1, `c` only arm 2, `d` neither; `n = a + b + c + d`, `p1 = (a + b) / n`,
//! `p2 = (a + c) / n`, `delta = p1 - p2 = (b - c) / n`.
//!
//! The interval is Newcombe (1998) method 10, the hybrid score / Wilson MOVER interval without
//! continuity correction (Newcombe, Stat Med 17:2635; ratesci `pairbinci.R`, `xmoverpairci` with
//! `corc = FALSE` and a Wilson base):
//!
//! ```text
//! (l1, u1) = Wilson(a + b, n)     (l2, u2) = Wilson(a + c, n)
//! phi = (a*d - b*c) / sqrt((a+b)(c+d)(a+c)(b+d)),  0 when that denominator is 0
//! L = delta - sqrt((p1-l1)^2 - 2*phi*(p1-l1)*(u2-p2) + (u2-p2)^2)
//! U = delta + sqrt((u1-p1)^2 - 2*phi*(u1-p1)*(p2-l2) + (p2-l2)^2)
//! ```
//!
//! clamped to `[-1, 1]`. The worked vectors in the tests were computed from the same formula, so
//! they are regression vectors, not an independent oracle; the property tests guard the shape.

use serde::{Deserialize, Serialize};

use crate::corpus_stats::{ConfidenceInterval, wilson_interval};

/// Newcombe method-10 interval for `p1 - p2` from paired 2x2 counts. `n == 0` gives `(0, -1, 1)`.
#[must_use]
pub fn paired_difference_ci(a: usize, b: usize, c: usize, d: usize, z: f64) -> ConfidenceInterval {
    let n = a + b + c + d;
    if n == 0 {
        return ConfidenceInterval {
            point: 0.0,
            low: -1.0,
            high: 1.0,
        };
    }
    let w1 = wilson_interval(a + b, n, z);
    let w2 = wilson_interval(a + c, n, z);
    let (p1, p2) = (w1.point, w2.point);
    let (fa, fb, fc, fd) = (a as f64, b as f64, c as f64, d as f64);
    let den = ((fa + fb) * (fc + fd) * (fa + fc) * (fb + fd)).sqrt();
    let phi = if den == 0.0 {
        0.0
    } else {
        (fa * fd - fb * fc) / den
    };
    let delta = p1 - p2;
    // Square-and-add of the two marginal half-widths, corrected for their correlation.
    let mover = |x: f64, y: f64| (x * x - 2.0 * phi * x * y + y * y).max(0.0).sqrt();
    ConfidenceInterval {
        point: delta,
        low: (delta - mover(p1 - w1.low, w2.high - p2)).max(-1.0),
        high: (delta + mover(w1.high - p1, p2 - w2.low)).min(1.0),
    }
}

/// Non-inferiority verdict of arm 1 against arm 2 at `margin` (a positive proportion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NiVerdict {
    /// The whole interval lies above `-margin`.
    NonInferior,
    /// The interval straddles `-margin`.
    Inconclusive,
    /// The whole interval lies below `-margin`.
    Inferior,
}

/// Map an interval to the verdict: non-inferior if `low > -margin`, inferior if
/// `high < -margin`, else inconclusive (strict inequalities: touching the margin is inconclusive).
#[must_use]
pub fn ni_verdict(ci: &ConfidenceInterval, margin: f64) -> NiVerdict {
    if ci.low > -margin {
        NiVerdict::NonInferior
    } else if ci.high < -margin {
        NiVerdict::Inferior
    } else {
        NiVerdict::Inconclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_stats::Z_95;

    fn close(x: f64, y: f64) -> bool {
        (x - y).abs() < 5e-4
    }

    #[test]
    fn paired_ni_worked_vectors() {
        use NiVerdict::*;
        let cases = [
            ((120, 10, 8, 12), -0.0439, 0.0712, NonInferior),
            ((110, 12, 8, 20), -0.0325, 0.0864, NonInferior),
            ((125, 3, 12, 10), -0.1150, -0.0098, Inconclusive),
            ((100, 5, 30, 15), -0.2399, -0.0938, Inconclusive),
            ((150, 0, 0, 0), -0.0250, 0.0250, NonInferior),
            ((140, 0, 10, 0), -0.1184, -0.0276, Inconclusive),
        ];
        for ((a, b, c, d), low, high, verdict) in cases {
            let ci = paired_difference_ci(a, b, c, d, Z_95);
            let n = (a + b + c + d) as f64;
            assert!(
                close(ci.point, (b as f64 - c as f64) / n),
                "{a},{b},{c},{d}: point {ci:?}"
            );
            assert!(
                close(ci.low, low) && close(ci.high, high),
                "{a},{b},{c},{d}: {ci:?}"
            );
            assert_eq!(ni_verdict(&ci, 0.10), verdict, "{a},{b},{c},{d}");
        }
    }

    #[test]
    fn paired_ni_properties_over_small_tables() {
        for a in 0..5 {
            for b in 0..5 {
                for c in 0..5 {
                    for d in 0..5 {
                        if a + b + c + d == 0 {
                            continue;
                        }
                        let ci = paired_difference_ci(a, b, c, d, Z_95);
                        let tag = format!("{a},{b},{c},{d}: {ci:?}");
                        assert!(
                            ci.low <= ci.point + 1e-12 && ci.point <= ci.high + 1e-12,
                            "{tag}"
                        );
                        assert!(-1.0 <= ci.low && ci.high <= 1.0, "{tag}");
                        let swapped = paired_difference_ci(a, c, b, d, Z_95);
                        assert!(
                            (swapped.low + ci.high).abs() < 1e-12,
                            "{tag} swap {swapped:?}"
                        );
                        assert!(
                            (swapped.high + ci.low).abs() < 1e-12,
                            "{tag} swap {swapped:?}"
                        );
                        if b == c {
                            assert_eq!(ci.point, 0.0, "{tag}");
                            assert!((ci.low + ci.high).abs() < 1e-12, "{tag}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn paired_ni_empty_table_is_uninformative() {
        let ci = paired_difference_ci(0, 0, 0, 0, Z_95);
        assert_eq!((ci.point, ci.low, ci.high), (0.0, -1.0, 1.0));
    }

    #[test]
    fn paired_ni_verdict_mapping_is_strict_at_the_margin() {
        let ci = |low, high| ConfidenceInterval {
            point: (low + high) / 2.0,
            low,
            high,
        };
        assert_eq!(ni_verdict(&ci(-0.09, 0.05), 0.10), NiVerdict::NonInferior);
        assert_eq!(ni_verdict(&ci(-0.20, -0.11), 0.10), NiVerdict::Inferior);
        assert_eq!(ni_verdict(&ci(-0.12, -0.05), 0.10), NiVerdict::Inconclusive);
        assert_eq!(ni_verdict(&ci(-0.10, 0.02), 0.10), NiVerdict::Inconclusive);
        assert_eq!(
            serde_json::to_string(&NiVerdict::NonInferior).unwrap(),
            "\"non_inferior\""
        );
    }
}
