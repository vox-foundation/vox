//! Unified adapter artifact schema v3 (method + quant + base key map + layer order).
//!
//! Ported verbatim from `vox-populi/src/mens/tensor/adapter_schema_v3.rs` (SP3 sub-batch C).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::finetune_contract::{AdapterMethod, BaseQuantMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopuliAdapterManifestV3 {
    pub format: String,
    pub version: u32,
    #[serde(flatten)]
    pub method: AdapterMethodFields,
    pub quant: QuantFields,
    pub base_key_map: HashMap<String, String>,
    pub layer_order: Vec<String>,
    pub vocab: usize,
    pub d_model: usize,
    pub rank: usize,
    pub alpha: usize,
    #[serde(default)]
    pub use_rslora: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<AdapterProvenanceFields>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMethodFields {
    pub adapter_method: AdapterMethod,
}

// vox:defactored-from vox-config 2026-09-27
#[rustfmt::skip] // keeps the toestub-ignore comment pinned to the fn signature line
fn double_quant_default() -> bool { // toestub-ignore(skeleton/hollow-fn): trivial serde default matching the field's own meaning — nothing more to implement
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantFields {
    pub base_quant: BaseQuantMode,
    #[serde(default = "double_quant_default")]
    pub double_quant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdapterProvenanceFields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_class: Option<String>,
    #[serde(default)]
    pub attribution_required: bool,
}

impl PopuliAdapterManifestV3 {
    pub const FORMAT: &'static str = "vox_mens_adapter";
    pub const VERSION: u32 = 3;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        method: AdapterMethod,
        base_quant: BaseQuantMode,
        double_quant: bool,
        base_key_map: HashMap<String, String>,
        layer_order: Vec<String>,
        vocab: usize,
        d_model: usize,
        rank: usize,
        alpha: usize,
        base_model: Option<String>,
        provenance: Option<AdapterProvenanceFields>,
    ) -> Self {
        Self {
            format: Self::FORMAT.to_string(),
            version: Self::VERSION,
            method: AdapterMethodFields {
                adapter_method: method,
            },
            quant: QuantFields {
                base_quant,
                double_quant,
            },
            base_key_map,
            layer_order,
            vocab,
            d_model,
            rank,
            alpha,
            use_rslora: false,
            base_model,
            provenance,
        }
    }

    /// Mark whether this adapter was trained with rank-stabilized LoRA scaling
    /// (`alpha / sqrt(rank)` instead of `alpha / rank`).
    #[must_use]
    pub fn with_rslora(mut self, use_rslora: bool) -> Self {
        self.use_rslora = use_rslora;
        self
    }

    /// Effective `alpha` numerator for `scale = effective_alpha / rank` when merging
    /// or running adapter inference: returns `alpha * sqrt(rank)` when `use_rslora`
    /// is enabled so non-square ranks preserve full `f64` precision.
    #[must_use]
    pub fn effective_alpha_f64(&self) -> f64 {
        if self.use_rslora && self.rank > 0 {
            (self.alpha as f64) * (self.rank as f64).sqrt()
        } else {
            self.alpha as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AdapterMethod, BaseQuantMode, PopuliAdapterManifestV3};
    use std::collections::HashMap;

    #[test]
    fn effective_alpha_f64_preserves_non_square_rslora_precision() {
        let base = PopuliAdapterManifestV3::new(
            AdapterMethod::Qlora,
            BaseQuantMode::Nf4,
            true,
            HashMap::new(),
            vec![],
            32,
            64,
            2,
            1,
            None,
            None,
        );
        assert!(!base.use_rslora);
        assert!((base.effective_alpha_f64() - 1.0).abs() < 1e-12);

        let rslora = base.with_rslora(true);
        assert!(rslora.use_rslora);
        let expected = 2.0_f64.sqrt();
        assert!((rslora.effective_alpha_f64() - expected).abs() < 1e-12);
        // Scale alpha / sqrt(rank) == 1.0 / sqrt(2)
        let scale = rslora.effective_alpha_f64() / (rslora.rank as f64);
        assert!((scale - (1.0 / 2.0_f64.sqrt())).abs() < 1e-12);
    }
}
