//! OOM classification and reporting for the training step loop.
//!
//! `is_oom` / `OomEvent` / `render_oom` are defactored from
//! `vox-populi/src/mens/tensor/memory_budget.rs` (2026-09-11): this plugin
//! crate cannot depend on `vox-populi` (no allowed edge in
//! `contracts/ci/crate-edges.allow.v1.json`), and the three functions are
//! well under the ~50-line defactor threshold in `AGENTS.md` §Dependency
//! Discipline. Keep both copies' `is_oom` string matches in sync by hand.
// vox:defactored-from vox-populi 2026-09-11

/// A single out-of-memory event observed mid-training.
#[derive(Debug, Clone)]
pub(super) struct OomEvent {
    pub step: u32,
    pub observed_bytes: u64,
    pub batch_size: usize,
    pub seq_len: usize,
    pub predicted_bytes: u64,
    /// `Some(reason)` when the OOM-triggered checkpoint save itself failed
    /// (plausible: saving may need to allocate on the same device that just
    /// OOMed). `None` means the checkpoint save reported success.
    pub checkpoint_error: Option<String>,
}

/// True if `e` is a CUDA or Metal allocator failure, false for anything else
/// (I/O, config, etc.).
pub(super) fn is_oom(e: &anyhow::Error) -> bool {
    let s = e.to_string().to_ascii_lowercase();
    s.contains("out of memory")
        || s.contains("greater than the maximum allowed buffer size")
        || s.contains("mtlcommandbuffererror(8)")
}

/// Format a byte count for the OOM report. `0` is the sentinel this crate
/// uses for "no telemetry reading available" (see `training_loop/mod.rs`),
/// not a real zero measurement — render it as `"unknown"` so the report
/// doesn't read as a fabricated zero.
fn fmt_gib_or_unknown(bytes: u64) -> String {
    if bytes == 0 {
        "unknown".to_string()
    } else {
        format!("{:.2} GiB", bytes as f64 / 1e9)
    }
}

/// Render an [`OomEvent`] into an actionable report: where training died,
/// what shape it was attempting, and whether the run is actually resumable
/// from a checkpoint.
pub(super) fn render_oom(ev: &OomEvent) -> String {
    let checkpoint_note = match &ev.checkpoint_error {
        None => "checkpoint flushed, resume the run to continue".to_string(),
        Some(reason) => format!(
            "checkpoint save ALSO failed: {reason} — this run cannot be resumed from a checkpoint"
        ),
    };
    format!(
        "out of memory at step {step}: attempting batch {batch} seq_len {seq_len} \
         (predicted {predicted}, observed {observed} peak); {checkpoint_note}",
        step = ev.step,
        batch = ev.batch_size,
        seq_len = ev.seq_len,
        predicted = fmt_gib_or_unknown(ev.predicted_bytes),
        observed = fmt_gib_or_unknown(ev.observed_bytes),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_lanes_oom_is_recognised_and_nothing_else_is() {
        for s in [
            "CUDA out of memory. Tried to allocate 2.00 GiB",
            "[METAL] Attempting to allocate 122.10 GiB which is greater than the maximum allowed buffer size",
            "Metal error: MTLCommandBufferError(8)",
        ] {
            assert!(is_oom(&anyhow::anyhow!("{s}")), "missed a real OOM: {s}");
        }
        assert!(
            !is_oom(&anyhow::anyhow!(
                "failed to open train.jsonl: No such file or directory"
            )),
            "a disk error must not be reported as an OOM, or the fix suggested is wrong"
        );
    }

    #[test]
    fn an_oom_report_names_the_step_the_shape_and_the_recovery() {
        let s = render_oom(&OomEvent {
            step: 412,
            observed_bytes: 96_000_000_000,
            batch_size: 4,
            seq_len: 1024,
            predicted_bytes: 122_100_000_000,
            checkpoint_error: None,
        });
        assert!(s.contains("412"), "say where it died: {s}");
        assert!(
            s.contains("batch 4") && s.contains("1024"),
            "say what it was attempting: {s}"
        );
        assert!(s.contains("checkpoint"), "say the run is resumable: {s}");
        assert!(
            !s.contains("unknown"),
            "real byte counts must not render as unknown: {s}"
        );
    }

    #[test]
    fn a_zero_byte_count_renders_as_unknown_not_a_fabricated_zero() {
        let s = render_oom(&OomEvent {
            step: 1,
            observed_bytes: 0,
            batch_size: 1,
            seq_len: 8,
            predicted_bytes: 0,
            checkpoint_error: None,
        });
        assert!(
            !s.contains("0.00 GiB"),
            "an unknown reading must not look like a real zero measurement: {s}"
        );
        assert!(
            s.contains("unknown"),
            "must say the reading is unknown: {s}"
        );
    }

    #[test]
    fn a_failed_checkpoint_save_is_reported_not_hidden_behind_the_usual_wording() {
        let s = render_oom(&OomEvent {
            step: 5,
            observed_bytes: 1_000_000_000,
            batch_size: 1,
            seq_len: 8,
            predicted_bytes: 1_000_000_000,
            checkpoint_error: Some("disk full".to_string()),
        });
        assert!(
            s.contains("cannot be resumed"),
            "a failed save must not claim the run is resumable: {s}"
        );
        assert!(
            s.contains("disk full"),
            "must surface the save failure reason: {s}"
        );
    }
}
