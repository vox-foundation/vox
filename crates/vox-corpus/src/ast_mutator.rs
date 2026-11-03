use rand::Rng;
use rand::seq::SliceRandom;
use std::io::Write;
use vox_compiler::ast::decl::Module;

#[derive(Debug, Clone)]
pub struct Mutation {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

const MUTATION_NAMES: &[&str] = &[
    "delta", "epsilon", "omega", "flux", "core", "node", "shard", "pulse", "buffer", "cache",
    "stream", "handler", "proxy", "bridge", "nexus", "vertex",
];

pub fn generate_mutations(source: &str, _module: &Module) -> Vec<Mutation> {
    let mut rng = rand::thread_rng();
    let mut mutations = Vec::new();

    // Identifier renaming (greedy camelCase or PascalCase)
    let id_re =
        regex::Regex::new(r"\b([a-z][a-zA-Z0-9]*[A-Z][a-zA-Z0-9]*|[A-Z][a-zA-Z0-9]+)\b").unwrap();
    for cap in id_re.captures_iter(source) {
        if rng.gen_bool(0.2)
            && let Some(m) = cap.get(1)
        {
            let replacement = MUTATION_NAMES.choose(&mut rng).unwrap().to_string();
            mutations.push(Mutation {
                start: m.start(),
                end: m.end(),
                replacement,
            });
        }
    }

    // Number substitution
    let num_re = regex::Regex::new(r"\b(\d+)\b").unwrap();
    for cap in num_re.captures_iter(source) {
        if rng.gen_bool(0.15)
            && let Some(m) = cap.get(1)
            && let Ok(val) = m.as_str().parse::<i64>()
        {
            let replacement = (val + rng.gen_range(-2..=2)).to_string();
            mutations.push(Mutation {
                start: m.start(),
                end: m.end(),
                replacement,
            });
        }
    }

    mutations
}

pub fn apply_mutations(source: &str, mut mutations: Vec<Mutation>) -> String {
    mutations.sort_by_key(|m| m.start);
    let mut result = String::with_capacity(source.len());
    let mut last_end = 0;

    for m in mutations {
        if m.start >= last_end && m.end <= source.len() {
            result.push_str(&source[last_end..m.start]);
            result.push_str(&m.replacement);
            last_end = m.end;
        }
    }
    result.push_str(&source[last_end..]);
    result
}

pub fn mutate_corpus(
    input_path: &std::path::Path,
    out: &mut impl Write,
    factor: usize,
) -> anyhow::Result<usize> {
    use std::io::BufRead;
    let file = std::fs::File::open(input_path)?;
    let reader = std::io::BufReader::new(file);
    let mut actual = 0;

    let dummy_result = vox_compiler::pipeline::run_frontend_str("", "<mutant>")
        .map_err(|e| anyhow::anyhow!("Pipeline failure: {:?}", e))?;
    let dummy_module = dummy_result.module;

    for line in reader.lines() {
        let line = line?;
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&line) {
            let resp = match v.get("response").and_then(|r| r.as_str()) {
                Some(r) => r.to_string(),
                None => continue,
            };

            for _ in 0..factor {
                let mutations = generate_mutations(&resp, &dummy_module);
                if !mutations.is_empty() {
                    let mutated = apply_mutations(&resp, mutations);
                    v["response"] = serde_json::Value::String(mutated);
                    v["category"] = serde_json::Value::String("semantic_mutant".to_string());
                    v["lane"] = serde_json::Value::String("vox_lang_tier_b".to_string());
                    writeln!(out, "{}", serde_json::to_string(&v)?)?;
                    actual += 1;
                }
            }
        }
    }

    Ok(actual)
}

