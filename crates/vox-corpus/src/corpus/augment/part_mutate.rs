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
