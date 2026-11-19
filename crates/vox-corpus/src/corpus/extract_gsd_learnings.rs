//! Cross-milestone GSD phase learnings extraction (`*-LEARNINGS.md`, `.planning/LEARNINGS.md`)
//! and estimate-vs-actual token calibration (`.planning/estimation-calibration.json`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use vox_bounded_fs::read_utf8_path_capped;

use crate::corpus::extract_gsd::{
    PhaseDirEntry, discover_phase_dirs, extract_xml_tag, extract_yaml_coverage,
    extract_yaml_tokens, file_name_str, phase_sort_key, split_frontmatter, yaml_str_list,
};

const LEARNINGS_DATE: &str = "2026-11-17";
const DEC_HEADS: &[&str] = &[
    "decisions made",
    "key decisions",
    "delivered changes",
    "delivered scope",
    "what was built",
    "accomplishments",
    "deliverables",
];
const LES_HEADS: &[&str] = &[
    "issues encountered",
    "deviations from plan",
    "lessons",
    "what was inefficient",
    "notes for the orchestrator",
];
const PAT_HEADS: &[&str] = &["patterns established", "patterns"];

/// One extracted learning item for `LEARNINGS.md`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GsdLearningEntry {
    pub title: String,
    pub summary: String,
    pub detail_label: String,
    pub detail: String,
    pub source: String,
}

/// Structured learnings extracted from one completed phase directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GsdPhaseLearnings {
    pub phase_number: String,
    pub padded_phase: String,
    pub phase_name: String,
    pub decisions: Vec<GsdLearningEntry>,
    pub lessons: Vec<GsdLearningEntry>,
    pub patterns: Vec<GsdLearningEntry>,
    pub surprises: Vec<GsdLearningEntry>,
    pub missing_artifacts: Vec<String>,
}

/// Paired token estimate vs. actual sample for `.planning/estimation-calibration.json`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationSample {
    #[serde(rename = "estimateTokens")]
    pub estimate_tokens: u64,
    #[serde(rename = "actualTokens")]
    pub actual_tokens: u64,
}

/// Summary of workspace-wide GSD learning extraction and calibration rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GsdLearningsReport {
    pub phases_processed: usize,
    pub unique_phases: usize,
    pub decisions: usize,
    pub lessons: usize,
    pub patterns: usize,
    pub surprises: usize,
    pub calibration_samples: usize,
    pub calibration_factor: f64,
    pub written_files: Vec<PathBuf>,
}

fn md_stem(fname: &str, suffix: &str) -> String {
    fname
        .strip_suffix(suffix)
        .unwrap_or_else(|| fname.trim_end_matches(".md"))
        .to_string()
}

fn clean_title(text: &str, max_chars: usize) -> String {
    let clean = text.replace("**", "").replace('`', "");
    let clean = clean.trim_start_matches(['-', '*', ' ']).trim();
    let line = clean.split('\n').next().unwrap_or(clean).trim();
    let cut = line
        .find(". ")
        .into_iter()
        .chain(line.find(": "))
        .min()
        .unwrap_or(line.len());
    let head = line[..cut].trim();
    let src = if head.len() >= 12 { head } else { clean };
    let mut out: String = src.chars().take(max_chars).collect();
    if src.chars().nth(max_chars).is_some() {
        out.push_str("...");
    }
    out.trim().to_string()
}

