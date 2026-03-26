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
}

impl MixConfigSchema {
    /// Read and validate a mix YAML file from disk.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = crate::bounded_fs::read_utf8_path_capped(path)
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
        _ => Ok(trimmed.to_string()),
    }
}

/// Same as [`run_mix_with_options`] with [`MixRunOptions::default`] (lenient; writes report).
pub fn run_mix(config_path: &Path) -> anyhow::Result<()> {
    run_mix_with_options(config_path, MixRunOptions::default())
}

/// Concatenate sources in order, repeating each file's lines proportional to `weight` (rounded up to one copy minimum).
pub fn run_mix_with_options(config_path: &Path, options: MixRunOptions) -> anyhow::Result<()> {
    let cfg = MixConfigSchema::load(config_path)?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
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

    eprintln!("  [mix] wrote {} lines → {}", total_out, out_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_refine_normalizes_to_training_pair_shape() {
        let raw = r#"{"noisy_text":"hello  wrld","corrected_text":"hello world","rating":4}"#;
        let out = normalize_training_jsonl_line(raw, Some("asr_refine")).expect("ok");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let prompt = v["prompt"].as_str().unwrap();
        assert!(prompt.contains("hello  wrld"));
        assert!(prompt.starts_with("Correct the following noisy"));
        assert_eq!(v["response"].as_str(), Some("hello world"));
        assert_eq!(v["rating"].as_u64(), Some(4));
        assert_eq!(v["category"].as_str(), Some("asr_refine"));
    }

    #[test]
    fn passthrough_without_format() {
        let raw = r#"{"prompt":"a","response":"b"}"#;
        let out = normalize_training_jsonl_line(raw, None).unwrap();
        assert_eq!(out, raw);
    }

    #[test]
    fn tool_trace_normalizes_to_training_pair_shape() {
        let raw = r#"{"task_prompt":"Run fmt","tool_name":"shell","arguments_json":"{\"cmd\":\"cargo fmt\"}","result_json":"{\"ok\":true}","success":true}"#;
        let out = normalize_training_jsonl_line(raw, Some("tool_trace")).expect("ok");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            v["prompt"]
                .as_str()
                .unwrap()
                .contains("[vox_tool_supervision]")
        );
        assert!(v["prompt"].as_str().unwrap().contains("Run fmt"));
        let resp = v["response"].as_str().unwrap();
        assert!(resp.contains("shell"));
        assert!(resp.contains("cargo fmt"));
        assert_eq!(v["category"].as_str(), Some("tool_trace"));
    }

    #[test]
    fn tool_trace_uses_followup_when_present() {
        let raw = r#"{"task_prompt":"x","tool_name":"t","arguments_json":"{}","result_json":"{}","success":true,"followup_text":"Done."}"#;
        let out = normalize_training_jsonl_line(raw, Some("tool_trace")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["response"].as_str(), Some("Done."));
    }

    #[test]
    fn strict_rejects_missing_required_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("mix.yaml");
        let absent = dir.path().join("nope.jsonl");
        let out_j = dir.path().join("out.jsonl");
        let p_abs = absent.to_string_lossy().replace('\\', "/");
        let p_out = out_j.to_string_lossy().replace('\\', "/");
        std::fs::write(
            &cfg_path,
            format!("sources:\n  - path: \"{p_abs}\"\n    weight: 1\noutput: \"{p_out}\"\n"),
        )
        .unwrap();
        let err = run_mix_with_options(
            &cfg_path,
            MixRunOptions {
                strict: true,
                write_report: false,
            },
        )
        .expect_err("strict missing");
        let s = format!("{err:#}");
        assert!(s.contains("strict") || s.contains("missing"), "{s}");
    }
}
