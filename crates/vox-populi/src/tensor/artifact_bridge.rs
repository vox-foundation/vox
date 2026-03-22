//! Cross-kernel **artifact** guidance (Burn `Checkpoint` bins vs Candle QLoRA safetensors).
//!
//! Centralizes operator-facing copy for CLI errors and doc parity tests. There is **no** automatic
//! tensor rename map between Burn `LoraVoxTransformer` checkpoints and Candle `midN`/`lm_head`
//! adapter layouts.

/// Message when `vox populi merge-qlora` receives a **`*.bin`** path (Burn checkpoint).
///
/// Keep in sync with [`populi-training-ssot.md`](../../../../docs/src/architecture/populi-training-ssot.md) merge table.
pub const MERGE_QLORA_REJECTS_BURN_BIN: &str = "`merge-qlora` expects a Candle **safetensors** adapter (`candle_qlora_adapter.safetensors`), \
     not a Burn LoRA **`*.bin`** checkpoint.\n\
     For Burn checkpoints from `vox populi train --backend lora`, use **`vox populi merge-weights`** \
     to produce `model_merged.bin`.\n\
     Burn LoRA targets attention Q/K/V separately; Candle v2 adapters use `midN` / `lm_head` names tied to \
     the qlora-rs projection stack — there is **no** supported automatic Burn→Candle adapter conversion.\n\
     See `docs/src/architecture/populi-training-ssot.md`.";

#[cfg(test)]
mod tests {
    #[test]
    fn burn_bin_rejection_mentions_merge_weights_and_ssot() {
        let s = super::MERGE_QLORA_REJECTS_BURN_BIN;
        assert!(s.contains("merge-weights"), "{s}");
        assert!(s.contains("populi-training-ssot"), "{s}");
        assert!(s.contains("safetensors"), "{s}");
    }
}
