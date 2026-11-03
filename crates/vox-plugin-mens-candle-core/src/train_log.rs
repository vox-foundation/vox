//! Training progress and GPU visibility via [`tracing`].
//!
//! Canonical home: previously forked between `vox-plugin-mens-candle-metal` and
//! `vox-plugin-mens-candle-cuda`; CUDA's copy had gained the stderr echo below
//! and Metal's hadn't. Both plugins now re-export this module, so Metal gets
//! the same dll-boundary-safe visibility CUDA already had.

use std::fmt::Display;

/// Format a scalar loss for **human-readable** progress logs.
#[must_use]
pub fn format_loss_for_log(loss: f64) -> String {
    if loss.is_nan() {
        return "nan".to_string();
    }
    if loss.is_infinite() {
        return if loss.is_sign_positive() {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    let ax = loss.abs();
    if ax == 0.0 {
        return "0".to_string();
    }
    if !(1e-3..1e3).contains(&ax) {
        format!("{loss:.4e}")
    } else {
        format!("{loss:.4}")
    }
}

pub fn info(msg: &str) {
    // Emit on tracing AND stderr. This crate is loaded as a dynamic cdylib whose
    // `tracing` dispatcher is NOT connected to the host's subscriber across the
    // dll boundary, so tracing-only events are silently dropped — leaving the
    // preflight/graph-build progress invisible during a run. eprintln on stderr
    // (unbuffered, boundary-safe) guarantees these diagnostics are visible live.
    tracing::info!(target: "vox_mens_train", "{}", msg);
    eprintln!("  [mens] {msg}");
}

pub fn warn(msg: &str) {
    tracing::warn!(target: "vox_mens_train", "{}", msg);
    eprintln!("  [mens][warn] {msg}");
}

pub fn _error(msg: impl Display) {
    tracing::error!(target: "vox_mens_train", "{}", msg);
}

/// Human progress line: emit at least every this many optimizer steps.
pub const PROGRESS_EVERY_OPT_STEPS: u32 = 10;
/// Human progress line: emit at least this often (wall clock), even when no
/// optimizer step completed — a run that is skipping every row must not look
/// like a hang.
pub const PROGRESS_MAX_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// Cadence rule for the stderr progress line. Checked once per loop iteration.
///
/// Due on the first optimizer step, then every [`PROGRESS_EVERY_OPT_STEPS`]
/// optimizer steps or every [`PROGRESS_MAX_INTERVAL`], whichever comes first.
#[must_use]
pub fn progress_line_due(
    printed_any: bool,
    opt_steps_since_line: u32,
    since_line: std::time::Duration,
) -> bool {
    (!printed_any && opt_steps_since_line > 0)
        || opt_steps_since_line >= PROGRESS_EVERY_OPT_STEPS
        || since_line >= PROGRESS_MAX_INTERVAL
}

/// Linear ETA from optimizer steps completed in this process (not counting a
/// resumed prefix). `None` until at least one step has completed.
#[must_use]
pub fn eta_secs(
    opt_steps_done_this_run: u32,
    opt_steps_remaining: u32,
    elapsed: std::time::Duration,
) -> Option<u64> {
    if opt_steps_done_this_run == 0 {
        return None;
    }
    let per_step = elapsed.as_secs_f64() / f64::from(opt_steps_done_this_run);
    Some((per_step * f64::from(opt_steps_remaining)).round() as u64)
}

/// Rows skipped so far, by reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkipCounts {
    pub curriculum: u64,
    pub short_seq: u64,
    pub no_supervision: u64,
    pub non_finite: u64,
    pub token_oob: u64,
}

/// Everything shown on one human progress line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressLine {
    pub epoch: usize,
    pub epochs: usize,
    pub opt_step: u32,
    pub opt_steps_planned: u32,
    pub micro_step: u32,
    /// Smoothed (EMA) loss; `None` before the first supervised step.
    pub loss_ema: Option<f64>,
    pub loss_last: Option<f32>,
    pub lr: f64,
    pub tokens_per_sec: Option<f64>,
    pub eta_secs: Option<u64>,
    pub skips: SkipCounts,
}

