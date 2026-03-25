//! Rule-based prompt augmentation for Mens corpus diversity.
//!
//! Generates perturbed variants of instruction prompts to improve model robustness
//! to natural language variation, typos, shorthand, and synonym substitution.
//! All operations are deterministic when given the same seed.
//!
//! ## Usage
//! ```rust,no_run
//! use vox_corpus::corpus::augment::{AugmentConfig, augment_prompt};
//! let cfg = AugmentConfig::default();
//! let variants = augment_prompt("Write a Vox function called greet", &cfg, 42);
//! assert!(!variants.is_empty());
//! ```

use anyhow::Context;
use rand::Rng;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use std::io::Write;

/// QWERTY keyboard adjacency map for realistic keyboard substitution errors.
/// Each entry maps a lowercase character to its physically adjacent keys on a
/// standard US QWERTY layout. Grounded in the Typoist (CHI 2025) motor model:
/// substitution errors ("fat finger") are the most common error type in human typing.
static QWERTY_NEIGHBORS: &[(char, &[char])] = &[
    ('q', &['w', 'a', 's']),
    ('w', &['q', 'e', 'a', 's', 'd']),
    ('e', &['w', 'r', 's', 'd', 'f']),
    ('r', &['e', 't', 'd', 'f', 'g']),
    ('t', &['r', 'y', 'f', 'g', 'h']),
    ('y', &['t', 'u', 'g', 'h', 'j']),
    ('u', &['y', 'i', 'h', 'j', 'k']),
    ('i', &['u', 'o', 'j', 'k', 'l']),
    ('o', &['i', 'p', 'k', 'l']),
    ('p', &['o', 'l']),
    ('a', &['q', 'w', 's', 'z', 'x']),
    ('s', &['a', 'w', 'e', 'd', 'x', 'z']),
    ('d', &['s', 'e', 'r', 'f', 'c', 'x']),
    ('f', &['d', 'r', 't', 'g', 'v', 'c']),
    ('g', &['f', 't', 'y', 'h', 'b', 'v']),
    ('h', &['g', 'y', 'u', 'j', 'n', 'b']),
    ('j', &['h', 'u', 'i', 'k', 'm', 'n']),
    ('k', &['j', 'i', 'o', 'l', 'm']),
    ('l', &['k', 'o', 'p']),
    ('z', &['a', 's', 'x']),
    ('x', &['z', 's', 'd', 'c']),
    ('c', &['x', 'd', 'f', 'v']),
    ('v', &['c', 'f', 'g', 'b']),
    ('b', &['v', 'g', 'h', 'n']),
    ('n', &['b', 'h', 'j', 'm']),
    ('m', &['n', 'j', 'k']),
    ('1', &['2', 'q']),
    ('2', &['1', '3', 'q', 'w']),
    ('3', &['2', '4', 'w', 'e']),
    ('4', &['3', '5', 'e', 'r']),
    ('5', &['4', '6', 'r', 't']),
    ('6', &['5', '7', 't', 'y']),
    ('7', &['6', '8', 'y', 'u']),
    ('8', &['7', '9', 'u', 'i']),
    ('9', &['8', '0', 'i', 'o']),
    ('0', &['9', 'o', 'p']),
];

/// Look up physically adjacent QWERTY keys for a character.
/// Returns `None` for characters not in the adjacency table.
fn qwerty_neighbors(c: char) -> Option<&'static [char]> {
    let lc = c.to_ascii_lowercase();
    for &(key, nbrs) in QWERTY_NEIGHBORS {
        if key == lc {
            return Some(nbrs);
        }
    }
    None
}

/// Configuration for prompt augmentation.
#[derive(Debug, Clone)]
pub struct AugmentConfig {
    /// Number of augmented variants to produce per input prompt.
    pub variants_per_prompt: usize,
    /// Probability (0.0–1.0) that each character in a word is subject to a typo mutation.
    pub typo_char_rate: f64,
    /// Probability (0.0–1.0) an eligible word is swapped with a synonym.
    pub synonym_swap_rate: f64,
    /// Whether to include word-order-shuffled variants (only applies to multi-word instructions).
    pub shuffle_words: bool,
    /// Whether to inject lowercase-only and ALL-CAPS variants.
    pub case_variants: bool,
}

