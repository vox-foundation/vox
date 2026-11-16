//! Tier 0 deterministic lexical, negation, directional-antonym, numeric, and `statcheck`
//! pre-filter for calibrated claim verification (`VERI-01`, `VERI-02`).

#[rustfmt::skip]
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "for", "that", "with", "this", "from", "are", "was", "were",
    "has", "have", "had", "into", "over", "under", "when", "where", "which", "while", "than",
    "then", "also", "such", "only", "both", "each", "some", "very", "can", "will", "may", "by",
    "to", "in", "on", "of", "at", "as", "is", "it", "be",
];

#[rustfmt::skip]
const NEGATION_PHRASES: &[&str] = &[
    "does not", "do not", "did not", "is not", "are not", "was not", "were not", "can not",
    "will not", "has not", "have not", "had not", "fails to", "failed to",
    "not statistically significant", "no statistically significant",
];

#[rustfmt::skip]
const NEGATION_TOKENS: &[&str] = &[
    "not", "no", "never", "without", "none", "neither", "nothing", "fails", "failed", "cannot",
    "can't", "doesn't", "don't", "didn't", "isn't", "aren't", "wasn't", "weren't", "won't",
    "hasn't", "haven't", "hadn't", "impossible", "unsupported",
];

#[rustfmt::skip]
const DEC_WORDS: &[&str] = &[
    "reduce", "reduces", "reduced", "reducing", "reduction", "decrease", "decreases", "decreased",
    "decreasing", "drop", "drops", "dropped", "dropping", "lower", "lowers", "lowered", "lowering",
    "fell", "fall", "falls", "shrink", "shrunk",
];

#[rustfmt::skip]
const INC_WORDS: &[&str] = &[
    "increase", "increases", "increased", "increasing", "raise", "raises", "raised", "raising",
    "higher", "grew", "grow", "grows", "growing", "rose", "rise", "rises", "rising", "expand",
    "expanded",
];

#[rustfmt::skip]
const FAST_WORDS: &[&str] = &["faster", "accelerate", "accelerated", "accelerates", "speedup"];
#[rustfmt::skip]
const SLOW_WORDS: &[&str] = &["slower", "slowdown", "decelerate", "decelerated", "decelerates"];

#[rustfmt::skip]
const POS_WORDS: &[&str] = &[
    "improve", "improves", "improved", "improving", "improvement", "pass", "passes", "passed",
    "passing", "succeed", "succeeded", "succeeds",
];

#[rustfmt::skip]
const NEG_WORDS: &[&str] = &[
    "degrade", "degrades", "degraded", "degrading", "degradation", "regress", "regressed",
    "regresses", "fail", "fails", "failed", "failing", "failure", "worse", "worsen", "worsened",
];

#[rustfmt::skip]
const ALLOW_WORDS: &[&str] = &[
    "enable", "enabled", "enables", "allow", "allowed", "allows", "accept", "accepted",
];

#[rustfmt::skip]
const DENY_WORDS: &[&str] = &[
    "disable", "disabled", "disables", "reject", "rejected", "rejects", "forbid", "forbidden",
];

const ANTONYM_GROUPS: &[(&[&str], &[&str])] = &[
    (DEC_WORDS, INC_WORDS),
    (FAST_WORDS, SLOW_WORDS),
    (POS_WORDS, NEG_WORDS),
    (ALLOW_WORDS, DENY_WORDS),
];

/// Deterministic Tier 0 lexical & conflict evaluation output.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier0Score {
    pub support_score: f64,
    pub contradiction_score: f64,
    pub has_negation_conflict: bool,
    pub has_antonym_conflict: bool,
    pub has_numeric_conflict: bool,
    pub has_statcheck_conflict: bool,
    pub token_overlap: f64,
    pub anchor_overlap: f64,
}

fn normalize_word(w: &str) -> String {
    w.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '\'' && c != '.')
        .trim_matches('.')
        .to_ascii_lowercase()
}

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(normalize_word)
        .filter(|w| !w.is_empty())
        .collect()
}

fn word_matches(a: &str, b: &str) -> bool {
    a == b || (a.len() >= 4 && b.len() >= 4 && (a.starts_with(b) || b.starts_with(a)))
}

fn is_numeric_measurement(tok: &str) -> bool {
    tok.chars().any(|c| c.is_ascii_digit())
        && tok.chars().next().is_some_and(|c| c.is_ascii_digit())
}

