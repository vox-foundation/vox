//! Records a card is rendered from, loaded once per run directory.
//!
//! No wording lives here: this module reads files and exposes typed facts. Every word a
//! card shows about them lives in `templates/hub/` (spec decision D4).

use std::path::Path;

use serde_json::Value;
use vox_eval::reasoning_bench::SLICE_SIZE;

use vox_eval::collateral_evidence::{
    CollateralRefusal, CollateralRequirement, check_collateral_report,
};

/// The only held-out prompt protocol whose scores a card shows.
pub const PROMPT_PROTOCOL: &str = "qwen3_non_thinking_codegen_v1";

/// How the adapter was served when `vox mens eval` produced the adapter checkpoint (phase 335).
///
/// Mirrors vox-ml-cli `item_checkpoint::ADAPTER_SERVING`; 339-04's parity test fails on drift.
/// Adapter outputs and scores recorded under any other serving path are not shown on a card.
/// This is the only place in vox-populi the value is written.
pub const CURRENT_ADAPTER_SERVING: &str = "nf4_base_plus_lora_arm v2";

/// Held-out reports scored before [`PROMPT_PROTOCOL`] (think128). Listed as invalid, never scored.
const LEGACY_EVIDENCE: &[&str] = &[
    "evidence/heldout_base.json",
    "evidence/heldout_adapter.json",
    "evidence/benchmark_passatk.think128.json",
];

/// Records a card is rendered from. Each field is the parsed file, or `None` when absent.
#[derive(Debug, Clone, Default)]
pub struct RunRecords {
    pub has_adapter: bool,
    /// Relative paths of complete merged checkpoint files (`vox mens merge-qlora`) when present.
    pub merged_model_files: Vec<String>,
    /// Whether a complete merged checkpoint (`recombined_full/model.safetensors`, sharded index, or merged safetensors) exists.
    pub has_complete_merged_model: bool,
    pub train_record: Option<Value>,
    pub run_summary: Option<Value>,
    pub training_manifest: Option<Value>,
    pub adapter_manifest: Option<Value>,
    pub adapter_card: Option<Value>,
    pub gate_receipt: Option<Value>,
    pub collateral: Option<Value>,
    pub heldout_base: Option<Value>,
    pub heldout_adapter: Option<Value>,
    pub compare: Option<Value>,
    /// `generated_text` of each item line of the held-out checkpoint, in file order.
    pub heldout_base_ckpt: Option<Vec<String>>,
    pub heldout_adapter_ckpt: Option<Vec<String>>,
    /// `inputs.model.adapter_serving` of the adapter checkpoint header; `None` when it has none.
    pub adapter_serving: Option<String>,
    /// Held-out reports on disk that use a retired protocol. Named on the card, never scored.
    pub invalid_evidence: Vec<String>,
    /// The collateral rule current reports are held to; `None` when the policy could not be read.
    pub collateral_req: Option<CollateralRequirement>,
    /// Telemetry loss points `(step, loss)` downsampled to at most 64 points (EMA-smoothed).
    pub telemetry_loss: Vec<(u32, f32)>,
    /// Raw micro-step loss points `(step, loss)` downsampled to at most 64 points.
    pub telemetry_raw: Vec<(u32, f32)>,
    /// Per-spoke gate rows evaluated for this run directory.
    pub spoke_gates: Vec<vox_eval::gate_state::SpokeGateRow>,
}

fn read_json(path: &Path) -> Option<Value> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    match &v {
        Value::Object(m) if m.is_empty() => None,
        Value::Array(a) if a.is_empty() => None,
        Value::Null => None,
        _ => Some(v),
    }
}

fn protocol_ok(v: &Value) -> bool {
    v["prompt_protocol"].as_str() == Some(PROMPT_PROTOCOL)
}

/// Item texts and the serving field of an eval checkpoint. `None` unless the header names the
/// item-checkpoint schema and the current prompt protocol and at least one item parses.
/// Untrusted input: malformed lines are skipped.
fn read_ckpt(path: &Path) -> Option<(Vec<String>, Option<String>)> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    let header: Value = serde_json::from_str(lines.next()?).ok()?;
    if header["schema"].as_str() != Some("vox_eval_item_checkpoint.v1")
        || header["inputs"]["prompt_protocol"].as_str() != Some(PROMPT_PROTOCOL)
    {
        return None;
    }
    let texts: Vec<String> = lines
        .filter_map(|l| {
            let item: Value = serde_json::from_str(l).ok()?;
            let resp: Value = serde_json::from_str(item["response"].as_str()?).ok()?;
            Some(resp["generated_text"].as_str()?.to_string())
        })
        .collect();
    let serving = header["inputs"]["model"]["adapter_serving"]
        .as_str()
        .map(str::to_string);
    (!texts.is_empty()).then_some((texts, serving))
}

