//! Session lifecycle management for Vox agents.
//!
//! Inspired by OpenClaw's session model:
//!
//! # Architecture
//! - [`SessionManager`] owns the active cache of all sessions across agents.
//! - Restart state is authoritatively loaded from VoxDb via [`SessionManager::load`].
//! - Each session has its own context, permissions, and state.
//! - Supports reset, cleanup, idle timeout, and daily reset policies.

mod config;
mod errors;
mod manager;
mod state;

pub use config::SessionConfig;
pub use errors::SessionError;
pub use manager::SessionManager;
pub use state::{Session, SessionEvent, SessionState, SessionTurn};
