//! Deterministic offline v7.6 6-axis before/after audit harness (`AUDT-01`).
//!
//! Invoked via `vox scientia v76-audit --stage before` to lock the pre-remediation
//! benchmark and live bug reproduction baseline at
//! `contracts/reports/scientia-v76-before-baseline.v1.json`.

pub mod after_closure;
pub mod bug_probes;
pub mod cassettes;
pub mod lessons;
pub mod scoring;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vox_cli_core::scientia::ScientiaAuditStageCli;

use self::after_closure::{
    DEFAULT_AFTER_JSONL_REL, PairedComparisonReport, build_paired_comparison,
    evaluate_after_six_axes,
};
use self::bug_probes::{LiveBugReproductions, collect_live_bug_reproductions};
use self::cassettes::{
    CassetteSummary, DETERMINISTIC_SEED, SixAxesReport, evaluate_six_axes, load_cassettes,
};
use self::lessons::{
    LessonPipelineResult, LessonsSummary, default_v76_closure_lessons,
    run_quarantined_lessons_pipeline,
};

pub const AUDIT_SCHEMA_REL: &str = "contracts/reports/scientia-v76-audit.v1.schema.json";
pub const BEFORE_BASELINE_REL: &str = "contracts/reports/scientia-v76-before-baseline.v1.json";
pub const AFTER_REPORT_REL: &str = "contracts/reports/scientia-v76-after-closure.v1.json";

/// Stage discriminator for `ScientiaV76AuditReport`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditStage {
    Before,
    After,
}

impl From<ScientiaAuditStageCli> for AuditStage {
    fn from(value: ScientiaAuditStageCli) -> Self {
        match value {
            ScientiaAuditStageCli::Before => Self::Before,
            ScientiaAuditStageCli::After => Self::After,
        }
    }
}

/// Canonical v7.6 6-axis benchmark & live bug reproduction report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScientiaV76AuditReport {
    #[serde(rename = "x-vox-version")]
    pub x_vox_version: u32,
    pub schema_version: u32,
    pub stage: AuditStage,
    pub deterministic_seed: u64,
    pub six_axes: SixAxesReport,
    pub live_bug_reproductions: LiveBugReproductions,
    pub cassette_summary: CassetteSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paired_comparison: Option<PairedComparisonReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lessons_summary: Option<LessonsSummary>,
}

/// Build the deterministic v7.6 audit report entirely offline.
pub async fn build_v76_audit_report(
    repo_root: &Path,
    stage: AuditStage,
) -> anyhow::Result<ScientiaV76AuditReport> {
    let staging = repo_root.join("target/scientia-lessons-staging");
    let (report, _) = build_v76_audit_report_with_options(repo_root, stage, None, &staging).await?;
    Ok(report)
}