/// Read and downsample telemetry loss curve from `telemetry.jsonl`.
/// Scoped to the active run when `target_run_id` matches a `train_start` payload `run_id`,
/// or to the final segment in the file so retries never mix curves.
/// Returns `(ema_smoothed_pts, raw_pts)`.
fn read_telemetry_loss(
    path: &Path,
    target_run_id: Option<&str>,
) -> (Vec<(u32, f32)>, Vec<(u32, f32)>) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return (Vec::new(), Vec::new());
    };
    let mut segments: Vec<(Option<String>, Vec<(u32, f32)>)> = vec![(None, Vec::new())];
    for line in content.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let event = v["event"].as_str();
        if event == Some("train_start") {
            let run_id = v["payload"]["run_id"].as_str().map(str::to_string);
            segments.push((run_id, Vec::new()));
            continue;
        }
        if event == Some("train_step") || event == Some("step") {
            let payload = &v["payload"];
            let step = payload["step"]
                .as_u64()
                .or_else(|| payload["optimizer_step"].as_u64());
            let loss = payload["loss"].as_f64();
            if let (Some(s), Some(l)) = (step, loss)
                && l.is_finite()
                && let Some((_, pts)) = segments.last_mut()
            {
                pts.push((s as u32, l as f32));
            }
        }
    }
    let mut raw_pts = target_run_id
        .and_then(|want| {
            segments
                .iter()
                .rev()
                .find(|(id, pts)| id.as_deref() == Some(want) && !pts.is_empty())
                .map(|(_, pts)| pts.clone())
        })
        .or_else(|| {
            segments
                .into_iter()
                .rev()
                .find(|(_, pts)| !pts.is_empty())
                .map(|(_, pts)| pts)
        })
        .unwrap_or_default();
    if raw_pts.is_empty() {
        return (Vec::new(), Vec::new());
    }
    raw_pts.sort_by_key(|&(s, _)| s);

    // Compute Exponential Moving Average (alpha = 0.06) over chronological sequence
    let mut ema = 0.0f32;
    let mut ema_pts = Vec::with_capacity(raw_pts.len());
    for (i, &(s, l)) in raw_pts.iter().enumerate() {
        if i == 0 {
            ema = l;
        } else {
            ema = 0.94 * ema + 0.06 * l;
        }
        ema_pts.push((s, ema));
    }

    let n = raw_pts.len();
    if n <= 64 {
        return (ema_pts, raw_pts);
    }
    let mut sampled_ema = Vec::with_capacity(64);
    let mut sampled_raw = Vec::with_capacity(64);
    sampled_ema.push(ema_pts[0]);
    sampled_raw.push(raw_pts[0]);
    let mut last_idx = 0;
    for i in 1..63 {
        let ideal = (i as f64 * (n - 1) as f64 / 63.0).round() as usize;
        let idx = ideal.clamp(last_idx + 1, n - (64 - i));
        sampled_ema.push(ema_pts[idx]);
        sampled_raw.push(raw_pts[idx]);
        last_idx = idx;
    }
    sampled_ema.push(ema_pts[n - 1]);
    sampled_raw.push(raw_pts[n - 1]);
    (sampled_ema, sampled_raw)
}

