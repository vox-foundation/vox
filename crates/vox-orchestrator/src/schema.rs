//! Schema definitions for orchestrator document collections (SSOT: `vox-db` crate).

use vox_db::schema_digest::SchemaDigest;

/// Standard [`SchemaDigest`] for `Orchestrator::init_db` — defined in `vox-db::schema::spec`.
#[inline]
#[must_use]
pub fn orchestrator_schema() -> SchemaDigest {
    vox_db::schema::orchestrator_schema_digest()
}
