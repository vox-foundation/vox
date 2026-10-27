//! Diagram-authoring eval (EVAL-01): do agents author VoxDiagram at least as reliably as Mermaid?
//!
//! Paired design: every task goes to both arms (VoxDiagram, Mermaid) with the same model, and each
//! arm's output is judged by a deterministic parser, never an LLM: `vox_diagram::check` here, the
//! pinned real Mermaid parser in Node (`docs-astro/tests/lib/mermaid-batch.mjs`). Failed attempts
//! get up to two repair turns fed only that arm's own parser diagnostics.
//!
//! The numbers re-derive from the recording without the network, split by runtime so neither side
//! runs the other:
//!
//! ```text
//! tasks -> reference-inputs (Rust) -> reference-outputs (Node)
//! attempts + reference-outputs -> oracle-inputs (Rust) -> oracle-outputs (Node)
//! attempts + oracle-outputs -> tallies.json + tables.md (Rust)
//! ```
//!
//! The pre-registered decision rule (primary endpoint, interval, margin) lives in the dataset
//! manifest's `decision_rule` and in the findings report; this module reads it, never redefines it.

pub mod oracle;
pub mod render;
pub mod score;
pub mod tally;

use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use vox_doc_pipeline::diagram::{Emit, ValidateOpts, validate};

/// Repair turns after the first pass; turn numbers run `0..=MAX_REPAIR_TURNS`.
pub const MAX_REPAIR_TURNS: u32 = 2;

/// The two authoring languages compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arm {
    Voxdiagram,
    Mermaid,
}

impl Arm {
    pub const BOTH: [Arm; 2] = [Arm::Voxdiagram, Arm::Mermaid];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Arm::Voxdiagram => "voxdiagram",
            Arm::Mermaid => "mermaid",
        }
    }
}

/// One natural-language task; `reference` is a VoxDiagram source whose Mermaid export defines the
/// expected canonical sets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub kind: String,
    pub prompt: String,
    pub reference: String,
}

/// One model slot of a run (A = primary, B = replication).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSlot {
    pub slot: String,
    pub requested_id: String,
    pub provider: String,
    pub family: String,
}

/// Line 1 of `attempts.jsonl`. Never holds an `LlmConfig`, headers or keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunHeader {
    pub schema_version: u32,
    /// A hand-written scorer fixture, never a measurement.
    pub synthetic: bool,
    pub freeze_commit: String,
    pub tasks_hash: String,
    pub card_hashes: std::collections::BTreeMap<String, String>,
    pub prompt_template_hash: String,
    pub intent: String,
    pub scope: String,
    pub selection: serde_json::Value,
    pub models: Vec<ModelSlot>,
    pub temperature: f64,
    pub seed_base: u64,
    pub max_tokens: u32,
    pub max_spend_usd: f64,
    pub mermaid_version: String,
    pub vox_version: String,
    pub probes: serde_json::Value,
    pub started_at: String,
}

/// One model call (or one transport failure) for a task, slot, arm and turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    pub task_id: String,
    pub arm: Arm,
    pub slot: String,
    pub turn: u32,
    pub seed: u64,
    pub requested_model: String,
    pub resolved_model: Option<String>,
    /// Text sent in this turn: the previous turn's parser diagnostics; `None` at turn 0.
    pub feedback: Option<String>,
    pub raw_response: Option<String>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub transport_error: Option<String>,
    pub retries: u32,
}

impl Attempt {
    /// Oracle line id `<task_id>:<slot>:<arm>:<turn>`.
    #[must_use]
    pub fn oracle_id(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.task_id,
            self.slot,
            self.arm.as_str(),
            self.turn
        )
    }
}

/// Why a run ended early (spend cap, no provider, ...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    pub reason: String,
    pub spent_usd: f64,
}

/// One line of `attempts.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Line {
    Run(RunHeader),
    Attempt(Attempt),
    Stop(Stop),
}

/// One line sent to the Node oracle; `expected` is absent for reference inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleInput {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<serde_json::Value>,
}

/// One line the Node oracle wrote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleOutput {
    pub id: String,
    pub ok: bool,
    pub error: Option<String>,
    pub sets: serde_json::Value,
    #[serde(rename = "match")]
    pub matched: Option<bool>,
}

/// The pre-registered decision rule (`decision_rule` in the dataset manifest).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub primary_endpoint: String,
    pub interval: String,
    pub confidence: f64,
    pub margin: f64,
    pub max_repair_turns: u32,
    pub max_infra_rate: f64,
}

/// The machine part of `manifest.v1.yaml`; other keys (prose, hashes) are ignored here.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Manifest {
    pub corpus_size: usize,
    pub decision_rule: Rule,
}

