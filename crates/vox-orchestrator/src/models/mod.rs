pub(crate) mod spec;
mod registry;
pub mod scoring;
#[cfg(test)]
mod tests;

pub use registry::ModelRegistry;
pub use spec::{
    ModelCapabilities, ModelConfig, ModelRouteBackend, ModelSpec, ModelTier, ProviderType,
    provider_family_strengths, route_backend_for_model, task_category_premium_key,
};
