//! Verify parity between `docs/src/architecture/docs-system-ssot.md` (§4 gate table, §6 gaps)
//! and the actual guard wiring in `pre_push.rs`, `ssot_drift.rs`, `lefthook.yml`, and
//! `.github/workflows/*.yml`.
//!
//! Runs as stage `docs_system_parity` inside `vox ci ssot-drift`.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use anyhow::{Result, bail};

use crate::commands::ci::pre_push;
use crate::commands::ci::run_body::run_body_helpers::ssot_drift;

const SSOT_DOC_REL: &str = "docs/src/architecture/docs-system-ssot.md";

/// Parsed execution location token from §4's `Where it runs` column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loc {
    PreCommit,
    PrePushFast,
    PrePushComplete,
    SsotDrift,
    PrCi,
    CargoTest(String),
    Workflow(String),
}

/// Parsed gate row from §4 of `docs-system-ssot.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRow {
    pub id: String,
    pub where_cell: String,
    pub locs: Vec<Loc>,
    pub line: usize,
}

/// Parsed gap bullet from §6 of `docs-system-ssot.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapBullet {
    pub id: String,
    pub text: String,
    pub is_closed: bool,
    pub line: usize,
}

/// Code-side probe binding a documented gate ID to a source file and substring needle.
#[derive(Debug, Clone, Copy)]
pub struct Probe {
    pub gate_id: &'static str,
    pub source_rel: &'static str,
    pub needle: &'static str,
}

pub const GATE_PROBES: &[Probe] = &[
    Probe {
        gate_id: "doc-lint",
        source_rel: "crates/vox-cli/src/commands/ci/pre_push.rs",
        needle: "step_doc_frontmatter",
    },
    Probe {
        gate_id: "doctest-md",
        source_rel: "crates/vox-cli/src/commands/ci/pre_push.rs",
        needle: "step_doctest",
    },
    Probe {
        gate_id: "facet-backlog",
        source_rel: "crates/vox-cli/src/commands/ci/pre_push.rs",
        needle: "step_facet_backlog_gate",
    },
    Probe {
        gate_id: "mermaid-ratchet",
        source_rel: "crates/vox-cli/src/commands/ci/pre_push.rs",
        needle: "step_mermaid_lint",
    },
    Probe {
        gate_id: "ssot-drift",
        source_rel: "crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs",
        needle: "fn ssot_drift_stages",
    },
    Probe {
        gate_id: "ssot-autoregen",
        source_rel: ".github/workflows/ci.yml",
        needle: "ssot-autoregen:",
    },
    Probe {
        gate_id: "check-links",
        source_rel: "crates/vox-cli/src/commands/ci/pre_push.rs",
        needle: "step_check_links",
    },
    Probe {
        gate_id: "starlight-links",
        source_rel: "docs-astro/astro.config.mjs",
        needle: "starlight-links-validator",
    },
    Probe {
        gate_id: "external-links",
        source_rel: ".github/workflows/docs-drift-refresh.yml",
        needle: "lycheeverse/lychee-action",
    },
    Probe {
        gate_id: "check-drift",
        source_rel: ".github/workflows/docs-quality.yml",
        needle: "audit docs --check-drift",
    },
    Probe {
        gate_id: "retired-symbol-check",
        source_rel: "crates/vox-cli/src/commands/ci/pre_push.rs",
        needle: "step_retired_symbol_check",
    },
    Probe {
        gate_id: "canonical-map-verify",
        source_rel: "crates/vox-cli/src/commands/ci/pre_push.rs",
        needle: "step_canonical_map_verify",
    },
    Probe {
        gate_id: "sync-ignore-files",
        source_rel: "crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs",
        needle: "sync-ignore-files",
    },
    Probe {
        gate_id: "playwright-docs",
        source_rel: ".github/workflows/docs-quality.yml",
        needle: "tests/build-output/",
    },
    Probe {
        gate_id: "frontmatter-governance-parity",
        source_rel: "crates/vox-doc-pipeline/tests/frontmatter_governance_parity.rs",
        needle: "test_frontmatter_governance_parity",
    },
    Probe {
        gate_id: "overlay-parity",
        source_rel: "crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs",
        needle: "overlay_parity",
    },
    Probe {
        gate_id: "docs-cli-claims",
        source_rel: "crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs",
        needle: "docs_cli_claims",
    },
    Probe {
        gate_id: "docs-orphan-pages",
        source_rel: "crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs",
        needle: "docs_orphan_pages",
    },
    Probe {
        gate_id: "docs-drift-refresh",
        source_rel: ".github/workflows/docs-drift-refresh.yml",
        needle: "docs-reality --update-ledger",
    },
    Probe {
        gate_id: "docs-system-parity",
        source_rel: "crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs",
        needle: "docs_system_parity",
    },
];

