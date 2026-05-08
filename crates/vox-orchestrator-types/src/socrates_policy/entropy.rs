/// Shannon entropy in bits from a probability vector.
#[must_use]
pub fn shannon_entropy_bits(probabilities: &[f64]) -> f64 {
    let mass: f64 = probabilities.iter().copied().filter(|p| *p > 0.0).sum();
    if mass <= f64::EPSILON {
        return 0.0;
    }
    probabilities
        .iter()
        .copied()
        .filter(|p| *p > 0.0)
        .map(|p| {
            let pn = p / mass;
            -(pn * pn.log2())
        })
        .sum()
}

/// Expected information gain in bits for one candidate question.
///
/// `prior_hypothesis_probs` and each posterior row can be unnormalized (they are normalized internally).
/// `outcome_probabilities` should match `posterior_hypothesis_probs.len()`.
#[must_use]
pub fn expected_information_gain_bits(
    prior_hypothesis_probs: &[f64],
    posterior_hypothesis_probs: &[Vec<f64>],
    outcome_probabilities: &[f64],
) -> f64 {
    if posterior_hypothesis_probs.is_empty()
        || posterior_hypothesis_probs.len() != outcome_probabilities.len()
    {
        return 0.0;
    }
    let prior_h = shannon_entropy_bits(prior_hypothesis_probs);
    let outcome_mass: f64 = outcome_probabilities
        .iter()
        .copied()
        .filter(|p| *p > 0.0)
        .sum();
    if outcome_mass <= f64::EPSILON {
        return 0.0;
    }
    let expected_post_h: f64 = posterior_hypothesis_probs
        .iter()
        .zip(outcome_probabilities.iter().copied())
        .filter(|(_, p)| *p > 0.0)
        .map(|(posterior, p)| (p / outcome_mass) * shannon_entropy_bits(posterior))
        .sum();
    (prior_h - expected_post_h).max(0.0)
}
