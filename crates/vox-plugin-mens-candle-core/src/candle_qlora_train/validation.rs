//! Hold-out validation pass re-export (delegates to training_loop/validation.rs).
//!
//! SP3-C: kept as thin wrapper for symmetry with vox-populi module layout.

use rand::SeedableRng;
use rand::seq::SliceRandom;
use vox_tensor::data::TrainingPair;

/// Hold out roughly `val_count` rows, grouped by answer text, so rows that share a
/// response (paraphrased prompts, augmentation variants) never straddle the split.
/// A random row split leaked ~75% of the vox-lang validation set into training,
/// making `val_loss` a memorization score. Returns `(train, eval)`.
pub fn split_validation_by_response(
    pairs: Vec<TrainingPair>,
    val_count: usize,
    seed: u64,
) -> (Vec<TrainingPair>, Vec<TrainingPair>) {
    use std::collections::HashMap;
    use std::hash::{DefaultHasher, Hash, Hasher};
    if val_count == 0 || pairs.len() <= val_count {
        return (pairs, Vec::new());
    }
    let key = |p: &TrainingPair| {
        let mut h = DefaultHasher::new();
        match (&p.response, &p.messages) {
            (Some(r), _) => r.trim().hash(&mut h),
            (None, Some(turns)) => turns.last().map(|t| t.content.trim()).hash(&mut h),
            (None, None) => p.prompt.hash(&mut h),
        }
        h.finish()
    };
    let mut groups: HashMap<u64, Vec<TrainingPair>> = HashMap::new();
    for p in pairs {
        groups.entry(key(&p)).or_default().push(p);
    }
    let mut groups: Vec<(u64, Vec<TrainingPair>)> = groups.into_iter().collect();
    groups.sort_by_key(|(k, _)| *k); // HashMap order is random; make the split seed-deterministic
    groups.shuffle(&mut rand::rngs::StdRng::seed_from_u64(seed));
    let (mut train, mut eval) = (Vec::new(), Vec::new());
    for (_, g) in groups {
        if eval.len() < val_count {
            eval.extend(g);
        } else {
            train.extend(g);
        }
    }
    (train, eval)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<TrainingPair> {
        (0..60)
            .map(|i| TrainingPair {
                prompt: Some(format!("p{i}")),
                response: Some(format!("r{}", i % 20)),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn rows_sharing_a_response_stay_on_one_side() {
        let (train, eval) = split_validation_by_response(rows(), 9, 7);
        assert_eq!(train.len() + eval.len(), 60);
        assert!(eval.len() >= 9);
        let eval_resp: std::collections::HashSet<_> =
            eval.iter().map(|p| p.response.clone()).collect();
        assert!(
            train.iter().all(|p| !eval_resp.contains(&p.response)),
            "response leaked across split"
        );
        let (_, again) = split_validation_by_response(rows(), 9, 7);
        assert_eq!(eval.len(), again.len(), "split must be seed-deterministic");
    }
}
