//! User-defined SQL migrations sharing the same `schema_version` table as built-in Arca migrations.
//!
//! Prefer [`crate::builtin_migrations`] when you need the canonical baseline snapshot as a single
//! migration row (**version 1**). For custom migrations, ensure [`Migration::up_sql`] is compatible with
//! [`turso::Connection::execute_batch`] (no row-returning statements).

use crate::arca_store::StoreError;

/// One forward migration applied in monotonically increasing [`Self::version`] order.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Must be unique, greater than zero, and strictly increasing across the slice passed to [`crate::VoxDb::apply_migrations`].
    pub version: i64,
    /// Human-readable label (logging only; not stored in DB).
    pub name: String,
    /// Semicolon-separated SQL executed via `execute_batch` when `version` is ahead of the DB.
    pub up_sql: String,
}

impl Migration {
    /// Construct a migration entry (does not run SQL until [`crate::VoxDb::apply_migrations`]).
    pub fn new(version: i64, name: impl Into<String>, up_sql: impl Into<String>) -> Self {
        Self {
            version,
            name: name.into(),
            up_sql: up_sql.into(),
        }
    }
}

/// Validate strictly increasing versions and no duplicates.
///
/// **Note:** validation failures are reported as [`StoreError::NotFound`] with a message string for
/// historical reasons; they are not “not found” semantically. Callers should match on the message
/// or treat any `Err` as fatal.
pub fn validate_migrations(migrations: &[Migration]) -> Result<(), StoreError> {
    let mut seen = std::collections::BTreeSet::new();
    let mut last = 0i64;
    for migration in migrations {
        if migration.version <= 0 {
            return Err(StoreError::NotFound(
                "migration version must be > 0".to_string(),
            ));
        }
        if migration.version <= last {
            return Err(StoreError::NotFound(
                "migrations must be sorted by increasing version".to_string(),
            ));
        }
        if !seen.insert(migration.version) {
            return Err(StoreError::NotFound(format!(
                "duplicate migration version {}",
                migration.version
            )));
        }
        last = migration.version;
    }
    Ok(())
}

/// Returns the canonical baseline migration (**version 1**) from the `vox-pm` schema manifest.
pub fn builtin_migrations() -> Vec<Migration> {
    vec![Migration::new(
        crate::schema::BASELINE_VERSION,
        "arca_baseline_v1",
        crate::schema::baseline_sql().to_string(),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_sorted_unique() {
        let migrations = vec![
            Migration::new(1, "one", "CREATE TABLE a(id INTEGER);"),
            Migration::new(2, "two", "CREATE TABLE b(id INTEGER);"),
        ];
        assert!(validate_migrations(&migrations).is_ok());
    }
}
