//! Idempotent Ludus / gamification DDL alignment for Arca baseline databases.
//!
//! Baseline [`crate::schema::baseline_sql`] only includes core `gamify_*` tables; this cutover adds
//! extended Ludus tables and repairs historical naming drift (`gamify_collegiums`, `counter_name`).

use turso::Connection;

use crate::store::types::StoreError;

/// Previously applied Ludus gamification schema additions.
/// All affected tables are now present in SCHEMA_GAMIFICATION_COORDINATION baseline fragment.
/// All column renames (collegiums→collegium, mode→mode_label, etc.) are complete for
/// BASELINE_VERSION >= 47. This function is now a no-op and will be deleted in CARD-03b.
pub async fn apply_ludus_gamify_cutover(_conn: &Connection) -> Result<(), StoreError> {
    Ok(())
}
