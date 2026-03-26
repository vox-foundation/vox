//! `vox ci scaling-audit` — validate scaling SSOT and emit per-crate backlog artifacts.

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::bounded_read::read_utf8_path_capped;
use super::cargo_bin;
use super::cmd_enums::ScalingAuditCmd;

const POLICY_REL: &str = "contracts/scaling/policy.yaml";
const POLICY_SCHEMA_REL: &str = "contracts/scaling/policy.schema.json";
const TEMPLATES_REL: &str = "contracts/scaling/task-templates.yaml";
const REPORT_ROOT: &str = "contracts/reports/scaling-audit";
const FINDINGS_LATEST_REL: &str = "contracts/reports/scaling-audit/findings-latest.json";
const FINDINGS_ARRAY_SCHEMA_REL: &str =
    "contracts/reports/scaling-audit/findings-array.v1.schema.json";
const GOLD_DATASET_REL: &str = "contracts/toestub/gold-dataset.v1.json";
const GOLD_SCHEMA_REL: &str = "contracts/toestub/gold-dataset.v1.schema.json";
const REMEDIATION_LANES_REL: &str = "contracts/reports/toestub-remediation/REMEDIATION-LANES.yaml";
const REMEDIATION_DELTA_REL: &str =
    "contracts/reports/toestub-remediation/delta-after-remediation.json";
const REMEDIATION_DELTA_SCHEMA_REL: &str =
    "contracts/reports/toestub-remediation/delta-after-remediation.v1.schema.json";
const REMEDIATION_REPORT_ROOT: &str = "contracts/reports/toestub-remediation";

#[derive(Debug, Deserialize)]
struct TaskTemplatesFile {
    #[allow(dead_code)]
    schema_version: u32,
    categories: Vec<TaskCategory>,
}

#[derive(Debug, Deserialize)]
struct TaskCategory {
    id: String,
    pattern: String,
    impact: String,
    risk: String,
    title: String,
    verification: String,
}

/// Validate `contracts/scaling/policy.yaml` against `policy.schema.json`.
fn verify_policy_schema(repo_root: &Path) -> Result<()> {
    let policy_path = repo_root.join(POLICY_REL);
    let raw = read_utf8_path_capped(&policy_path)
        .with_context(|| format!("read {}", policy_path.display()))?;
    let schema_path = repo_root.join(POLICY_SCHEMA_REL);
    if !schema_path.is_file() {
        return Err(anyhow!("missing {}", schema_path.display()));
    }
    let schema_val: JsonValue = serde_json::from_str(
        &read_utf8_path_capped(&schema_path)
            .with_context(|| format!("read {}", schema_path.display()))?,
    )
    .with_context(|| format!("parse {}", schema_path.display()))?;
    let instance: JsonValue =
        serde_yaml::from_str(&raw).context("parse scaling policy as JSON value")?;
    let validator =
        jsonschema::validator_for(&schema_val).context("compile scaling policy.schema.json")?;
    validator
        .validate(&instance)
        .map_err(|e| anyhow!("scaling policy does not match schema: {e}"))?;
    println!("scaling policy YAML OK (schema validated)");
    Ok(())
}

/// Embedded policy must match on-disk file (vox-scaling-policy `include_str`).
fn verify_embedded_policy_roundtrip(repo_root: &Path) -> Result<()> {
    let policy_path = repo_root.join(POLICY_REL);
    let disk = read_utf8_path_capped(&policy_path)
        .with_context(|| format!("read {}", policy_path.display()))?;
    let parsed = vox_scaling_policy::ScalingPolicy::from_yaml_str(&disk)
        .context("parse policy with vox-scaling-policy")?;
    if parsed.schema_version < 1 {
        return Err(anyhow!("scaling policy schema_version must be >= 1"));
    }
    println!("vox-scaling-policy parse OK");
    Ok(())
}

