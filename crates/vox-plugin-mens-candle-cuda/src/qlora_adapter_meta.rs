//! QLoRA adapter v2 sidecar metadata.
//!
//! Ported from `vox-populi/src/mens/tensor/candle_qlora_merge.rs` (SP3 sub-batch C).
//! Only the metadata struct is needed here (the actual merge logic stays in vox-populi).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Sidecar written next to `candle_qlora_adapter.safetensors` for format v2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QloraAdapterMetaV2 {
    pub format: String,
    pub version: u32,
    pub embed_key: String,
    pub vocab: usize,
    pub d_model: usize,
    pub rank: usize,
    pub alpha: usize,
    /// Adapter logical names in training order.
    pub layer_order: Vec<String>,
    /// Maps adapter name → base safetensors key for that frozen weight.
    pub base_key_map: HashMap<String, String>,
    /// Base model repo ID or path.
    pub base_model: Option<String>,
}

impl QloraAdapterMetaV2 {
    pub const FORMAT: &'static str = "vox_mens_qlora_lora_only_v2";
    pub const VERSION: u32 = 2;
}
