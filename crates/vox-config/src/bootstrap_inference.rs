//! Emergency **bootstrap** model identifiers when dynamic catalogs are unavailable.
//!
//! Prefer OpenRouter virtual routes and registry resolution at runtime. The
//! concrete fallback model ids below are NOT defined here: they are the
//! contract defaults from `contracts/orchestration/model-defaults.v1.yaml`
//! (Task 14), re-exported under their historical names via
//! [`crate::model_defaults`].

/// OpenRouter dynamic auto-selection route.
pub const OPENROUTER_AUTO: &str = "openrouter/auto";

/// OpenRouter free-tier preference route (VIRTUAL registry id — NOT dispatchable to
/// the OpenRouter API directly; resolve to concrete `:free` slugs before egress).
#[allow(dead_code)]
pub const OPENROUTER_FREE: &str = "openrouter/free";

/// Concrete, **dispatchable** OpenRouter free-tier model slugs (all end in `:free`,
/// guaranteeing $0), ordered most-capable-first — contract role `free_floor`.
/// Unlike the virtual [`OPENROUTER_FREE`] route, these are real model ids the
/// OpenRouter API accepts directly.
pub const OPENROUTER_FREE_FALLBACK_MODELS: &[&str] = crate::model_defaults::FREE_FLOOR;

/// Research / planner / claim stages when no registry candidate exists — contract
/// role `research`.
pub const RESEARCH_FLASH_FALLBACK: &str = crate::model_defaults::RESEARCH;

/// Review / judge premium fallback when no registry candidate exists — contract
/// role `judge`.
pub const REVIEW_PREMIUM_FALLBACK: &str = crate::model_defaults::JUDGE;

/// NLI / verifier default before a research run replaces it with a resolved
/// model — contract role `claim_extraction`.
pub const NLI_FALLBACK: &str = crate::model_defaults::CLAIM_EXTRACTION;

/// Preferred model for the `vox repair` LLM loop — contract role `code_repair`.
pub const REPAIR_LOOP_PREFERRED: &str = crate::model_defaults::CODE_REPAIR;
