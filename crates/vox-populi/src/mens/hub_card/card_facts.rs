//! The one input the model-card templates read (spec decision D4).
//!
//! [`card_facts`] turns already-loaded [`RunRecords`] into typed facts and formatted numbers.
//! It reads no file and writes no sentence: every word a card says lives in `templates/hub/`.

use serde_json::Value;
use vox_eval::reasoning_bench::SLICE_NAMES;

use super::config::hub_cards;
use super::examples::{Example, ExampleFacts, fence_for, pick_examples};
use super::facts::{
    AdapterEvidence, GateFact, GateState, active_spoke_id, adapter_evidence, gate_of,
};
use super::{CardContext, PROMPT_PROTOCOL, RunRecords, heldout_scores, repo_name};

/// Shared explanation page every card links into.
pub(super) const DOCS: &str = "https://voxlang.org/reference/mens-models/";

/// The user turn the held-out eval sends, up to the item description. Protocol data, not card
/// wording: a test ties it to `eval_local_prompt.rs` in vox-ml-cli.
pub(crate) const EVAL_USER_TURN_PREFIX: &str = "Write a Vox program that: ";

/// Everything a model-card template shows. No prose fields.
#[derive(Debug, Clone)]
pub(crate) struct CardFacts {
    pub size: String,
    #[allow(dead_code)]
    pub model_label: String,
    pub model_family: String,
    pub uses_no_think_switch: bool,
    pub repo: String,
    pub namespace: String,
    pub repo_id: String,
    pub playground_space_id: String,
    pub local_run_dir: String,
    pub base_repo: String,
    pub base_rev: String,
    pub base_pin: String,
    pub license: String,
    pub protocol: &'static str,
    pub trained: bool,
    pub has_peft_adapter: bool,
    #[allow(dead_code)]
    pub has_merged_causal_lm: bool,
    pub inference_provider_ready: bool,
    pub has_merged_model: bool,
    pub merged_files: Vec<String>,
    pub spoke: Option<SpokeFact>,
    pub gate: GateFact,
    pub adapter_evidence: AdapterEvidence,
    /// A held-out adapter report is on disk (whether or not its evidence is current).
    pub adapter_report: bool,
    pub has_receipt: bool,
    /// The per-size sentences from `hub-cards.yaml`, or the config error text.
    pub why: Vec<String>,
    pub why_is_error: bool,
    /// Measured training peak in MiB and the record that named it.
    pub peak: Option<(u64, String)>,
    pub heldout: Option<HeldOut>,
    /// D2 examples; `Some` only for a trained size whose adapter evidence is `Current` (checker B1).
    pub examples: Option<ExampleFacts>,
    /// The chat request of the "Use it" example, as JSON: the eval's user turn for
    /// `examples.use_it`, lane `codegen`, and the report's decode settings when recorded.
    /// `Some` only on a `Passed` gate with `Current` adapter evidence and recorded adapter code.
    pub request_json: Option<String>,
    /// The adapter report records both `max_tokens` and `temperature`.
    pub decode_recorded: bool,
    /// Per-slice rows; filled only when the gate is `Passed`.
    pub collateral: Vec<SliceRow>,
    /// The paired-test alpha of the collateral report; filled only when the gate is `Passed`.
    pub alpha: Option<f64>,
    pub training: TrainingFacts,
    pub docs: &'static str,
    #[allow(dead_code)]
    pub train_lanes: Vec<String>,
    #[allow(dead_code)]
    pub spokes: Vec<String>,
    pub datasets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpokeFact {
    pub id: String,
    pub label: String,
}

/// Held-out pass@1. The adapter fields are `Some` only for `AdapterEvidence::Current`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HeldOut {
    pub total: u64,
    pub base_passed: Option<u64>,
    pub base_rate: Option<f64>,
    pub adapter_passed: Option<u64>,
    pub adapter_rate: Option<f64>,
}

/// One reasoning slice of a current collateral report, copied as stored.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SliceRow {
    pub bench: String,
    pub n: u64,
    pub base_acc: f64,
    pub adapter_acc: f64,
    pub p_value: f64,
}

/// Training numbers as display strings; `None` when the record lacks them.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TrainingFacts {
    pub optimizer_steps: Option<String>,
    pub total_tokens: Option<String>,
    pub final_loss_ema: Option<String>,
    pub avg_train_loss: Option<String>,
    pub avg_val_loss_last_epoch: Option<String>,
    pub wall: Option<String>,
    pub git_sha: Option<String>,
}