fn validate_json_file_against_schema(
    repo_root: &Path,
    instance_rel: &str,
    schema_rel: &str,
    label: &str,
) -> Result<()> {
    let inst_path = repo_root.join(instance_rel);
    if !inst_path.is_file() {
        return Err(anyhow!("missing {} file {}", label, inst_path.display()));
    }
    let schema_path = repo_root.join(schema_rel);
    if !schema_path.is_file() {
        return Err(anyhow!(
            "missing {} schema {}",
            label,
            schema_path.display()
        ));
    }
    let schema_val: JsonValue = serde_json::from_str(
        &read_utf8_path_capped(&schema_path)
            .with_context(|| format!("read {}", schema_path.display()))?,
    )
    .with_context(|| format!("parse {}", schema_path.display()))?;
    let instance_val: JsonValue = serde_json::from_str(
        &read_utf8_path_capped(&inst_path)
            .with_context(|| format!("read {}", inst_path.display()))?,
    )
    .with_context(|| format!("parse {}", inst_path.display()))?;
    let validator = jsonschema::validator_for(&schema_val)
        .with_context(|| format!("compile {label} schema"))?;
    validator.validate(&instance_val).map_err(|e| {
        anyhow!(
            "{label} does not match schema ({}): {e}",
            inst_path.display()
        )
    })?;
    println!("{label} OK (schema validated)");
    Ok(())
}

/// `findings-latest.json` is a bare array; validate against findings-array v1 schema.
fn verify_findings_latest_schema(repo_root: &Path) -> Result<()> {
    validate_json_file_against_schema(
        repo_root,
        FINDINGS_LATEST_REL,
        FINDINGS_ARRAY_SCHEMA_REL,
        "findings-latest.json",
    )
}

fn verify_remediation_delta_schema(repo_root: &Path) -> Result<()> {
    validate_json_file_against_schema(
        repo_root,
        REMEDIATION_DELTA_REL,
        REMEDIATION_DELTA_SCHEMA_REL,
        "remediation delta-after-remediation.json",
    )
}

fn verify_gold_dataset_schema(repo_root: &Path) -> Result<()> {
    validate_json_file_against_schema(
        repo_root,
        GOLD_DATASET_REL,
        GOLD_SCHEMA_REL,
        "TOESTUB gold-dataset.v1.json",
    )
}