/// Gap IDs resolved by Phase 343 that must never reappear as open in §6.
pub const CLOSED_GAPS: &[&str] = &[
    "overlay-contradiction",
    "aspirational-cli-claim",
    "silent-noop-guard",
    "orphan-page",
    "dormant-freshness-ledger",
    "blocking-external-links",
];

fn strip_parenthetical(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    let mut depth = 0usize;
    for ch in token.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.replace('`', "").trim().to_string()
}

/// Parse a §4 `Where it runs` cell into canonical `Loc` values.
pub fn parse_locations(cell: &str) -> Result<Vec<Loc>> {
    let mut locs = Vec::new();
    for raw in cell.split(',') {
        let cleaned = strip_parenthetical(raw);
        let lower = cleaned.to_ascii_lowercase();
        if lower.is_empty() {
            continue;
        }
        let loc = match lower.as_str() {
            "pre-commit" => Loc::PreCommit,
            "pre-push fast" => Loc::PrePushFast,
            "pre-push complete" => Loc::PrePushComplete,
            "ssot-drift" => Loc::SsotDrift,
            "pr ci" => Loc::PrCi,
            _ if lower.starts_with("cargo test -p ") => {
                let pkg = cleaned["cargo test -p ".len()..].trim().to_string();
                if pkg.is_empty() {
                    bail!("empty package in location `{raw}`");
                }
                Loc::CargoTest(pkg)
            }
            _ if lower.ends_with(".yml") || lower.ends_with(".yaml") => Loc::Workflow(cleaned),
            _ => bail!("unrecognized location token `{raw}` in Where it runs cell `{cell}`"),
        };
        locs.push(loc);
    }
    if locs.is_empty() {
        bail!("empty Where it runs cell `{cell}`");
    }
    Ok(locs)
}

fn extract_html_marker(text: &str, prefix: &str) -> Option<String> {
    let start = text.find(prefix)?;
    let rest = &text[start + prefix.len()..];
    let end = rest.find("-->")?;
    let id = rest[..end].trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// Parse §4 table rows carrying `<!-- gate: <id> -->` markers.
pub fn parse_gate_rows(md: &str) -> Result<Vec<GateRow>> {
    let mut in_sec4 = false;
    let mut seen_header = false;
    let mut rows = Vec::new();
    for (idx, line) in md.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            if in_sec4 {
                break;
            }
            if trimmed.starts_with("## 4.") {
                in_sec4 = true;
            }
            continue;
        }
        if !in_sec4 {
            continue;
        }
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
        // Leading and trailing `|` produce empty outer entries -> >= 5 slices for a 4-col table.
        if cells.len() < 5 {
            continue;
        }
        let col1 = cells[1];
        let col3 = cells[3];
        if col1 == "Gate" || col1.chars().all(|c| c == '-' || c == ':') {
            seen_header = true;
            continue;
        }
        if !seen_header {
            continue;
        }
        let Some(gate_id) = extract_html_marker(col1, "<!-- gate:") else {
            bail!(
                "{SSOT_DOC_REL}:{}: §4 gate row missing `<!-- gate: <id> -->` marker in Gate cell `{col1}`",
                idx + 1
            );
        };
        let locs = parse_locations(col3)
            .map_err(|e| anyhow::anyhow!("{SSOT_DOC_REL}:{}: {e}", idx + 1))?;
        rows.push(GateRow {
            id: gate_id,
            where_cell: col3.to_string(),
            locs,
            line: idx + 1,
        });
    }
    if !in_sec4 {
        bail!("{SSOT_DOC_REL}: missing `## 4.` section");
    }
    Ok(rows)
}

/// Parse §6 bullets carrying `- <!-- gap: <id> -->` markers.
pub fn parse_gap_bullets(md: &str) -> Result<Vec<GapBullet>> {
    let mut in_sec6 = false;
    let mut in_closed_subsec = false;
    let mut bullets = Vec::new();
    for (idx, line) in md.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            if in_sec6 {
                break;
            }
            if trimmed.starts_with("## 6.") {
                in_sec6 = true;
            }
            continue;
        }
        if !in_sec6 {
            continue;
        }
        if trimmed.starts_with("### ") {
            in_closed_subsec = trimmed.to_ascii_lowercase().contains("closed");
            continue;
        }
        if !trimmed.starts_with("- ") {
            continue;
        }
        let Some(id) = extract_html_marker(trimmed, "<!-- gap:") else {
            bail!(
                "{SSOT_DOC_REL}:{}: §6 bullet missing `<!-- gap: <id> -->` marker: `{trimmed}`",
                idx + 1
            );
        };
        let is_closed = in_closed_subsec || trimmed.contains("**Closed");
        bullets.push(GapBullet {
            id,
            text: trimmed.to_string(),
            is_closed,
            line: idx + 1,
        });
    }
    if !in_sec6 {
        bail!("{SSOT_DOC_REL}: missing `## 6.` section");
    }
    Ok(bullets)
}

