//! Model cards and the ladder index for `vox mens hub sync` (Phase 336, split in phase 339).
//!
//! One renderer for every ladder size. Every number on a card comes from a record in
//! the run directory and names that record; a missing record renders as "not
//! recorded", never as a score. The output depends only on the records, the size, the
//! pin and the namespace (no clock, no Hub state, no host paths), so re-syncing an
//! unchanged run is byte-identical and [`super::hub_sync::manifests_match`] holds.
//!
//! Layout: `facts` reads the records, `card_facts` and `index_facts` turn them into the typed input
//! of the model and index templates, `render` fills the askama templates.

mod card_facts;
mod config;
mod examples;
mod facts;
mod index_facts;
#[cfg(test)]
mod lints;
mod render;
pub mod svg;

#[cfg(test)]
mod tests;

pub use facts::CURRENT_ADAPTER_SERVING;
pub(crate) use facts::{GateState, gate_of};
pub use facts::{PROMPT_PROTOCOL, RunRecords, Score, compare_json_path, heldout_scores};
pub use render::{render_index_readme, render_model_card};
pub use svg::{RenderedAsset, render_index_assets, render_model_assets};

/// `vox-mens-qwen3-<size>`: the repo name, also the local cards subdirectory.
pub fn repo_name(size: &str) -> String {
    format!("vox-mens-qwen3-{}", size.to_ascii_lowercase())
}

/// Everything a size card needs besides its records.
pub struct CardContext<'a> {
    pub size: &'a str,
    pub base_pin: &'a str,
    pub namespace: &'a str,
}
