//! GA roll-up — `vox audit --gate all --strict-block-ga`.
//!
//! Runs every registered gate (foundation first per CR-F0), folds in the
//! standalone product binaries ([`product_binary_gates`]), and writes
//! `contracts/reports/_snapshot/<UTC>.json`. If any foundation gate is red,
//! every downstream gate is forced `blocked_by_foundation` and GA fails.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GateRow {
    pub thing: String,
    pub tier: String,
    pub met: bool,
    pub blocked_by_foundation: bool,
    pub exit_code: i32,
    #[serde(default)]
    pub external_infra: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GaSnapshot {
    pub schema_version: u32,
    pub measured_at: String,
    pub strict_block_ga: bool,
    pub foundation_red: bool,
    pub ga_met: bool,
    pub exit_code: i32,
    pub gates: Vec<GateRow>,
}

impl GaSnapshot {
    /// Build the snapshot from already-evaluated rows, applying CR-F0
    /// foundation-blocking and computing the GA verdict + exit code.
    pub fn from_rows(mut gates: Vec<GateRow>, strict_block_ga: bool) -> Self {
        let foundation_red = gates.iter().any(|g| g.tier == "foundation" && !g.met);
        if foundation_red {
            for g in gates.iter_mut() {
                if g.tier != "foundation" {
                    g.blocked_by_foundation = true;
                }
            }
        }
        // GA is met when every non-tooling gate is met (external_infra gates
        // included — their honest-red state must be cleared for GA) and no
        // foundation gate is red.
        let ga_met = !foundation_red && gates.iter().filter(|g| g.tier != "tooling").all(|g| g.met);
        let exit_code = if strict_block_ga && !ga_met { 1 } else { 0 };
        Self {
            schema_version: 1,
            measured_at: now_rfc3339(),
            strict_block_ga,
            foundation_red,
            ga_met,
            exit_code,
            gates,
        }
    }

    /// Write to `contracts/reports/_snapshot/<YYYY-MM-DD>.json` under the root.
    pub fn write_canonical(&self, root: &std::path::Path) -> std::io::Result<()> {
        let dir = root.join("contracts").join("reports").join("_snapshot");
        std::fs::create_dir_all(&dir)?;
        let date = today_yyyymmdd();
        let body =
            serde_json::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(dir.join(format!("{date}.json")), body)
    }
}

/// Descriptor for a standalone `bin/cr-*.rs` product gate.
pub struct ProductBin {
    pub bin: &'static str,
    pub thing: &'static str,
    pub external_infra: bool,
}

/// Descriptors for the standalone `bin/cr-*.rs` product gates. CR-P* are
/// external_infra (live deploy / soak). cr-p3/cr-e3 are not yet bins — they
/// land in Phase 6.
pub fn product_binary_descriptors() -> Vec<ProductBin> {
    vec![
        ProductBin {
            bin: "cr-a1",
            thing: "cr-a1",
            external_infra: false,
        },
        ProductBin {
            bin: "cr-a2",
            thing: "cr-a2",
            external_infra: false,
        },
        ProductBin {
            bin: "cr-a4",
            thing: "cr-a4",
            external_infra: false,
        },
        ProductBin {
            bin: "cr-d3",
            thing: "cr-d3",
            external_infra: false,
        },
        ProductBin {
            bin: "cr-e1",
            thing: "cr-e1",
            external_infra: false,
        },
        ProductBin {
            bin: "cr-e2",
            thing: "cr-e2",
            external_infra: false,
        },
        ProductBin {
            bin: "cr-p1",
            thing: "cr-p1",
            external_infra: true,
        },
        ProductBin {
            bin: "cr-p2",
            thing: "cr-p2",
            external_infra: true,
        },
    ]
}

/// Run each product binary by resolving its sibling path next to the current
/// executable; record a [`GateRow`]. A missing binary is recorded as a
/// non-met row with `exit_code -1` ("not measured") rather than a panic.
pub fn product_binary_gates(_args: &crate::CommonArgs) -> Vec<GateRow> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    product_binary_descriptors()
        .into_iter()
        .map(|d| {
            let (met, code) = match &exe_dir {
                Some(dir) => {
                    let mut path = dir.join(d.bin);
                    if cfg!(windows) {
                        path.set_extension("exe");
                    }
                    if path.exists() {
                        match std::process::Command::new(&path).output() {
                            Ok(o) => (o.status.success(), o.status.code().unwrap_or(-1)),
                            Err(_) => (false, -1),
                        }
                    } else {
                        (false, -1)
                    }
                }
                None => (false, -1),
            };
            GateRow {
                thing: d.thing.to_string(),
                tier: "product".to_string(),
                met,
                blocked_by_foundation: false,
                exit_code: code,
                external_infra: d.external_infra,
            }
        })
        .collect()
}

/// Outcome of comparing a GA snapshot with the committed known-unmet baseline.
#[derive(Debug, Default, PartialEq)]
pub struct GaRatchet {
    /// GA-relevant gates that are unmet now but not in the baseline (fail the job).
    pub regressions: Vec<String>,
    /// Baseline gates that are met now (tighten the baseline).
    pub newly_met: Vec<String>,
}

