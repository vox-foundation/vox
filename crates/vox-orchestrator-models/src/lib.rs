//! Model calibration, usage accounting, and budget policy types for the Vox orchestrator.

pub mod calibration;
pub mod routing_task;
pub mod usage;
pub mod usage_policy;
pub mod vram_probe;

pub use routing_task::RoutingTask;
pub use vram_probe::VramProbe;

#[cfg(test)]
mod tests {
    #[test]
    fn reexport_paths_resolve() {
        let _: Option<crate::usage::RemainingBudget> = None;
    }
}
