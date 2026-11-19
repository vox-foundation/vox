//! GSD (Get Shit Done) training corpus and learnings extractor for Mens (`gsd_workflows.jsonl`).
//!
//! Extracts workflow supervision, skill-routing, wave decomposition, phase learnings,
//! and MCP tool-call pairs across `.planning/` milestones and workstreams.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_yaml::Value as YamlValue;
use vox_bounded_fs::read_utf8_path_capped;

pub use crate::corpus::extract_gsd_learnings::{
    CalibrationSample, GsdLearningEntry, GsdLearningsReport, GsdPhaseLearnings,
    extract_and_write_all_learnings,
};
use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

const SKIP_PLANNING_DIRS: &[&str] = &[
    ".cache",
    "classifications",
    "codebase",
    "todos",
    "ui-reviews",
    "research",
    "intel",
    ".git",
];

/// Configuration for GSD corpus extraction.
#[derive(Debug, Clone)]
pub struct ExtractGsdConfig {
    /// Repository root directory containing `assets/skills/` and `.planning/`.
    pub root: PathBuf,
    /// Maximum number of pairs to emit (`0` = unlimited).
    pub limit: usize,
}

impl Default for ExtractGsdConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            limit: 0,
        }
    }
}

/// One extracted GSD workflow / tool-calling training pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GsdTrainingPair {
    pub source_path: PathBuf,
    pub category: String,
    pub prompt: String,
    pub response: String,
    pub rating: u8,
    pub difficulty: u8,
    pub lane: String,
    pub task_family: String,
    pub metadata: serde_json::Value,
}

impl GsdTrainingPair {
    /// Serialize into one JSONL row compatible with `vox_corpus::corpus::mix`.
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        json!({
            "prompt": self.prompt, "response": self.response, "category": self.category,
            "rating": self.rating, "difficulty": self.difficulty, "lane": self.lane,
            "task_family": self.task_family, "source": self.source_path.display().to_string(),
            "metadata": self.metadata,
        })
        .to_string()
    }

    /// Convert into a content-hashed [`TrainingRecordV2`] for multi-spoke ingestion.
    #[must_use]
    pub fn to_training_record_v2(&self, spoke: SpokeKind) -> TrainingRecordV2 {
        let receipt = OracleReceipt {
            spoke,
            passed: true,
            stage: "gsd_extract".to_string(),
            diagnostics: Vec::new(),
            exit_code: Some(0),
        };
        let prov = RecordProvenance {
            origin: "gsd_corpus".to_string(),
            source_id: self.source_path.display().to_string(),
            task_family: self.task_family.clone(),
            difficulty: self.difficulty,
        };
        TrainingRecordV2::new(
            spoke,
            self.prompt.clone(),
            "You are a Vox GSD workflow & orchestrator agent.",
            self.response.clone(),
            receipt,
            prov,
        )
    }
}

pub(crate) struct PhaseDirEntry {
    pub(crate) dir: PathBuf,
    pub(crate) dir_name: String,
    pub(crate) phase_number: String,
    pub(crate) padded_phase: String,
    pub(crate) phase_name: String,
    pub(crate) plans: Vec<PathBuf>,
    pub(crate) summaries: Vec<PathBuf>,
    pub(crate) verifications: Vec<PathBuf>,
    pub(crate) uats: Vec<PathBuf>,
    pub(crate) reviews: Vec<PathBuf>,
    pub(crate) has_state: bool,
}

pub(crate) fn split_frontmatter(content: &str) -> (Option<YamlValue>, &str) {
    let trimmed = content.trim_start_matches('\u{feff}').trim_start();
    if let Some(rest) = trimmed.strip_prefix("---")
        && let Some(end_idx) = rest.find("\n---")
    {
        return (
            serde_yaml::from_str::<YamlValue>(&rest[..end_idx]).ok(),
            rest[end_idx + 4..].trim(),
        );
    }
    (None, content.trim())
}

pub(crate) fn extract_xml_tag(text: &str, tag: &str) -> Option<String> {
    let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    let inner = text[start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!inner.is_empty()).then_some(inner)
}

