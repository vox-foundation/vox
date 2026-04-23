//! Placeholder `vox ci …` Mens helpers (Phase 2 sketch). **They do not read the DB or corpus.**
//! Production-style gates live in **`vox ci mesh-gate`** (`scripts/populi/gates.yaml`) and **`vox mens corpus …`**.

use anyhow::Result;
use std::path::Path;

pub async fn run_mens_corpus_health(
    _root: &Path,
    min_pairs: usize,
    min_human_ratio: f64,
) -> Result<()> {
    let _ = std::hint::black_box((min_pairs, min_human_ratio.to_bits()));
    println!(
        "mens-corpus-health (placeholder): would verify >= {min_pairs} pairs, >= {min_human_ratio} human ratio."
    );
    Ok(())
}

pub async fn run_grpo_reward_baseline(_root: &Path) -> Result<()> {
    let _ = std::hint::black_box(_root.as_os_str().len());
    println!(
        "grpo-reward-baseline (placeholder): no-op; see mesh-gate / Populi tests for real GRPO paths."
    );
    Ok(())
}

pub async fn run_collateral_damage_gate(_root: &Path, max_damage_rate: f64) -> Result<()> {
    let _ = std::hint::black_box(max_damage_rate.to_bits());
    println!("collateral-damage-gate (placeholder): max_damage_rate={max_damage_rate}");
    Ok(())
}

pub async fn run_constrained_gen_smoke(_root: &Path, n_samples: usize) -> Result<()> {
    let _ = std::hint::black_box((_root.as_os_str().len(), n_samples));
    println!("constrained-gen-smoke (placeholder): would validate {n_samples} samples.");
    Ok(())
}
