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
        "Aegis", "Vortex", "Cerebro", "Nebula", "Quasar", "Tachyon", "Zenith", "Halcyon", "Equinox", "Solstice",
        "Vanguard", "Apex", "Horizon", "Pinnacle", "Vertex", "Meridian", "Eclipse", "Singularity", "Onyx", "Obsidian",
        "Cobalt", "Crimson", "Azure", "Sylvan", "Umbra", "Radiance", "Aurora", "Prism", "Fractal", "Enigma",
        "Nexus", "Matrix", "Labyrinth", "Cipher", "Oracle", "Paradigm", "Synthesis", "Genesis", "Revelation", "Epoch",
        "Continuum", "Infinity", "Eternity", "Cosmos", "Galaxy", "Universe", "Multiverse", "Dimension", "Realm", "Domain",
        "Sphere", "Globe", "Planet", "Star", "Moon", "Comet", "Asteroid", "Meteor", "Nebula", "Supernova",
        "Pulsar", "Quasar", "BlackHole", "Wormhole", "Void", "Abyss", "Chasm", "Rift", "Fault", "Fracture"
    ];

    let actions = [
        "calibrated", "synchronized", "depleted", "amplified", "polarized", "inverted", "stabilized", "merged",
        "catalyzed", "synthesized", "extracted", "refined", "purified", "distilled", "fractionated", "crystallized",
        "modulated", "attenuated", "amplified", "rectified", "filtered", "smoothed", "integrated", "differentiated",
        "correlated", "convoluted", "transformed", "mapped", "projected", "embedded", "encoded", "decoded"
    ];

    let versions = [
        "v1.0", "v2.1", "v3.5", "v4.0", "v5.2", "v6.8", "alpha-7", "beta-9",
        "v7.2", "v8.1", "v9.0", "v10.5", "v11.3", "v12.7", "v13.4", "v14.9",
        "rc-1", "rc-2", "rc-3", "stable", "latest", "nightly", "canary", "dev"
    ];

    for _ in 0..count {
        let hop_count = rng.gen_range(2..=5);
        let mut facts = Vec::new();
        let chain_entities = entities.choose_multiple(&mut rng, hop_count + 1).collect::<Vec<_>>();
        
        let mut synthesis_steps = Vec::new();
        
        // A -> B, B -> C, C -> D
        for i in 0..hop_count {
            let e1 = chain_entities[i];
            let e2 = chain_entities[i+1];
            let action = actions.choose(&mut rng).unwrap();
            let version = versions.choose(&mut rng).unwrap();
            
            let fact_idx = rng.gen_range(0..6);
            let fact = match fact_idx {
                0 => format!("The {} module was {} by the {} in {}.", e1, action, e2, version), // Temporal
                1 => format!("Since {}, the {} interface {} with the {}.", version, e1, action, e2), // Temporal
                2 => format!("The {} protocol was {} in version {} to cross-support {}.", e2, action, version, e1), // Conditional intent
                3 => format!("If {} becomes {}, then {} initiates a fallback.", e1, action, e2), // Conditional
                4 => format!("Despite the instability in {}, {} remained {} under the {} standard.", e1, e2, action, version), // Contrastive
                _ => format!("While {} targets {}, {} was originally {}.", e1, version, e2, action), // Contrastive
            };
            facts.push(fact.clone());
            
            let synthesis_step = match fact_idx {
                0..=2 => format!("{} influences {} ({})", e1, e2, action),
                3 => format!("{} triggers {} under condition ({})", e1, e2, action),
                _ => format!("{} contrasts with {} regarding ({})", e1, e2, action),
            };
            synthesis_steps.push(synthesis_step);
        }

        facts.shuffle(&mut rng); // Scramble evidence order for reasoning challenge

        let question = format!("How does {} relate to {} according to the provided evidence?", chain_entities[0], chain_entities[hop_count]);
        
        let synthesis = format!("According to the evidence, {} is linked to {} through a series of interactions: {}. Consequently, the relationship relies on multi-step propagation.", chain_entities[0], chain_entities[hop_count], synthesis_steps.join(" -> "));

        let record = json!({
            "instruction": "You are a research synthesis agent. Given the following pieces of disconnected evidence, construct the logical chain answering the question. You must cite evidence.",
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
