//! Stable `vox_scientia_core::*` module paths for Scientia publication models.
//!
//! Implementation still lives in [`vox_publisher`] until the split in
//! `docs/src/architecture/scientia-pipeline-ssot-2026.md` is completed; this crate is a
//! **facade** so `vox-scientia-api` / `vox-scientia-runtime` can depend on a narrow core surface
//! without a `vox-publisher` → `vox-scientia-core` dependency cycle.

#![forbid(unsafe_code)]

pub mod contracts {
    pub use vox_publisher::scientia_contracts::*;
}

pub mod discovery {
    pub use vox_publisher::scientia_discovery::*;
}

pub mod evidence {
    pub use vox_publisher::scientia_evidence::*;
}

pub mod finding_ledger {
    pub use vox_publisher::scientia_finding_ledger::*;
}

pub mod heuristics {
    pub use vox_publisher::scientia_heuristics::*;
}

pub mod prior_art {
    pub use vox_publisher::scientia_prior_art::*;
}

/// Worthiness scoring and inputs (publisher `publication_worthiness` module).
pub mod worthiness {
    pub use vox_publisher::publication_worthiness::*;
}
