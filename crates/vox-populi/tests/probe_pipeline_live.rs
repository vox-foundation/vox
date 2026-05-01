// Live hardware integration tests.
//
// These tests require actual GPU hardware and are gated behind the
// `hw-probe-live-test` feature flag.  Do NOT enable this flag on CI.
//
// Run manually:
//   cargo test -p vox-populi --test probe_pipeline_live --features hw-probe-live-test
//
// All tests in this file are skipped at compile time unless the feature is active.

#![cfg(feature = "hw-probe-live-test")]

use vox_populi::mens::hardware::pipeline::ProbePipeline;
use vox_populi::mens::hardware::probe::ProbeOutcome;

/// At least one probe must return `Found` on a machine with a GPU.
#[tokio::test]
async fn live_default_pipeline_finds_gpu() {
    let report = ProbePipeline::default_for_platform().run().await;
    let found = report
        .attempts
        .iter()
        .any(|a| matches!(a.outcome, ProbeOutcome::Found(_)));
    assert!(
        found,
        "expected at least one probe to return Found on a GPU machine; attempts: {:?}",
        report
            .attempts
            .iter()
            .map(|a| (a.probe_name, &a.outcome))
            .collect::<Vec<_>>()
    );
}

/// VRAM must be > 0 on a machine with a dedicated GPU.
#[tokio::test]
async fn live_pipeline_reports_nonzero_vram() {
    let report = ProbePipeline::default_for_platform().run().await;
    assert!(
        report.summary.vram_mb > 0,
        "expected vram_mb > 0 on a GPU machine; got {}",
        report.summary.vram_mb
    );
}

/// Probe must complete within a reasonable wall-clock timeout.
#[tokio::test]
async fn live_pipeline_completes_within_10s() {
    let start = std::time::Instant::now();
    let _ = ProbePipeline::default_for_platform().run().await;
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 10,
        "pipeline took too long: {elapsed:?}"
    );
}

/// gpu_count must be at least 1 on a GPU machine.
#[tokio::test]
async fn live_pipeline_reports_at_least_one_gpu() {
    let report = ProbePipeline::default_for_platform().run().await;
    assert!(
        report.summary.gpu_count >= 1,
        "expected gpu_count >= 1; got {}",
        report.summary.gpu_count
    );
}
