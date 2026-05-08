//! Cross-kernel **artifact** guidance (legacy Burn bins vs Candle QLoRA safetensors).
//!
//! Burn has been removed from the codebase. This module keeps the rejection message for
//! `merge-qlora` in case someone passes a legacy Burn `.bin` checkpoint.

/// Message when `vox schola merge-qlora` receives a **`*.bin`** path (legacy Burn checkpoint).
///
/// Keep in sync with [`mens-training-ssot.md`](../../../../docs/src/architecture/mens-training-ssot.md) merge table.
pub const MERGE_QLORA_REJECTS_BURN_BIN: &str = "`merge-qlora` expects a Candle **safetensors** adapter (`candle_qlora_adapter.safetensors`), \
     not a Burn LoRA **`*.bin`** checkpoint.\n\
     Burn has been retired from this codebase. Only Candle QLoRA adapters produced by `vox mens train` are supported.\n\
     Candle v2 adapters use `midN` / `lm_head` names tied to the qlora-rs projection stack.\n\
     See `docs/src/architecture/mens-training-ssot.md`.";

#[cfg(test)]
mod tests {
    #[test]
    fn burn_bin_rejection_mentions_candle_and_ssot() {
        let s = super::MERGE_QLORA_REJECTS_BURN_BIN;
        assert!(s.contains("Candle"), "{s}");
        assert!(s.contains("mens-training-ssot"), "{s}");
        assert!(s.contains("safetensors"), "{s}");
    }
}