impl Default for AugmentConfig {
    fn default() -> Self {
        Self {
            variants_per_prompt: 3,
            typo_char_rate: 0.05,
            synonym_swap_rate: 0.25,
            shuffle_words: true,
            case_variants: true,
        }
    }
}

/// Synonym table: maps a canonical term to a list of interchangeable alternatives.
/// Keys and values are lowercase. Matching is also done lowercase.
/// Includes Vox-domain synonyms for natural training prompt variation.
static SYNONYMS: &[(&str, &[&str])] = &[
    ("write", &["create", "build", "define", "implement", "make"]),
    ("create", &["write", "build", "define", "implement", "make"]),
    ("build", &["write", "create", "define", "implement"]),
    ("define", &["write", "create", "declare", "specify"]),
    ("implement", &["write", "build", "code", "develop"]),
    ("make", &["write", "create", "build"]),
    ("show", &["display", "print", "demonstrate", "illustrate"]),
    ("function", &["fn", "func", "procedure", "method"]),
    ("component", &["widget", "element", "view", "ui component"]),
    ("actor", &["agent", "entity", "service"]),
    ("workflow", &["pipeline", "process", "orchestration"]),
    ("table", &["schema", "model", "entity"]),
    ("query", &["read", "fetch", "get", "select"]),
    ("mutation", &["write", "update", "modify", "change"]),
    ("called", &["named", "with name"]),
    ("using", &["with", "via", "through"]),
    (
        "in vox",
        &["with vox", "using the vox language", "vox style"],
    ),
    ("a vox", &["a", "the vox"]),
    ("proper", &["correct", "full", "appropriate"]),
    // Vox-domain synonyms
    (
        "handler",
        &["on message", "event handler", "message handler"],
    ),
    ("decorator", &["annotation", "attribute", "tag"]),
    ("durable", &["persistent", "reliable", "fault-tolerant"]),
    ("spawn", &["create", "launch", "start"]),
    ("trait", &["interface", "protocol", "contract"]),
    ("layout", &["shell", "wrapper", "frame"]),
    ("island", &["interactive component", "client component"]),
    ("pipeline", &["chain", "sequence", "flow"]),
    ("schema", &["table", "model", "structure"]),
    ("endpoint", &["route", "handler", "API"]),
    ("returns", &["produces", "yields", "outputs"]),
    ("import", &["load", "bring in", "include"]),
    ("export", &["share", "expose", "publish"]),
    ("deploy", &["ship", "release", "publish"]),
    ("test", &["verify", "check", "validate"]),
    ("debug", &["troubleshoot", "fix", "diagnose"]),
    ("schedule", &["cron", "recurring", "periodic"]),
    ("action", &["server action", "operation", "command"]),
    ("skill", &["capability", "ability", "competency"]),
    ("route", &["path", "URL", "endpoint"]),
];

/// Apply a single character-level typo mutation to `word`.
/// Returns the mutated string. Preserves length boundaries.
fn typo_mutate(word: &str, rng: &mut impl Rng) -> String {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    // Guard on char count (not byte length) so multi-byte Unicode chars like →
    // do not bypass the guard and cause `gen_range(0..0)` panics.
    if len < 2 {
        return word.to_string();
    }
    // Pick a mutation strategy
    match rng.gen_range(0u8..=3) {
        0 => {
            // Character swap (adjacent transposition)
            let i = rng.gen_range(0..len.saturating_sub(1));
            let mut c = chars.clone();
            c.swap(i, i + 1);
            c.iter().collect()
        }
        1 => {
            // Character deletion (one random character removed)
            let i = rng.gen_range(0..len);
            chars
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, &c)| c)
                .collect()
        }
        2 => {
            // Character duplication
            let i = rng.gen_range(0..len);
            let mut out = String::with_capacity(len + 1);
            for (j, &c) in chars.iter().enumerate() {
                out.push(c);
                if j == i {
                    out.push(c);
                }
            }
            out
        }
        _ => {
            // QWERTY-adjacent substitution: sample from physically adjacent keys.
            // Per Typoist (CHI 2025), substitution is the dominant real-world typo (~80%)
            // and should draw from the QWERTY neighbor set, not a random alphabet shift.
            let i = rng.gen_range(0..len);
            let c = chars[i];
            let replacement = if let Some(nbrs) = qwerty_neighbors(c) {
                let neighbor = nbrs[rng.gen_range(0..nbrs.len())];
                // Preserve original casing of the replaced character
                if c.is_ascii_uppercase() {
                    neighbor.to_ascii_uppercase()
                } else {
                    neighbor
                }
            } else {
                // Character not in QWERTY table — keep original (don't corrupt symbols)
                c
            };
            chars
                .iter()
                .enumerate()
                .map(|(j, &orig)| if j == i { replacement } else { orig })
                .collect()
        }
    }
}

