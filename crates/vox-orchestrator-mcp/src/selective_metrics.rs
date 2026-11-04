//! Selective-prediction metrics for labeled decision fixtures (phase 322
//! DEC-04): Laplace precision, expected calibration error, coverage and risk at
//! a threshold, and coverage at a target error. Test-only eval tooling; an item
//! is `(confidence, correct)` for one answered question.

/// `(correct + 1) / (fired + 2)`; 0/0 gives the 0.5 prior.
pub(crate) fn laplace_precision(correct: usize, fired: usize) -> f64 {
    (correct as f64 + 1.0) / (fired as f64 + 2.0)
}

/// Equal-width confidence bins over [0, 1]; sum over non-empty bins of
/// |accuracy - mean confidence| x bin size / item count.
pub(crate) fn ece(items: &[(f32, bool)], bins: usize) -> f64 {
    if items.is_empty() || bins == 0 {
        return 0.0;
    }
    // (count, correct, confidence sum) per bin
    let mut acc = vec![(0usize, 0usize, 0.0f64); bins];
    for &(c, ok) in items {
        let b = ((f64::from(c) * bins as f64) as usize).min(bins - 1);
        acc[b].0 += 1;
        acc[b].1 += usize::from(ok);
        acc[b].2 += f64::from(c);
    }
    acc.iter()
        .filter(|(n, _, _)| *n > 0)
        .map(|&(n, ok, sum)| (ok as f64 / n as f64 - sum / n as f64).abs() * n as f64)
        .sum::<f64>()
        / items.len() as f64
}

/// `(coverage, risk)` among items with confidence >= `tau`: coverage is over
/// `n_total` turns, risk is the error rate among them (0.0 when none).
pub(crate) fn coverage_and_risk(items: &[(f32, bool)], n_total: usize, tau: f32) -> (f64, f64) {
    let answered: Vec<bool> = items.iter().filter(|i| i.0 >= tau).map(|i| i.1).collect();
    if answered.is_empty() || n_total == 0 {
        return (0.0, 0.0);
    }
    let errors = answered.iter().filter(|ok| !**ok).count();
    (
        answered.len() as f64 / n_total as f64,
        errors as f64 / answered.len() as f64,
    )
}

/// Largest coverage over thresholds drawn from the distinct item confidences
/// whose risk is <= `eps`; 0.0 when no threshold qualifies.
pub(crate) fn coverage_at_error(items: &[(f32, bool)], n_total: usize, eps: f64) -> f64 {
    items
        .iter()
        .map(|i| coverage_and_risk(items, n_total, i.0))
        .filter(|&(_, risk)| risk <= eps)
        .map(|(coverage, _)| coverage)
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS: [(f32, bool); 4] = [(0.9, true), (0.9, false), (0.6, true), (0.6, true)];

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn laplace_precision_matches_hand_computed() {
        assert!(close(laplace_precision(3, 4), 0.6667));
        assert!(close(laplace_precision(0, 0), 0.5));
    }

    #[test]
    fn ece_matches_hand_computed() {
        assert!(close(ece(&ITEMS, 10), 0.4));
        assert!(close(ece(&[], 10), 0.0));
    }

    #[test]
    fn coverage_and_risk_matches_hand_computed() {
        let (coverage, risk) = coverage_and_risk(&ITEMS, 5, 0.8);
        assert!(
            close(coverage, 0.4) && close(risk, 0.5),
            "{coverage} {risk}"
        );
        assert_eq!(coverage_and_risk(&ITEMS, 5, 0.95), (0.0, 0.0));
    }

    #[test]
    fn coverage_at_error_matches_hand_computed() {
        assert!(close(coverage_at_error(&ITEMS, 5, 0.0), 0.0));
        assert!(close(coverage_at_error(&ITEMS, 5, 0.25), 0.8));
    }
}
