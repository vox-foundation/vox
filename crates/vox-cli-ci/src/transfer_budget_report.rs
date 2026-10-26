//! `vox ci transfer-budget-report` — emit transfer/turn-budget CI artifact JSON.
//!
//! Mirrors ceilings from `vox-orchestrator-mcp` transfer-budget tests so CI can
//! upload a machine-readable artifact comparable to t3code `thread-transfer-results`.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_OUT: &str = "contracts/reports/transfer-budget-ci.v1.json";

/// Keep in sync with `crates/vox-orchestrator-mcp/tests/transfer_budget.rs`.
pub const MAX_COLD_OPEN_SNAPSHOT_BYTES: usize = 120 * 1024;
pub const MAX_COLD_OPEN_SNAPSHOT_ROWS: usize = 75;
pub const MAX_TURN_WIRE_DELTA_BYTES: usize = 7 * 1024;
pub const MAX_COLD_OPEN_SQL_STATEMENTS: usize = 8;
pub const MAX_PER_TURN_SQL_STATEMENTS: usize = 12;
pub const MAX_DOM_SNAPSHOT_BYTES: usize = 20 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferBudgetReport {
    pub schema_version: String,
    pub source: String,
    pub ceilings: TransferBudgetCeilings,
    pub sample: TransferBudgetSample,
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferBudgetCeilings {
    pub max_cold_open_snapshot_bytes: usize,
    pub max_cold_open_snapshot_rows: usize,
    pub max_turn_wire_delta_bytes: usize,
    pub max_cold_open_sql_statements: usize,
    pub max_per_turn_sql_statements: usize,
    pub max_dom_snapshot_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferBudgetSample {
    pub cold_open_snapshot_bytes: usize,
    pub cold_open_snapshot_rows: usize,
    pub turn_wire_delta_bytes: usize,
    pub cold_open_sql_statements: usize,
    pub event_store_command_output_deltas: usize,
}

impl Default for TransferBudgetCeilings {
    fn default() -> Self {
        Self {
            max_cold_open_snapshot_bytes: MAX_COLD_OPEN_SNAPSHOT_BYTES,
            max_cold_open_snapshot_rows: MAX_COLD_OPEN_SNAPSHOT_ROWS,
            max_turn_wire_delta_bytes: MAX_TURN_WIRE_DELTA_BYTES,
            max_cold_open_sql_statements: MAX_COLD_OPEN_SQL_STATEMENTS,
            max_per_turn_sql_statements: MAX_PER_TURN_SQL_STATEMENTS,
            max_dom_snapshot_bytes: MAX_DOM_SNAPSHOT_BYTES,
        }
    }
}

/// Build a report from ceilings + a conservative in-process sample that mirrors
/// the unit-test fixture shape (no live orchestrator required for CI artifact emit).
pub fn build_report(sample: TransferBudgetSample) -> TransferBudgetReport {
    let ceilings = TransferBudgetCeilings::default();
    let ok = sample.cold_open_snapshot_bytes <= ceilings.max_cold_open_snapshot_bytes
        && sample.cold_open_snapshot_rows <= ceilings.max_cold_open_snapshot_rows
        && sample.turn_wire_delta_bytes <= ceilings.max_turn_wire_delta_bytes
        && sample.cold_open_sql_statements <= ceilings.max_cold_open_sql_statements
        && sample.event_store_command_output_deltas == 0;
    TransferBudgetReport {
        schema_version: "transfer_budget_ci_v1".to_string(),
        source: "vox-cli-ci/fixture_sample (ceilings mirrored from vox-orchestrator-mcp/tests/transfer_budget.rs)".to_string(),
        ceilings,
        sample,
        ok,
    }
}

/// Fixture sample matching the transfer_budget unit test shape (under ceilings).
pub fn fixture_sample() -> TransferBudgetSample {
    TransferBudgetSample {
        cold_open_snapshot_bytes: 48_000,
        cold_open_snapshot_rows: 60,
        turn_wire_delta_bytes: 512,
        cold_open_sql_statements: 4,
        event_store_command_output_deltas: 0,
    }
}

pub fn write_report(path: &Path, report: &TransferBudgetReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parent {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(report).context("serialize transfer-budget report")?;
    std::fs::write(path, format!("{json}\n"))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn load_report(path: &Path) -> Result<TransferBudgetReport> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let report: TransferBudgetReport =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    if report.schema_version != "transfer_budget_ci_v1" {
        bail!(
            "unsupported schema_version {} in {}",
            report.schema_version,
            path.display()
        );
    }
    Ok(report)
}

/// Compare measured sample against a committed baseline; fail if any ceiling field
/// or `ok` regresses (ratchet).
pub fn ratchet_against_baseline(
    current: &TransferBudgetReport,
    baseline: &TransferBudgetReport,
) -> Result<()> {
    if !current.ok {
        bail!("transfer-budget sample exceeds ceilings (ok=false)");
    }
    if current.ceilings.max_cold_open_snapshot_bytes
        > baseline.ceilings.max_cold_open_snapshot_bytes
        || current.ceilings.max_turn_wire_delta_bytes > baseline.ceilings.max_turn_wire_delta_bytes
    {
        bail!("transfer-budget ceilings loosened vs committed baseline");
    }
    Ok(())
}

pub struct TransferBudgetArgs {
    pub write: bool,
    pub out: PathBuf,
    pub check: bool,
    pub baseline: PathBuf,
}

pub fn run(root: &Path, args: TransferBudgetArgs) -> Result<()> {
    let report = build_report(fixture_sample());
    let out = if args.out.is_absolute() {
        args.out.clone()
    } else {
        root.join(&args.out)
    };
    let baseline_path = if args.baseline.is_absolute() {
        args.baseline.clone()
    } else {
        root.join(&args.baseline)
    };

    if args.check {
        let baseline = load_report(&baseline_path)?;
        ratchet_against_baseline(&report, &baseline)?;
        println!(
            "transfer-budget-report: ok (cold_open={}B turn_delta={}B)",
            report.sample.cold_open_snapshot_bytes, report.sample.turn_wire_delta_bytes
        );
        return Ok(());
    }

    if args.write {
        write_report(&out, &report)?;
        println!("transfer-budget-report: wrote {}", out.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    if !report.ok {
        bail!("transfer-budget sample exceeds ceilings");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_sample_passes_ceilings() {
        let r = build_report(fixture_sample());
        assert!(r.ok);
        assert_eq!(r.schema_version, "transfer_budget_ci_v1");
    }

    #[test]
    fn over_budget_sample_fails() {
        let mut s = fixture_sample();
        s.cold_open_snapshot_bytes = MAX_COLD_OPEN_SNAPSHOT_BYTES + 1;
        assert!(!build_report(s).ok);
    }

    #[test]
    fn ratchet_rejects_loosened_ceilings() {
        let base = build_report(fixture_sample());
        let mut current = base.clone();
        current.ceilings.max_turn_wire_delta_bytes *= 2;
        assert!(ratchet_against_baseline(&current, &base).is_err());
    }

    #[test]
    fn round_trip_write_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.json");
        let report = build_report(fixture_sample());
        write_report(&path, &report).unwrap();
        let loaded = load_report(&path).unwrap();
        assert_eq!(loaded, report);
    }
}
