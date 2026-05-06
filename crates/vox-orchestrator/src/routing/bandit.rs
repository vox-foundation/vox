//! Thompson sampling helper: Beta(α, β) via independent Gamma samples.

use rand::Rng;
use rand_distr::{Distribution, Gamma};

/// Posterior sample for Binomial successes/failures with Beta(1,1) prior → Beta(s+1, f+1).
#[must_use]
pub fn sample_beta_thompson<R: Rng>(rng: &mut R, successes: u32, failures: u32) -> f64 {
    let a = successes as f64 + 1.0;
    let b = failures as f64 + 1.0;
    let ga = Gamma::new(a, 1.0).expect("gamma a");
    let gb = Gamma::new(b, 1.0).expect("gamma b");
    let x: f64 = ga.sample(rng);
    let y: f64 = gb.sample(rng);
    let d = x + y;
    if d <= f64::EPSILON {
        0.5
    } else {
        x / d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn thompson_in_unit_interval() {
        let mut r = StdRng::seed_from_u64(42);
        for _ in 0..100 {
            let v = sample_beta_thompson(&mut r, 2, 3);
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
