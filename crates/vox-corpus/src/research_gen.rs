use serde_json::json;
use std::io::Write;
use rand::seq::SliceRandom;
use rand::Rng;

pub fn generate_research_chains(out: &mut impl Write, count: usize) -> anyhow::Result<usize> {
    let mut rng = rand::thread_rng();
    let mut actual_count = 0;

    let entities = [
        "Aetherium", "Borealis", "Chronos", "Dyson", "Epsilon", "Flux", "Gaea", "Helios", "Ion", "Juno",
        "Krypton", "Lumen", "Magma", "Nova", "Orion", "Pulse", "Quantum", "Rift", "Solar", "Titan",
    ];

    let actions = [
        "calibrated", "synchronized", "depleted", "amplified", "polarized", "inverted", "stabilized", "merged",
    ];

    let versions = [
        "v1.0", "v2.1", "v3.5", "v4.0", "v5.2", "v6.8", "alpha-7", "beta-9",
    ];

    for _ in 0..count {
        let hop_count = rng.gen_range(2..=4);
        let mut facts = Vec::new();
        let chain_entities = entities.choose_multiple(&mut rng, hop_count + 1).collect::<Vec<_>>();
        
        // A -> B, B -> C, C -> D
        for i in 0..hop_count {
            let e1 = chain_entities[i];
            let e2 = chain_entities[i+1];
            let action = actions.choose(&mut rng).unwrap();
            let version = versions.choose(&mut rng).unwrap();
            
            let fact = match rng.gen_range(0..3) {
                0 => format!("The {} {} was {} by the {} in {}.", e1, e2, action, e1, version),
                1 => format!("Since {}, the {} interface {} with the {}.", version, e1, action, e2),
                _ => format!("The {} protocol was {} in version {} to support {}.", e2, action, version, e1),
            };
            facts.push(fact);
        }

        let question = format!("How does {} relate to {} according to the provided evidence?", chain_entities[0], chain_entities[hop_count]);
        
        let mut synthesis = format!("According to the evidence, {} is linked to {} through a series of interactions: ", chain_entities[0], chain_entities[hop_count]);
        for (i, p) in facts.iter().enumerate() {
            synthesis.push_str(&format!("({}) {}; ", i+1, p));
        }

        let record = json!({
            "instruction": "You are a research synthesis agent. Given the following pieces of evidence, answer the question with citations.",
            "input": format!("<evidence>\n{}\n</evidence>\n<question>{}</question>", facts.join("\n"), question),
            "output": synthesis,
            "lane": "vox_research_expert",
            "response_mode": "structured",
            "task_family": "retrieve_and_synthesize",
            "metadata": {
                "hop_count": hop_count,
                "domain": "fictional_chains"
            }
        });

        writeln!(out, "{}", serde_json::to_string(&record)?)?;
        actual_count += 1;
    }

    Ok(actual_count)
}
