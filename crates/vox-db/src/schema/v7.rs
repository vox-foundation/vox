/// V7: no-op marker (reserved); keeps version alignment with historical branches.
///
/// Must not contain row-returning statements: Turso applies migrations via
/// [`turso::Connection::execute_batch`], which uses `execute` and rejects `SELECT`.
pub const SCHEMA_V7: &str = "";
