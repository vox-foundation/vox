//! Agent lifecycle: spawn, retire, session mapping, handoff, pause/resume, heartbeat.
//!
//! All methods here operate on the `agents` / `agent_handles` maps and the supporting
//! subsystems (lock manager, affinity map, scope guard, heartbeat monitor).

mod spawn;
mod lifecycle_ops;
mod registration;
mod doubt;
mod handoff;
mod fallback;
