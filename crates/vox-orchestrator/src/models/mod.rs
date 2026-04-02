mod registry;
mod spec;
#[cfg(test)]
mod tests;

pub use registry::ModelRegistry;
pub use spec::{
    ModelCapabilities, ModelConfig, ModelRouteBackend, ModelSpec, ModelTier, ProviderType,
    route_backend_for_model, task_category_premium_key,
};
