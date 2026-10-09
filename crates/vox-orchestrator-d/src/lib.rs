//! Profile-gated boot for `vox-orchestrator-d` (capability profiles plan 03).
//!
//! The binary in `src/bin/vox_orchestrator_d.rs` builds a [`daemon_steps::DaemonBootCtx`]
//! and runs [`daemon_steps::DAEMON_STEPS`] through [`boot_plan::run_boot_plan`].

pub mod boot_plan;
pub mod daemon_steps;
