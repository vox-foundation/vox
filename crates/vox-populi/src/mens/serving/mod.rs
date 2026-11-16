//! Serving layer for Mens fine-tuned adapters.
//!
//! Provides a vLLM multi-LoRA client ([`vllm_lora`]) with provenance enforcement,
//! LRU slot management, and schema-guided-decoding request construction.

#[cfg(feature = "mens-hf-hub")]
pub mod spoke_route;
pub mod vllm_lora;

#[cfg(feature = "mens-hf-hub")]
pub use spoke_route::resolve_spoke_adapter;
pub use vllm_lora::{AdapterEntry, VllmLoraClient};
