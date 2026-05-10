//! Unified task hopper — Hp-T1 (L1 module, Option A single-machine).
//!
//! The hopper is the intake funnel for all developer-sourced work. Items flow:
//!   submit → Inbox → Assigned → Done
//!   (any non-terminal state can be Overridden by a DeveloperOverride cap)
//!
//! ## Architecture
//!
//! - `types` — pure domain types (IntakeItem, ItemState, PriorityHint, …)
//! - `capability` — DeveloperOverride capability token + minter
//! - `store` — HopperIntake trait + InMemoryHopper (Option A)
//!
//! When Hp-T5 lands (vox-db hopper_inbox table), swap `InMemoryHopper` for
//! the persistent impl through `Arc<dyn HopperIntake>` — the dashboard adapter
//! and HTTP handlers need no changes.

pub mod capability;
pub mod store;
pub mod types;

pub use capability::{DeveloperOverride, DeveloperOverrideMint};
pub use store::{HopperError, HopperIntake, InMemoryHopper};
pub use types::{
    HopperItemId, IntakeItem, IntakeSource, ItemState, PriorityHint, PriorityOverrideRecord,
};
