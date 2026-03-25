//! Capability registry for **Mens chat** tool surfaces.
//!
//! Entries with [`PopuliExposure::Auto`] are candidates for advertisement to LLM tool-calling
//! clients; callers must still intersect with in-process executors (e.g. `vox_tools::DirectToolExecutor`).
//! The `vox_tools::mens_chat` module builds OpenAI-style tool definitions from this registry ∩ executor.

mod openai;
mod registry;
mod types;

pub use openai::{capability_to_openai_function, mens_chat_parameters};
pub use registry::default_registry;
pub use types::{CapabilityDescriptor, CapabilityRegistry, InvocationForms, PopuliExposure};