fn format_hms(s: u64) -> String {
    if s >= 3600 {
        format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

/// Render a [`ProgressLine`] as one concise line (no `[mens]` prefix — [`info`] adds it).
#[must_use]
pub fn format_progress_line(p: &ProgressLine) -> String {
    let pct = if p.opt_steps_planned > 0 {
        100.0 * f64::from(p.opt_step) / f64::from(p.opt_steps_planned)
    } else {
        0.0
    };
    let loss = match (p.loss_ema, p.loss_last) {
        (Some(ema), Some(last)) => format!(
            "loss {} (last {})",
            format_loss_for_log(ema),
            format_loss_for_log(f64::from(last))
        ),
        _ => "loss --".to_string(),
    };
    let tps = p
        .tokens_per_sec
        .map_or_else(|| "-- tok/s".to_string(), |t| format!("{t:.0} tok/s"));
    let eta = p.eta_secs.map_or_else(|| "--".to_string(), format_hms);
    let s = p.skips;
    format!(
        "E{}/{} step {}/{} ({pct:.1}%) micro {} | {loss} | lr {:.2e} | {tps} | ETA {eta} | skipped curric={} short={} no_sup={} nonfinite={} oob={}",
        p.epoch,
        p.epochs,
        p.opt_step,
        p.opt_steps_planned,
        p.micro_step,
        p.lr,
        s.curriculum,
        s.short_seq,
        s.no_supervision,
        s.non_finite,
        s.token_oob,
    )
}

#[cfg(test)]
mod progress_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn first_optimizer_step_is_due() {
        assert!(!progress_line_due(false, 0, Duration::from_secs(1)));
        assert!(progress_line_due(false, 1, Duration::from_millis(10)));
        // After the first line, one step alone is not enough.
        assert!(!progress_line_due(true, 1, Duration::from_millis(10)));
    }

    #[test]
    fn step_count_or_wall_clock_whichever_first() {
        let n = PROGRESS_EVERY_OPT_STEPS;
        assert!(!progress_line_due(true, n - 1, Duration::from_secs(29)));
        assert!(progress_line_due(true, n, Duration::from_secs(1)));
        // Heartbeat with zero steps (every row skipped) still fires.
        assert!(progress_line_due(true, 0, PROGRESS_MAX_INTERVAL));
        assert!(progress_line_due(false, 0, PROGRESS_MAX_INTERVAL));
    }

    #[test]
    fn eta_is_linear_in_remaining_steps() {
        assert_eq!(eta_secs(0, 10, Duration::from_secs(5)), None);
        assert_eq!(eta_secs(4, 6, Duration::from_secs(8)), Some(12));
        assert_eq!(eta_secs(3, 0, Duration::from_secs(8)), Some(0));
    }

    #[test]
    fn line_format_full() {
        let line = format_progress_line(&ProgressLine {
            epoch: 1,
            epochs: 2,
            opt_step: 12,
            opt_steps_planned: 30,
            micro_step: 24,
            loss_ema: Some(1.23456),
            loss_last: Some(1.1),
            lr: 1e-4,
            tokens_per_sec: Some(512.4),
            eta_secs: Some(200),
            skips: SkipCounts {
                curriculum: 1,
                short_seq: 2,
                no_supervision: 3,
                non_finite: 4,
                token_oob: 5,
            },
        });
        assert_eq!(
            line,
            "E1/2 step 12/30 (40.0%) micro 24 | loss 1.2346 (last 1.1000) | lr 1.00e-4 | 512 tok/s | ETA 3m20s | skipped curric=1 short=2 no_sup=3 nonfinite=4 oob=5"
        );
    }

    #[test]
    fn line_format_before_first_step() {
        let line = format_progress_line(&ProgressLine {
            epoch: 1,
            epochs: 1,
            opt_step: 0,
            opt_steps_planned: 0,
            micro_step: 0,
            loss_ema: None,
            loss_last: None,
            lr: 0.0,
            tokens_per_sec: None,
            eta_secs: None,
            skips: SkipCounts::default(),
        });
        assert!(line.contains("step 0/0 (0.0%)"), "{line}");
        assert!(line.contains("loss --"), "{line}");
        assert!(line.contains("-- tok/s | ETA --"), "{line}");
    }

    #[test]
    fn hms_ranges() {
        assert_eq!(format_hms(59), "59s");
        assert_eq!(format_hms(61), "1m01s");
        assert_eq!(format_hms(3725), "1h02m");
    }
}
