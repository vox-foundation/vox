//! Core connection and async bridge helpers for [`VoxDb`].
//!
//! Domain-specific CRUD lives in the sibling submodules:
//! - [`super::ops_cas`]: content-addressed storage (`objects`, `names`, `schema_version`)
//! - [`super::ops_agents`]: agent sessions and LLM interactions
//! - [`super::ops_memory`]: memories, knowledge nodes, embeddings, components
//! - [`super::ops_learning`]: behavioral learning, patterns, training data
//! - [`super::ops_codex`]: codex reactivity, research graph, reliability, eval, corpus



impl crate::VoxDb {
    /// Borrow the underlying libSQL connection.
    ///
    /// Prefer typed `VoxDb` methods over calling SQL directly; use this only for
    /// one-off queries that do not belong to any domain module or for test verification.
    #[inline]
    #[must_use]
    pub fn connection(&self) -> &turso::Connection {
        &self.conn
    }

    /// Run an async future from a synchronous call site (e.g. a `std::thread` worker).
    ///
    /// Uses `block_in_place` when a Tokio runtime is active; falls back to a fresh
    /// single-threaded runtime otherwise. Panics if the runtime cannot be created.
    pub fn block_on<R: Send>(&self, fut: impl std::future::Future<Output = R> + Send) -> R {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("VoxDb::block_on could not build Tokio runtime")
                .block_on(fut),
        }
    }
}