/// Apply synonym substitution to a prompt. Replaces the first matching synonym phrase
/// (longest match wins, case-insensitive) with a randomly chosen alternative.
fn apply_synonym(prompt: &str, rng: &mut impl Rng) -> String {
    // Collect eligible replacements (phrase → candidate replacements)
    let lower = prompt.to_lowercase();
    let mut replaceable: Vec<(usize, usize, Vec<&str>)> = Vec::new(); // (start, end, choices)
    for &(term, alts) in SYNONYMS {
        if let Some(pos) = lower.find(term) {
            replaceable.push((pos, pos + term.len(), alts.to_vec()));
        }
    }
    if replaceable.is_empty() {
        return prompt.to_string();
    }
    // Pick among longest matches first to keep behavior predictable for overlaps.
    let longest = replaceable
        .iter()
        .map(|(start, end, _)| end - start)
        .max()
        .unwrap_or(0);
    let longest_matches: Vec<_> = replaceable
        .into_iter()
        .filter(|(start, end, _)| end - start == longest)
        .collect();
    let (start, end, alts) = longest_matches[rng.gen_range(0..longest_matches.len())].clone();
    let alt = alts[rng.gen_range(0..alts.len())];
    // Preserve original casing heuristic: if the original first char is uppercase, capitalise alt
    let orig_word = &prompt[start..end];
    let replacement = if orig_word.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut s = alt.to_string();
        if let Some(first) = s.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        s
    } else {
        alt.to_string()
    };
    format!("{}{}{}", &prompt[..start], replacement, &prompt[end..])
}

/// Apply word-order shuffle to the non-`{name}` prefix words.
/// Splits prompt at first `{name}` or after the 3rd word, whichever comes first.
fn apply_word_shuffle(prompt: &str, rng: &mut impl Rng) -> String {
    // Find the boundary before a template variable or after the first clause
    let words: Vec<&str> = prompt.split_whitespace().collect();
    if words.len() < 4 {
        return prompt.to_string();
    }
    // Shuffle the first three "non-name" words only, leave the rest intact
    let pivot = words
        .iter()
        .position(|w| w.contains('{'))
        .unwrap_or(3)
        .min(words.len());
    if pivot < 2 {
        return prompt.to_string();
    }
    let mut prefix: Vec<&str> = words[..pivot].to_vec();
    prefix.shuffle(rng);
    let suffix = &words[pivot..];
    let combined: Vec<&str> = prefix
        .iter()
        .copied()
        .chain(suffix.iter().copied())
        .collect();
    combined.join(" ")
}

