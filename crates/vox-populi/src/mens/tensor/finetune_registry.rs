//! Registries for **adapter methods**, **quantization providers**, and **target selectors** (extension points).

use std::collections::HashMap;

use super::finetune_contract::{AdapterMethod, AdapterTargetMask, BaseQuantMode};
use super::train_backend::PopuliTrainBackend;

/// Declares which execution kernel implements a method today.
#[derive(Debug, Clone)]
pub struct AdapterMethodRecord {
    pub id: AdapterMethod,
    pub default_kernel: PopuliTrainBackend,
}

#[derive(Debug, Default)]
pub struct AdapterMethodRegistry {
    inner: HashMap<AdapterMethod, AdapterMethodRecord>,
}

impl AdapterMethodRegistry {
    pub fn builtin() -> Self {
        let mut inner = HashMap::new();
        inner.insert(
            AdapterMethod::Lora,
            AdapterMethodRecord {
                id: AdapterMethod::Lora,
                default_kernel: PopuliTrainBackend::BurnLora,
            },
        );
        inner.insert(
            AdapterMethod::Qlora,
            AdapterMethodRecord {
                id: AdapterMethod::Qlora,
                default_kernel: PopuliTrainBackend::CandleQlora,
            },
        );
        Self { inner }
    }

    pub fn resolve(&self, m: AdapterMethod) -> Option<&AdapterMethodRecord> {
        self.inner.get(&m)
    }
}

#[derive(Debug, Clone)]
pub struct QuantRecord {
    pub mode: BaseQuantMode,
    pub default_kernel: PopuliTrainBackend,
}

#[derive(Debug, Default)]
pub struct QuantizationProviderRegistry {
    inner: HashMap<BaseQuantMode, QuantRecord>,
}

impl QuantizationProviderRegistry {
    pub fn builtin() -> Self {
        let mut inner = HashMap::new();
        inner.insert(
            BaseQuantMode::None,
            QuantRecord {
                mode: BaseQuantMode::None,
                default_kernel: PopuliTrainBackend::BurnLora,
            },
        );
        inner.insert(
            BaseQuantMode::Nf4,
            QuantRecord {
                mode: BaseQuantMode::Nf4,
                default_kernel: PopuliTrainBackend::CandleQlora,
            },
        );
        Self { inner }
    }

    pub fn resolve(&self, q: BaseQuantMode) -> Option<&QuantRecord> {
        self.inner.get(&q)
    }
}

#[derive(Debug, Clone)]
pub struct TargetSelectorRecord {
    pub mask: AdapterTargetMask,
    pub note: &'static str,
}

#[derive(Debug, Default)]
pub struct TargetSelectorRegistry {
    inner: HashMap<AdapterTargetMask, TargetSelectorRecord>,
}

impl TargetSelectorRegistry {
    pub fn builtin() -> Self {
        let mut inner = HashMap::new();
        inner.insert(
            AdapterTargetMask::FullGraph,
            TargetSelectorRecord {
                mask: AdapterTargetMask::FullGraph,
                note: "Burn causal LoRA on all adapted linears in LoraVoxTransformer",
            },
        );
        inner.insert(
            AdapterTargetMask::LmHeadProxy,
            TargetSelectorRecord {
                mask: AdapterTargetMask::LmHeadProxy,
                note: "Candle qlora-rs: o_proj stack + LM head (proxy; not full block parity)",
            },
        );
        Self { inner }
    }

    pub fn resolve(&self, m: AdapterTargetMask) -> Option<&TargetSelectorRecord> {
        self.inner.get(&m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_custom_method_variant_without_cli_breakage() {
        // Proving enum extension pattern: new methods are added to AdapterMethod + registry together.
        let reg = AdapterMethodRegistry::builtin();
        assert!(reg.resolve(AdapterMethod::Lora).is_some());
        assert!(reg.resolve(AdapterMethod::Qlora).is_some());
    }
}
