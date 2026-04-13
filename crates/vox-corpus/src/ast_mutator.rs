use vox_compiler::ast::decl::Module;
use rand::Rng;
use rand::seq::SliceRandom;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct Mutation {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

const MUTATION_NAMES: &[&str] = &[
    "delta", "epsilon", "omega", "flux", "core", "node", "shard", "pulse",
    "buffer", "cache", "stream", "handler", "proxy", "bridge", "nexus", "vertex",
];

pub fn generate_mutations(source: &str, _module: &Module) -> Vec<Mutation> {
    let mut rng = rand::thread_rng();
    let mut mutations = Vec::new();
    
    // Identifier renaming (greedy camelCase or PascalCase)
    let id_re = regex::Regex::new(r"\b([a-z][a-zA-Z0-9]*[A-Z][a-zA-Z0-9]*|[A-Z][a-zA-Z0-9]+)\b").unwrap();
    for cap in id_re.captures_iter(source) {
        if rng.gen_bool(0.2) {
            if let Some(m) = cap.get(1) {
                let replacement = MUTATION_NAMES.choose(&mut rng).unwrap().to_string();
                mutations.push(Mutation {
                    start: m.start(),
                    end: m.end(),
                    replacement,
                });
            }
        }
    }

    // Number substitution
    let num_re = regex::Regex::new(r"\b(\d+)\b").unwrap();
    for cap in num_re.captures_iter(source) {
        if rng.gen_bool(0.15) {
            if let Some(m) = cap.get(1) {
                if let Ok(val) = m.as_str().parse::<i64>() {
                    let replacement = (val + rng.gen_range(-2..=2)).to_string();
                    mutations.push(Mutation {
                        start: m.start(),
                        end: m.end(),
                        replacement,
                    });
                }
            }
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

pub fn mutate_corpus(input_path: &std::path::Path, out: &mut impl Write, factor: usize) -> anyhow::Result<usize> {
    use std::io::BufRead;
    let file = std::fs::File::open(input_path)?;
    let reader = std::io::BufReader::new(file);
    let mut actual = 0;

    let dummy_result = vox_compiler::pipeline::run_frontend_str("", "<mutant>").map_err(|e| anyhow::anyhow!("Pipeline failure: {:?}", e))?;
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
