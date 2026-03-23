//! Async CRUD and analytics for `CodeStore`.
//!
//! This file contains core connection management and common helpers. 
//! Specialized domain methods are in `ops_*.rs` submodules.

use crate::store::CodeStore;

impl CodeStore {
    /// Borrow the underlying libSQL connection (`vox-db`, migrations, tests).
    #[inline]
    #[must_use]
    pub fn connection(&self) -> &turso::Connection {
        &self.conn
    }

    /// Run an async database operation from synchronous call sites (e.g. `std::thread` workers).
    ///
    /// If called from a Tokio worker, uses `block_in_place` + the current handle; otherwise builds
    /// a single-threaded runtime for the duration of the future.
    pub fn block_on<R: Send>(&self, fut: impl std::future::Future<Output = R> + Send) -> R {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build Tokio runtime for CodeStore::block_on")
                .block_on(fut),
        }
    }
}