/// If `promotion-metrics.json` exists (after `emit-reports`), enforce minimal contract for CI dashboards.
fn verify_promotion_metrics_optional(repo_root: &Path) -> Result<()> {
    let p = PathBuf::from(repo_root)
        .join(REMEDIATION_REPORT_ROOT)
        .join("promotion-metrics.json");
    if !p.is_file() {
        println!("promotion-metrics.json: absent (optional until emit-reports)");
        return Ok(());
    }
    let raw = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    let v: JsonValue = serde_json::from_str(&raw)
        .with_context(|| format!("parse {}", p.display()))?;
    if v.get("version").and_then(|x| x.as_u64()) != Some(1) {
        return Err(anyhow!(
            "{}: expected version 1",
            p.display()
        ));
    }
    if v.get("findings_total_latest").is_none() {
        return Err(anyhow!(
            "{}: missing findings_total_latest",
            p.display()
        ));
    }
    println!("promotion-metrics.json OK (rollup contract)");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct RemediationLanesFile {
    #[allow(dead_code)]
    version: u32,
    lanes: Vec<RemediationLane>,
}

#[derive(Debug, Deserialize)]
struct RemediationLane {
    id: String,
    rule_family: String,
    status: String,
    #[serde(default)]
    #[allow(dead_code)]
    primary_crate: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    focus: Option<String>,
}

fn verify_remediation_lanes(repo_root: &Path) -> Result<()> {
    let p = repo_root.join(REMEDIATION_LANES_REL);
    let raw = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    let doc: RemediationLanesFile =
        serde_yaml::from_str(&raw).context("parse REMEDIATION-LANES.yaml")?;
    if doc.version != 1 {
        return Err(anyhow!(
            "REMEDIATION-LANES.yaml: expected version 1, got {}",
            doc.version
        ));
    }
    let mut ids = std::collections::HashSet::<&str>::new();
    for lane in &doc.lanes {
        if lane.id.is_empty() {
            return Err(anyhow!("remediation lane missing id"));
        }
        if !ids.insert(lane.id.as_str()) {
            return Err(anyhow!("duplicate remediation lane id: {}", lane.id));
        }
        if lane.rule_family.is_empty() {
            return Err(anyhow!("lane {}: rule_family required", lane.id));
        }
        if lane.status.is_empty() {
            return Err(anyhow!("lane {}: status required", lane.id));
        }
    }
    if doc.lanes.is_empty() {
        return Err(anyhow!(
            "REMEDIATION-LANES.yaml: expected at least one lane"
        ));
    }
    println!(
        "REMEDIATION-LANES.yaml OK ({} unique lanes)",
        doc.lanes.len()
    );
    Ok(())
}

fn workspace_crates(repo_root: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let crates_dir = repo_root.join("crates");
    for e in
        fs::read_dir(&crates_dir).with_context(|| format!("read_dir {}", crates_dir.display()))?
    {
        let e = e?;
        if !e.file_type()?.is_dir() {
            continue;
        }
        if e.path().join("Cargo.toml").is_file() {
            out.push(e.file_name().to_string_lossy().into_owned());
        }
    }
    out.sort();
    if out.is_empty() {
        return Err(anyhow!("no crates found under {}", crates_dir.display()));
    }
    Ok(out)
}

fn load_templates(repo_root: &Path) -> Result<TaskTemplatesFile> {
    let p = repo_root.join(TEMPLATES_REL);
    let raw = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    serde_yaml::from_str(&raw).context("parse task-templates.yaml")
}

fn emit_per_crate_backlogs(
    repo_root: &Path,
    crates: &[String],
    templates: &TaskTemplatesFile,
) -> Result<()> {
    let by_crate = PathBuf::from(repo_root).join(REPORT_ROOT).join("by-crate");
    fs::create_dir_all(&by_crate).with_context(|| format!("create {}", by_crate.display()))?;

    for c in crates {
        let mut md = String::new();
        md.push_str(&format!("# Scaling backlog: `{c}`\n\n"));
        md.push_str("Generated by `vox ci scaling-audit emit-reports`. Do not hand-edit IDs; regenerate instead.\n\n");
        for cat in &templates.categories {
            let id = format!(
                "SCALE-{}-{}",
                c.to_uppercase().replace('-', "_"),
                cat.id.to_uppercase()
            );
            md.push_str(&format!("## {id}\n"));
            md.push_str(&format!("- **Pattern**: `{}`\n", cat.pattern));
            md.push_str(&format!("- **Impact**: {}\n", cat.impact));
            md.push_str(&format!("- **Risk**: {}\n", cat.risk));
            md.push_str(&format!(
                "- **Task**: {}\n",
                cat.title.replace("{crate}", c)
            ));
            md.push_str(&format!(
                "- **Verification**: {}\n\n",
                cat.verification.replace("{crate}", c)
            ));
        }
        let dest = by_crate.join(format!("{c}.md"));
        fs::write(&dest, md).with_context(|| format!("write {}", dest.display()))?;
    }
    println!(
        "Wrote {} per-crate backlog files under {}",
        crates.len(),
        by_crate.display()
    );
    Ok(())
}

#[derive(Serialize)]
struct RollupEntry {
    id: String,
    #[serde(rename = "crate")]
    krate: String,
    template: String,
    pattern: String,
    file: String,
}

fn emit_rollup_index(
    repo_root: &Path,
    crates: &[String],
    templates: &TaskTemplatesFile,
) -> Result<()> {
    let rollup_dir = PathBuf::from(repo_root).join(REPORT_ROOT).join("rollup");
    fs::create_dir_all(&rollup_dir).context("create rollup dir")?;
    let mut entries: Vec<RollupEntry> = Vec::new();
    for c in crates {
        for cat in &templates.categories {
            let id = format!(
                "SCALE-{}-{}",
                c.to_uppercase().replace('-', "_"),
                cat.id.to_uppercase()
            );
            entries.push(RollupEntry {
                id,
                krate: c.clone(),
                template: cat.id.clone(),
                pattern: cat.pattern.clone(),
                file: format!("by-crate/{c}.md"),
            });
        }
    }
    let header = concat!(
        "# Scaling audit rollup — stable IDs (SSOT)\n",
        "# Generated by `vox ci scaling-audit emit-reports`.\n\n",
    );
    let body = serde_yaml::to_string(&entries).context("serialize rollup INDEX")?;
    let dest = rollup_dir.join("INDEX.yaml");
    fs::write(&dest, format!("{header}{body}"))
        .with_context(|| format!("write {}", dest.display()))?;
    println!("Wrote {}", dest.display());
    Ok(())
}

/// Parse `toestub --format json` output: full envelope (or legacy array) plus findings value.
fn parse_toestub_stdout(stdout: &[u8]) -> Result<(JsonValue, JsonValue)> {
    let v: JsonValue = serde_json::from_slice(stdout).context("parse toestub stdout as JSON")?;
    let findings = if let Some(f) = v.get("findings") {
        f.clone()
    } else if v.is_array() {
        v.clone()
    } else {
        anyhow::bail!("toestub json: expected top-level array or object with `findings`");
    };
    Ok((v, findings))
}

fn findings_array_to_pretty(findings: &JsonValue) -> Result<String> {
    serde_json::to_string_pretty(findings).context("serialize findings array")
}

/// Upper bound for `rust_parse_failures` in the TOESTUB JSON envelope (`toestub --format json`).
/// Unset or non-numeric ⇒ no limit. Documented in `docs/src/reference/env-vars.md`.
fn toestub_rust_parse_failure_limit_from_env() -> u64 {
    std::env::var("VOX_TOESTUB_MAX_RUST_PARSE_FAILURES")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(u64::MAX)
}

fn enforce_toestub_rust_parse_budget(envelope: &JsonValue, max_failures: u64) -> Result<()> {
    let Some(n) = envelope
        .get("rust_parse_failures")
        .and_then(|x| x.as_u64())
    else {
        return Ok(());
    };
    if n > max_failures {
        return Err(anyhow!(
            "toestub rust_parse_failures={n} exceeds allowed maximum {max_failures} (set via VOX_TOESTUB_MAX_RUST_PARSE_FAILURES)"
        ));
    }
    Ok(())
}

fn emit_remediation_board(repo_root: &Path) -> Result<()> {
    let findings_path = repo_root.join(FINDINGS_LATEST_REL);
    let findings_raw = read_utf8_path_capped(&findings_path)
        .with_context(|| format!("read {}", findings_path.display()))?;
    let findings_val: JsonValue =
        serde_json::from_str(&findings_raw).context("parse findings-latest.json")?;
    let arr = findings_val
        .as_array()
        .ok_or_else(|| anyhow!("findings-latest.json must be a JSON array"))?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for f in arr {
        if let Some(p) = f.get("file").and_then(|x| x.as_str()) {
            *counts.entry(p.to_string()).or_insert(0) += 1;
        }
    }
    let mut pairs: Vec<_> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let lanes_path = repo_root.join(REMEDIATION_LANES_REL);
    let lanes_raw = read_utf8_path_capped(&lanes_path)
        .unwrap_or_else(|_| "# (missing REMEDIATION-LANES.yaml)\n".to_string());

    let mut md = String::new();
    md.push_str("# TOESTUB remediation board (auto-generated)\n\n");
    md.push_str("Generated by `vox ci scaling-audit emit-reports`. Pair with `REMEDIATION-LANES.yaml` and `promotion-metrics.json`.\n\n");
    md.push_str("## Top files by finding count (20)\n\n");
    md.push_str("| Count | File |\n|---:|---|\n");
    for (path, c) in pairs.iter().take(20) {
        md.push_str(&format!("| {c} | `{path}` |\n"));
    }
    md.push_str("\n## Lane definitions (verbatim)\n\n```yaml\n");
    md.push_str(&lanes_raw);
    md.push_str("\n```\n");

    let dest = PathBuf::from(repo_root)
        .join(REMEDIATION_REPORT_ROOT)
        .join("board.md");
    fs::create_dir_all(dest.parent().unwrap())
        .with_context(|| format!("create {}", dest.parent().unwrap().display()))?;
    fs::write(&dest, md.as_bytes()).with_context(|| format!("write {}", dest.display()))?;
    println!("Wrote {}", dest.display());
    Ok(())
}

fn emit_promotion_metrics(repo_root: &Path) -> Result<()> {
    let findings_path = repo_root.join(FINDINGS_LATEST_REL);
    let findings_raw = read_utf8_path_capped(&findings_path)
        .with_context(|| format!("read {}", findings_path.display()))?;
    let findings_val: JsonValue =
        serde_json::from_str(&findings_raw).context("parse findings-latest.json")?;
    let arr = findings_val.as_array().cloned().unwrap_or_default();
    let n = arr.len();
    let mut by_rule: HashMap<String, usize> = HashMap::new();
    let mut by_family: HashMap<String, usize> = HashMap::new();
    for item in &arr {
        let Some(rid) = item.get("rule_id").and_then(|x| x.as_str()) else {
            continue;
        };
        *by_rule.entry(rid.to_string()).or_insert(0) += 1;
        let fam = rid
            .split_once('/')
            .map(|(a, _)| a.to_string())
            .unwrap_or_else(|| rid.to_string());
        *by_family.entry(fam).or_insert(0) += 1;
    }
    let mut rule_rows: Vec<_> = by_rule.into_iter().collect();
    rule_rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let rule_top: HashMap<String, usize> = rule_rows.into_iter().take(40).collect();
    let mut fam_rows: Vec<_> = by_family.into_iter().collect();
    fam_rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let delta_path = repo_root.join(REMEDIATION_DELTA_REL);
    let delta_val: JsonValue = if delta_path.is_file() {
        let raw = read_utf8_path_capped(&delta_path)
            .with_context(|| format!("read {}", delta_path.display()))?;
        serde_json::from_str(&raw).unwrap_or(JsonValue::Null)
    } else {
        JsonValue::Null
    };

    let out = serde_json::json!({
        "version": 1,
        "generated_at": Utc::now().to_rfc3339(),
        "findings_total_latest": n,
        "findings_top_rules": rule_top,
        "findings_by_family": serde_json::to_value(fam_rows).unwrap_or(JsonValue::Null),
        "remediation_delta_snapshot": delta_val,
        "canary_rollout": {
            "cli_flags": "toestub --canary-crates vox-cli,vox-mcp --feature-flags unwired-graph",
            "strictness_promotion_gate": "`vox ci scaling-audit verify` + `cargo test -p vox-toestub --test gold_dataset`",
        },
    });

    let dest = PathBuf::from(repo_root)
        .join(REMEDIATION_REPORT_ROOT)
        .join("promotion-metrics.json");
    fs::create_dir_all(dest.parent().unwrap())
        .with_context(|| format!("create {}", dest.parent().unwrap().display()))?;
    fs::write(
        &dest,
        serde_json::to_string_pretty(&out).context("serialize promotion-metrics")?,
    )
    .with_context(|| format!("write {}", dest.display()))?;
    println!("Wrote {}", dest.display());
    Ok(())
}

fn run_toestub_json_snapshot(repo_root: &Path) -> Result<()> {
    let cargo = cargo_bin();
    let out_path = PathBuf::from(repo_root)
        .join(REPORT_ROOT)
        .join("findings-latest.json");
    fs::create_dir_all(out_path.parent().unwrap()).ok();
    let output = std::process::Command::new(&cargo)
        .current_dir(repo_root)
        .args(["run", "-q", "-p", "vox-toestub", "--bin", "toestub", "--"])
        .arg("--mode")
        .arg("audit")
        .arg("--format")
        .arg("json")
        .arg("--min-severity")
        .arg("info")
        .arg("crates")
        .output()
        .context("spawn toestub json")?;
    if !output.status.success() {
        return Err(anyhow!(
            "toestub audit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    // `toestub --format json` emits a versioned envelope (`findings`, telemetry); SSOT artifact
    // stays a bare findings array for downstream baselines.
    let (envelope, findings) =
        parse_toestub_stdout(&output.stdout).context("parse toestub JSON (envelope)")?;
    enforce_toestub_rust_parse_budget(&envelope, toestub_rust_parse_failure_limit_from_env())?;
    let normalized = findings_array_to_pretty(&findings)
        .context("normalize toestub JSON (findings array)")?;
    fs::write(&out_path, normalized.as_bytes())
        .with_context(|| format!("write {}", out_path.display()))?;
    println!("Wrote {}", out_path.display());
    Ok(())
}

/// Run scaling-audit subcommand.
pub fn run(repo_root: &Path, cmd: ScalingAuditCmd) -> Result<()> {
    match cmd {
        ScalingAuditCmd::Verify => {
            verify_policy_schema(repo_root)?;
            verify_embedded_policy_roundtrip(repo_root)?;
            verify_findings_latest_schema(repo_root)?;
            verify_remediation_delta_schema(repo_root)?;
            verify_remediation_lanes(repo_root)?;
            verify_gold_dataset_schema(repo_root)?;
            verify_promotion_metrics_optional(repo_root)?;
            println!("scaling-audit verify OK");
            Ok(())
        }
        ScalingAuditCmd::EmitReports => {
            verify_policy_schema(repo_root)?;
            verify_embedded_policy_roundtrip(repo_root)?;
            let crates = workspace_crates(repo_root)?;
            let templates = load_templates(repo_root)?;
            let n_tasks = crates.len() * templates.categories.len();
            if n_tasks < 300 {
                return Err(anyhow!(
                    "expected at least 300 tasks (crates * templates); got {n_tasks}. Add crates or template rows."
                ));
            }
            emit_per_crate_backlogs(repo_root, &crates, &templates)?;
            emit_rollup_index(repo_root, &crates, &templates)?;
            run_toestub_json_snapshot(repo_root)?;
            emit_remediation_board(repo_root)?;
            emit_promotion_metrics(repo_root)?;
            println!(
                "scaling-audit emit-reports OK ({} crates × {} categories = {} tasks)",
                crates.len(),
                templates.categories.len(),
                n_tasks
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod toestub_parse_budget_tests {
    use super::enforce_toestub_rust_parse_budget;
    use serde_json::json;

    #[test]
    fn budget_skips_when_field_absent() {
        enforce_toestub_rust_parse_budget(&json!({ "findings": [] }), 0).unwrap();
    }

    #[test]
    fn budget_allows_at_cap() {
        enforce_toestub_rust_parse_budget(&json!({ "rust_parse_failures": 2 }), 2).unwrap();
    }

    #[test]
    fn budget_rejects_over_cap() {
        let e = enforce_toestub_rust_parse_budget(&json!({ "rust_parse_failures": 5 }), 3)
            .unwrap_err();
        assert!(
            e.to_string().contains("rust_parse_failures=5"),
            "{e}"
        );
    }
}
