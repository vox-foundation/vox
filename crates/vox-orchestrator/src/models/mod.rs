mod registry;
mod spec;
#[cfg(test)]
mod tests;

pub use registry::ModelRegistry;
pub use spec::{
    ModelCapabilities, ModelConfig, ModelSpec, ModelTier, ProviderType, task_category_premium_key,
};