/// Where `vox mens eval-reasoning-compare` writes its result for this base revision.
pub fn compare_json_path(run_dir: &Path, size: &str, base_pin: &str) -> Option<std::path::PathBuf> {
    let staged = run_dir.join("evidence/reasoning_compare.json");
    if staged.is_file() {
        return Some(staged);
    }
    let (repo, rev) = base_pin.split_once('@')?;
    let rev12 = rev.get(..12)?;
    let compare_dir = run_dir.parent()?.join("reasoning_compare");
    if let Some(repo_short) = repo.rsplit('/').next().filter(|s| !s.is_empty()) {
        let by_repo = compare_dir.join(format!("{repo_short}-{rev12}.json"));
        if by_repo.is_file() {
            return Some(by_repo);
        }
    }
    let model_label = crate::mens::hub_repo::model_label_for(if base_pin.trim().is_empty() {
        size
    } else {
        base_pin
    });
    let by_label = compare_dir.join(format!("{model_label}-{rev12}.json"));
    if by_label.is_file() {
        return Some(by_label);
    }
    let legacy = compare_dir.join(format!("Qwen3-{size}-{rev12}.json"));
    legacy.is_file().then_some(legacy)
}

/// Detect complete merged checkpoint files created by `vox mens merge-qlora` in `run_dir`.
///
/// Prefers `recombined_full/` (full SafeTensors + tokenizer + config written with `--keep-merged`),
/// or standalone `merged_model.safetensors` / `merged.safetensors` / `model.safetensors` / sharded
/// `model-*-of-*.safetensors` in `run_dir` root. Never treats training adapter checkpoints
/// (`candle_qlora_adapter.safetensors`, `checkpoint_*.safetensors`) as complete merged models.
pub fn detect_merged_checkpoint_files(run_dir: &Path) -> Vec<(std::path::PathBuf, String)> {
    const CONFIG_AND_TOKENIZER: &[&str] = &[
        "model.safetensors.index.json",
        "config.json",
        "tokenizer.json",
        "tokenizer_config.json",
        "generation_config.json",
        "special_tokens_map.json",
    ];
    let mut out = Vec::new();
    let recombined = run_dir.join("recombined_full");
    if recombined.is_dir()
        && let Ok(rd) = std::fs::read_dir(&recombined)
    {
        let mut weights = Vec::new();
        let mut extras = Vec::new();
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let Some(name) = p.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
                continue;
            };
            if name.ends_with(".safetensors") {
                weights.push((p, name));
            } else if CONFIG_AND_TOKENIZER.contains(&name.as_str()) {
                extras.push((p, name));
            }
        }
        if !weights.is_empty() {
            out.extend(weights);
            out.extend(extras);
            out.sort_by(|a, b| a.1.cmp(&b.1));
            return out;
        }
    }

    if let Ok(rd) = std::fs::read_dir(run_dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let Some(name) = p.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
                continue;
            };
            let is_merged_weight = matches!(
                name.as_str(),
                "merged_model.safetensors" | "merged.safetensors" | "model.safetensors"
            ) || (name.starts_with("model-")
                && name.ends_with(".safetensors")
                && name.contains("-of-"));
            if is_merged_weight {
                out.push((p, name));
            }
        }
    }
    if !out.is_empty() {
        for &extra in CONFIG_AND_TOKENIZER {
            let p = run_dir.join(extra);
            if p.is_file() {
                out.push((p, extra.to_string()));
            }
        }
        out.sort_by(|a, b| a.1.cmp(&b.1));
    }
    out
}