fn extract_section_items(body: &str, headings: &[&str], limit: usize) -> Vec<String> {
    let (mut items, mut in_sec, mut buf) = (Vec::new(), false, String::new());
    let flush = |items: &mut Vec<String>, buf: &mut String| {
        let t = buf.trim();
        if !t.is_empty() && !t.ends_with(':') && !t.ends_with(":**") && items.len() < limit {
            items.push(t.to_string());
        }
        buf.clear();
    };
    for raw in body.lines() {
        let t = raw.trim();
        if let Some(h) = t.strip_prefix("## ").or_else(|| t.strip_prefix("### ")) {
            flush(&mut items, &mut buf);
            let hl = h.to_ascii_lowercase();
            in_sec = headings.iter().any(|k| hl.contains(k));
            continue;
        }
        if !in_sec {
            continue;
        }
        let low = t.to_ascii_lowercase();
        if low.is_empty()
            || matches!(low.as_str(), "none" | "none." | "n/a" | "---")
            || ["none -", "none —", "none (", "```", "|"]
                .iter()
                .any(|p| low.starts_with(p))
        {
            flush(&mut items, &mut buf);
            continue;
        }
        let is_bullet = t.starts_with("- ")
            || t.starts_with("* ")
            || (t.len() > 3
                && t.chars().next().is_some_and(|c| c.is_ascii_digit())
                && t.contains(". "));
        if is_bullet {
            flush(&mut items, &mut buf);
            buf = t
                .trim_start_matches(|c: char| c.is_ascii_digit() || matches!(c, '.' | '-' | '*'))
                .trim()
                .to_string();
        } else {
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(t);
        }
    }
    flush(&mut items, &mut buf);
    items
}

struct CatAcc {
    detail_label: &'static str,
    items: Vec<GsdLearningEntry>,
    seen: BTreeSet<String>,
}

