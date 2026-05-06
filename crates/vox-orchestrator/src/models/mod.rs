pub mod generated;
pub mod key_guard;
mod registry;
pub mod routing_table;
pub mod scoring;
pub(crate) mod spec;
#[cfg(test)]
mod tests;

pub use generated::{
    infer_capabilities, infer_prompt_intents, intent_required_capabilities, Capability,
    CapabilityFlags, ModelTier, PromptIntent, StrengthTag, TaskCategory,
};
pub use registry::{ModelRegistry, ModelScore};
pub use spec::{
    ModelCapabilities, ModelConfig, ModelRouteBackend, ModelSpec, PricingSource, ProviderType,
    route_backend_for_model, task_category_premium_key, task_category_strength,
};