impl RunRecords {
    /// Read every record a card uses from `run_dir`. Unreadable or placeholder files count as absent.
    pub fn load(run_dir: &Path, size: &str, base_pin: &str) -> Self {
        let j = |rel: &str| read_json(&run_dir.join(rel));
        let mut invalid_evidence = Vec::new();
        let mut nothink = |rel: &str| {
            let v = j(rel)?;
            if protocol_ok(&v) {
                Some(v)
            } else {
                invalid_evidence.push(rel.to_string());
                None
            }
        };
        let heldout_base = nothink("evidence/heldout_base_nothink.json");
        let heldout_adapter = nothink("evidence/heldout_adapter_nothink.json");
        for rel in LEGACY_EVIDENCE {
            if j(rel).is_some_and(|v| !protocol_ok(&v)) {
                invalid_evidence.push((*rel).to_string());
            }
        }
        let heldout_base_ckpt =
            read_ckpt(&run_dir.join("evidence/heldout_base_nothink.ckpt.jsonl")).map(|(t, _)| t);
        let (heldout_adapter_ckpt, adapter_serving) =
            match read_ckpt(&run_dir.join("evidence/heldout_adapter_nothink.ckpt.jsonl")) {
                Some((t, s)) => (Some(t), s),
                None => (None, None),
            };
        // A compare file is keyed by base revision; only use it when it scored this rung's run dir.
        let legacy_rung = format!("qwen3-{}", size.to_ascii_lowercase());
        let family_rung =
            crate::mens::hub_repo::local_run_dir_for_size(if base_pin.trim().is_empty() {
                size
            } else {
                base_pin
            })
            .ok()
            .map(|d| d.trim_start_matches("mens/runs/").to_string());
        let run_dir_name = run_dir.file_name().and_then(|n| n.to_str());
        let compare = compare_json_path(run_dir, size, base_pin)
            .and_then(|p| read_json(&p))
            .filter(|c| {
                c["status"].as_str() != Some("compared")
                    || c["adapter"]["dir"].as_str().is_some_and(|d| {
                        let trimmed = d.trim_end_matches(['/', '\\']);
                        trimmed.ends_with(&legacy_rung)
                            || family_rung.as_deref().is_some_and(|r| trimmed.ends_with(r))
                            || run_dir_name
                                .is_some_and(|name| !name.is_empty() && trimmed.ends_with(name))
                    })
            });
        let run_summary = j("run_summary.json");
        let target_run_id = run_summary.as_ref().and_then(|s| s["run_id"].as_str());
        let (telemetry_loss, telemetry_raw) =
            read_telemetry_loss(&run_dir.join("telemetry.jsonl"), target_run_id);
        let root = vox_corpus::training::contract::find_workspace_root();
        let collateral_req = load_requirement(root.as_deref()).ok();
        let spoke_gates = crate::mens::gate_policy::spoke_gate_rows(run_dir, root.as_deref());
        let merged_model_files: Vec<String> = detect_merged_checkpoint_files(run_dir)
            .into_iter()
            .map(|(_, rel)| rel)
            .collect();
        let has_complete_merged_model = !merged_model_files.is_empty()
            || run_dir.join("recombined_full/model.safetensors").is_file()
            || run_dir
                .join("recombined_full/model.safetensors.index.json")
                .is_file()
            || run_dir.join("model.safetensors.index.json").is_file();
        Self {
            has_adapter: run_dir.join("candle_qlora_adapter.safetensors").is_file()
                || run_dir.join("adapter.safetensors").is_file(),
            merged_model_files,
            has_complete_merged_model,
            train_record: j("train-record.json"),
            run_summary,
            training_manifest: j("training_manifest.json"),
            adapter_manifest: j("adapter_manifest.json"),
            adapter_card: j("adapter_card.json"),
            gate_receipt: j("gate_receipt.json"),
            collateral: j("collateral_damage_report.json"),
            heldout_base,
            heldout_adapter,
            compare,
            heldout_base_ckpt,
            heldout_adapter_ckpt,
            adapter_serving,
            invalid_evidence,
            collateral_req,
            telemetry_loss,
            telemetry_raw,
            spoke_gates,
        }
    }
}

/// Held-out pass@1 for one side, from a `*_nothink` report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    pub rate: f64,
    pub passed: f64,
    pub total: u64,
}

fn score(v: &Option<Value>) -> Option<Score> {
    let v = v.as_ref()?;
    Some(Score {
        rate: v["pass_rate_at_1"].as_f64()?,
        passed: v["passed_at_1"].as_f64()?,
        total: v["total"].as_u64()?,
    })
}

/// `(base, adapter)` held-out pass@1 under [`PROMPT_PROTOCOL`], each `None` without its record.
pub fn heldout_scores(rec: &RunRecords) -> (Option<Score>, Option<Score>) {
    (score(&rec.heldout_base), score(&rec.heldout_adapter))
}

/// Whether the adapter's recorded evidence came from the serving path the code uses now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AdapterEvidence {
    Current,
    /// No usable adapter checkpoint; a report alone is never accepted.
    NoCheckpoint,
    /// Recorded under another serving path, or before the path was recorded.
    Stale {
        recorded: Option<String>,
    },
}

/// Classify the adapter held-out evidence. Only `Current` may feed a card.
pub(crate) fn adapter_evidence(rec: &RunRecords) -> AdapterEvidence {
    if rec.heldout_adapter_ckpt.is_none() {
        return AdapterEvidence::NoCheckpoint;
    }
    match rec.adapter_serving.as_deref() {
        Some(CURRENT_ADAPTER_SERVING) => AdapterEvidence::Current,
        other => AdapterEvidence::Stale {
            recorded: other.map(str::to_string),
        },
    }
}

