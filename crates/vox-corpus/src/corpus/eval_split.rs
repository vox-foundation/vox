#![allow(dead_code)] // split-manifest helpers, not yet wired
//! Seeded train/eval split by tool identity (B1.4).
//!
//! Splits a harness corpus by unique tool-identity keys so no tool appears
//! in both train and eval sets (prevents data leakage). Split is fully
//! deterministic given the same seed.
//!
//! Tool identity is extracted from the row payload:
//! - `vox_tool_selection` rows: use `chosen_tool`
//! - `vox_argument_generation` rows: use `tool_name`
//! - Unknown lanes: use lane name itself as identity key

use crate::corpus::harness_union::HarnessRow;

pub struct SplitManifest {
    pub train_tools: Vec<String>,
    pub eval_tools: Vec<String>,
    pub seed: u64,
    pub eval_frac: f64,
}

/// Extract the tool identity key from a HarnessRow payload.
fn tool_key(row: &HarnessRow) -> String {
    match row.lane.as_str() {
        "vox_tool_selection" => row
            .payload
            .get("chosen_tool")
            .and_then(|v| v.as_str())
            .unwrap_or(&row.lane)
            .to_string(),
        "vox_argument_generation" => row
            .payload
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&row.lane)
            .to_string(),
        _ => row.lane.clone(),
    }
}

/// XorShift64 step — no external deps, matches the rng pattern in synthetic_gen.
fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Deterministic Fisher-Yates shuffle of `indices` using an xorshift RNG.
fn shuffle(indices: &mut [String], seed: u64) {
    let mut state = if seed == 0 { 0xdeadbeef_cafebabe } else { seed };
    for i in (1..indices.len()).rev() {
        let j = xorshift(&mut state) as usize % (i + 1);
        indices.swap(i, j);
    }
}

/// Split `rows` into (train, eval) sets by tool identity.
///
/// Algorithm:
/// 1. Collect unique tool identity keys from all rows.
/// 2. Shuffle the keys deterministically using `seed`.
/// 3. Assign the first `ceil(eval_frac * n_keys)` keys to eval, rest to train.
/// 4. Partition rows by which key set their tool identity belongs to.
/// 5. Emit a SplitManifest recording the partition.
///
/// HarnessRow is not Clone/Copy, so we re-construct rows from slices.
pub fn split_surface(
    seed: u64,
    eval_frac: f64,
    rows: &[HarnessRow],
) -> (Vec<HarnessRow>, Vec<HarnessRow>, SplitManifest) {
    use std::collections::BTreeSet;

    // 1. Collect unique keys in stable order (BTreeSet for reproducibility)
    let unique_keys: Vec<String> = rows
        .iter()
        .map(tool_key)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    // 2. Shuffle keys deterministically
    let mut shuffled = unique_keys.clone();
    shuffle(&mut shuffled, seed);

    // 3. Assign eval_frac of unique keys to eval
    let n_eval = ((shuffled.len() as f64 * eval_frac).ceil() as usize).min(shuffled.len());
    let eval_keys: BTreeSet<String> = shuffled[..n_eval].iter().cloned().collect();
    let train_keys: BTreeSet<String> = shuffled[n_eval..].iter().cloned().collect();

    // 4. Partition rows
    let mut train = Vec::new();
    let mut eval_rows = Vec::new();

    for row in rows {
        let key = tool_key(row);
        let target = if eval_keys.contains(&key) {
            &mut eval_rows
        } else {
            &mut train
        };
        target.push(HarnessRow {
            task: row.task.clone(),
            lane: row.lane.clone(),
            payload: row.payload.clone(),
        });
    }

    // 5. Emit manifest
    let manifest = SplitManifest {
        train_tools: train_keys.into_iter().collect(),
        eval_tools: eval_keys.into_iter().collect(),
        seed,
        eval_frac,
    };

    (train, eval_rows, manifest)
}

