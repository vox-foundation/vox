//! Near-miss detection for facet keys (WR-03): `Facets` ignores unknown keys by design, so a
//! typo such as `training_eligble: false` would silently leave a page eligible and unfaceted.

use crate::frontmatter::frontmatter_yaml;

/// Every key `Facets` reads (kept equal to the struct by a test against the generated schema).
pub const FACET_KEYS: &[&str] = &[
    "topic",
    "domain",
    "kb_tier",
    "trust_tier",
    "scope",
    "review_by",
    "verified_at",
    "sources",
    "authorship",
    "authorship_model",
    "origin",
    "human_attested_by",
    "supersedes",
    "superseded_by",
    "sensitivity",
    "training_eligible",
];

/// The keys whose typo changes MENS eligibility; a near-miss of one fails closed.
pub const ELIGIBILITY_KEYS: &[&str] = &[
    "training_eligible",
    "human_attested_by",
    "origin",
    "authorship",
];

/// An unknown frontmatter key that looks like a misspelling of a known facet key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyNearMiss {
    /// The key as written.
    pub key: String,
    /// The facet key it most likely meant.
    pub suggestion: &'static str,
}

impl KeyNearMiss {
    /// True when the intended key decides MENS eligibility.
    #[must_use]
    pub fn guards_eligibility(&self) -> bool {
        ELIGIBILITY_KEYS.contains(&self.suggestion)
    }
}

// vox:defactored-from vox-doc-pipeline 2026-10-11 (levenshtein, ~12 lines; L0 cannot depend up)
fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push(
                (prev[j] + usize::from(ca != *cb))
                    .min(prev[j + 1] + 1)
                    .min(cur[j] + 1),
            );
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The facet key `key` is a near-miss of: equal ignoring case and `-` vs `_`, or within one edit
/// (two for keys of 10+ characters). `None` for a known facet key or an unrelated key.
fn near_miss_of(key: &str) -> Option<&'static str> {
    if FACET_KEYS.contains(&key) {
        return None;
    }
    let norm = key.trim().to_ascii_lowercase().replace('-', "_");
    if let Some(k) = FACET_KEYS.iter().find(|k| **k == norm) {
        return Some(k);
    }
    FACET_KEYS
        .iter()
        .map(|k| (levenshtein(&norm, k), *k))
        .filter(|(d, k)| *d <= if k.len() >= 10 { 2 } else { 1 })
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k)
}

/// Unknown top-level frontmatter keys that look like misspelled facet keys. Empty when there is
/// no closed frontmatter block or it is not a mapping (`parse_facets` reports those).
#[must_use]
pub fn facet_key_near_misses(doc_text: &str) -> Vec<KeyNearMiss> {
    let Some(yaml) = frontmatter_yaml(doc_text) else {
        return Vec::new();
    };
    let Ok(serde_yaml::Value::Mapping(map)) = serde_yaml::from_str::<serde_yaml::Value>(yaml)
    else {
        return Vec::new();
    };
    map.keys()
        .filter_map(serde_yaml::Value::as_str)
        .filter_map(|key| {
            near_miss_of(key).map(|suggestion| KeyNearMiss {
                key: key.to_string(),
                suggestion,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nm(fm: &str) -> Vec<(String, &'static str)> {
        facet_key_near_misses(&format!("---\n{fm}\n---\n# T\n"))
            .into_iter()
            .map(|m| (m.key, m.suggestion))
            .collect()
    }

    #[test]
    fn typos_and_spelling_variants_of_facet_keys_are_flagged() {
        for (typo, want) in [
            ("training_eligble", "training_eligible"),
            ("human-attested-by", "human_attested_by"),
            ("Origin", "origin"),
            ("authorship_model_", "authorship_model"),
            ("authorshp", "authorship"),
            ("human_atested_by", "human_attested_by"),
            ("topc", "topic"),
            ("Training-Eligible", "training_eligible"),
        ] {
            assert_eq!(
                nm(&format!("{typo}: x")),
                [(typo.to_string(), want)],
                "{typo}"
            );
        }
    }

    #[test]
    fn known_and_unrelated_keys_are_not_flagged() {
        let fm = "title: T\ncategory: C\nstatus: s\nschema_type: x\nrelated: [a]\nsort_order: 3\n\
                  training_rationale: why\nvox_relevance: 1\nsourced_at: 2026-01-01\naudience: a\n\
                  keywords: [k]\nauthored: a\nauthors: [a]\ndate: d\ntier: t\nsee_also: s\n\
                  topic: t\ndomain: d\nsupersedes: [p]\nsuperseded_by: q\ntraining_eligible: true";
        assert!(nm(fm).is_empty(), "{:?}", nm(fm));
    }

    #[test]
    fn absent_unterminated_or_non_mapping_frontmatter_has_no_near_misses() {
        assert!(facet_key_near_misses("# T\n").is_empty());
        assert!(facet_key_near_misses("---\ntraining_eligble: false\n# T\n").is_empty());
        assert!(facet_key_near_misses("---\n- a\n---\n").is_empty());
    }

    #[test]
    fn eligibility_guard_covers_exactly_the_four_keys() {
        let guards = |fm: &str| {
            facet_key_near_misses(&format!("---\n{fm}: x\n---\n"))
                .iter()
                .any(KeyNearMiss::guards_eligibility)
        };
        assert!(guards("training_eligble") && guards("human-attested-by"));
        assert!(guards("Origin") && guards("authorshp"));
        assert!(!guards("topc") && !guards("authorship_model_"));
    }
}