/// Current collateral requirement: `<root>/mens/config/eval-gates-ladder.yaml` when the
/// root has it, else the embedded copy.
pub(super) fn load_requirement(
    workspace_root: Option<&Path>,
) -> Result<CollateralRequirement, String> {
    Ok(crate::mens::gate_policy::ladder_requirement(workspace_root))
}

/// Where a ladder size stands against its release gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Passed,
    /// A pass recorded under a rule, slice set or grader that is no longer current.
    NeedsReGate,
    Failed,
    NotEvaluated,
}
impl GateState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::NeedsReGate => "needs re-gate",
            Self::Failed => "failed",
            Self::NotEvaluated => "not evaluated",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::Passed => "gate-passed",
            Self::NeedsReGate => "gate-needs-re-gate",
            Self::Failed => "gate-failed",
            Self::NotEvaluated => "gate-not-evaluated",
        }
    }
}

/// Why a size is not `Passed`. Record values only; every word a card says lives in templates.
#[derive(Debug, Clone, PartialEq)]
pub enum Blocker {
    NoAdapter {
        record_reason: Option<String>,
    },
    NotPublished,
    /// Records (or gates) that have not been produced.
    Missing(Vec<String>),
    Stale {
        refusal: String,
        recorded_slice_n: Option<u64>,
        current_slice_n: u64,
    },
    Failed {
        refusal: Option<String>,
        gates: Vec<(String, String)>,
    },
}

/// The gate fact for one run: weights trained, gate state, and what blocks it.
#[derive(Debug, Clone, PartialEq)]
pub struct GateFact {
    pub state: GateState,
    pub trained: bool,
    pub blocker: Option<Blocker>,
}
fn recorded_slice_n(c: &Value) -> Option<u64> {
    c["reasoning_slices"]["slice_size"]
        .as_u64()
        .or_else(|| c["reports"][0]["n"].as_u64())
}

