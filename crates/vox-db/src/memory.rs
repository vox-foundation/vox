//! Parameters for [`crate::VoxDb::store_memory`].
//!
//! Alias of [`crate::arca_store::SaveMemoryParams`] so application code can depend on `vox-db` only.

pub type MemoryParams<'a> = crate::arca_store::SaveMemoryParams<'a>;