fn is_antonym_token(tok: &str) -> bool {
    ANTONYM_GROUPS
        .iter()
        .any(|(left, right)| left.contains(&tok) || right.contains(&tok))
}

fn is_negation_token(tok: &str) -> bool {
    NEGATION_TOKENS.contains(&tok)
}

/// Returns `true` if `text` contains a surface negation token or phrase.
#[must_use]
pub fn contains_negation(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if NEGATION_PHRASES.iter().any(|p| lower.contains(p)) {
        return true;
    }
    tokenize(text).iter().any(|t| is_negation_token(t.as_str()))
}

/// Returns `true` if `claim` and `sentence` use opposite-polarity directional antonyms.
#[must_use]
pub fn has_directional_antonym_flip(claim: &str, sentence: &str) -> bool {
    let claim_toks = tokenize(claim);
    let sent_toks = tokenize(sentence);
    for (left, right) in ANTONYM_GROUPS {
        let c_left = claim_toks.iter().any(|t| left.contains(&t.as_str()));
        let c_right = claim_toks.iter().any(|t| right.contains(&t.as_str()));
        let s_left = sent_toks.iter().any(|t| left.contains(&t.as_str()));
        let s_right = sent_toks.iter().any(|t| right.contains(&t.as_str()));
        if (c_left && !c_right && s_right && !s_left) || (c_right && !c_left && s_left && !s_right)
        {
            return true;
        }
    }
    false
}

