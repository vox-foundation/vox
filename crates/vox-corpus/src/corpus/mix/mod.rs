//! Weighted merge of multiple JSONL corpus files into one training set.
//!
//! ## `record_format: asr_refine`
//!
//! Lines are JSON objects with `noisy_text` and `corrected_text` (see `mens/schemas/asr_refine_pairs.schema.json`).
//! They are rewritten to `vox_tensor::data::TrainingPair`-compatible JSON (`prompt`, `response`, optional `rating`,
//! `category`), with a fixed instruction prefix on `prompt` so Mens LoRA sees a consistent correction task.
//!
//! ## `record_format: tool_trace`
//!
//! Lines are JSON objects matching [`crate::tool_workflow_corpus::ToolTraceRecord`]: tool invocations for SFT.
//! JSON Schema: `mens/schemas/tool_trace_record.schema.json`; example JSONL: `mens/data/tool_traces.example.jsonl`.
//! They become `prompt`/`response` rows with `category` `tool_trace` (use `--context-filter tool_trace` in training
//! to select only these rows).
//!
//! ## `record_format: speech_to_code`
//!
//! Lines are JSON objects with `refined_transcript` (spoken intent) and `vox_code` (validated .vox source), optional
//! `transcript_alternatives`, `repair_metadata`, and `diagnostics_snapshot` (compiler/LSP repair loop). See `mens/schemas/speech_to_code_trace.schema.json`.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;

use crate::tool_workflow_corpus::ToolTraceRecord;

/// Parse JSON string as [`serde_json::Value`], or wrap as a JSON string if parse fails.
fn json_or_string_fragment(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

/// Prepended to `noisy_text` when normalizing `asr_refine` rows.
pub const ASR_REFINE_INSTRUCTION: &str = "Correct the following noisy transcript, preserving intent. Fix phonetic errors, restore punctuation, and normalize code identifiers.\n\n";

/// Instruction prefix for speech-to-code SFT rows (`record_format: speech_to_code`).
pub const SPEECH_TO_CODE_INSTRUCTION: &str = "Given the following spoken request, emit valid Vox source that satisfies it. Preserve identifiers and paths mentioned in the transcript.\n\nTranscript:\n";

/// One JSONL source file and its repeat weight for [`run_mix`].
#[derive(Debug, Deserialize)]
pub struct MixSource {
    /// Path to the JSONL file, relative to the process cwd unless absolute.
    pub path: String,
    /// Repeat factor: each line is emitted `ceil(max(weight,0)).max(1)` times.
    #[serde(default = "default_weight")]
    pub weight: f64,
    /// When `asr_refine`, parse each line as ASR refinement JSON and emit `prompt`/`response` training rows.
    #[serde(default)]
    pub record_format: Option<String>,
    /// When `true`, silently skip this source if the file does not exist (no warning printed).
    #[serde(default)]
    pub optional: bool,
}

fn default_weight() -> f64 {
    1.0
}

/// YAML shape for a corpus mix job: weighted sources and output path.
#[derive(Debug, Deserialize)]
pub struct MixConfigSchema {
    /// Ordered list of JSONL inputs and weights.
    pub sources: Vec<MixSource>,
    /// Output JSONL path (relative to cwd unless absolute).
    pub output: String,
    /// Optional lane allow-list. When set, only rows in these lanes are emitted.
    #[serde(default)]
    pub include_lanes: Vec<String>,
    /// Optional lane deny-list. Rows in these lanes are skipped.
    #[serde(default)]
    pub exclude_lanes: Vec<String>,
}

impl MixConfigSchema {
    /// Read and validate a mix YAML file from disk.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = vox_bounded_fs::read_utf8_path_capped(path)
            .with_context(|| format!("read mix config {}", path.display()))?;
        let cfg: Self = serde_yaml::from_str(&raw)
            .with_context(|| format!("parse mix YAML {}", path.display()))?;
        anyhow::ensure!(
            !cfg.sources.is_empty(),
            "mix config {}: sources must be non-empty",
            path.display()
        );
        Ok(cfg)
    }
}