/// Build a v7.6 audit report with explicit `compare` baseline and lesson `staging_root`.
pub async fn build_v76_audit_report_with_options(
    repo_root: &Path,
    stage: AuditStage,
    compare_path: Option<&Path>,
    staging_root: &Path,
) -> anyhow::Result<(ScientiaV76AuditReport, Option<LessonPipelineResult>)> {
    if stage == AuditStage::Before {
        let locked_path = repo_root.join(BEFORE_BASELINE_REL);
        if locked_path.is_file() {
            let raw = std::fs::read_to_string(&locked_path)?;
            let parsed: ScientiaV76AuditReport = serde_json::from_str(&raw)?;
            return Ok((parsed, None));
        }
    }

    let cassettes = load_cassettes(repo_root)?;
    let (live_bug_reproductions, verifier_metrics) = collect_live_bug_reproductions(
        repo_root,
        &cassettes.claim_pairs,
        &cassettes.novelty_bundles,
    )
    .await?;

    if stage == AuditStage::Before {
        let (six_axes, cassette_summary) =
            evaluate_six_axes(&cassettes, &live_bug_reproductions, &verifier_metrics);
        return Ok((
            ScientiaV76AuditReport {
                x_vox_version: 1,
                schema_version: 1,
                stage,
                deterministic_seed: DETERMINISTIC_SEED,
                six_axes,
                live_bug_reproductions,
                cassette_summary,
                paired_comparison: None,
                lessons_summary: None,
            },
            None,
        ));
    }

    let details =
        evaluate_after_six_axes(&cassettes, &live_bug_reproductions, &verifier_metrics).await?;
    let (before_file, before_rel) = match compare_path {
        Some(p) => (p.to_path_buf(), p.display().to_string()),
        None => (
            repo_root.join(BEFORE_BASELINE_REL),
            BEFORE_BASELINE_REL.to_string(),
        ),
    };
    let paired_comparison = if before_file.is_file() {
        let raw = std::fs::read_to_string(&before_file)?;
        let before_report: ScientiaV76AuditReport = serde_json::from_str(&raw)?;
        Some(build_paired_comparison(
            &before_report,
            &details,
            &live_bug_reproductions,
            &before_rel,
        ))
    } else {
        None
    };

    let lessons_result =
        run_quarantined_lessons_pipeline(staging_root, &default_v76_closure_lessons(), false)
            .await?;
    let lessons_summary = Some(lessons_result.summary.clone());

    let report = ScientiaV76AuditReport {
        x_vox_version: 1,
        schema_version: 1,
        stage,
        deterministic_seed: DETERMINISTIC_SEED,
        six_axes: details.six_axes,
        live_bug_reproductions,
        cassette_summary: details.cassette_summary,
        paired_comparison,
        lessons_summary,
    };
    Ok((report, Some(lessons_result)))
}

/// Validate `report` against `contracts/reports/scientia-v76-audit.v1.schema.json`
/// and render deterministic pretty-printed JSON with a trailing newline.
pub fn render_validated_audit_json(
    repo_root: &Path,
    report: &ScientiaV76AuditReport,
) -> anyhow::Result<String> {
    let schema_path = repo_root.join(AUDIT_SCHEMA_REL);
    let schema_src = std::fs::read_to_string(&schema_path)?;
    let validator = vox_jsonschema_util::compile_validator_from_utf8(&schema_src, &schema_path)?;
    let instance = serde_json::to_value(report)?;
    vox_jsonschema_util::validate(&instance, &validator, schema_path.display())?;
    let mut rendered = serde_json::to_string_pretty(&instance)?;
    rendered.push('\n');
    Ok(rendered)
}

fn resolve_cli_path(p: PathBuf) -> anyhow::Result<PathBuf> {
    if p.is_absolute() {
        Ok(p)
    } else {
        Ok(std::env::current_dir()?.join(p))
    }
}