pub(crate) fn yaml_str_list(yaml: Option<&YamlValue>, path: &[&str]) -> Vec<String> {
    let mut cur = yaml;
    for k in path {
        cur = cur.and_then(|v| v.get(*k));
    }
    cur.and_then(|v| v.as_sequence())
        .into_iter()
        .flatten()
        .filter_map(|v| {
            v.as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .collect()
}

pub(crate) fn extract_yaml_coverage(yaml: Option<&YamlValue>, key: &str) -> Vec<String> {
    yaml.and_then(|v| v.get("coverage")?.as_sequence())
        .into_iter()
        .flatten()
        .filter_map(|r| {
            r.get(key)?
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .collect()
}

pub(crate) fn extract_yaml_tokens(raw: &str, block_key: &str, fields: &[&str]) -> Option<u64> {
    let (yaml, _) = split_frontmatter(raw);
    if let Some(block) = yaml.as_ref().and_then(|d| d.get(block_key))
        && let Some(n) = fields.iter().find_map(|f| block.get(*f)?.as_u64())
    {
        return Some(n);
    }
    let rest = raw
        .trim_start_matches('\u{feff}')
        .trim_start()
        .strip_prefix("---")?;
    let mut in_block = false;
    for line in rest[..rest.find("\n---")?].lines() {
        let t = line.trim();
        if t == format!("{block_key}:") {
            in_block = true;
        } else if in_block && !line.starts_with([' ', '\t']) && !t.is_empty() {
            in_block = false;
        } else if in_block
            && let Some(n) = fields
                .iter()
                .find_map(|f| t.strip_prefix(&format!("{f}:"))?.trim().parse::<u64>().ok())
        {
            return Some(n);
        }
    }
    None
}

pub(crate) fn file_name_str(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("artifact.md")
        .to_string()
}

pub(crate) fn phase_sort_key(padded: &str) -> (u32, u32, String) {
    let (maj, min) = padded.split_once('.').unwrap_or((padded, "0"));
    (
        maj.parse().unwrap_or(9999),
        min.parse().unwrap_or(0),
        padded.to_string(),
    )
}

fn parse_frontmatter_fields(content: &str) -> (Option<String>, Option<String>, String) {
    let (yaml, body) = split_frontmatter(content);
    let mut name = yaml
        .as_ref()
        .and_then(|d| d.get("name")?.as_str())
        .map(str::to_string);
    let mut desc = yaml
        .as_ref()
        .and_then(|d| d.get("description")?.as_str())
        .map(str::to_string);
    if name.is_none()
        && desc.is_none()
        && let Some(rest) = content.trim_start().strip_prefix("---")
        && let Some(end) = rest.find("\n---")
    {
        for line in rest[..end].lines() {
            if let Some(v) = line.trim().strip_prefix("name:") {
                name = Some(v.trim().trim_matches('"').to_string());
            } else if let Some(v) = line.trim().strip_prefix("description:") {
                desc = Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    (name, desc, body.to_string())
}

fn parse_phase_dir_meta(dir_name: &str) -> (String, String, String) {
    let (padded, name) = match dir_name.split_once('-') {
        Some((p, n)) if p.chars().any(|c| c.is_ascii_digit()) => (p.to_string(), n.to_string()),
        _ => ("00".to_string(), dir_name.to_string()),
    };
    let num = match padded.split_once('.') {
        Some((maj, min)) => {
            let m = maj.trim_start_matches('0');
            format!("{}.{}", if m.is_empty() { "0" } else { m }, min)
        }
        None => {
            let t = padded.trim_start_matches('0');
            if t.is_empty() { "0" } else { t }.to_string()
        }
    };
    (num, padded, name)
}

pub(crate) fn discover_phase_dirs(planning_dir: &Path) -> Vec<PhaseDirEntry> {
    let mut stack = vec![planning_dir.to_path_buf()];
    let mut found = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut files = Vec::new();
        for p in entries.flatten().map(|e| e.path()) {
            if p.is_dir() {
                let dname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !SKIP_PLANNING_DIRS.contains(&dname) && !dname.starts_with('.') {
                    stack.push(p);
                }
            } else if p.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(p);
            }
        }
        files.sort();
        let (mut plans, mut summaries, mut verifications, mut uats, mut reviews) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for f in files {
            let fname = f.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if fname == "PLAN.md" || fname.ends_with("-PLAN.md") {
                plans.push(f);
            } else if fname == "SUMMARY.md" || fname.ends_with("-SUMMARY.md") {
                summaries.push(f);
            } else if fname == "VERIFICATION.md" || fname.ends_with("-VERIFICATION.md") {
                verifications.push(f);
            } else if fname == "UAT.md" || fname.ends_with("-UAT.md") {
                uats.push(f);
            } else if fname == "REVIEW.md"
                || fname.ends_with("-REVIEW.md")
                || fname.ends_with("-REVIEW-FIX.md")
            {
                reviews.push(f);
            }
        }
        if !plans.is_empty() && !summaries.is_empty() {
            let dir_name = file_name_str(&dir);
            let (phase_number, padded_phase, phase_name) = parse_phase_dir_meta(&dir_name);
            let mut has_state = false;
            let mut cur = dir.parent();
            while let Some(c) = cur {
                if c.join("STATE.md").is_file() {
                    has_state = true;
                    break;
                }
                if c == planning_dir {
                    break;
                }
                cur = c.parent();
            }
            found.push(PhaseDirEntry {
                dir,
                dir_name,
                phase_number,
                padded_phase,
                phase_name,
                plans,
                summaries,
                verifications,
                uats,
                reviews,
                has_state,
            });
        }
    }
    found.sort_by(|a, b| {
        phase_sort_key(&a.padded_phase)
            .cmp(&phase_sort_key(&b.padded_phase))
            .then_with(|| a.dir.cmp(&b.dir))
    });
    found
}

fn collect_skill_pairs(root: &Path, out: &mut Vec<GsdTrainingPair>) {
    let Ok(entries) = fs::read_dir(root.join("assets/skills")) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|x| x.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            p.is_dir() && (n.starts_with("gsd-") || n == "brainstorming")
        })
        .collect();
    dirs.sort();
    for dir in dirs {
        let skill_md = dir.join("SKILL.md");
        let Ok(raw) = read_utf8_path_capped(&skill_md) else {
            continue;
        };
        let (name_opt, desc_opt, body) = parse_frontmatter_fields(&raw);
        let skill_name = name_opt.unwrap_or_else(|| file_name_str(&dir));
        let desc = desc_opt.unwrap_or_else(|| format!("Run GSD workflow skill `{skill_name}`."));
        let excerpt: String = body.lines().take(24).collect::<Vec<_>>().join("\n");
        out.push(GsdTrainingPair {
            source_path: skill_md, category: "gsd_workflow".to_string(),
            prompt: format!("When should a Vox orchestrator agent select `/{skill_name}` ({desc}) and what procedure does it follow?"),
            response: format!("Select `/{skill_name}` when: {desc}\n\nWorkflow procedure:\n{excerpt}"),
            rating: 5, difficulty: 4, lane: "vox_tooling".to_string(),
            task_family: "gsd_skill_routing".to_string(), metadata: json!({ "skill": skill_name }),
        });
    }
}

fn collect_planning_pairs(root: &Path, out: &mut Vec<GsdTrainingPair>) {
    let planning_dir = root.join(".planning");
    if !planning_dir.is_dir() {
        return;
    }
    for rel in [
        "PROJECT.md",
        "ROADMAP.md",
        "REQUIREMENTS.md",
        "STATE.md",
        "MILESTONES.md",
        "RETROSPECTIVE.md",
        "LEARNINGS.md",
    ] {
        let path = planning_dir.join(rel);
        let Ok(raw) = read_utf8_path_capped(&path) else {
            continue;
        };
        let snippet: String = raw.lines().take(36).collect::<Vec<_>>().join("\n");
        if snippet.trim().is_empty() {
            continue;
        }
        out.push(GsdTrainingPair {
            source_path: path, category: "gsd_workflow".to_string(),
            prompt: format!("How does Vox store and synchronize `.planning/{rel}` in GSD milestone state and Arca CAS?"),
            response: format!("`.planning/{rel}` is indexed in `VoxDb` (`gsd_documents` + Arca CAS `objects`) via `vox_gsd_store_document` and synchronized cleanly on milestone/phase transitions.\n\nExcerpt:\n```markdown\n{snippet}\n```"),
            rating: 5, difficulty: 5, lane: "vox_tooling".to_string(),
            task_family: "gsd_planning_artifact".to_string(), metadata: json!({ "artifact": rel }),
        });
    }
    let mut seen_phases = BTreeSet::new();
    for entry in discover_phase_dirs(&planning_dir) {
        if !seen_phases.insert((entry.padded_phase.clone(), entry.phase_name.clone())) {
            continue;
        }
        for plan in &entry.plans {
            let fname = file_name_str(plan);
            if let Ok(raw) = read_utf8_path_capped(plan) {
                let excerpt: String = raw.lines().take(32).collect::<Vec<_>>().join("\n");
                out.push(GsdTrainingPair {
                    source_path: plan.clone(), category: "gsd_workflow".to_string(),
                    prompt: format!("Decompose GSD phase plan `{fname}` ({}) into dependency-ordered execution waves for `vox-orchestrator`.", entry.phase_name),
                    response: excerpt, rating: 5, difficulty: 6, lane: "vox_tooling".to_string(),
                    task_family: "gsd_wave_decomposition".to_string(), metadata: json!({ "plan_file": fname, "phase": entry.phase_number }),
                });
            }
        }
        let learnings_file = entry
            .dir
            .join(format!("{}-LEARNINGS.md", entry.padded_phase));
        let src = if learnings_file.is_file() {
            learnings_file
        } else if let Some(s) = entry.summaries.first() {
            s.clone()
        } else {
            continue;
        };
        if let Ok(raw) = read_utf8_path_capped(&src) {
            let excerpt: String = raw.lines().take(36).collect::<Vec<_>>().join("\n");
            if !excerpt.trim().is_empty() {
                out.push(GsdTrainingPair {
                    source_path: src, category: "gsd_workflow".to_string(),
                    prompt: format!("What architectural decisions, lessons, and patterns were extracted from GSD Phase {} ({})?", entry.phase_number, entry.phase_name),
                    response: excerpt, rating: 5, difficulty: 6, lane: "vox_tooling".to_string(),
                    task_family: "gsd_phase_learning".to_string(), metadata: json!({ "phase": entry.phase_number, "phase_name": entry.phase_name }),
                });
            }
        }
    }
}

fn collect_mcp_supervision_pairs(root: &Path, out: &mut Vec<GsdTrainingPair>) {
    let examples = [
        (
            "Initialize GSD milestone v7.1 named \"Autonomic Waves\" starting at phase 413 with 4 phases.",
            "vox_tool_selection",
            json!({"tool_name": "vox_gsd_new_milestone", "arguments": {"version": "v7.1", "title": "Autonomic Waves", "start_phase": 413, "phases": 4}}),
        ),
        (
            "Inspect current active GSD milestone progress, phase statuses, and recommended next skill.",
            "vox_tool_selection",
            json!({"tool_name": "vox_gsd_get_status", "arguments": {}}),
        ),
        (
            "Synthesize or load the wave DAG plan for GSD phase 411 under milestone v7.0.",
            "vox_argument_generation",
            json!({"tool_name": "vox_gsd_plan_phase", "arguments": {"phase": 411, "mvp": false, "milestone_id": "v7.0"}}),
        ),
        (
            "Launch wave-parallel PlanNode execution for GSD phase 411 in vox-orchestrator.",
            "vox_argument_generation",
            json!({"tool_name": "vox_gsd_execute_phase", "arguments": {"phase": 411, "milestone_id": "v7.0"}}),
        ),
        (
            "Record passing UAT verification evidence for GSD phase 411.",
            "vox_argument_generation",
            json!({"tool_name": "vox_gsd_verify_work", "arguments": {"phase": 411, "passed": true, "evidence": "Playwright gsd-suite.spec.ts passed", "milestone_id": "v7.0"}}),
        ),
        (
            "Store updated `.planning/STATE.md` into disk and Arca CAS-backed `gsd_documents`.",
            "vox_argument_generation",
            json!({"tool_name": "vox_gsd_store_document", "arguments": {"rel_path": ".planning/STATE.md", "content": "# State\nCurrent Phase: 412\n", "kind": "state", "milestone_id": "v7.0"}}),
        ),
    ];
    for (prompt, lane, payload) in examples {
        out.push(GsdTrainingPair {
            source_path: root.join("crates/vox-orchestrator-mcp/src/gsd_tools.rs"),
            category: "gsd_workflow".to_string(),
            prompt: format!("[vox_tool_supervision]\nTask:\n{prompt}\n\nRespond with a single JSON object containing tool_name and arguments."),
            response: payload.to_string(), rating: 5, difficulty: 5, lane: lane.to_string(),
            task_family: "gsd_mcp_tool".to_string(), metadata: json!({ "tool": payload["tool_name"] }),
        });
    }
}

/// Extract GSD workflow, skill routing, `.planning/` artifact, and `vox_gsd_*` tool pairs.
pub fn extract_gsd_corpus(config: &ExtractGsdConfig) -> anyhow::Result<Vec<GsdTrainingPair>> {
    let mut pairs = Vec::new();
    collect_skill_pairs(&config.root, &mut pairs);
    collect_planning_pairs(&config.root, &mut pairs);
    collect_mcp_supervision_pairs(&config.root, &mut pairs);
    if config.limit > 0 && pairs.len() > config.limit {
        pairs.truncate(config.limit);
    }
    Ok(pairs)
}

/// Write extracted GSD training pairs to a UTF-8 JSONL file.
pub fn write_gsd_to_jsonl(pairs: &[GsdTrainingPair], out: &Path) -> anyhow::Result<usize> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut file = fs::File::create(out).with_context(|| format!("create {}", out.display()))?;
    for pair in pairs {
        writeln!(file, "{}", pair.to_jsonl())?;
    }
    Ok(pairs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_gsd_skills_planning_and_mcp_pairs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        fs::create_dir_all(root.join("assets/skills/gsd-new-milestone")).unwrap();
        fs::write(
            root.join("assets/skills/gsd-new-milestone/SKILL.md"),
            "---\nname: gsd-new-milestone\ndescription: Start a GSD milestone.\n---\n# Body\nStep 1\n",
        ).unwrap();
        fs::create_dir_all(root.join(".planning")).unwrap();
        fs::write(
            root.join(".planning/ROADMAP.md"),
            "# Roadmap\n## Phase 1: Init\n",
        )
        .unwrap();

        let pairs = extract_gsd_corpus(&ExtractGsdConfig {
            root: root.to_path_buf(),
            limit: 0,
        })
        .expect("extract");
        assert!(pairs.len() >= 8);
        assert!(
            pairs[0]
                .to_training_record_v2(SpokeKind::AgentHarness)
                .verify_integrity()
        );

        let out_jsonl = root.join("out/gsd.jsonl");
        assert_eq!(
            write_gsd_to_jsonl(&pairs, &out_jsonl).expect("write jsonl"),
            pairs.len()
        );
    }

    #[test]
    fn workspace_extract_all_gsd_learnings_and_corpus() {
        if std::env::var_os("VOX_EXTRACT_WORKSPACE_LEARNINGS").is_none() {
            return;
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let report = extract_and_write_all_learnings(&root).expect("workspace extract learnings");
        println!(
            "WORKSPACE_LEARNINGS: phases_processed={} unique_phases={} decisions={} lessons={} patterns={} surprises={} calibration_samples={} calibration_factor={:.4}",
            report.phases_processed,
            report.unique_phases,
            report.decisions,
            report.lessons,
            report.patterns,
            report.surprises,
            report.calibration_samples,
            report.calibration_factor
        );
        let pairs = extract_gsd_corpus(&ExtractGsdConfig {
            root: root.clone(),
            limit: 0,
        })
        .expect("workspace gsd corpus");
        let out_path = root.join("mens/data/mix_sources/gsd_workflows.jsonl");
        let written = write_gsd_to_jsonl(&pairs, &out_path).expect("write gsd_workflows.jsonl");
        println!(
            "WORKSPACE_CORPUS: pairs_written={} path={}",
            written,
            out_path.display()
        );
        assert!(report.unique_phases > 50);
        assert!(written > 50);
    }
}
