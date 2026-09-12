//! Human-readable `MODEL_CARD.md` next to checkpoints.
//!
//! Canonical home for the two plugin crates' copies (previously
//! byte-identical between `vox-plugin-mens-candle-metal` and
//! `vox-plugin-mens-candle-cuda`), now a single definition both re-export.
//!
//! `vox-populi/src/mens/tensor/model_card.rs` carries a THIRD, still-separate
//! copy of this exact type. Folding it in here too would need vox-populi
//! (layer 2 in `contracts/ci/crate-layers.v1.json`) to depend on this crate
//! (layer 4 — set by its own dependency on `vox-tensor`/`vox-hf-layout`,
//! which sit at layers 3-4), an upward edge that `vox ci crate-edges` blocks
//! unless a user adds a grandfathered `exceptions` entry (USER-AUTHORIZED-ONLY
//! per AGENTS.md §Dependency Discipline — not something to add unilaterally).

use std::path::Path;

pub struct ModelCard {
    pub title: String,
    pub base_model: Option<String>,
    pub train_file: String,
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_layers: usize,
    pub n_heads: usize,
    pub notes: String,
}

pub fn write(out_dir: &Path, card: &ModelCard) -> anyhow::Result<()> {
    let mut s = String::new();
    s.push_str("# ");
    s.push_str(&card.title);
    s.push_str("\n\n");
    if let Some(ref b) = card.base_model {
        s.push_str("## Base model\n");
        s.push_str(b);
        s.push_str("\n\n");
    }
    s.push_str("## Data\n");
    s.push_str(&format!("- train file: `{}`\n", card.train_file));
    s.push_str("\n## Architecture\n");
    s.push_str(&format!(
        "- vocab: {}\n- d_model: {}\n- layers: {}\n- heads: {}\n\n",
        card.vocab_size, card.d_model, card.n_layers, card.n_heads
    ));
    s.push_str("## Notes\n");
    s.push_str(&card.notes);
    s.push('\n');
    std::fs::write(out_dir.join("MODEL_CARD.md"), s)?;
    Ok(())
}
