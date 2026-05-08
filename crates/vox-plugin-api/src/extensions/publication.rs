//! Publication extension point — for plugins that crawl external feeds
//! (RSS/Atom) and publish to platforms (Reddit, YouTube, scholarly job feeds).
//!
//! Used by `vox-plugin-publication`. Routed through the host so that
//! `vox-cli` does not take a direct dependency on the publication crate
//! (and its transitive `feed-rs` / `vox-publisher` deps).

use abi_stable::{sabi_trait, std_types::*};

pub const PUBLICATION_REVISION: u32 = 1;

#[sabi_trait]
pub trait Publication: Send + Sync {
    fn revision(&self) -> u32 {
        PUBLICATION_REVISION
    }

    /// Run one batch of RSS/Atom crawling + semantic dedup tick.
    ///
    /// `feed_id` filters to a single registered feed source by id when
    /// `RSome`; `RNone` means "ingest from all registered sources". `limit`
    /// caps the number of items processed per feed in this tick.
    ///
    /// Telemetry / progress is emitted by the plugin via `tracing` and
    /// stdout/stderr; this method returns `Ok(())` on a successful tick
    /// even if individual feeds fail to crawl (those failures are reported
    /// inline). It returns `Err` only on a fatal setup problem (DB
    /// connection failure, embedder init failure, etc.).
    fn ingest_tick(
        &self,
        feed_id: ROption<RString>,
        limit: u32,
    ) -> RResult<(), RBoxError>;
}
