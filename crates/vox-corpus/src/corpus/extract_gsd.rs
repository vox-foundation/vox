//! GSD (Get Shit Done) training corpus extractor for Mens (`gsd_workflows.jsonl`).
//!
//! Extracts workflow supervision, skill-routing, wave decomposition, and MCP tool-call
//! pairs from `assets/skills/gsd-*/SKILL.md`, `assets/skills/brainstorming/SKILL.md`,
//! `.planning/{ROADMAP,REQUIREMENTS}.md`, `.planning/phases/*/*-PLAN.md`, and canonical
//! `vox_gsd_*` tool schemas.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use vox_bounded_fs::read_utf8_path_capped;

use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

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
            "prompt": self.prompt,
            "response": self.response,
            "category": self.category,
            "rating": self.rating,
            "difficulty": self.difficulty,
            "lane": self.lane,
            "task_family": self.task_family,
            "source": self.source_path.display().to_string(),
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

fn parse_frontmatter_fields(content: &str) -> (Option<String>, Option<String>, String) {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---")
        && let Some(end_idx) = rest.find("\n---")
    {
        let yaml_block = &rest[..end_idx];
        let body = rest[end_idx + 4..].trim().to_string();
        let mut name = None;
        let mut desc = None;
        for line in yaml_block.lines() {
            if let Some(v) = line.trim().strip_prefix("name:") {
                name = Some(v.trim().trim_matches('"').to_string());
            } else if let Some(v) = line.trim().strip_prefix("description:") {
                desc = Some(v.trim().trim_matches('"').to_string());
            }
        }
        return (name, desc, body);
    }
    (None, None, content.trim().to_string())
}

fn collect_skill_pairs(root: &Path, out: &mut Vec<GsdTrainingPair>) {
    let skills_root = root.join("assets/skills");
    let Ok(entries) = fs::read_dir(&skills_root) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|x| x.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            p.is_dir() && (name.starts_with("gsd-") || name == "brainstorming")
        })
        .collect();
    dirs.sort();

    for dir in dirs {
        let skill_md = dir.join("SKILL.md");
        let Ok(raw) = read_utf8_path_capped(&skill_md) else {
            continue;
        };
        let dir_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("gsd-skill")
            .to_string();
        let (name_opt, desc_opt, body) = parse_frontmatter_fields(&raw);
        let skill_name = name_opt.unwrap_or_else(|| dir_name.clone());
        let desc = desc_opt.unwrap_or_else(|| format!("Run GSD workflow skill `{skill_name}`."));
        let excerpt: String = body.lines().take(24).collect::<Vec<_>>().join("\n");

        out.push(GsdTrainingPair {
            source_path: skill_md.clone(),
            category: "gsd_workflow".to_string(),
            prompt: format!(
                "When should a Vox orchestrator agent select `/{skill_name}` ({desc}) and what procedure does it follow?"
            ),
            response: format!(
                "Select `/{skill_name}` when: {desc}\n\nWorkflow procedure:\n{excerpt}"
            ),
            rating: 5,
            difficulty: 4,
            lane: "vox_tooling".to_string(),
            task_family: "gsd_skill_routing".to_string(),
            metadata: json!({ "skill": skill_name }),
        });
    }
}

