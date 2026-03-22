//! SQLite pragmas applied on every [`crate::store::CodeStore`] open path.
//!
//! Implemented in [`CodeStore::apply_pragmas`](crate::store::CodeStore::apply_pragmas) using
//! [`turso::Connection::pragma_update`] because `PRAGMA name = value` returns a result row and
//! must not be sent through [`turso::Connection::execute_batch`] (which uses `execute` only).
//!
//! Applied settings:
//! - `journal_mode=WAL`
//! - `busy_timeout=5000`
//! - `synchronous=NORMAL`
//! - `foreign_keys=ON`
//! - `cache_size=-8000`
