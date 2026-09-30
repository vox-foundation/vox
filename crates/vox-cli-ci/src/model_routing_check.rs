use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::collections::HashSet;
use std::path::Path;

pub fn run(root: &Path) -> Result<()> {
    println!("{}", "Checking model-routing.v1.yaml contract...".cyan());

    let yaml_path = root.join("contracts/orchestration/model-routing.v1.yaml");
    if !yaml_path.exists() {
        anyhow::bail!("Missing model-routing.v1.yaml at {}", yaml_path.display());
    }

    let contents =
        std::fs::read_to_string(&yaml_path).context("Failed to read model-routing.v1.yaml")?;

    let config: vox_config::ModelRoutingConfig = serde_yaml::from_str(&contents)
        .context("Failed to parse model-routing.v1.yaml against the ModelRoutingConfig schema")?;

    println!("{} Parsed successfully.", "✓".green());

    if config.exploration.budget_usd_per_day <= 0.0 {
        anyhow::bail!("exploration.budget_usd_per_day must be > 0.0");
    }
    println!(
        "{} Exploration budget is sane: ${:.2}/day",
        "✓".green(),
        config.exploration.budget_usd_per_day
    );

    if config.latency_bands.excellent_ms >= config.latency_bands.poor_ms {
        anyhow::bail!("latency_bands.excellent_ms must be strictly less than poor_ms");
    }

    let qw = &config.quality_weights;
    let qw_sum = qw.socrates_factuality
        + qw.contradiction_inverse
        + qw.success_rate
        + qw.p50_latency_inverse
        + qw.cost_inverse;
    if qw_sum <= 0.0 {
        anyhow::bail!("quality_weights must sum to a positive value");
    }
    println!(
        "{} quality_weights present (sum={:.2})",
        "✓".green(),
        qw_sum
    );

    if config.safety.max_cost_usd_per_request <= 0.0 {
        anyhow::bail!("safety.max_cost_usd_per_request must be > 0.0");
    }

    // Task 14: premium_alias ids live only in model-defaults.v1.yaml (no second
    // copy here or in model-pins to drift), so check the loaded aliases.
    if !config.premium_alias.is_empty() {
        anyhow::bail!(
            "model-routing.v1.yaml must not define premium_alias ids; they live in model-defaults.v1.yaml"
        );
    }
    let aliases = vox_config::load_model_routing_config().premium_alias;

    // Retired ids must never appear as live premium aliases.
    if let Some(pins) = vox_config::load_model_pins_config() {
        let retired: HashSet<&str> = pins.retired_ids.iter().map(String::as_str).collect();
        for (k, v) in &aliases {
            if retired.contains(v.as_str()) {
                anyhow::bail!(
                    "premium_alias {k} -> {v} references a retired model id from model-pins.v1.yaml"
                );
            }
        }
    }

    // Runtime wiring guard: scorer must reference quality_weights symbol.
    let scoring_rs = root.join("crates/vox-orchestrator/src/models/scoring.rs");
    let scoring_src = std::fs::read_to_string(&scoring_rs).with_context(|| {
        format!(
            "Failed to read {} for SSOT wiring guard",
            scoring_rs.display()
        )
    })?;
    if !scoring_src.contains("scoreboard_feedback_boost")
        || !scoring_src.contains("quality_weights")
    {
        anyhow::bail!(
            "scoring.rs missing quality_weights/scoreboard_feedback_boost wiring (SSOT drift)"
        );
    }
    println!(
        "{} orchestrator scoring references quality_weights",
        "✓".green()
    );

    println!("{} Model routing contract is valid.", "PASS".green().bold());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_routing_check_passes_on_repo_contract() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        run(&root).expect("model routing SSOT check should pass");
    }
}
