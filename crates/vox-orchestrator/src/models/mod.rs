pub mod generated;
pub mod key_guard;
mod registry;
pub mod routing_table;
pub mod scoring;
pub(crate) mod spec;
#[cfg(test)]
mod tests;

pub use generated::{ModelTier, StrengthTag, TaskCategory};
pub use registry::{ModelRegistry, ModelScore};
pub use spec::{
    ModelCapabilities, ModelConfig, ModelRouteBackend, ModelSpec, ProviderType,
    route_backend_for_model, task_category_premium_key, task_category_strength,
};
