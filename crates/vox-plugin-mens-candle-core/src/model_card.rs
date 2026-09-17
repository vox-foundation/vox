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
    /// Resolved license class of `base_model` (from `gpu-specs.yaml` `train_bases`
    /// or an explicit `--license-class` override). `None` when no run-level
    /// provenance was threaded through to this card (e.g. a plain `--model` not
    /// in `train_bases` and no override) — the training path itself hard-errors
    /// in that case, so a `None` here only occurs on card-writer paths that
    /// don't carry provenance at all.
    pub license_class: Option<String>,
    /// Whether downstream publication of this artifact must carry an
    /// attribution notice for the base model's license terms.
    pub attribution_required: bool,
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
    if let Some(ref lic) = card.license_class {
        s.push_str("## License\n");
        s.push_str(&format!("- base model license: `{lic}`\n"));
        if card.attribution_required {
            s.push_str(
                "- **Attribution required**: this base model's license requires an \
                 attribution notice on any downstream publication of this artifact.\n",
            );
        }
        s.push('\n');
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

#[cfg(test)]
mod tests {
    use super::*;

    fn base_card() -> ModelCard {
        ModelCard {
            title: "t".into(),
            base_model: Some("Qwen/Qwen3-8B".into()),
            train_file: "train.jsonl".into(),
            vocab_size: 1,
            d_model: 1,
            n_layers: 1,
            n_heads: 1,
            notes: "n".into(),
            license_class: None,
            attribution_required: false,
        }
    }

    #[test]
    fn write_renders_license_and_attribution_notice_when_required() {
        let dir = tempfile::tempdir().unwrap();
        let card = ModelCard {
            license_class: Some("qwen-research".into()),
            attribution_required: true,
            ..base_card()
        };
        write(dir.path(), &card).unwrap();
        let text = std::fs::read_to_string(dir.path().join("MODEL_CARD.md")).unwrap();
        assert!(text.contains("qwen-research"));
        assert!(text.contains("Attribution required"));
    }

    #[test]
    fn write_omits_license_section_when_none() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), &base_card()).unwrap();
        let text = std::fs::read_to_string(dir.path().join("MODEL_CARD.md")).unwrap();
        assert!(!text.contains("## License"));
    }
}