/// Compare a GA snapshot with the committed known-unmet baseline. Only GA-relevant gates
/// count (not `tooling`, matching `ga_met`). This never changes the honest GA verdict
/// (`ga_met`); it lets CI fail on *regressions* instead of on "v1.0 is not done yet".
pub fn ratchet(snap: &GaSnapshot, known_unmet: &[String]) -> GaRatchet {
    let mut out = GaRatchet::default();
    for g in snap.gates.iter().filter(|g| g.tier != "tooling") {
        let known = known_unmet.iter().any(|k| k == &g.thing);
        if !g.met && !known {
            out.regressions.push(g.thing.clone());
        } else if g.met && known {
            out.newly_met.push(g.thing.clone());
        }
    }
    out
}

/// Parse `contracts/reports/cr-l-ga-baseline.v1.json`: `{"known_unmet": ["gate", ...]}`.
pub fn parse_ga_baseline(text: &str) -> Result<Vec<String>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let arr = v
        .get("known_unmet")
        .and_then(|x| x.as_array())
        .ok_or("GA baseline needs a `known_unmet` array")?;
    arr.iter()
        .map(|x| {
            x.as_str()
                .map(str::to_string)
                .ok_or_else(|| "known_unmet entries must be strings".to_string())
        })
        .collect()
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn today_yyyymmdd() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(thing: &str, tier: crate::Tier, met: bool) -> GateRow {
        GateRow {
            thing: thing.into(),
            tier: tier.as_str().into(),
            met,
            blocked_by_foundation: false,
            exit_code: if met { 0 } else { 1 },
            external_infra: false,
        }
    }

    #[test]
    fn ratchet_fails_only_on_gates_outside_the_known_unmet_baseline() {
        let rows = vec![
            row("behavioral-goldens", crate::Tier::Foundation, true),
            row("deploy", crate::Tier::Product, false), // known, still red: fine
            row("retirement", crate::Tier::Product, false), // was green: regression
            row("cr-e1", crate::Tier::Product, true),   // known, now green: tighten
            row("stdlib-coverage", crate::Tier::Tooling, false), // tooling: not GA
        ];
        let snap = GaSnapshot::from_rows(rows, true);
        let known = vec!["deploy".to_string(), "cr-e1".to_string()];
        let r = ratchet(&snap, &known);
        assert_eq!(r.regressions, vec!["retirement".to_string()]);
        assert_eq!(r.newly_met, vec!["cr-e1".to_string()]);
    }

    #[test]
    fn ratchet_is_clean_when_only_known_gates_are_red() {
        let rows = vec![
            row("behavioral-goldens", crate::Tier::Foundation, true),
            row("deploy", crate::Tier::Product, false),
        ];
        let snap = GaSnapshot::from_rows(rows, true);
        let r = ratchet(&snap, &["deploy".to_string()]);
        assert!(r.regressions.is_empty() && r.newly_met.is_empty());
        assert!(
            !snap.ga_met,
            "the ratchet never changes the honest GA verdict"
        );
    }

    #[test]
    fn ga_baseline_parses_known_unmet() {
        let parsed =
            parse_ga_baseline(r#"{"schema_version":1,"known_unmet":["deploy","cr-p1"]}"#).unwrap();
        assert_eq!(parsed, vec!["deploy".to_string(), "cr-p1".to_string()]);
        assert!(parse_ga_baseline("{}").is_err(), "known_unmet is required");
    }

    #[test]
    fn red_foundation_blocks_all_downstream() {
        let rows = vec![
            row("behavioral-goldens", crate::Tier::Foundation, false),
            row("retirement", crate::Tier::Product, true),
        ];
        let snap = GaSnapshot::from_rows(rows, /* strict */ true);
        assert!(snap.foundation_red);
        let downstream = snap.gates.iter().find(|g| g.thing == "retirement").unwrap();
        assert!(
            downstream.blocked_by_foundation,
            "product row must be blocked when foundation is red"
        );
        assert!(!snap.ga_met);
        assert_ne!(snap.exit_code, 0);
    }

    #[test]
    fn all_green_passes_ga() {
        let rows = vec![
            row("behavioral-goldens", crate::Tier::Foundation, true),
            row("retirement", crate::Tier::Product, true),
        ];
        let snap = GaSnapshot::from_rows(rows, true);
        assert!(!snap.foundation_red);
        assert!(snap.ga_met);
        assert_eq!(snap.exit_code, 0);
    }

    #[test]
    fn external_infra_red_does_not_block_when_not_strict() {
        // A built-but-unrun external_infra gate is honest-red; non-strict GA
        // reports it but exits 0.
        let mut r = row("cr-p2", crate::Tier::Product, false);
        r.external_infra = true;
        let snap = GaSnapshot::from_rows(vec![r], /* strict */ false);
        assert!(!snap.ga_met);
        assert_eq!(snap.exit_code, 0, "non-strict run never fails the build");
    }

    #[test]
    fn product_binary_descriptors_cover_existing_bins() {
        let names: Vec<&str> = product_binary_descriptors().iter().map(|d| d.bin).collect();
        for expected in [
            "cr-a1", "cr-a2", "cr-a4", "cr-d3", "cr-e1", "cr-e2", "cr-p1", "cr-p2",
        ] {
            assert!(
                names.contains(&expected),
                "missing descriptor for {expected}"
            );
        }
        let p1 = product_binary_descriptors()
            .into_iter()
            .find(|d| d.bin == "cr-p1")
            .unwrap();
        assert!(p1.external_infra);
        let a1 = product_binary_descriptors()
            .into_iter()
            .find(|d| d.bin == "cr-a1")
            .unwrap();
        assert!(!a1.external_infra);
    }
}