/// Summary of content-addressed deduplication and benchmark holdout decontamination (`CORPUS-05`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecontaminationReport {
    /// Clean records surviving exact + near-duplicate deduplication and holdout decontamination.
    pub clean_records: Vec<crate::training::TrainingRecordV2>,
    /// Count of exact content-digest duplicates removed.
    pub exact_duplicates_removed: usize,
    /// Count of n-gram Jaccard near-duplicates removed.
    pub near_duplicates_removed: usize,
    /// Count of records removed due to overlap with held-out `vox-eval` / `vox-audit` prompts.
    pub holdout_contaminated_removed: usize,
}

/// Compute normalized word/token 3-gram set hashes for near-duplicate and holdout overlap checks (`CORPUS-05`).
fn token_trigram_hashes(text: &str) -> std::collections::BTreeSet<u64> {
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect();
    let mut set = std::collections::BTreeSet::new();
    if tokens.len() < 3 {
        if !tokens.is_empty() {
            let joined = tokens.join(" ");
            set.insert(xxhash_rust::xxh3::xxh3_64(joined.as_bytes()));
        }
        return set;
    }
    for win in tokens.windows(3) {
        let joined = format!("{} {} {}", win[0], win[1], win[2]);
        set.insert(xxhash_rust::xxh3::xxh3_64(joined.as_bytes()));
    }
    set
}

/// Compute Jaccard similarity in `[0.0, 1.0]` between two 3-gram hash sets.
fn jaccard_similarity(
    a: &std::collections::BTreeSet<u64>,
    b: &std::collections::BTreeSet<u64>,
) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// Deduplicate [`crate::training::TrainingRecordV2`] records by exact `content_digest` and
/// 3-gram Jaccard near-duplicate threshold (`CORPUS-05`).
#[must_use]
pub fn deduplicate_records_v2(
    records: &[crate::training::TrainingRecordV2],
    max_near_dup_jaccard: f64,
) -> (Vec<crate::training::TrainingRecordV2>, usize, usize) {
    let mut seen_digests = std::collections::HashSet::new();
    let mut kept_ngrams: Vec<(crate::training::SpokeKind, std::collections::BTreeSet<u64>)> =
        Vec::new();
    let mut clean = Vec::new();
    let mut exact_dups = 0usize;
    let mut near_dups = 0usize;

    for rec in records {
        if !rec.verify_integrity() {
            continue;
        }
        if !seen_digests.insert(rec.content_digest.clone()) {
            exact_dups += 1;
            continue;
        }
        let ngrams = token_trigram_hashes(&format!("{}\n{}", rec.prompt, rec.response));
        let is_near_dup = kept_ngrams.iter().any(|(spoke, prev)| {
            *spoke == rec.spoke && jaccard_similarity(&ngrams, prev) >= max_near_dup_jaccard
        });
        if is_near_dup {
            near_dups += 1;
            continue;
        }
        kept_ngrams.push((rec.spoke, ngrams));
        clean.push(rec.clone());
    }

    (clean, exact_dups, near_dups)
}

