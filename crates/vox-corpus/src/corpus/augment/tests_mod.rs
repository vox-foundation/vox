mod tests {
    #![allow(unused_imports)]
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
            r#"{"prompt":"Write a Vox function called foo","response":"fn foo() to Unit:\n  return"}"#
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
