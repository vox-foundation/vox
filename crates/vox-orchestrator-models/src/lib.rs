//! Model calibration, usage accounting, and budget policy types for the Vox orchestrator.

pub mod calibration;
pub mod usage;
pub mod usage_policy;

#[cfg(test)]
mod tests {
    #[test]
    fn reexport_paths_resolve() {
        let _: Option<crate::usage::RemainingBudget> = None;
    }
}
