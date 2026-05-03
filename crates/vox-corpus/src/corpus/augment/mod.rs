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

include!("part_mutate.rs");
include!("part_io.rs");

#[cfg(test)]
include!("tests_mod.rs");
