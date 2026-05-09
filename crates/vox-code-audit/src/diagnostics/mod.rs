//! Stable diagnostic ID catalog and explain infrastructure.
//!
//! See [`catalog`] for the full list of stable IDs.

pub mod catalog;

pub use catalog::{ALL_KNOWN_IDS, ALL_PHASE2_IDS, explain_url, is_known_id};