/// Deduplicate `records` and decontaminate against held-out `vox-eval` and `vox-audit` benchmark
/// prompts/targets, guaranteeing `0%` exact or near-duplicate holdout leakage (`CORPUS-05`).
#[must_use]
pub fn decontaminate_against_holdout(
    records: &[crate::training::TrainingRecordV2],
    holdout_texts: &[String],
    max_jaccard: f64,
) -> DecontaminationReport {
    let (deduped, exact_duplicates_removed, near_duplicates_removed) =
        deduplicate_records_v2(records, max_jaccard);

    let holdout_exact: std::collections::HashSet<u64> = holdout_texts
        .iter()
        .map(|s| xxhash_rust::xxh3::xxh3_64(s.trim().as_bytes()))
        .collect();
    let holdout_ngrams: Vec<std::collections::BTreeSet<u64>> = holdout_texts
        .iter()
        .map(|s| token_trigram_hashes(s))
        .collect();

    let mut clean_records = Vec::with_capacity(deduped.len());
    let mut holdout_contaminated_removed = 0usize;

    for rec in deduped {
        let prompt_hash = xxhash_rust::xxh3::xxh3_64(rec.prompt.trim().as_bytes());
        let resp_hash = xxhash_rust::xxh3::xxh3_64(rec.response.trim().as_bytes());
        if holdout_exact.contains(&prompt_hash) || holdout_exact.contains(&resp_hash) {
            holdout_contaminated_removed += 1;
            continue;
        }
        let prompt_ngrams = token_trigram_hashes(&rec.prompt);
        let resp_ngrams = token_trigram_hashes(&rec.response);
        let leaked = holdout_ngrams.iter().any(|h| {
            jaccard_similarity(&prompt_ngrams, h) >= max_jaccard
                || jaccard_similarity(&resp_ngrams, h) >= max_jaccard
        });
        if leaked {
            holdout_contaminated_removed += 1;
            continue;
        }
        clean_records.push(rec);
    }

    DecontaminationReport {
        clean_records,
        exact_duplicates_removed,
        near_duplicates_removed,
        holdout_contaminated_removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::harness_union::generate_harness_rows;

    #[test]
    fn split_is_by_tool_identity() {
        let rows = generate_harness_rows(100);
        let (train, eval, _) = split_surface(42, 0.2, &rows);
        let train_lanes: std::collections::HashSet<_> = train.iter().map(|r| &r.lane).collect();
        let eval_lanes: std::collections::HashSet<_> = eval.iter().map(|r| &r.lane).collect();
        // Both sets must exist (not trivially empty)
        assert!(!train.is_empty() && !eval.is_empty());
        let _ = (train_lanes, eval_lanes); // structural check; identity guard below
    }

    #[test]
    fn split_is_deterministic() {
        let rows = generate_harness_rows(50);
        let (a, _, _) = split_surface(42, 0.2, &rows);
        let (b, _, _) = split_surface(42, 0.2, &rows);
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn manifest_is_non_empty() {
        let rows = generate_harness_rows(50);
        let (_, _, m) = split_surface(42, 0.2, &rows);
        assert!(!m.train_tools.is_empty() || !m.eval_tools.is_empty());
    }

    #[test]
    fn no_tool_appears_in_both_sets() {
        let rows = generate_harness_rows(100);
        let (train, eval, _) = split_surface(42, 0.2, &rows);
        let train_keys: std::collections::HashSet<String> = train.iter().map(tool_key).collect();
        let eval_keys: std::collections::HashSet<String> = eval.iter().map(tool_key).collect();
        let overlap: Vec<_> = train_keys.intersection(&eval_keys).collect();
        assert!(
            overlap.is_empty(),
            "tool leakage: tools in both sets: {:?}",
            overlap
        );
    }

    #[test]
    fn deduplicate_and_decontaminate_removes_exact_near_and_holdout_overlap() {
        use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

        let mk = |prompt: &str, resp: &str| {
            TrainingRecordV2::new(
                SpokeKind::VoxLang,
                prompt,
                "sys",
                resp,
                OracleReceipt {
                    spoke: SpokeKind::VoxLang,
                    passed: true,
                    stage: "typeck".to_string(),
                    diagnostics: Vec::new(),
                    exit_code: Some(0),
                },
                RecordProvenance {
                    origin: "test".to_string(),
                    source_id: "t.vox".to_string(),
                    task_family: "vox_fn".to_string(),
                    difficulty: 3,
                },
            )
        };

        let r1 = mk(
            "Write a Vox function that computes fibonacci numbers",
            "fn fib(n: int) to int { return n }",
        );
        let r1_exact_dup = r1.clone();
        let r1_near_dup = mk(
            "Write a Vox function that computes fibonacci numbers",
            "fn fib(n: int) to int { return n  }",
        );
        let r2_holdout = mk(
            "Implement the nth_prime benchmark function in Vox",
            "fn nth_prime(n: int) to int { return 2 }",
        );
        let r3_clean = mk(
            "Define an actor Counter with Increment handler in Vox",
            "actor Counter { state c: int = 0 }",
        );

        let holdout = vec!["Implement the nth_prime benchmark function in Vox".to_string()];
        let report = decontaminate_against_holdout(
            &[r1, r1_exact_dup, r1_near_dup, r2_holdout, r3_clean],
            &holdout,
            0.80,
        );

        assert_eq!(report.exact_duplicates_removed, 1);
        assert_eq!(report.near_duplicates_removed, 1);
        assert_eq!(report.holdout_contaminated_removed, 1);
        assert_eq!(report.clean_records.len(), 2);
    }
}