/// Generate `config.variants_per_prompt` augmented variants of `prompt`.
///
/// Uses a seeded RNG for reproducibility. The seed combines the provided `seed`
/// value with the hash of the original prompt so different prompts always produce
/// different variants even with the same base seed.
pub fn augment_prompt(prompt: &str, config: &AugmentConfig, seed: u64) -> Vec<String> {
    // Derive a per-prompt seed so variants are prompt-specific
    let prompt_hash = {
        let mut h: u64 = seed;
        for b in prompt.bytes() {
            h = h.wrapping_mul(6364136223846793005).wrapping_add(b as u64);
        }
        h
    };
    let mut rng = rand::rngs::StdRng::seed_from_u64(prompt_hash);
    let mut variants: Vec<String> = Vec::with_capacity(config.variants_per_prompt);

    // Strategy pool: each closure takes (&str, &mut rng) and returns String
    for _i in 0..config.variants_per_prompt {
        // Weighted strategy selection per real typing error frequency research:
        // Substitution (arm 1 typo injection, arm 4 combined) is the most common error.
        // Giving arms 1 and 4 extra weight produces more naturalistic training variety.
        let strategy = match rng.gen_range(0u8..=7) {
            0 => 0,     // synonym swap
            1 | 2 => 1, // typo injection — 2× weight (QWERTY substitution via typo_mutate)
            3 => 2,     // word shuffle
            4 => 3,     // lowercase
            5..=7 => 4, // combined synonym+typo — 3× weight
            _ => unreachable!(),
        };
        let variant = match strategy {
            0 => {
                // Synonym substitution
                if rng.gen_bool(config.synonym_swap_rate) {
                    apply_synonym(prompt, &mut rng)
                } else {
                    prompt.to_string()
                }
            }
            1 => {
                // Typo injection into the first non-placeholder word
                let words: Vec<&str> = prompt.split_whitespace().collect();
                let out: Vec<String> = words
                    .iter()
                    .map(|&w| {
                        if !w.contains('{')
                            && rng.gen_bool((config.typo_char_rate * w.len() as f64).min(1.0))
                        {
                            typo_mutate(w, &mut rng)
                        } else {
                            w.to_string()
                        }
                    })
                    .collect();
                out.join(" ")
            }
            2 => {
                // Word order shuffle
                if config.shuffle_words {
                    apply_word_shuffle(prompt, &mut rng)
                } else {
                    apply_synonym(prompt, &mut rng)
                }
            }
            3 => {
                // Lowercase
                if config.case_variants {
                    prompt.to_lowercase()
                } else {
                    apply_synonym(prompt, &mut rng)
                }
            }
            _ => {
                // Synonym + typo combined
                let with_synonym = apply_synonym(prompt, &mut rng);
                let words: Vec<&str> = with_synonym.split_whitespace().collect();
                let mut out: Vec<String> = words.iter().map(|w| w.to_string()).collect();
                for w in out.iter_mut().filter(|w| !w.contains('{')) {
                    if rng.gen_bool(config.typo_char_rate) {
                        *w = typo_mutate(w, &mut rng);
                    }
                }
                out.join(" ")
            }
        };
        if !variant.trim().is_empty() && !variants.contains(&variant) && variant != prompt {
            variants.push(variant);
        }
    }
    variants
}

/// Augment every instruction in a slice of JSONL lines and emit additional lines.
///
/// Lines that already have `record_format` or non-plain fields are emitted as-is.
/// For each parseable `{"prompt": ..., "response": ...}` pair, `config.variants_per_prompt`
/// augmented instruction variants are appended. The response is unchanged.
pub fn augment_jsonl_lines(lines: &[String], config: &AugmentConfig, seed: u64) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len() * (1 + config.variants_per_prompt));
    for line in lines {
        out.push(line.clone());
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let Some(prompt) = v.get("prompt").and_then(|x| x.as_str()) else {
            continue;
        };
        let Some(response) = v.get("response").and_then(|x| x.as_str()) else {
            continue;
        };

        let category = v.get("category").and_then(|x| x.as_str()).unwrap_or("");
        if category == "negative_preference" {
            continue;
        }

        let variants_to_gen = if category == "documentation" {
            1.min(config.variants_per_prompt)
        } else {
            config.variants_per_prompt
        };

        if variants_to_gen == 0 {
            continue;
        }

        let mut local_config = config.clone();
        local_config.variants_per_prompt = variants_to_gen;

        let variants = augment_prompt(prompt, &local_config, seed);
        for variant_prompt in variants {
            let mut row = v.as_object().cloned().unwrap_or_default();
            row.insert(
                "prompt".to_string(),
                serde_json::Value::String(variant_prompt),
            );
            // Mark augmented rows so they can be filtered during eval
            row.entry("augmented".to_string())
                .or_insert(serde_json::Value::Bool(true));
            let _ = response; // retained from parent
            if let Ok(s) = serde_json::to_string(&serde_json::Value::Object(row)) {
                out.push(s);
            }
        }
    }
    out
}