fn collect_planning_pairs(root: &Path, out: &mut Vec<GsdTrainingPair>) {
    let planning_dir = root.join(".planning");
    if !planning_dir.is_dir() {
        return;
    }
    for rel in ["PROJECT.md", "ROADMAP.md", "REQUIREMENTS.md", "STATE.md"] {
        let path = planning_dir.join(rel);
        let Ok(raw) = read_utf8_path_capped(&path) else {
            continue;
        };
        let snippet: String = raw.lines().take(36).collect::<Vec<_>>().join("\n");
        if snippet.trim().is_empty() {
            continue;
        }
        out.push(GsdTrainingPair {
            source_path: path,
            category: "gsd_workflow".to_string(),
            prompt: format!(
                "How does Vox store and synchronize `.planning/{rel}` in GSD milestone state and Arca CAS?"
            ),
            response: format!(
                "`.planning/{rel}` is indexed in `VoxDb` (`gsd_documents` + Arca CAS `objects`) via `vox_gsd_store_document` and synchronized cleanly on milestone/phase transitions.\n\nExcerpt:\n```markdown\n{snippet}\n```"
            ),
            rating: 5,
            difficulty: 5,
            lane: "vox_tooling".to_string(),
            task_family: "gsd_planning_artifact".to_string(),
            metadata: json!({ "artifact": rel }),
        });
    }

    let phases_dir = planning_dir.join("phases");
    if let Ok(entries) = fs::read_dir(&phases_dir) {
        let mut p_dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|x| x.path())).collect();
        p_dirs.sort();
        for pdir in p_dirs.into_iter().take(12) {
            let Ok(files) = fs::read_dir(&pdir) else {
                continue;
            };
            for file in files.filter_map(|f| f.ok().map(|x| x.path())) {
                let fname = file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if !fname.ends_with("-PLAN.md") {
                    continue;
                }
                if let Ok(raw) = read_utf8_path_capped(&file) {
                    let excerpt: String = raw.lines().take(32).collect::<Vec<_>>().join("\n");
                    out.push(GsdTrainingPair {
                        source_path: file,
                        category: "gsd_workflow".to_string(),
                        prompt: format!(
                            "Decompose GSD phase plan `{fname}` into dependency-ordered execution waves for `vox-orchestrator`."
                        ),
                        response: excerpt,
                        rating: 5,
                        difficulty: 6,
                        lane: "vox_tooling".to_string(),
                        task_family: "gsd_wave_decomposition".to_string(),
                        metadata: json!({ "plan_file": fname }),
                    });
                }
            }
        }
    }
}

fn collect_mcp_supervision_pairs(root: &Path, out: &mut Vec<GsdTrainingPair>) {
    let examples = [
        (
            "Initialize GSD milestone v7.1 named \"Autonomic Waves\" starting at phase 413 with 4 phases.",
            "vox_tool_selection",
            json!({
                "tool_name": "vox_gsd_new_milestone",
                "arguments": { "version": "v7.1", "title": "Autonomic Waves", "start_phase": 413, "phases": 4 }
            }),
        ),
        (
            "Inspect current active GSD milestone progress, phase statuses, and recommended next skill.",
            "vox_tool_selection",
            json!({
                "tool_name": "vox_gsd_get_status",
                "arguments": {}
            }),
        ),
        (
            "Synthesize or load the wave DAG plan for GSD phase 411 under milestone v7.0.",
            "vox_argument_generation",
            json!({
                "tool_name": "vox_gsd_plan_phase",
                "arguments": { "phase": 411, "mvp": false, "milestone_id": "v7.0" }
            }),
        ),
        (
            "Launch wave-parallel PlanNode execution for GSD phase 411 in vox-orchestrator.",
            "vox_argument_generation",
            json!({
                "tool_name": "vox_gsd_execute_phase",
                "arguments": { "phase": 411, "milestone_id": "v7.0" }
            }),
        ),
        (
            "Record passing UAT verification evidence for GSD phase 411.",
            "vox_argument_generation",
            json!({
                "tool_name": "vox_gsd_verify_work",
                "arguments": { "phase": 411, "passed": true, "evidence": "Playwright gsd-suite.spec.ts passed", "milestone_id": "v7.0" }
            }),
        ),
        (
            "Store updated `.planning/STATE.md` into disk and Arca CAS-backed `gsd_documents`.",
            "vox_argument_generation",
            json!({
                "tool_name": "vox_gsd_store_document",
                "arguments": { "rel_path": ".planning/STATE.md", "content": "# State\nCurrent Phase: 412\n", "kind": "state", "milestone_id": "v7.0" }
            }),
        ),
    ];

    for (prompt, lane, payload) in examples {
        out.push(GsdTrainingPair {
            source_path: root.join("crates/vox-orchestrator-mcp/src/gsd_tools.rs"),
            category: "gsd_workflow".to_string(),
            prompt: format!(
                "[vox_tool_supervision]\nTask:\n{prompt}\n\nRespond with a single JSON object containing tool_name and arguments."
            ),
            response: payload.to_string(),
            rating: 5,
            difficulty: 5,
            lane: lane.to_string(),
            task_family: "gsd_mcp_tool".to_string(),
            metadata: json!({ "tool": payload["tool_name"] }),
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
        )
        .unwrap();
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
        let rec = pairs[0].to_training_record_v2(SpokeKind::AgentHarness);
        assert!(rec.verify_integrity());

        let out_jsonl = root.join("out/gsd.jsonl");
        let n = write_gsd_to_jsonl(&pairs, &out_jsonl).expect("write jsonl");
        assert_eq!(n, pairs.len());
    }
}
