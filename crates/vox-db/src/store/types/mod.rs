//! Errors for [`crate::VoxDb`]. Other request parameter and row types now live in
//! [`vox_db_types`] and are re-exported below for back-compat.

mod error;

pub use error::StoreError;
pub use vox_db_types::store_types::{mens::*, params::*, rows_core::*, rows_extended::*};