fn cell(passed: Option<u64>, rate: Option<f64>, total: u64) -> Option<String> {
    Some(format!("{}/{total} ({:.1}%)", passed?, rate? * 100.0))
}

impl HeldOut {
    pub fn base_cell(&self) -> String {
        cell(self.base_passed, self.base_rate, self.total)
            .unwrap_or_else(|| "not recorded".to_string())
    }

    /// A dash unless the adapter evidence is current (checker B1).
    pub fn adapter_cell(&self) -> String {
        cell(self.adapter_passed, self.adapter_rate, self.total).unwrap_or_else(|| "—".to_string())
    }
}

impl CardFacts {
    pub fn is_passed(&self) -> bool {
        self.gate.state == GateState::Passed
    }

    /// True only when the release gate passed AND a complete merged checkpoint exists.
    pub fn is_complete_model(&self) -> bool {
        self.is_passed() && self.has_merged_model
    }

    /// Plain-English delegation role headline for the card H1.
    pub fn role_headline(&self) -> &'static str {
        match self.size.as_str() {
            "0.8B" => "Smallest Open Model for Trying & Writing Vox Locally",
            "2B" => "Lightweight Laptop Model for Vox Functions, Tables & Tests",
            "4B" => "Everyday Local Model for Full-Stack Vox Files",
            "9B" => "Desktop & Server Model for Multi-Step Vox Workflows",
            "27B" => "Highest-Accuracy Private Model for Writing & Fixing Vox",
            _ => "Open Model for Writing & Fixing Vox Code",
        }
    }

    /// Non-redundant caption explaining the 3-stage architectural diagram in `assets/banner.jpg`.
    pub fn figure_caption(&self) -> String {
        match self.size.as_str() {
            "0.8B" => "Diagram (0.8B): a short prompt runs in the browser or on an everyday laptop CPU or GPU offline and returns compiler-checked `.vox` code.".to_string(),
            "2B" => "Diagram (2B): a task prompt runs locally on Windows, Linux, or Mac with low memory overhead to produce a working `.vox` function and `@test` block.".to_string(),
            "4B" => "Diagram (4B): one `.vox` file combines a database `table`, backend `query` and `mutation` handlers, and a web UI `component` without cross-language glue.".to_string(),
            "9B" => "Diagram (9B): multi-step `state_machine` and `workflow` code cycles through `vox check` error feedback on a desktop or server GPU until types hold.".to_string(),
            "27B" => "Diagram (27B): a large cloud model handles high-level app design upstream while the private 27B model writes and fixes `.vox` code via Ollama, vLLM, Python, or the Vox CLI.".to_string(),
            other => format!("Diagram ({other}): a developer prompt passes through the Vox Mens model and the `vox check` syntax and type check."),
        }
    }

    /// Hugging Face Hub family tag matching [`Self::model_family`].
    pub fn family_tag(&self) -> &'static str {
        match self.model_family.as_str() {
            "qwen3.5" => "qwen3-5",
            "qwen3.8" => "qwen3-8",
            _ => "qwen3",
        }
    }

    pub fn merged_files_text(&self) -> String {
        if self.merged_files.is_empty() {
            "model.safetensors".to_string()
        } else {
            self.merged_files.join("`, `")
        }
    }

    /// Whether the frontmatter has any `model-index` metric to carry.
    pub fn has_model_index(&self) -> bool {
        self.heldout
            .as_ref()
            .is_some_and(|h| h.adapter_rate.is_some())
            || !self.collateral.is_empty()
    }

    pub fn peak_text(&self) -> String {
        self.peak
            .as_ref()
            .map_or_else(|| "not recorded".to_string(), |(mb, _)| format!("{mb} MiB"))
    }

    pub fn base_cell(&self) -> String {
        self.heldout
            .as_ref()
            .map_or_else(|| "not recorded".to_string(), HeldOut::base_cell)
    }

    /// The "Use it" example, present exactly when its request is.
    pub fn use_it(&self) -> Option<&Example> {
        self.request_json.as_ref()?;
        self.examples.as_ref()?.use_it.as_ref()
    }

    /// Fence for the request block, longer than any backtick run in it.
    pub fn request_fence(&self) -> String {
        fence_for(self.request_json.as_deref().unwrap_or(""))
    }

    /// A dash unless the adapter evidence is current (checker B1).
    pub fn adapter_cell(&self) -> String {
        self.heldout
            .as_ref()
            .map_or_else(|| "—".to_string(), HeldOut::adapter_cell)
    }
}

