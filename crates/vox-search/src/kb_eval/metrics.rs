//! Pure IR metrics over `(qrels, ranked ids)`. BEIR definitions, stated verbatim so the
//! formulas are self-defining.
//!
//! - relevant = qrel score `> 0`
//! - `Recall@k = |relevant ∩ top-k| / |relevant|`
//! - `MRR@k = 1 / (rank of the first relevant id within top-k)`, else 0 (rank is 1-based)
//! - `nDCG@k = DCG / IDCG`, linear gain `rel`, discount `1 / log2(rank + 1)` (rank 1-based);
//!   IDCG is computed from the qrel scores sorted descending; 0 when IDCG is 0
//! - `answer_in_budget`: see [`answer_in_budget`]

use std::collections::{BTreeMap, BTreeSet};

use super::RankedChunk;

/// Context budget (tokens) for [`answer_in_budget`] (AP-11; Phase 7 replaces it per model).
pub const BUDGET_TOKENS: usize = 1500;

/// Token estimator recorded in reports and baselines.
pub const TOKEN_ESTIMATOR: &str = "bytes/4";

/// Qrels for one query: id to graded score.
pub type Qrels = BTreeMap<String, u8>;

/// Round to 6 decimal places.
#[must_use]
pub fn round6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

fn is_rel(rel: &Qrels, id: &str) -> bool {
    rel.get(id).is_some_and(|&s| s > 0)
}

/// `|relevant ∩ top-k| / |relevant|` (0 when nothing is relevant).
#[must_use]
pub fn recall_at_k(rel: &Qrels, ranked: &[String], k: usize) -> f64 {
    let n_rel = rel.values().filter(|&&s| s > 0).count();
    if n_rel == 0 {
        return 0.0;
    }
    let hit = ranked.iter().take(k).filter(|id| is_rel(rel, id)).count();
    hit as f64 / n_rel as f64
}

/// `1 / rank` of the first relevant id within top-k, else 0.
#[must_use]
pub fn mrr_at_k(rel: &Qrels, ranked: &[String], k: usize) -> f64 {
    ranked
        .iter()
        .take(k)
        .position(|id| is_rel(rel, id))
        .map_or(0.0, |p| 1.0 / (p + 1) as f64)
}

/// `DCG / IDCG` with linear gain and discount `1 / log2(rank + 1)`.
#[must_use]
pub fn ndcg_at_k(rel: &Qrels, ranked: &[String], k: usize) -> f64 {
    // `i` is the 0-based position, so rank = i + 1 and rank + 1 = i + 2.
    let disc = |i: usize| 1.0 / ((i + 2) as f64).log2();
    let dcg: f64 = ranked
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, id)| f64::from(*rel.get(id).unwrap_or(&0)) * disc(i))
        .sum();
    let mut ideal: Vec<u8> = rel.values().copied().filter(|&s| s > 0).collect();
    ideal.sort_unstable_by(|a, b| b.cmp(a));
    let idcg: f64 = ideal
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, &s)| f64::from(s) * disc(i))
        .sum();
    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

/// 1.0 when an answer-bearing chunk lies in the longest ranked prefix whose summed
/// `ceil(bytes / 4)` tokens fit `budget_tokens`; else 0.0. The prefix stops at the first
/// chunk that would overflow (prefix semantics, not knapsack).
#[must_use]
pub fn answer_in_budget(
    answer_ids: &BTreeSet<String>,
    ranked: &[RankedChunk],
    budget_tokens: usize,
) -> f64 {
    let mut used = 0usize;
    for c in ranked {
        used += c.bytes.div_ceil(4);
        if used > budget_tokens {
            break;
        }
        if answer_ids.contains(&c.id) {
            return 1.0;
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(pairs: &[(&str, u8)]) -> Qrels {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }
    fn run(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn hand_fixture_three_queries() {
        let q1 = q(&[("d1", 2), ("d3", 1)]);
        let r1 = run(&["d3", "d2", "d1", "d4"]);
        let q2 = q(&[("d5", 1)]);
        let r2 = run(&["x1", "x2", "x3", "x4", "x5", "x6", "d5", "x8", "x9", "x10"]);
        let q3 = q(&[("d8", 1), ("d9", 1)]);
        let r3 = run(&["y1", "y2", "y3", "y4", "y5", "y6", "y7", "y8", "y9", "y10"]);
        let cases = [(&q1, &r1), (&q2, &r2), (&q3, &r3)];

        let mean = |f: &dyn Fn(&Qrels, &[String]) -> f64| {
            round6(cases.iter().map(|(qr, r)| f(qr, r)).sum::<f64>() / 3.0)
        };
        assert_eq!(round6(ndcg_at_k(&q1, &r1, 10)), 0.760188);
        assert_eq!(round6(mrr_at_k(&q2, &r2, 10)), 0.142857);
        assert_eq!(round6(ndcg_at_k(&q2, &r2, 10)), 0.333333);
        assert_eq!(mean(&|a, b| recall_at_k(a, b, 5)), 0.333333);
        assert_eq!(mean(&|a, b| recall_at_k(a, b, 10)), 0.666667);
        assert_eq!(mean(&|a, b| mrr_at_k(a, b, 10)), 0.380952);
        assert_eq!(mean(&|a, b| ndcg_at_k(a, b, 10)), 0.364507);
    }

    fn rc(id: &str, bytes: usize) -> RankedChunk {
        RankedChunk {
            id: id.to_string(),
            bytes,
        }
    }

    #[test]
    fn answer_in_budget_prefix_stops_at_first_overflow() {
        let ans: BTreeSet<String> = ["b".to_string()].into();
        // tokens 1000, 600, 200: cumulative 1000, 1600 (overflow), so rank 2 is out.
        let ranked = [rc("a", 4000), rc("b", 2400), rc("c", 800)];
        assert_eq!(
            answer_in_budget(&["a".to_string()].into(), &ranked, 1500),
            1.0
        );
        assert_eq!(answer_in_budget(&ans, &ranked, 1500), 0.0);
        // Prefix semantics: "c" fits on its own but sits behind the overflow.
        assert_eq!(
            answer_in_budget(&["c".to_string()].into(), &ranked, 1500),
            0.0
        );
        // First chunk alone overflows (2000 tokens).
        let big = [rc("a", 8000)];
        assert_eq!(answer_in_budget(&["a".to_string()].into(), &big, 1500), 0.0);
    }
}
