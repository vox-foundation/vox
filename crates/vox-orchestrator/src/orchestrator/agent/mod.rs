//! Agent lifecycle: spawn, retire, session mapping, handoff, pause/resume, heartbeat.
//!
//! All methods here operate on the `agents` / `agent_handles` maps and the supporting
//! subsystems (lock manager, affinity map, scope guard, heartbeat monitor).

mod doubt;
mod fallback;
mod handoff;
mod lifecycle_ops;
mod registration;
mod spawn;