/// Deterministic 5-spoke hard-negative mutation generator (`AUDIT-03`).
///
/// Takes a verified positive [`crate::training::contract::TrainingRecordV2`] (`chosen`)
/// and synthesizes a realistic hard-negative mutation (`rejected`) for the same spoke,
/// verifying via the spoke's round-trip oracle that `rejected` fails deterministically.
#[must_use]
pub fn mint_hard_negative_dpo_pair(
    chosen_record: &crate::training::contract::TrainingRecordV2,
) -> crate::training::mix_prepare::ChosenRejectedPairV2 {
    use crate::training::contract::{RecordProvenance, SpokeKind};
    use crate::training::mix_prepare::ChosenRejectedPairV2;
    use crate::training::spoke_roundtrip::{
        verify_agent_harness_roundtrip, verify_mcp_roundtrip, verify_rust_roundtrip,
        verify_shell_roundtrip, verify_voxlang_roundtrip,
    };

    let (rejected_text, rejected_receipt) = match chosen_record.spoke {
        SpokeKind::VoxLang => {
            let mutant = format!(
                "@table type LegacyBroken {{ id: int }}\n{}",
                chosen_record.response.replace(" to ", " -> ")
            );
            let r = verify_voxlang_roundtrip("hard_neg_vox", &chosen_record.prompt, &mutant);
            (mutant, r.oracle_receipt)
        }
        SpokeKind::RustWorkspace => {
            let banned = format!(
                "use {}::Hasher;\npub fn broken_stub() {{ todo!() }}",
                "blake3"
            );
            let r = verify_rust_roundtrip("hard_neg_rs", &chosen_record.prompt, &banned);
            (banned, r.oracle_receipt)
        }
        SpokeKind::StructuredShell => {
            let unsafe_chain = format!("{} && rm -rf /", chosen_record.response);
            let r =
                verify_shell_roundtrip("hard_neg_sh", &chosen_record.prompt, &unsafe_chain, &[]);
            (unsafe_chain, r.oracle_receipt)
        }
        SpokeKind::McpToolChain => {
            let bad_mcp =
                r#"{"tool_name":"unknown_forbidden_mcp_tool","arguments":null}"#.to_string();
            let r = verify_mcp_roundtrip("hard_neg_mcp", &chosen_record.prompt, &bad_mcp, None);
            (bad_mcp, r.oracle_receipt)
        }
        SpokeKind::AgentHarness => {
            let bad_plan = r#"{"goal":"","steps":[],"files":["../outside_workspace.rs"],"budget_tokens":999999999,"verification_gate":"rm -rf /"}"#.to_string();
            let r = verify_agent_harness_roundtrip(
                "hard_neg_harness",
                &chosen_record.prompt,
                &bad_plan,
                None,
            );
            (bad_plan, r.oracle_receipt)
        }
    };

    ChosenRejectedPairV2::new(
        chosen_record.spoke,
        chosen_record.prompt.clone(),
        chosen_record.system_context.clone(),
        chosen_record.response.clone(),
        rejected_text,
        chosen_record.oracle_receipt.clone(),
        rejected_receipt,
        RecordProvenance {
            origin: "ast_mutator_hard_negative".to_string(),
            source_id: chosen_record.provenance.source_id.clone(),
            task_family: chosen_record.provenance.task_family.clone(),
            difficulty: chosen_record.provenance.difficulty.saturating_add(1),
        },
    )
}

#[cfg(test)]
mod semcov_wave2_tests {
    use super::*;
    use vox_compiler::ast::span::Span;

    fn mk(start: usize, end: usize, replacement: &str) -> Mutation {
        Mutation {
            start,
            end,
            replacement: replacement.to_string(),
        }
    }

    #[test]
    fn apply_mutations_empty_list_returns_source_unchanged() {
        assert_eq!(apply_mutations("hello world", vec![]), "hello world");
    }

    #[test]
    fn apply_mutations_single_replacement_in_middle() {
        let result = apply_mutations("foo BAR baz", vec![mk(4, 7, "QUX")]);
        assert_eq!(result, "foo QUX baz");
    }

    #[test]
    fn apply_mutations_sorts_and_applies_non_overlapping() {
        // Provided in reverse order; must be sorted before application.
        let result = apply_mutations("aXbYc", vec![mk(3, 4, "2"), mk(1, 2, "1")]);
        assert_eq!(result, "a1b2c");
    }

    #[test]
    fn apply_mutations_skips_overlapping_mutation() {
        // Second mutation starts at 1 < last_end=3 and must be skipped.
        let result = apply_mutations("abcdef", vec![mk(0, 3, "XYZ"), mk(1, 4, "SKIP")]);
        assert_eq!(result, "XYZdef");
    }

    #[test]
    fn apply_mutations_at_start_and_end() {
        let result = apply_mutations("hello world", vec![mk(0, 5, "hi"), mk(6, 11, "earth")]);
        assert_eq!(result, "hi earth");
    }

    #[test]
    fn apply_mutations_replacement_can_be_empty_string() {
        assert_eq!(apply_mutations("aXb", vec![mk(1, 2, "")]), "ab");
    }

    #[test]
    fn generate_mutations_produces_valid_spans() {
        let dummy = Module {
            declarations: vec![],
            span: Span::new(0, 0),
        };
        let src = "let fooBar = 42;";
        for _ in 0..20 {
            let muts = generate_mutations(src, &dummy);
            for m in &muts {
                assert!(m.start < m.end);
                assert!(m.end <= src.len());
            }
        }
    }

    #[test]
    fn generate_mutations_eventually_mutates_pascal_case() {
        let dummy = Module {
            declarations: vec![],
            span: Span::new(0, 0),
        };
        let src = "FooBar";
        let found = (0..100).any(|_| !generate_mutations(src, &dummy).is_empty());
        assert!(found, "expected at least one mutation across 100 runs");
    }

    #[test]
    fn mint_hard_negative_dpo_pair_fails_oracle_across_all_five_spokes() {
        use crate::training::spoke_roundtrip::{
            canonical_five_spoke_golden_suite, evaluate_spoke_case,
        };

        let suite = canonical_five_spoke_golden_suite();
        for case in &suite {
            let receipt = evaluate_spoke_case(case);
            let record = receipt
                .admitted_record
                .expect("golden suite case must produce admitted TrainingRecordV2");
            let dpo_pair = mint_hard_negative_dpo_pair(&record);
            assert!(
                dpo_pair.verify_pair_integrity(),
                "DPO pair integrity failed for spoke {:?}",
                record.spoke
            );
            assert!(dpo_pair.chosen_receipt.passed);
            assert!(!dpo_pair.rejected_receipt.passed);
        }
    }
}