fn extract_p_values(text: &str) -> Vec<(char, f64)> {
    let chars: Vec<char> = text.to_ascii_lowercase().chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let prev_boundary = i == 0 || !chars[i - 1].is_ascii_alphanumeric();
        if prev_boundary && chars[i] == 'p' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && matches!(chars[j], '<' | '>' | '=') {
                let op = chars[j];
                j += 1;
                if j < chars.len() && chars[j] == '=' {
                    j += 1;
                }
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                let start = j;
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                    j += 1;
                }
                if j > start
                    && let Ok(val) = chars[start..j].iter().collect::<String>().parse::<f64>()
                {
                    out.push((op, val));
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `statcheck`-style statistical consistency check between `claim` and `sentence`.
#[must_use]
pub fn has_statcheck_conflict(claim: &str, sentence: &str) -> bool {
    let claim_lower = claim.to_ascii_lowercase();
    let sent_lower = sentence.to_ascii_lowercase();
    let claim_p = extract_p_values(&claim_lower);
    let sent_p = extract_p_values(&sent_lower);

    let claims_sig = claim_lower.contains("statistically significant")
        && !claim_lower.contains("not statistically significant")
        && !claim_lower.contains("no statistically significant");

    if claims_sig {
        if claim_p
            .iter()
            .chain(sent_p.iter())
            .any(|&(op, v)| op == '>' || v > 0.05)
        {
            return true;
        }
        if sent_lower.contains("not statistically significant")
            || sent_lower.contains("no statistically significant")
        {
            return true;
        }
    }

    for &(c_op, c_val) in &claim_p {
        for &(s_op, s_val) in &sent_p {
            if (c_op == '<' && c_val <= 0.05) && (s_op == '>' || s_val > 0.05) {
                return true;
            }
            if (c_op == '=' || s_op == '=') && (c_val - s_val).abs() > 1e-4 {
                return true;
            }
        }
    }
    false
}

fn split_context_sentences(context: &str) -> Vec<String> {
    let chars: Vec<(usize, char)> = context.char_indices().collect();
    let mut sentences = Vec::new();
    let mut start = 0usize;
    for idx in 0..chars.len() {
        let (byte_pos, ch) = chars[idx];
        let is_break = match ch {
            '!' | '?' | '\n' => true,
            '.' => {
                let prev_digit = idx > 0 && chars[idx - 1].1.is_ascii_digit();
                let next_digit = chars.get(idx + 1).is_some_and(|(_, c)| c.is_ascii_digit());
                !(prev_digit && next_digit)
            }
            _ => false,
        };
        if is_break {
            let end = byte_pos + ch.len_utf8();
            let s = context[start..end].trim();
            if !s.is_empty() {
                sentences.push(s.to_string());
            }
            start = end;
        }
    }
    let tail = context[start..].trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }
    sentences
}

/// Evaluate a claim against grounding `context` using pure Tier 0 lexical, negation,
/// antonym, numeric, and `statcheck` rules.
#[must_use]
pub fn evaluate_tier0_lexical(claim: &str, context: &str) -> Tier0Score {
    let claim_tokens = tokenize(claim);
    if claim_tokens.is_empty() || context.trim().is_empty() {
        return Tier0Score {
            support_score: 0.0,
            contradiction_score: 0.0,
            has_negation_conflict: false,
            has_antonym_conflict: false,
            has_numeric_conflict: false,
            has_statcheck_conflict: false,
            token_overlap: 0.0,
            anchor_overlap: 0.0,
        };
    }

    let claim_anchors: Vec<&str> = claim_tokens
        .iter()
        .map(String::as_str)
        .filter(|t| {
            !STOP_WORDS.contains(t)
                && !is_numeric_measurement(t)
                && !is_antonym_token(t)
                && !is_negation_token(t)
        })
        .collect();
    let claim_numbers: Vec<&str> = claim_tokens
        .iter()
        .map(String::as_str)
        .filter(|t| is_numeric_measurement(t))
        .collect();

    let mut candidates = split_context_sentences(context);
    if candidates.is_empty() {
        candidates.push(context.trim().to_string());
    }

    let score_sentence = |sentence: &str| -> (f64, f64) {
        let sent_tokens = tokenize(sentence);
        if sent_tokens.is_empty() {
            return (0.0, 0.0);
        }
        let matched_all = claim_tokens
            .iter()
            .filter(|ct| sent_tokens.iter().any(|st| word_matches(ct, st)))
            .count();
        let tok_overlap = matched_all as f64 / claim_tokens.len() as f64;
        let anc_overlap = if claim_anchors.is_empty() {
            tok_overlap
        } else {
            let matched_anc = claim_anchors
                .iter()
                .filter(|ca| sent_tokens.iter().any(|st| word_matches(ca, st)))
                .count();
            matched_anc as f64 / claim_anchors.len() as f64
        };
        (tok_overlap, anc_overlap)
    };

    let mut best_sentence = candidates[0].as_str();
    let (mut token_overlap, mut anchor_overlap) = score_sentence(best_sentence);
    for cand in &candidates[1..] {
        let (tok_ov, anc_ov) = score_sentence(cand);
        if (anc_ov + tok_ov) > (anchor_overlap + token_overlap) {
            token_overlap = tok_ov;
            anchor_overlap = anc_ov;
            best_sentence = cand.as_str();
        }
    }

    let sent_tokens = tokenize(best_sentence);
    let has_negation_conflict =
        contains_negation(claim) != contains_negation(best_sentence) && anchor_overlap >= 0.50;
    let has_antonym_conflict =
        has_directional_antonym_flip(claim, best_sentence) && anchor_overlap >= 0.50;
    let has_statcheck = has_statcheck_conflict(claim, best_sentence);

    let context_tokens = tokenize(context);
    let missing_number_in_best = !claim_numbers.is_empty()
        && claim_numbers
            .iter()
            .any(|n| !sent_tokens.iter().any(|st| st == n));
    let missing_number_in_context = !claim_numbers.is_empty()
        && claim_numbers
            .iter()
            .any(|n| !context_tokens.iter().any(|ct| ct == n));
    let sent_has_any_number = sent_tokens.iter().any(|t| is_numeric_measurement(t));

    let has_numeric_conflict = missing_number_in_context
        || (missing_number_in_best && sent_has_any_number && anchor_overlap >= 0.50);

    let clear_contradiction = has_negation_conflict
        || has_antonym_conflict
        || has_statcheck
        || (has_numeric_conflict && anchor_overlap >= 0.50);

    let (support_score, contradiction_score) = if clear_contradiction {
        let contra = (0.85 + 0.15 * anchor_overlap).clamp(0.85, 1.0);
        let sup = (0.10 * (1.0 - anchor_overlap)).clamp(0.0, 0.15);
        (sup, contra)
    } else if has_numeric_conflict {
        let sup = (0.35 * token_overlap).clamp(0.0, 0.45);
        (sup, 0.65)
    } else if token_overlap >= 0.75 {
        let sup = (0.85 + 0.15 * token_overlap).clamp(0.85, 1.0);
        (sup, 0.0)
    } else {
        let sup = (0.75 * token_overlap).clamp(0.0, 0.74);
        (sup, 0.0)
    };

    Tier0Score {
        support_score,
        contradiction_score,
        has_negation_conflict,
        has_antonym_conflict,
        has_numeric_conflict,
        has_statcheck_conflict: has_statcheck,
        token_overlap,
        anchor_overlap,
    }
}