/// Check parity between `md` (the SSOT doc markdown) and code/workflow ground truth under `root`.
pub fn check_parity(root: &Path, md: &str) -> Vec<String> {
    let mut errors = Vec::new();

    let rows = match parse_gate_rows(md) {
        Ok(r) => r,
        Err(e) => {
            errors.push(e.to_string());
            return errors;
        }
    };

    let mut seen_ids = HashSet::new();
    let mut doc_ids = BTreeSet::new();
    for row in &rows {
        if !seen_ids.insert(row.id.clone()) {
            errors.push(format!(
                "{SSOT_DOC_REL}:{}: duplicate gate id `{}`",
                row.line, row.id
            ));
        }
        doc_ids.insert(row.id.clone());
    }

    let probe_ids: BTreeSet<&'static str> = GATE_PROBES.iter().map(|p| p.gate_id).collect();
    for id in &doc_ids {
        if !probe_ids.contains(id.as_str()) {
            errors.push(format!(
                "{SSOT_DOC_REL}: §4 gate `{id}` has no probe entry in docs_system_parity::GATE_PROBES"
            ));
        }
    }
    for probe in GATE_PROBES {
        if !doc_ids.contains(probe.gate_id) {
            errors.push(format!(
                "{SSOT_DOC_REL}: docs_system_parity::GATE_PROBES gate `{}` is missing from §4 table",
                probe.gate_id
            ));
        }
        let p = root.join(probe.source_rel);
        match std::fs::read_to_string(&p) {
            Ok(body) => {
                if !body.contains(probe.needle) {
                    errors.push(format!(
                        "gate `{}`: source `{}` does not contain needle `{}`",
                        probe.gate_id, probe.source_rel, probe.needle
                    ));
                }
            }
            Err(e) => {
                errors.push(format!(
                    "gate `{}`: missing probe source `{}`: {e}",
                    probe.gate_id, probe.source_rel
                ));
            }
        }
    }

    let stages: HashSet<&'static str> = ssot_drift::ssot_drift_stages()
        .into_iter()
        .map(|s| s.name)
        .collect();
    let lefthook_text = std::fs::read_to_string(root.join("lefthook.yml")).ok();
    let is_git_repo = root.join(".git").exists();
    let fast_labels = if is_git_repo {
        pre_push::tier_labels(root, false).ok()
    } else {
        None
    };
    let complete_labels = if is_git_repo {
        pre_push::tier_labels(root, true).ok()
    } else {
        None
    };

    for row in &rows {
        for loc in &row.locs {
            match loc {
                Loc::PreCommit => {
                    let ok = lefthook_text
                        .as_deref()
                        .is_some_and(|t| t.contains("pre-commit:"));
                    if !ok {
                        errors.push(format!(
                            "{SSOT_DOC_REL}:{}: gate `{}` claims pre-commit, but lefthook.yml lacks `pre-commit:`",
                            row.line, row.id
                        ));
                    }
                }
                Loc::PrePushFast => {
                    if let Some(ref labels) = fast_labels
                        && labels.is_empty()
                    {
                        errors.push(format!(
                            "{SSOT_DOC_REL}:{}: pre-push fast tier resolved zero steps for `{}`",
                            row.line, row.id
                        ));
                    }
                }
                Loc::PrePushComplete => {
                    if let Some(ref labels) = complete_labels
                        && labels.is_empty()
                    {
                        errors.push(format!(
                            "{SSOT_DOC_REL}:{}: pre-push complete tier resolved zero steps for `{}`",
                            row.line, row.id
                        ));
                    }
                }
                Loc::SsotDrift => {
                    let stage_name = row.id.replace('-', "_");
                    if row.id != "ssot-drift"
                        && row.id != "sync-ignore-files"
                        && !stages.contains(stage_name.as_str())
                    {
                        errors.push(format!(
                            "{SSOT_DOC_REL}:{}: gate `{}` claims `ssot-drift`, but `{stage_name}` is not in ssot_drift_stages()",
                            row.line, row.id
                        ));
                    }
                }
                Loc::PrCi => {
                    let has_ci = root.join(".github/workflows/ci.yml").is_file()
                        || root.join(".github/workflows/docs-quality.yml").is_file();
                    if !has_ci {
                        errors.push(format!(
                            "{SSOT_DOC_REL}:{}: gate `{}` claims PR CI, but ci.yml/docs-quality.yml missing",
                            row.line, row.id
                        ));
                    }
                }
                Loc::CargoTest(pkg) => {
                    let crate_dir = root.join("crates").join(pkg);
                    if !crate_dir.join("Cargo.toml").is_file() {
                        errors.push(format!(
                            "{SSOT_DOC_REL}:{}: gate `{}` references crate `{pkg}`, but `{}` is missing",
                            row.line,
                            row.id,
                            crate_dir.display()
                        ));
                    }
                }
                Loc::Workflow(wf) => {
                    let wf_path = root.join(".github/workflows").join(wf);
                    if !wf_path.is_file() {
                        errors.push(format!(
                            "{SSOT_DOC_REL}:{}: gate `{}` names workflow `{wf}`, which does not exist at `{}`",
                            row.line,
                            row.id,
                            wf_path.display()
                        ));
                    }
                }
            }
        }
    }

    match parse_gap_bullets(md) {
        Ok(bullets) => {
            let mut gap_ids = HashSet::new();
            for b in bullets {
                if !gap_ids.insert(b.id.clone()) {
                    errors.push(format!(
                        "{SSOT_DOC_REL}:{}: duplicate §6 gap id `{}`",
                        b.line, b.id
                    ));
                }
                if CLOSED_GAPS.contains(&b.id.as_str()) && !b.is_closed {
                    errors.push(format!(
                        "{SSOT_DOC_REL}:{}: closed gap `{}` is still listed as open in §6",
                        b.line, b.id
                    ));
                }
            }
        }
        Err(e) => errors.push(e.to_string()),
    }

    errors
}