/// Gate fact from the run's records. Fails closed: precedence is
/// Failed > NeedsReGate > NotEvaluated > Passed.
pub(crate) fn gate_of(rec: &RunRecords) -> GateFact {
    let trained = rec.has_adapter && rec.train_record.is_some();
    if !trained {
        let t = rec.train_record.as_ref();
        let blocker = if !rec.has_adapter && t.is_some_and(|t| t["status"] == "trained") {
            Blocker::NotPublished
        } else {
            Blocker::NoAdapter {
                record_reason: t
                    .filter(|t| t["status"] == "not_trained")
                    .and_then(|t| t["reason"].as_str())
                    .map(str::to_string),
            }
        };
        return GateFact {
            state: GateState::NotEvaluated,
            trained,
            blocker: Some(blocker),
        };
    }
    let mut missing: Vec<String> = Vec::new();
    let mut gates: Vec<(String, String)> = Vec::new();
    let mut failed_refusal = None;
    let mut stale = None;
    match (&rec.collateral, &rec.collateral_req) {
        (None, _) => missing.push("collateral_damage_report.json".to_string()),
        (Some(c), None) => {
            stale = Some(Blocker::Stale {
                refusal: "the current collateral requirement could not be loaded, re-run `vox mens eval-collateral-damage` for this size".to_string(),
                recorded_slice_n: recorded_slice_n(c),
                current_slice_n: SLICE_SIZE as u64,
            });
        }
        (Some(c), Some(req)) => match check_collateral_report(c, req) {
            Ok(()) => {}
            Err(CollateralRefusal::Failed(m)) => failed_refusal = Some(m),
            Err(CollateralRefusal::Stale(m)) => {
                stale = Some(Blocker::Stale {
                    refusal: m,
                    recorded_slice_n: recorded_slice_n(c),
                    current_slice_n: SLICE_SIZE as u64,
                });
            }
        },
    }
    match &rec.gate_receipt {
        None => missing.push("gate_receipt.json".to_string()),
        Some(r) => {
            let rows = r["gates"].as_array().cloned().unwrap_or_default();
            for g in &rows {
                let name = g["name"].as_str().unwrap_or("?");
                if g["passed"].as_bool() == Some(true)
                    || g["block"].as_bool() != Some(true)
                    || name == "collateral_damage"
                {
                    continue; // collateral is judged from its own report above
                }
                let msg = g["message"].as_str().unwrap_or("");
                if msg.contains("missing") {
                    missing.push(name.to_string());
                } else {
                    gates.push((name.to_string(), msg.to_string()));
                }
            }
            let substantive = rows
                .iter()
                .filter(|g| {
                    let m = g["message"].as_str().unwrap_or("");
                    !m.contains("not applicable")
                        && !m.contains("skipped")
                        && !m.contains("missing")
                })
                .count();
            let overall = r["overall_passed"].as_bool() == Some(true);
            if !overall
                && gates.is_empty()
                && missing.is_empty()
                && failed_refusal.is_none()
                && stale.is_none()
            {
                gates.push(("overall_passed".to_string(), "not true".to_string()));
            } else if overall && substantive < 2 {
                missing.push("gate_receipt.json".to_string());
            }
        }
    }
    if let Some(ladder) = rec.spoke_gates.iter().find(|r| r.spoke == "ladder") {
        match ladder.state {
            vox_eval::gate_state::GateState::NeedsReGate => {
                if stale.is_none() {
                    stale = Some(Blocker::Stale {
                        refusal: ladder.reason.clone(),
                        recorded_slice_n: rec.collateral.as_ref().and_then(recorded_slice_n),
                        current_slice_n: SLICE_SIZE as u64,
                    });
                }
            }
            vox_eval::gate_state::GateState::Failed => {
                if gates.is_empty() && failed_refusal.is_none() {
                    gates.push(("ladder".to_string(), ladder.reason.clone()));
                }
            }
            vox_eval::gate_state::GateState::NotRun => {
                if missing.is_empty()
                    && gates.is_empty()
                    && failed_refusal.is_none()
                    && stale.is_none()
                {
                    missing.push(
                        ladder
                            .receipt_file
                            .clone()
                            .unwrap_or_else(|| "gate_receipt.json".to_string()),
                    );
                }
            }
            vox_eval::gate_state::GateState::Passed => {}
        }
    }
    let (state, blocker) = if failed_refusal.is_some() || !gates.is_empty() {
        (
            GateState::Failed,
            Some(Blocker::Failed {
                refusal: failed_refusal,
                gates,
            }),
        )
    } else if !missing.is_empty() {
        (GateState::NotEvaluated, Some(Blocker::Missing(missing)))
    } else if stale.is_some() {
        (GateState::NeedsReGate, stale)
    } else {
        (GateState::Passed, None)
    };
    GateFact {
        state,
        trained,
        blocker,
    }
}