impl CatAcc {
    fn new(detail_label: &'static str) -> Self {
        Self {
            detail_label,
            items: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    fn push(&mut self, summary: String, detail: &str, source: &str) {
        let title = clean_title(&summary, 72);
        if !title.is_empty()
            && !summary.trim().is_empty()
            && self.seen.insert(title.to_ascii_lowercase())
        {
            self.items.push(GsdLearningEntry {
                title,
                summary,
                detail_label: self.detail_label.to_string(),
                detail: detail.to_string(),
                source: source.to_string(),
            });
        }
    }

    fn extend(&mut self, items: impl IntoIterator<Item = String>, detail: &str, src: &str) {
        for s in items {
            self.push(s, detail, src);
        }
    }
}

fn extract_phase_learnings(
    entry: &PhaseDirEntry,
    cal_map: &mut BTreeMap<(String, String), CalibrationSample>,
) -> GsdPhaseLearnings {
    let mut decisions = CatAcc::new("Rationale");
    let mut lessons = CatAcc::new("Context");
    let mut patterns = CatAcc::new("When to use");
    let mut surprises = CatAcc::new("Impact");
    let mut plan_tokens: BTreeMap<String, (String, u64)> = BTreeMap::new();
    let mut plan_objs: Vec<(String, String)> = Vec::new();

    for p in &entry.plans {
        let Ok(raw) = read_utf8_path_capped(p) else {
            continue;
        };
        let fname = file_name_str(p);
        if let Some(est) = extract_yaml_tokens(&raw, "estimate", &["raw_tokens", "tokens"]) {
            plan_tokens.insert(md_stem(&fname, "-PLAN.md"), (fname.clone(), est));
        }
        let (_, body) = split_frontmatter(&raw);
        if let Some(obj) = extract_xml_tag(body, "objective").or_else(|| {
            extract_section_items(body, &["objective", "goal"], 1)
                .into_iter()
                .next()
        }) {
            plan_objs.push((fname, obj));
        }
    }

    for s in &entry.summaries {
        let Ok(raw) = read_utf8_path_capped(s) else {
            continue;
        };
        let fname = file_name_str(s);
        let stem = md_stem(&fname, "-SUMMARY.md");
        if let Some(act) = extract_yaml_tokens(&raw, "actuals", &["tokens"]) {
            let matched = plan_tokens.get(&stem).cloned().or_else(|| {
                (entry.plans.len() == 1)
                    .then(|| plan_tokens.values().next().cloned())
                    .flatten()
            });
            if let Some((pf, est)) = matched
                && est > 0
                && act > 0
            {
                let sample = CalibrationSample {
                    estimate_tokens: est,
                    actual_tokens: act,
                };
                cal_map.insert((entry.dir_name.clone(), stem.clone()), sample);
                let ratio = act as f64 / est as f64;
                if !(0.5..=1.5).contains(&ratio) {
                    surprises.push(
                        format!("Estimate variance on {stem}: planned {est} tokens vs {act} actual ({ratio:.2}x)."),
                        "Captured in estimation calibration to refine future token budgets.",
                        &format!("{pf}, {fname}"),
                    );
                }
            }
        }
        let (yaml, body) = split_frontmatter(&raw);
        decisions.extend(
            yaml_str_list(yaml.as_ref(), &["key-decisions"]),
            "Explicit decision recorded in summary frontmatter.",
            &fname,
        );
        decisions.extend(
            extract_section_items(body, DEC_HEADS, 5),
            "Documented implementation outcome from phase summary.",
            &fname,
        );
        if decisions.items.is_empty() {
            let meta = yaml_str_list(yaml.as_ref(), &["provides"])
                .into_iter()
                .chain(extract_yaml_coverage(yaml.as_ref(), "description"));
            decisions.extend(meta, "Capability delivered per summary metadata.", &fname);
        }
        let pats = yaml_str_list(yaml.as_ref(), &["patterns-established"])
            .into_iter()
            .chain(yaml_str_list(yaml.as_ref(), &["tech-stack", "patterns"]))
            .chain(extract_section_items(body, PAT_HEADS, 4));
        patterns.extend(
            pats,
            "Apply when modifying or extending the affected subsystem.",
            &fname,
        );
        for les in extract_section_items(body, LES_HEADS, 4)
            .into_iter()
            .chain(extract_yaml_coverage(yaml.as_ref(), "rationale"))
        {
            let low = les.to_ascii_lowercase();
            if low.contains("rule ") || low.contains("deviation") {
                surprises.push(
                    les.clone(),
                    "Required in-flight adjustment beyond the initial plan.",
                    &fname,
                );
            }
            lessons.push(
                les,
                "Recorded during phase execution and deviation tracking.",
                &fname,
            );
        }
    }

    if decisions.items.is_empty() {
        for (pf, obj) in plan_objs.into_iter().take(2) {
            decisions.push(obj, "Primary phase objective established in plan.", &pf);
        }
    }
    let mut scan = |paths: &[PathBuf], heads: &[&str], ctx: &str, lim: usize| {
        for p in paths {
            if let Ok(raw) = read_utf8_path_capped(p) {
                let fname = file_name_str(p);
                lessons.extend(
                    extract_section_items(split_frontmatter(&raw).1, heads, lim),
                    ctx,
                    &fname,
                );
            }
        }
    };
    scan(
        &entry.reviews,
        &["findings", "issues", "remediation", "outcome"],
        "Surfaced in structured phase code review.",
        3,
    );
    scan(
        &entry.verifications,
        &["root-cause", "gaps", "verification", "summary"],
        "Verified in phase acceptance checks.",
        2,
    );
    scan(
        &entry.uats,
        &["gaps", "summary", "issues", "result"],
        "Observed during user acceptance testing.",
        2,
    );
    let missing = [
        (entry.verifications.is_empty(), "VERIFICATION.md"),
        (entry.uats.is_empty(), "UAT.md"),
        (!entry.has_state, "STATE.md"),
    ]
    .into_iter()
    .filter_map(|(flag, name)| flag.then(|| name.to_string()))
    .collect();
    GsdPhaseLearnings {
        phase_number: entry.phase_number.clone(),
        padded_phase: entry.padded_phase.clone(),
        phase_name: entry.phase_name.clone(),
        decisions: decisions.items,
        lessons: lessons.items,
        patterns: patterns.items,
        surprises: surprises.items,
        missing_artifacts: missing,
    }
}

fn append_sections_md<'a, I>(out: &mut String, heading: &str, items: I)
where
    I: IntoIterator<Item = (Option<&'a str>, &'a GsdLearningEntry)>,
{
    let collected: Vec<_> = items.into_iter().collect();
    out.push_str(&format!("## {heading}\n\n"));
    for (i, (pnum, item)) in collected.iter().enumerate() {
        let title = match pnum {
            Some(p) => format!("[Phase {p}] {}", item.title),
            None => item.title.clone(),
        };
        out.push_str(&format!(
            "### {title}\n{}\n\n**{}:** {}\n**Source:** {}\n\n",
            item.summary, item.detail_label, item.detail, item.source
        ));
        if i + 1 < collected.len() {
            out.push_str("---\n\n");
        }
    }
}

fn render_phase_learnings_md(pl: &GsdPhaseLearnings) -> String {
    let mut out = format!(
        "---\nphase: {}\nphase_name: {:?}\nproject: \"Vox\"\ngenerated: {LEARNINGS_DATE:?}\ncounts:\n  decisions: {}\n  lessons: {}\n  patterns: {}\n  surprises: {}\n",
        pl.phase_number,
        pl.phase_name,
        pl.decisions.len(),
        pl.lessons.len(),
        pl.patterns.len(),
        pl.surprises.len()
    );
    if pl.missing_artifacts.is_empty() {
        out.push_str("missing_artifacts: []\n");
    } else {
        out.push_str("missing_artifacts:\n");
        for m in &pl.missing_artifacts {
            out.push_str(&format!("  - {m:?}\n"));
        }
    }
    out.push_str(&format!(
        "---\n\n# Phase {} Learnings: {}\n\n",
        pl.phase_number, pl.phase_name
    ));
    for (heading, items) in [
        ("Decisions", &pl.decisions),
        ("Lessons", &pl.lessons),
        ("Patterns", &pl.patterns),
        ("Surprises", &pl.surprises),
    ] {
        append_sections_md(&mut out, heading, items.iter().map(|e| (None, e)));
    }
    out
}

/// Extract structured learnings across all completed phase directories under `.planning/`
/// (`milestones/**`, `workstreams/**`, `phases/**`), write per-phase `*-LEARNINGS.md`,
/// consolidated `.planning/LEARNINGS.md`, and rebuild `.planning/estimation-calibration.json`.
pub fn extract_and_write_all_learnings(root: &Path) -> anyhow::Result<GsdLearningsReport> {
    let planning_dir = root.join(".planning");
    let entries = discover_phase_dirs(&planning_dir);
    let mut cal_map: BTreeMap<(String, String), CalibrationSample> = BTreeMap::new();
    let mut written_files = Vec::new();
    let mut unique: BTreeMap<(String, String), GsdPhaseLearnings> = BTreeMap::new();
    let total = |p: &GsdPhaseLearnings| {
        p.decisions.len() + p.lessons.len() + p.patterns.len() + p.surprises.len()
    };

    for entry in &entries {
        let pl = extract_phase_learnings(entry, &mut cal_map);
        let out_path = entry
            .dir
            .join(format!("{}-LEARNINGS.md", entry.padded_phase));
        fs::write(&out_path, render_phase_learnings_md(&pl))
            .with_context(|| format!("write {}", out_path.display()))?;
        written_files.push(out_path);
        let key = (entry.padded_phase.clone(), entry.phase_name.clone());
        if unique.get(&key).is_none_or(|prev| total(&pl) > total(prev)) {
            unique.insert(key, pl);
        }
    }

    let mut ordered: Vec<GsdPhaseLearnings> = unique.into_values().collect();
    ordered.sort_by_key(|p| phase_sort_key(&p.padded_phase));
    const HEADINGS: [&str; 4] = ["Decisions", "Lessons", "Patterns", "Surprises"];
    let mut cats: [Vec<(String, GsdLearningEntry)>; 4] = Default::default();
    let mut seen: [BTreeSet<(String, String)>; 4] = Default::default();
    for ph in &ordered {
        for (i, src) in [&ph.decisions, &ph.lessons, &ph.patterns, &ph.surprises]
            .into_iter()
            .enumerate()
        {
            for item in src {
                if seen[i].insert((ph.phase_number.clone(), item.title.to_ascii_lowercase())) {
                    cats[i].push((ph.phase_number.clone(), item.clone()));
                }
            }
        }
    }

    let mut root_md = format!(
        "---\nproject: \"Vox\"\ngenerated: {LEARNINGS_DATE:?}\nphases_covered: {}\ncounts:\n  decisions: {}\n  lessons: {}\n  patterns: {}\n  surprises: {}\n---\n\n# Vox Cross-Milestone GSD Learnings\n\n",
        ordered.len(),
        cats[0].len(),
        cats[1].len(),
        cats[2].len(),
        cats[3].len()
    );
    for (heading, list) in HEADINGS.into_iter().zip(&cats) {
        append_sections_md(
            &mut root_md,
            heading,
            list.iter().map(|(p, e)| (Some(p.as_str()), e)),
        );
    }
    let root_path = planning_dir.join("LEARNINGS.md");
    fs::write(&root_path, root_md).with_context(|| format!("write {}", root_path.display()))?;
    written_files.push(root_path);

    let samples: Vec<CalibrationSample> = cal_map.into_values().collect();
    let mut ratios: Vec<f64> = samples
        .iter()
        .filter(|s| s.estimate_tokens > 0)
        .map(|s| s.actual_tokens as f64 / s.estimate_tokens as f64)
        .collect();
    ratios.sort_by(f64::total_cmp);
    let factor = ratios.get(ratios.len() / 2).copied().unwrap_or(1.0);
    if !samples.is_empty() {
        let cal_path = planning_dir.join("estimation-calibration.json");
        let payload =
            serde_json::to_string_pretty(&json!({ "schema_version": 1, "samples": samples }))?;
        fs::write(&cal_path, format!("{payload}\n"))
            .with_context(|| format!("write {}", cal_path.display()))?;
        written_files.push(cal_path);
    }

    Ok(GsdLearningsReport {
        phases_processed: entries.len(),
        unique_phases: ordered.len(),
        decisions: cats[0].len(),
        lessons: cats[1].len(),
        patterns: cats[2].len(),
        surprises: cats[3].len(),
        calibration_samples: samples.len(),
        calibration_factor: factor,
        written_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_and_write_all_learnings_writes_phase_and_root_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        for base in [
            root.join(".planning/milestones/v1.0-phases/01-init-core"),
            root.join(".planning/workstreams/ws-copy/phases/01-init-core"),
        ] {
            fs::create_dir_all(&base).unwrap();
            fs::write(
                base.join("01-01-PLAN.md"),
                "---\nphase: 01-init-core\nestimate:\n  raw_tokens: 40000\n---\n<objective>Initialize core types cleanly.</objective>\n",
            ).unwrap();
            fs::write(
                base.join("01-01-SUMMARY.md"),
                "---\nphase: 01-init-core\nactuals:\n  tokens: 10000\nkey-decisions:\n  - \"Adopt L0 pure-data types before DB ops\"\npatterns-established:\n  - \"CAS content hashing before table upsert\"\n---\n## Issues Encountered\n- Rule 3: Added missing FromStr impl before compilation.\n",
            ).unwrap();
        }
        let report = extract_and_write_all_learnings(root).expect("extract learnings");
        assert_eq!(report.phases_processed, 2);
        assert_eq!(report.unique_phases, 1);
        assert_eq!(report.calibration_samples, 1);
        assert!((report.calibration_factor - 0.25).abs() < 1e-6);

        let phase_learnings = fs::read_to_string(
            root.join(".planning/milestones/v1.0-phases/01-init-core/01-LEARNINGS.md"),
        )
        .unwrap();
        assert!(phase_learnings.contains("## Decisions") && phase_learnings.contains("## Lessons"));
        assert!(
            phase_learnings.contains("## Patterns") && phase_learnings.contains("## Surprises")
        );
        assert!(phase_learnings.contains("**Source:** 01-01-SUMMARY.md"));

        let root_learnings = fs::read_to_string(root.join(".planning/LEARNINGS.md")).unwrap();
        assert!(root_learnings.contains("### [Phase 1] Adopt L0 pure-data types before DB ops"));
        assert_eq!(
            root_learnings
                .matches("Adopt L0 pure-data types before DB ops")
                .count(),
            2
        );

        let cal = fs::read_to_string(root.join(".planning/estimation-calibration.json")).unwrap();
        assert!(
            cal.contains("\"estimateTokens\": 40000") && cal.contains("\"actualTokens\": 10000")
        );
    }
}
