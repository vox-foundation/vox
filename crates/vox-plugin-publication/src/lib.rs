//! Publication plugin for Vox.
//!
//! Consolidates publication concerns as an opt-in plugin:
//! - Ingest: RSS/Atom feed crawling (`vox-scientia-ingest::FeedCrawler`) with
//!   semantic deduplication (`IngestDeduplicator`).
//! - Publish: Reddit, YouTube, and scholarly job feeds via `vox-publisher`
//!   (features: `scientia-reddit`, `scientia-youtube`, `scholarly-external-jobs`).
//!
//! Default `vox-cli` builds do not include this plugin or its heavy transitive
//! deps (`feed-rs`, `fnv`, platform HTTP clients). Users opt in via:
//!   `vox plugin install publication`

use abi_stable::{erased_types::TD_Opaque, export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn, std_types::*};
use vox_plugin_api::abi::{VoxPlugin, VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
use vox_plugin_api::host::VoxHost_TO;
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;

// Re-export public surface from dependent libraries so hosts can reach them
// without depending on those crates directly.
pub use vox_scientia_ingest::{FeedCrawler, FeedSource, InboundItem, IngestDeduplicator};

pub mod ingest;
pub use ingest::ingest_tick;

#[export_root_module]
fn root_module() -> VoxPluginRootRef {
    VoxPluginRoot {
        abi_version: VOX_PLUGIN_ABI_VERSION,
        manifest_json,
        init,
    }
    .leak_into_prefix()
}

#[sabi_extern_fn]
fn manifest_json() -> RString {
    RString::from(r#"{"id":"publication","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    tracing::info!("vox-plugin-publication loaded (RSS ingest + Reddit/YouTube/scholarly publish)");
    let plugin = PublicationPlugin;
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}

struct PublicationPlugin;

impl VoxPlugin for PublicationPlugin {
    fn id(&self) -> RString {
        RString::from("publication")
    }

    fn shutdown(&self) -> RResult<(), RBoxError> {
        RResult::ROk(())
    }
}