fn text(v: &Value) -> Option<String> {
    match v {
        Value::String(x) => Some(x.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn loss(rec: &Option<Value>, key: &str) -> Option<String> {
    Some(format!("{:.4}", rec.as_ref()?[key].as_f64()?))
}

fn hours(secs: f64) -> String {
    let m = (secs / 60.0).round() as u64;
    format!("{}h {:02}m", m / 60, m % 60)
}

fn training_facts(rec: &RunRecords) -> TrainingFacts {
    let get = |r: &Option<Value>, k: &str| r.as_ref().and_then(|v| text(&v[k]));
    TrainingFacts {
        optimizer_steps: get(&rec.train_record, "optimizer_steps")
            .or_else(|| get(&rec.run_summary, "optimizer_step_count")),
        total_tokens: get(&rec.run_summary, "total_tokens"),
        final_loss_ema: loss(&rec.train_record, "final_loss_ema")
            .or_else(|| loss(&rec.run_summary, "final_loss_ema")),
        avg_train_loss: loss(&rec.train_record, "avg_train_loss")
            .or_else(|| loss(&rec.run_summary, "avg_train_loss")),
        avg_val_loss_last_epoch: loss(&rec.run_summary, "avg_val_loss_last_epoch"),
        wall: rec
            .train_record
            .as_ref()
            .and_then(|t| t["train_wall_secs"].as_f64())
            .or_else(|| {
                rec.run_summary
                    .as_ref()
                    .and_then(|s| s["wall_secs"].as_f64())
            })
            .or_else(|| {
                rec.train_record
                    .as_ref()
                    .and_then(|t| t["total_wall_secs"].as_f64())
                    .filter(|&w| w > 0.0)
            })
            .map(hours),
        git_sha: get(&rec.training_manifest, "git_sha"),
    }
}

/// Slice rows and alpha from a collateral report. Only called for a `Passed` gate, so the
/// report already satisfied `check_collateral_report`; values are copied, never recomputed.
fn slice_rows(report: &Value) -> (Vec<SliceRow>, Option<f64>) {
    let block = &report["reasoning_slices"];
    let rows = SLICE_NAMES
        .iter()
        .filter_map(|&name| {
            let m = &block["benches"][name]["accuracy"];
            Some(SliceRow {
                bench: name.to_string(),
                n: m["n"].as_u64()?,
                base_acc: m["pre"].as_f64()?,
                adapter_acc: m["post"].as_f64()?,
                p_value: m["p_value"].as_f64()?,
            })
        })
        .collect();
    (rows, block["alpha"].as_f64())
}

pub(super) fn held_out(rec: &RunRecords, evidence: &AdapterEvidence) -> Option<HeldOut> {
    let (base, adapter) = heldout_scores(rec);
    // The adapter score counts only when its checkpoint records the current serving path.
    let adapter = adapter.filter(|_| *evidence == AdapterEvidence::Current);
    // ponytail: one shared total, taken from the base report; both sides run the same bench.
    let total = base.or(adapter)?.total;
    let passed = |s: Option<super::Score>| s.map(|s| s.passed.round() as u64);
    Some(HeldOut {
        total,
        base_passed: passed(base),
        base_rate: base.map(|s| s.rate),
        adapter_passed: passed(adapter),
        adapter_rate: adapter.map(|s| s.rate),
    })
}

/// Whether the adapter report records the decode settings of its run.
fn decode_recorded(adapter: Option<&Value>) -> bool {
    adapter.is_some_and(|r| r["max_tokens"].is_u64() && r["temperature"].is_number())
}

/// The chat request that reproduces `item`'s eval prompt through `vox mens serve`'s codegen lane.
/// Decode settings are copied from the report when it records them, never assumed.
fn request_json(adapter: Option<&Value>, item: &Example) -> Option<String> {
    item.adapter.code.as_ref()?;
    let mut req = serde_json::json!({
        "lane": "codegen",
        "messages": [{
            "role": "user",
            "content": format!("{EVAL_USER_TURN_PREFIX}{}", item.description),
        }],
    });
    let report = adapter?;
    if let Some(n) = report["max_tokens"].as_u64() {
        req["max_tokens"] = n.into();
    }
    if let Some(t) = report["temperature"].as_f64() {
        req["temperature"] = t.into();
    }
    serde_json::to_string_pretty(&req).ok()
}

/// Build the facts for one size card from its loaded records.
pub(crate) fn card_facts(cx: &CardContext, rec: &RunRecords) -> CardFacts {
    let (base_repo, base_rev) = cx.base_pin.split_once('@').unwrap_or((cx.base_pin, ""));
    let gate = gate_of(rec);
    let evidence = adapter_evidence(rec);
    let cfg = hub_cards();
    let spoke = match (&cfg, gate.trained) {
        (Ok(c), true) => {
            let sid = active_spoke_id(rec, Some(c.default_spoke.as_str()));
            c.spokes
                .iter()
                .find(|s| Some(s.id.as_str()) == sid.as_deref())
                .or_else(|| c.spokes.iter().find(|s| s.id == c.default_spoke))
                .map(|s| SpokeFact {
                    id: s.id.clone(),
                    label: s.label.clone(),
                })
        }
        _ => None,
    };
    let (why, why_is_error) = match &cfg {
        Ok(c) => match c.sizes.get(cx.size) {
            Some(e) => (e.why.clone(), false),
            None => (
                vec![format!("hub-cards.yaml has no size `{}`", cx.size)],
                true,
            ),
        },
        Err(e) => (vec![e.clone()], true),
    };
    let peak = rec
        .train_record
        .as_ref()
        .filter(|_| gate.trained)
        .and_then(|t| {
            Some((
                t["peak_mb"].as_u64()?,
                t["peak_source"]
                    .as_str()
                    .unwrap_or("not recorded")
                    .to_string(),
            ))
        });
    let (collateral, alpha) = match (&rec.collateral, gate.state) {
        (Some(c), GateState::Passed) => slice_rows(c),
        _ => (Vec::new(), None),
    };
    let examples = (gate.trained && evidence == AdapterEvidence::Current)
        .then(|| {
            pick_examples(
                rec.heldout_base.as_ref()?,
                rec.heldout_adapter.as_ref()?,
                rec.heldout_base_ckpt.as_deref(),
                rec.heldout_adapter_ckpt.as_deref(),
            )
        })
        .flatten();
    let request_json = (gate.state == GateState::Passed)
        .then(|| {
            let item = examples.as_ref()?.use_it.as_ref()?;
            request_json(rec.heldout_adapter.as_ref(), item)
        })
        .flatten();
    let pin_or_size = if crate::mens::hub_repo::size_of_pin(cx.base_pin).is_some() {
        cx.base_pin
    } else {
        cx.size
    };
    let model_label = crate::mens::hub_repo::model_label_for(pin_or_size);
    let model_family = crate::mens::hub_repo::family_of_model(pin_or_size).to_string();
    let uses_no_think_switch = model_family == "qwen3";
    let repo = crate::mens::hub_repo::repo_slug_for_model(pin_or_size)
        .unwrap_or_else(|_| repo_name(cx.size));
    let repo_id = crate::mens::hub_repo::repo_id_for_size(cx.namespace, pin_or_size)
        .unwrap_or_else(|_| format!("{}/{repo}", cx.namespace));
    let playground_space_id = crate::mens::hub_repo::space_repo_id(cx.namespace)
        .unwrap_or_else(|_| format!("{}/vox-mens-playground", cx.namespace));
    let local_run_dir = crate::mens::hub_repo::local_run_dir_for_size(pin_or_size)
        .unwrap_or_else(|_| format!("mens/runs/qwen3-{}", cx.size.to_ascii_lowercase()));
    let has_peft_adapter = rec.has_adapter;
    let has_merged_causal_lm = rec.has_complete_merged_model || !rec.merged_model_files.is_empty();
    let inference_provider_ready = gate.state == GateState::Passed && has_merged_causal_lm;
    let has_merged_model = has_merged_causal_lm && gate.state == GateState::Passed;
    let train_lanes: Vec<String> = rec
        .adapter_card
        .as_ref()
        .and_then(|c| c.get("train_lanes"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    let spokes: Vec<String> = rec
        .adapter_card
        .as_ref()
        .and_then(|c| c.get("spokes"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    let datasets = cite_datasets(cx.namespace, spoke.as_ref().map(|s| s.id.as_str()));
    CardFacts {
        size: cx.size.to_string(),
        model_label,
        model_family,
        uses_no_think_switch,
        repo_id,
        repo,
        namespace: cx.namespace.to_string(),
        playground_space_id,
        local_run_dir,
        base_repo: base_repo.to_string(),
        base_rev: base_rev.to_string(),
        base_pin: cx.base_pin.to_string(),
        license: rec
            .training_manifest
            .as_ref()
            .and_then(|m| m["provenance_license_class"].as_str())
            .unwrap_or("apache-2.0")
            .to_string(),
        protocol: PROMPT_PROTOCOL,
        trained: gate.trained,
        has_peft_adapter,
        has_merged_causal_lm,
        inference_provider_ready,
        has_merged_model,
        merged_files: rec.merged_model_files.clone(),
        spoke,
        adapter_report: rec.heldout_adapter.is_some(),
        has_receipt: rec.gate_receipt.is_some(),
        why,
        why_is_error,
        peak,
        heldout: held_out(rec, &evidence).filter(|_| gate.trained),
        examples,
        request_json,
        decode_recorded: decode_recorded(rec.heldout_adapter.as_ref()),
        collateral,
        alpha,
        training: training_facts(rec),
        docs: DOCS,
        train_lanes,
        spokes,
        datasets,
        adapter_evidence: evidence,
        gate,
    }
}

/// Canonical Hugging Face dataset repo citations for a card (`338-09`).
pub(crate) fn cite_datasets(namespace: &str, spoke: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let eval_suite = match spoke {
        Some("rust") => "rust_bench",
        Some("agents" | "tool-selection" | "argument-generation" | "harness") => "agent_episodes",
        _ => "vox_lang_bench",
    };
    if let Ok(repo) = crate::mens::hub_repo::eval_dataset_repo_id(namespace, eval_suite) {
        out.push(repo);
    }
    if let Some(sp) = spoke
        && let Ok(repo) = crate::mens::hub_repo::data_dataset_repo_id(namespace, sp)
    {
        out.push(repo);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mens::gate_policy::passing_collateral_report;
    use crate::mens::hub_card::facts::load_requirement;
    use serde_json::json;

    const PIN: &str = "Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"; // gitleaks:allow

    fn cx() -> CardContext<'static> {
        CardContext {
            size: "4B",
            base_pin: PIN,
            namespace: "ns",
        }
    }

    fn current_gate_run() -> RunRecords {
        let req = load_requirement(None).unwrap();
        RunRecords {
            has_adapter: true,
            train_record: Some(json!({"status": "trained", "peak_mb": 34460})),
            gate_receipt: Some(json!({
                "overall_passed": true,
                "gates": [
                    {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                    {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                ]
            })),
            collateral: Some(passing_collateral_report(&req)),
            collateral_req: Some(req),
            ..Default::default()
        }
    }

    #[test]
    fn slices_and_alpha_are_filled_only_for_a_passed_gate() {
        let rec = current_gate_run();
        let f = card_facts(&cx(), &rec);
        assert_eq!(f.gate.state, GateState::Passed);
        let names: Vec<&str> = f.collateral.iter().map(|r| r.bench.as_str()).collect();
        assert_eq!(names, SLICE_NAMES);
        let req = load_requirement(None).unwrap();
        assert_eq!(f.alpha, Some(req.alpha));
        assert!(f.collateral.iter().all(|r| r.n > 0));
        assert_eq!(f.peak, Some((34460, "not recorded".to_string())));

        // The same report with its rule removed is stale: no slice row, no alpha.
        let mut stale = rec.clone();
        stale.collateral.as_mut().unwrap()["reasoning_slices"]["rule"] = json!("retired");
        let f = card_facts(&cx(), &stale);
        assert_eq!(f.gate.state, GateState::NeedsReGate);
        assert!(f.collateral.is_empty() && f.alpha.is_none());
    }

    #[test]
    fn untrained_run_has_no_spoke_peak_or_scores() {
        let rec = RunRecords {
            train_record: Some(json!({"status": "not_trained", "peak_mb": 1})),
            ..Default::default()
        };
        let f = card_facts(&cx(), &rec);
        assert!(!f.trained && f.spoke.is_none() && f.peak.is_none() && f.heldout.is_none());
        assert_eq!(f.base_cell(), "not recorded");
        assert_eq!(f.adapter_cell(), "—");
        assert!(!f.has_model_index());
        assert_eq!(f.why.len() >= 2, !f.why_is_error);
    }

    #[test]
    fn adapter_score_needs_current_serving() {
        let mut rec = current_gate_run();
        rec.heldout_base = Some(json!({"pass_rate_at_1": 0.25, "passed_at_1": 2.0, "total": 8}));
        rec.heldout_adapter = Some(json!({"pass_rate_at_1": 0.5, "passed_at_1": 4.0, "total": 8}));
        rec.heldout_adapter_ckpt = Some(vec!["x".to_string()]);
        rec.adapter_serving = Some("old".to_string());
        let f = card_facts(&cx(), &rec);
        assert_eq!(f.base_cell(), "2/8 (25.0%)");
        assert_eq!(f.adapter_cell(), "—");
        assert!(f.adapter_report && f.adapter_evidence != AdapterEvidence::Current);

        rec.adapter_serving = Some(super::super::CURRENT_ADAPTER_SERVING.to_string());
        let f = card_facts(&cx(), &rec);
        assert_eq!(f.adapter_cell(), "4/8 (50.0%)");
        assert!(f.has_model_index());
    }

    #[test]
    fn cite_datasets_prefers_spoke_and_eval_suite() {
        assert_eq!(
            cite_datasets("ns", Some("rust")),
            vec![
                "ns/vox-mens-eval-rust-bench".to_string(),
                "ns/vox-mens-data-rust".to_string(),
            ]
        );
        assert_eq!(
            cite_datasets("ns", Some("agents")),
            vec![
                "ns/vox-mens-eval-agent-episodes".to_string(),
                "ns/vox-mens-data-agents".to_string(),
            ]
        );
        assert_eq!(
            cite_datasets("ns", Some("vox-lang")),
            vec![
                "ns/vox-mens-eval-vox-lang-bench".to_string(),
                "ns/vox-mens-data-vox-lang".to_string(),
            ]
        );

        let mut rec = current_gate_run();
        rec.adapter_card = Some(json!({
            "train_lanes": ["rust_authoring"],
            "spokes": ["rust"],
        }));
        let f = card_facts(&cx(), &rec);
        assert_eq!(f.train_lanes, vec!["rust_authoring".to_string()]);
        assert_eq!(f.spokes, vec!["rust".to_string()]);
        assert_eq!(
            f.datasets,
            cite_datasets("ns", f.spoke.as_ref().map(|s| s.id.as_str()))
        );
    }

    #[test]
    fn family_fields_and_serving_flags_track_pin_and_checkpoint_state() {
        let mut rec = current_gate_run();
        let f_35 = card_facts(&cx(), &rec);
        assert_eq!(f_35.model_label, "Qwen3.5-4B");
        assert_eq!(f_35.model_family, "qwen3.5");
        assert_eq!(f_35.family_tag(), "qwen3-5");
        assert!(!f_35.uses_no_think_switch);
        assert_eq!(f_35.repo, "vox-mens-qwen3.5-4b");
        assert_eq!(f_35.repo_id, "ns/vox-mens-qwen3.5-4b");
        assert_eq!(f_35.local_run_dir, "mens/runs/qwen3.5-4b");
        assert_eq!(f_35.playground_space_id, "ns/vox-mens-playground");
        assert!(f_35.has_peft_adapter);
        assert!(!f_35.has_merged_causal_lm);
        assert!(!f_35.inference_provider_ready);

        rec.has_complete_merged_model = true;
        let f_merged = card_facts(&cx(), &rec);
        assert!(f_merged.has_merged_causal_lm);
        assert!(f_merged.inference_provider_ready);

        let f_legacy = card_facts(
            &CardContext {
                size: "4B",
                base_pin: "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c", // gitleaks:allow
                namespace: "ns",
            },
            &rec,
        );
        assert_eq!(f_legacy.model_label, "Qwen3-4B");
        assert_eq!(f_legacy.model_family, "qwen3");
        assert_eq!(f_legacy.family_tag(), "qwen3");
        assert!(f_legacy.uses_no_think_switch);
        assert_eq!(f_legacy.repo, "vox-mens-qwen3-4b");
        assert_eq!(f_legacy.local_run_dir, "mens/runs/qwen3-4b");

        let f_38 = card_facts(
            &CardContext {
                size: "27B",
                base_pin: "Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0", // gitleaks:allow
                namespace: "ns",
            },
            &rec,
        );
        assert_eq!(f_38.model_label, "Qwen3.8-27B");
        assert_eq!(f_38.model_family, "qwen3.8");
        assert_eq!(f_38.family_tag(), "qwen3-8");
        assert!(!f_38.uses_no_think_switch);
        assert_eq!(f_38.repo, "vox-mens-qwen3.8-27b");
        assert_eq!(f_38.local_run_dir, "mens/runs/qwen3.8-27b");
    }
}
