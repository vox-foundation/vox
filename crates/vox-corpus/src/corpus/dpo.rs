use crate::ast_mutator;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[cfg(feature = "database")]
use vox_db::VoxDb;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DpoPair {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    pub category: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DpoConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub limit: usize,
}

pub fn generate_dpo_from_extract(config: &DpoConfig) -> anyhow::Result<usize> {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};

    let input_file = File::open(&config.input)?;
    let reader = BufReader::new(input_file);

    let mut out_file = File::create(&config.output)?;
    let mut count = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let value: serde_json::Value = serde_json::from_str(&line)?;
        let prompt = value
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let chosen = value
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let category = value
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("vox_source")
            .to_string();
        let source = value
            .get("source")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if chosen.is_empty() {
            continue;
        }

        // Generate rejected sample by mutating 'chosen'
        // For Vox files, we use the ast_mutator if it's not a generic prompt
        let rejected =
            if chosen.contains("fn ") || chosen.contains("actor ") || chosen.contains("@") {
                // Try to parse and mutate
                if let Ok(result) = vox_compiler::pipeline::run_frontend_str(&chosen, "<dpo-gen>") {
                    let mutations = ast_mutator::generate_mutations(&chosen, &result.module);
                    if !mutations.is_empty() {
                        ast_mutator::apply_mutations(&chosen, mutations)
                    } else {
                        // Fallback: simple string manipulation
                        chosen.replace("fn ", "function ")
                    }
                } else {
                    chosen.replace("fn ", "function ")
                }
            } else {
                chosen.replace("let ", "var ")
            };

        if rejected == chosen {
            // Skip pairs where mutation failed to change anything
            continue;
        }

        let pair = DpoPair {
            prompt,
            chosen,
            rejected,
            category,
            source,
        };

        let json = serde_json::to_string(&pair)?;
        writeln!(out_file, "{}", json)?;
        count += 1;

        if config.limit > 0 && count >= config.limit {
            break;
        }
    }

    Ok(count)
}

/// Export DPO preference pairs from VoxDB (corrections vs original failures).
#[cfg(feature = "database")]
pub async fn export_dogfood_dpo(db: &VoxDb, limit: i64, output: &PathBuf) -> anyhow::Result<usize> {
    use std::fs::File;
    use std::io::Write;

    let pairs = db.get_training_data(limit).await?;
    let mut out_file = File::create(output)?;
    let mut count = 0;

    for pair in pairs {
        if let Some(preferred) = pair.correction.as_ref().filter(|c: &&String| !c.is_empty()) {
            let preferred_str: &str = preferred.as_str();
            let dpo = DpoPair {
                prompt: pair.prompt,
                chosen: preferred_str.to_string(),
                rejected: pair.response,
                category: "agents_dogfood_dpo".to_string(),
                source: Some("vox_db".to_string()),
            };
            let json = serde_json::to_string(&dpo)?;
            writeln!(out_file, "{}", json)?;
            count += 1;
        }
    }

    Ok(count)
}