/// Options for [`run_mix_with_options`].
#[derive(Debug, Clone)]
pub struct MixRunOptions {
    /// When `true`, fail if a non-optional source is missing, unreadable, or has zero usable lines after normalization.
    pub strict: bool,
    /// When `true`, write a JSON report next to the output (`{output}.mix_report.json`).
    pub write_report: bool,
}

impl Default for MixRunOptions {
    fn default() -> Self {
        Self {
            strict: false,
            write_report: true,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MixSourceReportRow {
    pub path: String,
    pub weight: f64,
    pub optional: bool,
    pub record_format: Option<String>,
    pub resolved_path: String,
    pub input_lines: usize,
    pub repeats: usize,
    pub emitted_lines: usize,
    pub skipped_reason: Option<String>,
    pub share_of_output: f64,
}

#[derive(Debug, Serialize)]
pub struct MixRunReport {
    pub config_path: String,
    pub output_path: String,
    pub strict: bool,
    pub sources: Vec<MixSourceReportRow>,
    pub total_emitted: usize,
}

/// Rewrite one JSONL line for mixing. Pass-through unless `record_format` requests transformation.
pub fn normalize_training_jsonl_line(
    line: &str,
    record_format: Option<&str>,
) -> Result<String, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("empty line".into());
    }
    match record_format.map(str::trim).filter(|s| !s.is_empty()) {
        Some("tool_trace") => {
            let rec: ToolTraceRecord = serde_json::from_str(trimmed)
                .map_err(|e| format!("tool_trace: invalid json: {e}"))?;
            let prompt = format!(
                "[vox_tool_supervision]\nTask:\n{}\n\nRespond with a single JSON object describing the tool call and outcome. \
                 Use keys: tool_name (string), arguments (object), result (any), success (boolean).\n",
                rec.task_prompt.trim()
            );
            let response = if let Some(f) = rec
                .followup_text
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                f.to_string()
            } else {
                let body = serde_json::json!({
                    "tool_name": rec.tool_name,
                    "arguments": json_or_string_fragment(&rec.arguments_json),
                    "result": json_or_string_fragment(&rec.result_json),
                    "success": rec.success,
                });
                body.to_string()
            };
            let mut row = serde_json::Map::new();
            row.insert("prompt".to_string(), serde_json::Value::String(prompt));
            row.insert("response".to_string(), serde_json::Value::String(response));
            row.insert(
                "category".to_string(),
                serde_json::Value::String("tool_trace".into()),
            );
            if let Some(ref sid) = rec.session_id {
                let r = sid.trim();
                if !r.is_empty() {
                    row.insert(
                        "source".to_string(),
                        serde_json::Value::String(format!("tool_trace:{r}")),
                    );
                }
            }
            serde_json::to_string(&serde_json::Value::Object(row)).map_err(|e| e.to_string())
        }
        Some("asr_refine") => {
            let v: serde_json::Value =
                serde_json::from_str(trimmed).map_err(|e| format!("invalid json: {e}"))?;
            if v.get("prompt").and_then(|x| x.as_str()).is_some()
                && v.get("response").and_then(|x| x.as_str()).is_some()
            {
                return Ok(trimmed.to_string());
            }
            let noisy = v
                .get("noisy_text")
                .and_then(|x| x.as_str())
                .ok_or_else(|| "asr_refine: missing noisy_text".to_string())?;
            let corrected = v
                .get("corrected_text")
                .and_then(|x| x.as_str())
                .ok_or_else(|| "asr_refine: missing corrected_text".to_string())?;
            let prompt = format!("{ASR_REFINE_INSTRUCTION}{noisy}");
            let mut row = serde_json::Map::new();
            row.insert("prompt".to_string(), serde_json::Value::String(prompt));
            row.insert(
                "response".to_string(),
                serde_json::Value::String(corrected.to_string()),
            );
            if let Some(c) = v.get("category").filter(|x| !x.is_null()) {
                row.insert("category".to_string(), c.clone());
            } else {
                row.insert(
                    "category".to_string(),
                    serde_json::Value::String("asr_refine".into()),
                );
            }
            if let Some(r) = v.get("rating").filter(|x| !x.is_null()) {
                row.insert("rating".to_string(), r.clone());
            }
            serde_json::to_string(&serde_json::Value::Object(row)).map_err(|e| e.to_string())
        }
        Some("speech_to_code") => {
            let v: serde_json::Value =
                serde_json::from_str(trimmed).map_err(|e| format!("invalid json: {e}"))?;
            if v.get("prompt").and_then(|x| x.as_str()).is_some()
                && v.get("response").and_then(|x| x.as_str()).is_some()
            {
                return Ok(trimmed.to_string());
            }
            let transcript = v
                .get("refined_transcript")
                .or_else(|| v.get("transcript"))
                .and_then(|x| x.as_str())
                .ok_or_else(|| "speech_to_code: missing refined_transcript".to_string())?;
            let code = v
                .get("vox_code")
                .or_else(|| v.get("code"))
                .and_then(|x| x.as_str())
                .ok_or_else(|| "speech_to_code: missing vox_code".to_string())?;
            let mut prompt = format!("{SPEECH_TO_CODE_INSTRUCTION}{transcript}");
            if let Some(serde_json::Value::Array(alts)) = v.get("transcript_alternatives") {
                let joined: Vec<String> = alts
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect();
                if !joined.is_empty() {
                    prompt.push_str("\n\nAlternatives:\n");
                    for a in joined {
                        prompt.push_str("- ");
                        prompt.push_str(&a);
                        prompt.push('\n');
                    }
                }
            }
            let mut row = serde_json::Map::new();
            row.insert("prompt".to_string(), serde_json::Value::String(prompt));
            row.insert(
                "response".to_string(),
                serde_json::Value::String(code.to_string()),
            );
            if let Some(c) = v.get("category").filter(|x| !x.is_null()) {
                row.insert("category".to_string(), c.clone());
            } else {
                row.insert(
                    "category".to_string(),
                    serde_json::Value::String("speech_to_code".into()),
                );
            }
            if let Some(r) = v.get("rating").filter(|x| !x.is_null()) {
                row.insert("rating".to_string(), r.clone());
            }
            if let Some(ds) = v.get("diagnostics_snapshot").filter(|x| !x.is_null()) {
                row.insert("diagnostics_snapshot".to_string(), ds.clone());
            }
            serde_json::to_string(&serde_json::Value::Object(row)).map_err(|e| e.to_string())
        }
        _ => Ok(trimmed.to_string()),
    }
}