/// Resolve the active spoke id for a trained run from its records (`training_manifest`, `train_record`,
/// or `spoke_gates`), falling back to `default_spoke` from `hub-cards.yaml`.
pub(crate) fn active_spoke_id<'a>(
    rec: &'a RunRecords,
    default_spoke: Option<&'a str>,
) -> Option<String> {
    let from_manifest = rec.training_manifest.as_ref().and_then(|m| {
        m.get("spoke")
            .or_else(|| m.get("domain_profile"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
    });
    if let Some(s) = from_manifest {
        return Some(s.to_string());
    }
    let from_record = rec.train_record.as_ref().and_then(|t| {
        t.get("spoke")
            .or_else(|| t.get("domain_profile"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
    });
    if let Some(s) = from_record {
        return Some(s.to_string());
    }
    if let Some(row) = rec.spoke_gates.iter().find(|g| {
        g.spoke != "ladder" && (g.applies || g.state == vox_eval::gate_state::GateState::Passed)
    }) {
        return Some(row.spoke.clone());
    }
    default_spoke.map(str::to_string)
}

#[cfg(test)]
mod record_tests {
    use super::*;

    #[test]
    fn empty_run_dir_loads_as_all_absent() {
        let d = tempfile::tempdir().unwrap();
        let rec = RunRecords::load(d.path(), "0.6B", "Qwen/Qwen3-0.6B@abcdef0123456789");
        assert!(!rec.has_adapter && rec.train_record.is_none() && rec.collateral.is_none());
        assert_eq!(heldout_scores(&rec), (None, None));
        assert!(compare_json_path(d.path(), "0.6B", "Qwen/Qwen3-0.6B@abcdef0123456789").is_none());
    }

    #[test]
    fn compare_json_path_finds_qwen35_qwen38_and_legacy_qwen3_reports() {
        let rev = "0123456789abcdef0123456789abcdef01234567";
        let rev12 = &rev[..12];

        for (rung, size, pin, expected_file) in [
            (
                "qwen3.5-4b",
                "4B",
                format!("Qwen/Qwen3.5-4B@{rev}"),
                format!("Qwen3.5-4B-{rev12}.json"),
            ),
            (
                "qwen3.8-27b",
                "27B",
                format!("Qwen/Qwen3.8-27B@{rev}"),
                format!("Qwen3.8-27B-{rev12}.json"),
            ),
            (
                "qwen3-4b",
                "4B",
                format!("Qwen/Qwen3-4B@{rev}"),
                format!("Qwen3-4B-{rev12}.json"),
            ),
        ] {
            let root = tempfile::tempdir().unwrap();
            let run_dir = root.path().join(rung);
            let compare_dir = root.path().join("reasoning_compare");
            std::fs::create_dir_all(&run_dir).unwrap();
            std::fs::create_dir_all(&compare_dir).unwrap();
            let report = compare_dir.join(&expected_file);
            std::fs::write(&report, b"{}").unwrap();

            assert_eq!(
                compare_json_path(&run_dir, size, &pin),
                Some(report),
                "expected {expected_file} for {pin}"
            );
        }

        // Legacy fallback when only `Qwen3-{size}-{rev12}.json` exists on disk.
        let root = tempfile::tempdir().unwrap();
        let run_dir = root.path().join("qwen3.5-4b");
        let compare_dir = root.path().join("reasoning_compare");
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::create_dir_all(&compare_dir).unwrap();
        let legacy = compare_dir.join(format!("Qwen3-4B-{rev12}.json"));
        std::fs::write(&legacy, b"{}").unwrap();
        assert_eq!(
            compare_json_path(&run_dir, "4B", &format!("Qwen/Qwen3.5-4B@{rev}")),
            Some(legacy)
        );

        // Staged file inside `run_dir/evidence/` still wins over `reasoning_compare/`.
        std::fs::create_dir_all(run_dir.join("evidence")).unwrap();
        let staged = run_dir.join("evidence/reasoning_compare.json");
        std::fs::write(&staged, b"{}").unwrap();
        assert_eq!(
            compare_json_path(&run_dir, "4B", &format!("Qwen/Qwen3.5-4B@{rev}")),
            Some(staged)
        );
    }

    #[test]
    fn load_accepts_compared_report_for_qwen35_and_qwen38_run_dirs() {
        let rev = "0123456789abcdef0123456789abcdef01234567";
        let rev12 = &rev[..12];

        for (rung, size, pin, file_name) in [
            (
                "qwen3.5-4b",
                "4B",
                format!("Qwen/Qwen3.5-4B@{rev}"),
                format!("Qwen3.5-4B-{rev12}.json"),
            ),
            (
                "qwen3.8-27b",
                "27B",
                format!("Qwen/Qwen3.8-27B@{rev}"),
                format!("Qwen3.8-27B-{rev12}.json"),
            ),
        ] {
            let root = tempfile::tempdir().unwrap();
            let run_dir = root.path().join(rung);
            let compare_dir = root.path().join("reasoning_compare");
            std::fs::create_dir_all(&run_dir).unwrap();
            std::fs::create_dir_all(&compare_dir).unwrap();
            let report = compare_dir.join(&file_name);

            let matching = serde_json::json!({
                "status": "compared",
                "adapter": {"dir": format!("mens/runs/{rung}/")}
            });
            std::fs::write(&report, matching.to_string()).unwrap();
            let loaded = RunRecords::load(&run_dir, size, &pin);
            assert!(
                loaded.compare.is_some(),
                "expected compare report retained for {rung}"
            );

            let mismatched = serde_json::json!({
                "status": "compared",
                "adapter": {"dir": "mens/runs/qwen3.5-2b"}
            });
            std::fs::write(&report, mismatched.to_string()).unwrap();
            let filtered = RunRecords::load(&run_dir, size, &pin);
            assert!(
                filtered.compare.is_none(),
                "expected compare report filtered for mismatched adapter dir on {rung}"
            );
        }
    }
}

#[cfg(test)]
#[path = "facts_tests.rs"]
mod tests;
