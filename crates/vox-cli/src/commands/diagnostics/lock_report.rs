//! `vox lock-report` — aggregate report of recent lock events (BL009).

use anyhow::Result;

use crate::lock_telemetry::{aggregate_metrics, default_base, is_telemetry_enabled};

/// Run the lock report command.
/// If `fail_threshold` is Some((pkg_wait_max, build_wait_max)), exit non-zero when exceeded.
pub fn run(limit: u64, json: bool, fail_threshold: Option<(u64, u64)>) -> Result<()> {
    let base = default_base();
    if !base.exists() {
        if json {
            println!("{{\"enabled\":false,\"message\":\"No lock telemetry directory found\"}}");
        } else {
            println!(
                "No lock telemetry data found at {}.\n\
                 Enable with: VOX_LOCK_TELEMETRY=1",
                base.display()
            );
        }
        return Ok(());
    }

    let enabled = is_telemetry_enabled();
    let metrics = aggregate_metrics(&base, limit);
    let report = metrics.report();

    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("Lock telemetry enabled: {}", enabled);
        println!("Events (last {}): {}", limit, report.count);
        println!("  package_cache_wait: {}", report.package_cache_wait_count);
        println!("  build_dir_wait: {}", report.build_dir_wait_count);
        if let Some(p50) = report.p50_ms {
            println!("  p50 wait: {} ms", p50);
        }
        if let Some(p95) = report.p95_ms {
            println!("  p95 wait: {} ms", p95);
        }
    }

    if let Some((pkg_max, build_max)) = fail_threshold {
        if report.package_cache_wait_count > pkg_max {
            anyhow::bail!(
                "Lock telemetry threshold exceeded: package_cache_wait {} > {}",
                report.package_cache_wait_count,
                pkg_max
            );
        }
        if report.build_dir_wait_count > build_max {
            anyhow::bail!(
                "Lock telemetry threshold exceeded: build_dir_wait {} > {}",
                report.build_dir_wait_count,
                build_max
            );
        }
    }
    Ok(())
}
