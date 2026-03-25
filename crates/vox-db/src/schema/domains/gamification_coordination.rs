//! Arca SQL: Ludus gamification + mens coordination (gamification + coordination fragments).
//!
//! Raw DDL lives under [`sql/`](sql/) so [`crate::schema::baseline_sql`] and Ludus can share the
//! gamification slice without drifting.

/// Gamification tables only — matches Arca baseline; use for Ludus `SCHEMA_V5` alignment.
pub const SCHEMA_GAMIFICATION_ONLY: &str = include_str!("sql/gamification.sql");

pub const SCHEMA_GAMIFICATION_COORDINATION: &str = concat!(
    include_str!("sql/gamification.sql"),
    "\n\n",
    include_str!("sql/coordination.sql"),
);
