//! HTTP relay and database persistence for A2A messages.

mod db;

pub use db::{
    acknowledge_db_message, poll_inbox_from_db, prune_old_a2a_messages, send_to_db,
    send_to_db_with_breaker,
};

#[cfg(feature = "populi-transport")]
mod mesh;

#[cfg(feature = "populi-transport")]
pub use mesh::{
    drain_populi_remote_task_results, relay_remote_task_cancel, relay_remote_task_envelope,
    relay_to_mesh,
};