fn default_lane_from_row(row: &serde_json::Value) -> &'static str {
    let category = row
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if category.contains("tool_trace") {
        "vox_tooling"
    } else if category.contains("speech") || category.contains("asr_refine") {
        "vox_speech"
    } else if category.contains("documentation") {
        "vox_docs_qa"
    } else {
        "vox_codegen"
    }
}

fn default_response_mode_for_lane(lane: &str) -> &'static str {
    if lane == "vox_docs_qa" {
        "prose_only"
    } else {
        "code_only"
    }
}

fn default_task_family_from_lane(lane: &str) -> &'static str {
    match lane {
        "vox_docs_qa" => "docs_qa",
        "vox_tooling" => "tool_trace",
        "vox_speech" => "speech_to_code",
        _ => "vox_codegen",
    }
}

fn enrich_lane_metadata(line: &str) -> Result<(String, String), String> {
    let mut v: serde_json::Value =
        serde_json::from_str(line).map_err(|e| format!("invalid training row json: {e}"))?;
    let inferred_lane = default_lane_from_row(&v).to_string();
    let obj = v
        .as_object_mut()
        .ok_or_else(|| "training row must be JSON object".to_string())?;
    let lane = obj
        .get("lane")
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .unwrap_or(inferred_lane);
    if !obj.contains_key("lane") {
        obj.insert("lane".to_string(), serde_json::Value::String(lane.clone()));
    }
    if !obj.contains_key("response_mode") {
        obj.insert(
            "response_mode".to_string(),
            serde_json::Value::String(default_response_mode_for_lane(&lane).to_string()),
        );
    }
    if !obj.contains_key("task_family") {
        obj.insert(
            "task_family".to_string(),
            serde_json::Value::String(default_task_family_from_lane(&lane).to_string()),
        );
    }
    serde_json::to_string(&v)
        .map(|s| (s, lane))
        .map_err(|e| e.to_string())
}