/// Parse a manifest and refuse a rule this scorer does not implement.
pub fn parse_manifest(yaml: &str) -> Result<Manifest> {
    let m: Manifest = serde_yaml::from_str(yaml).context("manifest.v1.yaml")?;
    let r = &m.decision_rule;
    ensure!(
        r.primary_endpoint == "first_pass_validity_slot_a",
        "unsupported primary_endpoint {:?}",
        r.primary_endpoint
    );
    ensure!(
        r.interval == "newcombe_method_10",
        "unsupported interval {:?}",
        r.interval
    );
    ensure!(
        (r.confidence - 0.95).abs() < 1e-12,
        "unsupported confidence {}",
        r.confidence
    );
    ensure!(
        r.margin > 0.0 && r.margin < 1.0,
        "margin {} outside (0, 1)",
        r.margin
    );
    ensure!(
        r.max_repair_turns == MAX_REPAIR_TURNS,
        "max_repair_turns must be {MAX_REPAIR_TURNS}"
    );
    ensure!(
        (0.0..1.0).contains(&r.max_infra_rate),
        "max_infra_rate {} outside [0, 1)",
        r.max_infra_rate
    );
    Ok(m)
}

/// Parse JSONL (blank lines skipped); errors name the 1-based line.
pub fn read_jsonl<T: DeserializeOwned>(text: &str) -> Result<Vec<T>> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| serde_json::from_str(l).with_context(|| format!("JSONL line {}", i + 1)))
        .collect()
}

/// Serialize values as JSONL, one compact object per line.
pub fn write_jsonl<T: Serialize>(items: &[T]) -> Result<String> {
    let mut out = String::new();
    for item in items {
        out.push_str(&serde_json::to_string(item)?);
        out.push('\n');
    }
    Ok(out)
}

/// `reference-inputs.jsonl`: the Mermaid export of every task's reference, in task order.
pub fn reference_inputs(tasks: &[Task]) -> Result<String> {
    let mut lines = Vec::with_capacity(tasks.len());
    for t in tasks {
        let v = validate(
            &t.reference,
            ValidateOpts {
                from_mermaid: false,
                emit: Some(Emit::Mermaid),
            },
        );
        let text = v.emitted.filter(|_| v.valid).with_context(|| {
            format!(
                "task {}: reference does not check: {:?}",
                t.id, v.diagnostics
            )
        })?;
        lines.push(OracleInput {
            id: t.id.clone(),
            text,
            expected: None,
        });
    }
    write_jsonl(&lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn task(id: &str, reference: &str) -> Task {
        Task {
            id: id.into(),
            kind: "graph".into(),
            prompt: "p".into(),
            reference: reference.into(),
        }
    }

    pub(super) const GRAPH: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";

    #[test]
    fn reference_inputs_one_line_per_task_in_order() {
        let out = reference_inputs(&[task("t2", GRAPH), task("t1", GRAPH)]).unwrap();
        let lines: Vec<OracleInput> = read_jsonl(&out).unwrap();
        assert_eq!(
            lines.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
            ["t2", "t1"]
        );
        assert!(lines[0].text.starts_with("flowchart"), "{}", lines[0].text);
        assert!(lines[0].expected.is_none());
        assert!(!out.contains("expected"));
    }

    #[test]
    fn reference_inputs_names_the_invalid_task() {
        let bad = "graph {\n    node a: \"A\"\n    a -> b\n}\n";
        let err = reference_inputs(&[task("t1", GRAPH), task("bad7", bad)]).unwrap_err();
        assert!(err.to_string().contains("bad7"), "{err}");
    }

    #[test]
    fn manifest_rule_is_checked() {
        let ok = "corpus_size: 3\ndecision_rule:\n  primary_endpoint: first_pass_validity_slot_a\n  interval: newcombe_method_10\n  confidence: 0.95\n  margin: 0.10\n  max_repair_turns: 2\n  max_infra_rate: 0.05\n";
        let m = parse_manifest(ok).unwrap();
        assert_eq!(m.corpus_size, 3);
        assert_eq!(m.decision_rule.margin, 0.10);
        assert!(parse_manifest(&ok.replace("newcombe_method_10", "wald")).is_err());
        assert!(parse_manifest(&ok.replace("max_repair_turns: 2", "max_repair_turns: 3")).is_err());
    }

    #[test]
    fn jsonl_round_trips_and_names_bad_lines() {
        let line = Line::Stop(Stop {
            reason: "cap".into(),
            spent_usd: 1.5,
        });
        let text = write_jsonl(&[line.clone()]).unwrap();
        assert!(text.starts_with("{\"type\":\"stop\""), "{text}");
        assert_eq!(
            read_jsonl::<Line>(&format!("\n{text}")).unwrap(),
            vec![line]
        );
        let err =
            read_jsonl::<Line>("{\"type\":\"stop\",\"reason\":\"x\",\"spent_usd\":0}\n{bad\n")
                .unwrap_err();
        assert!(format!("{err:#}").contains("line 2"), "{err:#}");
    }

    #[test]
    fn oracle_output_reads_match_key() {
        let o: OracleOutput =
            serde_json::from_str(r#"{"id":"a","ok":true,"error":null,"sets":null,"match":true}"#)
                .unwrap();
        assert_eq!(o.matched, Some(true));
    }
}