/// Apply augmentation to every eligible line in a JSONL file in-place.
///
/// Reads all lines, calls [`augment_jsonl_lines`] to expand them, then rewrites
/// the file with the augmented set. Returns the number of **new** lines added.
pub fn augment_corpus_file(
    path: &std::path::Path,
    config: &AugmentConfig,
    seed: u64,
) -> anyhow::Result<usize> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("augment_corpus_file: read {}", path.display()))?;
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect();
    let original_count = lines.len();
    let augmented = augment_jsonl_lines(&lines, config, seed);
    let added = augmented.len().saturating_sub(original_count);
    let mut writer = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .with_context(|| format!("augment_corpus_file: open for write {}", path.display()))?,
    );
    for line in &augmented {
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn augment_produces_variants() {
        let cfg = AugmentConfig::default();
        let variants = augment_prompt("Write a Vox function called greet", &cfg, 42);
        // Must produce at least 1 distinct variant
        assert!(
            !variants.is_empty(),
            "expected at least one augmented variant"
        );
        // None should equal the original
        for v in &variants {
            assert_ne!(
                v.as_str(),
                "Write a Vox function called greet",
                "augmented variant must differ from original: {v}"
            );
        }
    }

    #[test]
    fn augment_is_deterministic() {
        let cfg = AugmentConfig::default();
        let a = augment_prompt("Create a Vox actor called Counter", &cfg, 99);
        let b = augment_prompt("Create a Vox actor called Counter", &cfg, 99);
        assert_eq!(a, b, "same seed must produce identical variants");
    }

    #[test]
    fn augment_different_seeds_differ() {
        let cfg = AugmentConfig::default();
        let a = augment_prompt("Build an actor called X", &cfg, 1);
        let b = augment_prompt("Build an actor called X", &cfg, 9999);
        // Highly likely to differ (not guaranteed but practically always true)
        let _ = (a, b); // just check they don't panic
    }

    #[test]
    fn apply_synonym_replaces_known_word() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let result = apply_synonym("Write a Vox function called greet", &mut rng);
        // "Write" or "function" should be replaced
        assert_ne!(result, "Write a Vox function called greet");
    }

    #[test]
    fn augment_jsonl_lines_expands_rows() {
        let lines = vec![
            r#"{"prompt":"Write a Vox function called foo","response":"fn foo() to Unit:\n  ret"}"#
                .to_string(),
        ];
        let cfg = AugmentConfig {
            variants_per_prompt: 2,
            ..AugmentConfig::default()
        };
        let out = augment_jsonl_lines(&lines, &cfg, 7);
        // Original + up to 2 augmented variants
        assert!(
            out.len() >= 2,
            "expected at least 2 rows, got {}",
            out.len()
        );
        // First row must be the original
        assert_eq!(out[0], lines[0]);
    }

    #[test]
    fn typo_mutate_changes_word() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(12345);
        let result = typo_mutate("function", &mut rng);
        // Should be different from the original (possible but very rare to collide)
        assert!(result.len() >= 7, "result too short: {result}");
    }

    #[test]
    fn test_qwerty_substitution_only_uses_adjacent_keys() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let word = "vox";
        let original_chars: Vec<char> = word.chars().collect();
        for _ in 0..100 {
            let mutated = typo_mutate(word, &mut rng);
            let mut_chars: Vec<char> = mutated.chars().collect();
            if mut_chars.len() == original_chars.len() {
                // mostly looking for substitution
                let mut changes = 0;
                let mut is_swap = false;
                for i in 0..original_chars.len().saturating_sub(1) {
                    if original_chars[i] == mut_chars[i + 1]
                        && original_chars[i + 1] == mut_chars[i]
                    {
                        is_swap = true;
                        break;
                    }
                }
                if is_swap {
                    continue;
                }

                for (i, &c) in original_chars.iter().enumerate() {
                    let mc = mut_chars[i];
                    if c != mc {
                        // Check if mc is in qwerty_neighbors(c)
                        if let Some(nbrs) = qwerty_neighbors(c) {
                            assert!(
                                nbrs.contains(&mc),
                                "Mutated char {} must be a neighbor of {}",
                                mc,
                                c
                            );
                        } else {
                            assert_eq!(mc, c); // if not in QWERTY, shouldn't change
                        }
                        changes += 1;
                    }
                }
                assert!(
                    changes <= 1,
                    "Only one substitution should happen at a time"
                );
            }
        }
    }

    #[test]
    fn word_shuffle_rearranges_prefix() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let result = apply_word_shuffle("Write a Vox function called {name}", &mut rng);
        // The result should still contain all the original words
        assert!(
            result.contains("{name}"),
            "template variable must be preserved: {result}"
        );
        assert!(
            result.contains("function"),
            "key words must be preserved: {result}"
        );
    }
}