/// Entrypoint called by `vox ci ssot-drift` stage `docs_system_parity`.
pub fn run(root: &Path) -> Result<()> {
    let ssot_path = root.join(SSOT_DOC_REL);
    let md = vox_bounded_fs::read_utf8_path_capped(&ssot_path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", ssot_path.display()))?;
    let errors = check_parity(root, &md);
    if !errors.is_empty() {
        bail!(
            "docs_system_parity failed ({} issue(s)):\n  - {}",
            errors.len(),
            errors.join("\n  - ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn real_repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn real_tree_docs_system_ssot_parity() {
        let root = real_repo_root();
        run(&root).expect("real repo docs-system-ssot.md parity must pass");
    }

    #[test]
    fn unknown_workflow_in_where_column_fails() {
        let root = real_repo_root();
        let ssot = std::fs::read_to_string(root.join(SSOT_DOC_REL)).unwrap();
        let mutated = ssot.replace("docs-drift-refresh.yml (weekly)", "nonexistent-wf.yml");
        assert_ne!(mutated, ssot);
        let errs = check_parity(&root, &mutated);
        assert!(
            errs.iter().any(|e| e.contains("nonexistent-wf.yml")),
            "expected nonexistent-wf.yml error, got: {errs:?}"
        );
    }

    #[test]
    fn missing_gate_marker_or_probe_mismatch_fails() {
        let root = real_repo_root();
        let ssot = std::fs::read_to_string(root.join(SSOT_DOC_REL)).unwrap();
        let mutated = ssot.replace("<!-- gate: doc-lint --> ", "");
        assert_ne!(mutated, ssot);
        let errs = check_parity(&root, &mutated);
        assert!(
            errs.iter().any(|e| e.contains("<!-- gate: <id> -->")),
            "expected missing marker error, got: {errs:?}"
        );
    }

    #[test]
    fn closed_gap_still_listed_as_open_fails() {
        let root = real_repo_root();
        let ssot = std::fs::read_to_string(root.join(SSOT_DOC_REL)).unwrap();
        let mutated = ssot.replace(
            "### Open gaps",
            "### Open gaps\n\n- <!-- gap: dormant-freshness-ledger --> Freshness ledger is still manual.",
        );
        assert_ne!(mutated, ssot);
        let errs = check_parity(&root, &mutated);
        assert!(
            errs.iter()
                .any(|e| e.contains("dormant-freshness-ledger")
                    && e.contains("still listed as open")),
            "expected closed gap error, got: {errs:?}"
        );
    }
}
