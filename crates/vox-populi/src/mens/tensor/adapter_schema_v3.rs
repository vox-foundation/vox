//! Unified adapter artifact **schema v3** (method + quant + base key map + layer order).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::finetune_contract::{AdapterMethod, BaseQuantMode};

/// On-disk adapter bundle descriptor (sidecar JSON next to `.safetensors`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopuliAdapterManifestV3 {
    pub format: String,
    pub version: u32,
    #[serde(flatten)]
    pub method: AdapterMethodFields,
    pub quant: QuantFields,
    /// Logical layer id → HF safetensors key for the frozen base that was adapted.
    pub base_key_map: HashMap<String, String>,
    /// Stable training / merge order.
    pub layer_order: Vec<String>,
    pub vocab: usize,
    pub d_model: usize,
    pub rank: usize,
    pub alpha: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMethodFields {
    pub adapter_method: AdapterMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantFields {
    pub base_quant: BaseQuantMode,
    #[serde(default = "default_true")]
    pub double_quant: bool,
}

fn default_true() -> bool {
    true
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
        }
    }
}

/// Upgrade Candle QLoRA v2 meta JSON into v3 view (lossless for merge inputs).
/// Build v2 sidecar shape for merge pipeline (adapter safetensors layout unchanged).
#[cfg(feature = "mens-candle-qlora")]
pub fn to_qlora_meta_v2_for_merge(
    m: &PopuliAdapterManifestV3,
) -> anyhow::Result<super::candle_qlora_merge::QloraAdapterMetaV2> {
    let embed_key = m.base_key_map.get("lm_head").cloned().unwrap_or_default();
    if embed_key.is_empty() {
        anyhow::bail!("adapter manifest v3: base_key_map missing `lm_head` → HF embed key");
    }
    Ok(super::candle_qlora_merge::QloraAdapterMetaV2 {
        format: super::candle_qlora_merge::QloraAdapterMetaV2::FORMAT.to_string(),
        version: super::candle_qlora_merge::QloraAdapterMetaV2::VERSION,
        embed_key,
        vocab: m.vocab,
        d_model: m.d_model,
        rank: m.rank,
        alpha: m.alpha,
        layer_order: m.layer_order.clone(),
        base_key_map: m.base_key_map.clone(),
        base_model: None,
    })
}

#[cfg(feature = "mens-candle-qlora")]
pub fn from_qlora_meta_v2(
    v2: &super::candle_qlora_merge::QloraAdapterMetaV2,
) -> PopuliAdapterManifestV3 {
    PopuliAdapterManifestV3::new(
        AdapterMethod::Qlora,
        BaseQuantMode::Nf4,
        true,
        v2.base_key_map.clone(),
        v2.layer_order.clone(),
        v2.vocab,
        v2.d_model,
        v2.rank,
        v2.alpha,
    )
}

#[cfg(all(test, feature = "mens-candle-qlora"))]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn v3_roundtrip_serde_and_merge_bridge() {
        let mut map = HashMap::new();
        map.insert("lm_head".into(), "wte.weight".into());
        map.insert("mid0".into(), "h.0.attn.c_proj.weight".into());
        let m = PopuliAdapterManifestV3::new(
            AdapterMethod::Qlora,
            BaseQuantMode::Nf4,
            true,
            map.clone(),
            vec!["mid0".into(), "lm_head".into()],
            99,
            32,
            4,
            8,
        );
        let json = serde_json::to_string(&m).expect("ser");
        let back: PopuliAdapterManifestV3 = serde_json::from_str(&json).expect("de");
        assert_eq!(back.vocab, 99);
        let v2 = to_qlora_meta_v2_for_merge(&back).expect("bridge");
        assert_eq!(v2.embed_key, "wte.weight");
        assert_eq!(v2.layer_order.len(), 2);
    }

    #[test]
    fn v3_double_quant_roundtrips_serde() {
        let mut map = HashMap::new();
        map.insert("lm_head".into(), "wte.weight".into());
        let mut m = PopuliAdapterManifestV3::new(
            AdapterMethod::Qlora,
            BaseQuantMode::Nf4,
            true,
            map,
            vec!["lm_head".into()],
            10,
            8,
            4,
            8,
        );
        m.quant.double_quant = false;
        let json = serde_json::to_string(&m).expect("ser");
        let back: PopuliAdapterManifestV3 = serde_json::from_str(&json).expect("de");
        assert!(!back.quant.double_quant);
    }
}
