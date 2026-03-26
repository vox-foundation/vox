//! Top-level `vox db` clap enum — flattened from [`DbCliCore`] and [`DbCliPublication`] for line budget.

use clap::Subcommand;

use super::{DbCliCore, DbCliPublication};

/// Subcommands for `vox db` (flat CLI surface via `flatten`).
#[derive(Subcommand)]
pub enum DbCli {
    #[command(flatten)]
    Core(DbCliCore),
    #[command(flatten)]
    Publication(DbCliPublication),
}