/// Same as [`run_mix_with_options`] with [`MixRunOptions::default`] (lenient; writes report).
///
/// Resolves relative `output` / source paths in the YAML against [`std::env::current_dir`].
pub fn run_mix(config_path: &Path) -> anyhow::Result<()> {
    run_mix_with_options(config_path, None, MixRunOptions::default())
}

/// Concatenate sources in order, repeating each file's lines proportional to `weight` (rounded up to one copy minimum).
///
/// Relative paths in the mix YAML are resolved against `path_base` when set (e.g. Cargo workspace root),
/// otherwise against [`std::env::current_dir`]. This must match
/// [`crate::training::mix_prepare::refresh_train_contract_override_from_mix`] so the written file
/// matches the override path passed to [`crate::training::preflight::validate_train_preflight`].
pub fn run_mix_with_options(
    config_path: &Path,
    path_base: Option<&Path>,
    options: MixRunOptions,
) -> anyhow::Result<()> {
    let cfg = MixConfigSchema::load(config_path)?;
    let base_buf = path_base.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
    });
    let cwd = base_buf.as_path();
    let out_path = cwd.join(&cfg.output);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let MixRunOptions {
        strict,
        write_report,
    } = options;

    let mut report_rows: Vec<MixSourceReportRow> = Vec::with_capacity(cfg.sources.len());

    for src in &cfg.sources {
        let p = cwd.join(&src.path);
        if !p.is_file() {
            if src.optional {
                tracing::trace!("[mix] skip optional missing source {}", p.display());
                report_rows.push(MixSourceReportRow {
                    path: src.path.clone(),
                    weight: src.weight,
                    optional: true,
                    record_format: src.record_format.clone(),
                    resolved_path: p.display().to_string(),
                    input_lines: 0,
                    repeats: 0,
                    emitted_lines: 0,
                    skipped_reason: Some("missing_file".into()),
                    share_of_output: 0.0,
                });
            } else if strict {
                anyhow::bail!("[mix] strict: required source missing: {}", p.display());
            } else {
                eprintln!(
                    "  [mix] ⚠ Missing required source: {}. Run 'vox mens corpus generate' first, or add 'optional: true' to mix.yaml.",
                    p.display()
                );
                report_rows.push(MixSourceReportRow {
                    path: src.path.clone(),
                    weight: src.weight,
                    optional: false,
                    record_format: src.record_format.clone(),
                    resolved_path: p.display().to_string(),
                    input_lines: 0,
                    repeats: 0,
                    emitted_lines: 0,
                    skipped_reason: Some("missing_file".into()),
                    share_of_output: 0.0,
                });
            }
            continue;
        }
        if src.weight <= 0.0 {
            eprintln!(
                "  [mix] source '{}' has weight 0.0 — it will be ignored",
                p.display()
            );
            report_rows.push(MixSourceReportRow {
                path: src.path.clone(),
                weight: src.weight,
                optional: src.optional,
                record_format: src.record_format.clone(),
                resolved_path: p.display().to_string(),
                input_lines: 0,
                repeats: 0,
                emitted_lines: 0,
                skipped_reason: Some("weight_zero".into()),
                share_of_output: 0.0,
            });
            continue;
        }
        let repeats = (src.weight.max(0.0)).ceil().max(1.0) as usize;
        let file = File::open(&p).with_context(|| format!("open {}", p.display()))?;
        let lines: Vec<String> = BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .collect();

        if lines.is_empty() && !src.optional && strict {
            anyhow::bail!(
                "[mix] strict: required source has no non-empty lines: {}",
                p.display()
            );
        }

        report_rows.push(MixSourceReportRow {
            path: src.path.clone(),
            weight: src.weight,
            optional: src.optional,
            record_format: src.record_format.clone(),
            resolved_path: p.display().to_string(),
            input_lines: lines.len(),
            repeats,
            emitted_lines: 0,
            skipped_reason: if lines.is_empty() {
                Some("empty_file".into())
            } else {
                None
            },
            share_of_output: 0.0,
        });
    }

    let mut out = File::create(&out_path)
        .with_context(|| format!("create mix output {}", out_path.display()))?;
    let mut total_out = 0usize;
    let include_lanes: HashSet<String> = if cfg.include_lanes.is_empty() {
        HashSet::from(["vox_codegen".to_string()])
    } else {
        cfg.include_lanes.iter().cloned().collect()
    };
    let exclude_lanes: HashSet<String> = cfg.exclude_lanes.iter().cloned().collect();
    let mut lane_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for (row_idx, src) in cfg.sources.iter().enumerate() {
        let p = cwd.join(&src.path);
        let row = report_rows
            .get_mut(row_idx)
            .expect("mix report row count matches sources");

        if !p.is_file() || src.weight <= 0.0 {
            continue;
        }

        let repeats = (src.weight.max(0.0)).ceil().max(1.0) as usize;
        let file = File::open(&p).with_context(|| format!("open {}", p.display()))?;
        let lines: Vec<String> = BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .collect();

        let mut emitted_this_src = 0usize;
        for _ in 0..repeats {
            for line in &lines {
                let normalized =
                    match normalize_training_jsonl_line(line, src.record_format.as_deref()) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("  [mix] skip line in {}: {e}", p.display());
                            continue;
                        }
                    };
                let (normalized, lane) = match enrich_lane_metadata(&normalized) {
                    Ok(pair) => pair,
                    Err(e) => {
                        eprintln!("  [mix] skip line in {}: {e}", p.display());
                        continue;
                    }
                };
                if !include_lanes.is_empty() && !include_lanes.contains(&lane) {
                    continue;
                }
                if exclude_lanes.contains(&lane) {
                    continue;
                }
                *lane_counts.entry(lane).or_insert(0) += 1;
                writeln!(out, "{normalized}")?;
                total_out += 1;
                emitted_this_src += 1;
            }
        }
        row.emitted_lines = emitted_this_src;
        if emitted_this_src == 0 && !lines.is_empty() {
            row.skipped_reason = Some("all_lines_failed_normalization".into());
        } else if emitted_this_src > 0 {
            row.skipped_reason = None;
        }
    }

    if strict {
        if total_out == 0 {
            anyhow::bail!("[mix] strict: mixed output would be empty (check sources and weights)");
        }
        for row in &report_rows {
            if row.optional || row.skipped_reason.as_deref() == Some("weight_zero") {
                continue;
            }
            if row.emitted_lines == 0 {
                anyhow::bail!(
                    "[mix] strict: required source {:?} contributed zero rows after mix (input_lines={}, reason={:?})",
                    row.path,
                    row.input_lines,
                    row.skipped_reason
                );
            }
        }
    }

    for row in report_rows.iter_mut() {
        row.share_of_output = if total_out > 0 {
            row.emitted_lines as f64 / total_out as f64
        } else {
            0.0
        };
    }

    if write_report {
        let report_name = out_path
            .file_stem()
            .map(|s| format!("{}.mix_report.json", s.to_string_lossy()))
            .unwrap_or_else(|| "mix_report.json".into());
        let report_path = out_path.with_file_name(report_name);
        let report = MixRunReport {
            config_path: config_path.display().to_string(),
            output_path: out_path.display().to_string(),
            strict,
            sources: report_rows,
            total_emitted: total_out,
        };
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into()),
        )
        .with_context(|| format!("write mix report {}", report_path.display()))?;
        eprintln!("  [mix] report → {}", report_path.display());
    }

    if !lane_counts.is_empty() {
        eprintln!("  [mix] lane distribution: {:?}", lane_counts);
    }
    eprintln!("  [mix] wrote {} lines → {}", total_out, out_path.display());
    Ok(())
}

#[cfg(test)]
mod tests;
