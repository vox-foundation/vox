//! Emergency **bootstrap** model identifiers when dynamic catalogs are unavailable.
//!
//! Prefer OpenRouter virtual routes and registry resolution at runtime; these strings are the
//! single workspace fallback surface until `vox-dei` is restored as a full member crate.

/// OpenRouter dynamic auto-selection route.
pub const OPENROUTER_AUTO: &str = "openrouter/auto";

/// OpenRouter free-tier preference route.
#[allow(dead_code)]
pub const OPENROUTER_FREE: &str = "openrouter/free";

/// Research / planner / claim stages when no registry candidate exists.
pub const RESEARCH_FLASH_FALLBACK: &str = "gpt-4o-mini";

/// Review / judge premium fallback when no registry candidate exists.
pub const REVIEW_PREMIUM_FALLBACK: &str = "gpt-4o";

/// NLI / verifier default before research run replaces it with a resolved model.
pub const NLI_FALLBACK: &str = "gpt-4o-mini";
