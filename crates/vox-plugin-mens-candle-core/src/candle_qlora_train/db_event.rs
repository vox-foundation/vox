//! Training-run telemetry event sent to the background VoxDB writer thread.
//!
//! Extracted from `candle_qlora_train::mod` (previously duplicated verbatim in both
//! `vox-plugin-mens-candle-metal` and `vox-plugin-mens-candle-cuda`) so `db_thread.rs`
//! and `epoch_boundary.rs` — themselves byte-identical between the two plugins — have
//! one home instead of two. `pub` here (not `pub(super)`) because both plugin crates
//! construct and match on it across the crate boundary.

pub enum TrainingDbEvent {
    Start {
        run_id: String,
        adapter_tag: Option<String>,
        model_name: Option<String>,
        output_dir: String,
        data_dir: String,
        planned_steps: Option<u32>,
    },
    Checkpoint {
        run_id: String,
        epoch: u32,
        global_step: u32,
        last_loss: Option<f32>,
        adapter_path: String,
    },
    EpochSummary {
        run_id: String,
        epoch: u32,
        global_step: u32,
        avg_loss: f64,
        avg_val_loss: f64,
        val_steps: u32,
    },
    Complete {
        run_id: String,
        global_step: u32,
        adapter_path: String,
    },
    Failed {
        run_id: String,
        global_step: u32,
    },
    GrpoStep {
        run_id: String,
        step: u32,
        mean_reward: f32,
        policy_loss: f32,
        clip_fraction: f32,
        parse_rate: f32,
    },
}
