//! Session lifecycle management for Vox agents.
//!
//! Inspired by OpenClaw's session model:
//! - When a DB is attached, **`agent_sessions` + `agent_session_events` in Codex** are the durable SSOT.
//! - JSONL under [`SessionConfig::sessions_dir`] is an **optional, non-authoritative** export when `persist` is enabled (telemetry / human-readable traces — not replay truth).
//! - Each session has its own context, permissions, and state.
//! - Supports reset, cleanup, idle timeout, and daily reset policies.
//! - Restarts should reload from Codex via [`SessionManager::load`]; JSONL replay is legacy fallback only ([`SessionManager::load_from_jsonl`]).

mod config;
mod errors;
mod manager;
mod state;

pub use config::SessionConfig;
pub use errors::SessionError;
pub use manager::SessionManager;
pub use state::{Session, SessionEvent, SessionState, SessionTurn};
