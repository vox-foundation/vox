//! Hold-out validation pass re-export (delegates to training_loop/validation.rs).
//!
//! SP3-C: kept as thin wrapper for symmetry with vox-populi module layout.

pub use crate::candle_qlora_train::training_loop::validation::run_validation_pass;