/// Emit multiplexed JSONL findings (`schema_version: 1`, `discriminator`) to `path`.
pub fn write_findings_jsonl(
    path: &Path,
    report: &ScientiaV76AuditReport,
    lessons: &LessonPipelineResult,
) -> anyhow::Result<()> {
    use serde_json::json as j;
    let mut lines = Vec::new();
    if let Some(ref paired) = report.paired_comparison {
        for a in &paired.axes {
            lines.push(serde_json::to_string(&j!({
                "schema_version": 1, "discriminator": "scientia_v76_axis_delta",
                "axis": a.axis, "metric": a.metric, "before_value": a.before_value,
                "after_value": a.after_value, "delta": a.delta, "ci_low": a.ci_low,
                "ci_high": a.ci_high, "ni_verdict": a.ni_verdict, "superior": a.superior,
            }))?);
        }
    }
    let b = &report.live_bug_reproductions;
    let bugs_obj = serde_json::to_value(b)?;
    for (id, ok) in [
        (
            "composer_research_scope_overwritten",
            !b.composer_research_scope_overwritten,
        ),
        (
            "research_turn_waves_hardcoded_to_one",
            !b.research_turn_waves_hardcoded_to_one,
        ),
        (
            "none_to_novel_false_positive_rate",
            b.none_to_novel_false_positive_rate == 0.0,
        ),
        (
            "artifact_json_wiped_on_doc_publish",
            !b.artifact_json_wiped_on_doc_publish,
        ),
        (
            "mock_verifier_fallback_rate",
            b.mock_verifier_fallback_rate == 0.0,
        ),
        (
            "contradiction_false_support_rate",
            b.contradiction_false_support_rate == 0.0,
        ),
        (
            "kv_cache_prefix_stability_ratio",
            b.kv_cache_prefix_stability_ratio >= 0.90,
        ),
        (
            "session_detail_ipc_payload_bytes",
            b.session_detail_ipc_payload_bytes <= 32_768,
        ),
        ("gui_cli_db_spawn_count", b.gui_cli_db_spawn_count == 0),
    ] {
        lines.push(serde_json::to_string(&j!({
            "schema_version": 1, "discriminator": "scientia_v76_bug_closure",
            "bug_id": id, "value": bugs_obj[id], "resolved": ok,
        }))?);
    }
    for f in &lessons.findings {
        lines.push(serde_json::to_string(&j!({
            "schema_version": 1, "discriminator": "scientia_v76_lesson",
            "finding_id": f.finding_id, "title": f.title, "trust_tier": f.trust_tier,
            "quarantined": f.quarantined, "injectable_as_curated": f.injectable_as_curated,
            "status": f.status, "source_count": f.sources.len(),
        }))?);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(path, body)?;
    Ok(())
}

/// Execute `vox scientia v76-audit` and write the validated JSON artifact to disk.
pub async fn run_v76_audit(
    stage: AuditStage,
    out: Option<PathBuf>,
    compare: Option<PathBuf>,
    jsonl_out: Option<PathBuf>,
) -> anyhow::Result<PathBuf> {
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let compare_resolved = match compare.as_ref() {
        Some(p) => Some(resolve_cli_path(p.clone())?),
        None => None,
    };
    let staging_root = repo_root.join("target/scientia-lessons-staging");
    let (report, lessons_opt) = build_v76_audit_report_with_options(
        &repo_root,
        stage,
        compare_resolved.as_deref(),
        &staging_root,
    )
    .await?;
    let rendered = render_validated_audit_json(&repo_root, &report)?;

    let target = match out {
        Some(p) => resolve_cli_path(p)?,
        None => repo_root.join(match stage {
            AuditStage::Before => BEFORE_BASELINE_REL,
            AuditStage::After => AFTER_REPORT_REL,
        }),
    };
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, &rendered)?;

    if stage == AuditStage::After && (jsonl_out.is_some() || compare.is_some()) {
        if let Some(ref lessons) = lessons_opt {
            let jsonl_target = match jsonl_out {
                Some(p) => resolve_cli_path(p)?,
                None => repo_root.join(DEFAULT_AFTER_JSONL_REL),
            };
            write_findings_jsonl(&jsonl_target, &report, lessons)?;
        }
    }

    println!(
        "scientia v76-audit ({stage:?}) written to {}",
        target.display()
    );
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("canonical repo root")
    }

    #[tokio::test]
    async fn audit_report_is_byte_for_byte_deterministic_and_schema_valid() {
        let root = test_repo_root();
        let r1 = build_v76_audit_report(&root, AuditStage::Before)
            .await
            .expect("first audit run");
        let r2 = build_v76_audit_report(&root, AuditStage::Before)
            .await
            .expect("second audit run");
        let bytes_run_1 = render_validated_audit_json(&root, &r1).expect("validate run 1");
        let bytes_run_2 = render_validated_audit_json(&root, &r2).expect("validate run 2");
        assert_eq!(bytes_run_1, bytes_run_2);
        assert!(bytes_run_1.contains("\"x-vox-version\": 1"));
        assert!(bytes_run_1.contains("\"stage\": \"before\""));

        let on_disk = root.join(BEFORE_BASELINE_REL);
        if on_disk.is_file() {
            let disk_str = std::fs::read_to_string(&on_disk).expect("read baseline");
            assert_eq!(
                disk_str.replace("\r\n", "\n"),
                bytes_run_1,
                "on-disk baseline must match deterministic build_v76_audit_report output"
            );
        }
    }

    #[tokio::test]
    async fn before_stage_captures_authentic_pre_v76_live_bug_reproductions() {
        let root = test_repo_root();
        let report = build_v76_audit_report(&root, AuditStage::Before)
            .await
            .expect("build before report");
        let bugs = &report.live_bug_reproductions;

        assert!(
            bugs.composer_research_scope_overwritten,
            "App.tsx submitFromComposer still overwrites research_scope in before baseline"
        );
        assert!(
            bugs.research_turn_waves_hardcoded_to_one,
            "research_turn.rs still hardcodes waves: 1 and General in before baseline"
        );
        assert!(
            bugs.none_to_novel_false_positive_rate > 0.0,
            "novelty.rs None => Novel must fire on lexical-only prior art"
        );
        assert!(
            bugs.artifact_json_wiped_on_doc_publish,
            "write_research_doc still overwrites artifact_json with {{}} in before baseline"
        );
        assert!(
            (bugs.mock_verifier_fallback_rate - 1.0).abs() < 1e-9,
            "MiniCheckVerifier::from_env must fall back to Mock when endpoint is unset"
        );
        assert!(
            bugs.contradiction_false_support_rate > 0.0,
            "Mock verifier must exhibit false support on antonym/numeric contradictions"
        );
        assert!(
            bugs.kv_cache_prefix_stability_ratio < 1.0,
            "volatile system prompt sections must yield prefix stability < 1.0"
        );
        assert!(
            bugs.session_detail_ipc_payload_bytes > 32_768,
            "unpaginated 50-source/100-claim session detail payload must exceed 32 KiB, got {}",
            bugs.session_detail_ipc_payload_bytes
        );
        assert_eq!(
            bugs.gui_cli_db_spawn_count, 3,
            "expected 3 execute_command spawns across PublicationsView.tsx and ClaimsView.tsx"
        );
    }

    #[tokio::test]
    async fn after_stage_closes_all_bugs_upgrades_six_axes_and_emits_paired_jsonl() {
        let root = test_repo_root();
        let tmp = tempfile::tempdir().expect("tempdir");
        let (report, lessons_opt) = build_v76_audit_report_with_options(
            &root,
            AuditStage::After,
            Some(&root.join(BEFORE_BASELINE_REL)),
            tmp.path(),
        )
        .await
        .expect("build after report");

        let rendered = render_validated_audit_json(&root, &report).expect("validate after schema");
        assert!(rendered.contains("\"stage\": \"after\""));

        let bugs = &report.live_bug_reproductions;
        assert!(!bugs.composer_research_scope_overwritten);
        assert!(!bugs.research_turn_waves_hardcoded_to_one);
        assert_eq!(bugs.none_to_novel_false_positive_rate, 0.0);
        assert!(!bugs.artifact_json_wiped_on_doc_publish);
        assert_eq!(bugs.mock_verifier_fallback_rate, 0.0);
        assert_eq!(bugs.contradiction_false_support_rate, 0.0);
        assert!(bugs.kv_cache_prefix_stability_ratio >= 0.90);
        assert!(bugs.session_detail_ipc_payload_bytes <= 32_768);
        assert_eq!(bugs.gui_cli_db_spawn_count, 0);

        let paired = report
            .paired_comparison
            .as_ref()
            .expect("paired comparison attached");
        assert!(paired.blinded_judge_protocol);
        assert_eq!(paired.axes.len(), 6);
        assert!(paired.axes.iter().all(|a| a.delta >= 0.0));

        let summary = report
            .lessons_summary
            .as_ref()
            .expect("lessons summary attached");
        assert_eq!(summary.unreviewed_ssot_mutations, 0);
        assert!(summary.quarantined_findings_count >= 3);

        let lessons = lessons_opt.expect("lessons result");
        let jsonl_path = tmp.path().join("findings.v1.jsonl");
        write_findings_jsonl(&jsonl_path, &report, &lessons).expect("write jsonl");
        let jsonl = std::fs::read_to_string(&jsonl_path).expect("read jsonl");
        assert!(jsonl.contains("\"schema_version\":1"));
        assert!(jsonl.contains("\"discriminator\":\"scientia_v76_axis_delta\""));
        assert!(jsonl.contains("\"discriminator\":\"scientia_v76_bug_closure\""));
        assert!(jsonl.contains("\"discriminator\":\"scientia_v76_lesson\""));
    }
}
