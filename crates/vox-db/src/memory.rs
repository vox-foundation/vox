//! Parameters for [`crate::VoxDb::store_memory`].
//!
//! Alias of [`vox_pm::SaveMemoryParams`] so application code can depend on `vox-db` only.

pub type MemoryParams<'a> = vox_pm::SaveMemoryParams<'a>;
