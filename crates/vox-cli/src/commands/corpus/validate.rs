use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped_async;

/// Pick Vox source to run through `run_frontend_str`, or `None` to skip compiler recheck for this row.
fn vox_source_for_compiler_recheck(record: &serde_json::Value) -> Option<String> {
    let response_mode = record
        .get("response_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if response_mode == "prose_only" {
        return None;
    }

    let code = record.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if !code.trim().is_empty() {
        return Some(code.trim().to_string());
    }

    let response = record
        .get("response")
        .or_else(|| record.get("output"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if response.trim().is_empty() {
        return None;
    }

    if let Some(inner) = extract_fenced_vox_block(response) {
        return Some(inner);
    }

    let lane = record.get("lane").and_then(|v| v.as_str()).unwrap_or("");
    let category = record
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let format = record.get("format").and_then(|v| v.as_str()).unwrap_or("");

    let codegen_like = format == "vox_source"
        || lane == "vox_codegen"
        || category.starts_with("vox_")
        || category == "golden"
        || category.starts_with("golden_");

    if codegen_like && response_opens_with_vox_decl(response) {
        return Some(response.trim().to_string());
    }

    if lane == "vox_docs_qa" {
        return None;
    }

    None
}

/// True when the first substantive line looks like top-level Vox (avoids running prose through the compiler).
fn response_opens_with_vox_decl(response: &str) -> bool {
    for line in response.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        return t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("actor ")
            || t.starts_with("workflow ")
            || t.starts_with("activity ")
            || t.starts_with("component ")
            || t.starts_with("import ")
            || t.starts_with("type ")
            || t.starts_with("const ")
            || t.starts_with("http ")
            || t.starts_with('@');
    }
    false
}

fn extract_fenced_vox_block(response: &str) -> Option<String> {
    let key = "```vox";
    let idx = response.find(key)?;
    let after = &response[idx + key.len()..];
    let after = after.strip_prefix('\r').unwrap_or(after);
    let after = after.strip_prefix('\n').unwrap_or(after);
    let end = after.find("```")?;
    let inner = after[..end].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

pub(super) async fn run_validate(
    input: &Path,
    output: &Path,
    recheck: bool,
    quarantine: Option<&Path>,
    report: Option<&Path>,
) -> Result<()> {
    if tokio::fs::metadata(input).await.is_err() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    let strict = std::env::var("VOX_MENS_TRAIN_JSONL_STRICT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let content = read_utf8_path_capped_async(input).await?;
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    let total = lines.len();
    let mut valid: Vec<serde_json::Value> = Vec::new();
    let mut rejected_malformed = 0u32;
    let mut rejected_compiler = 0u32;
    let mut construct_counts: HashMap<String, u32> = HashMap::new();
    let mut quarantine_rows: Vec<serde_json::Value> = Vec::new();
    let mut failure_samples: Vec<serde_json::Value> = Vec::new();

    for line in &lines {
        let record: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                rejected_malformed += 1;
                if quarantine.is_some() {
                    quarantine_rows.push(serde_json::json!({
                        "reason": "malformed_json",
                        "line": line,
                    }));
                }
                continue;
            }
        };

        if recheck {
            if let Some(src) = vox_source_for_compiler_recheck(&record) {
                let dummy_path = Path::new("__validate__.vox");
                match crate::pipeline::run_frontend_str(&src, dummy_path, false) {
                    Ok(result) if !result.has_errors() => {}
                    Ok(result) => {
                        rejected_compiler += 1;
                        let detail = serde_json::json!({
                            "reason": "type_or_hir_errors",
                            "errors": result.error_count(),
                            "source_preview": src.chars().take(200).collect::<String>(),
                        });
                        if failure_samples.len() < 32 {
                            failure_samples.push(detail.clone());
                        }
                        if quarantine.is_some() {
                            quarantine_rows.push(serde_json::json!({
                                "reason": "compiler_errors",
                                "detail": detail,
                                "record": record,
                            }));
                        }
                        continue;
                    }
                    Err(e) => {
                        rejected_compiler += 1;
                        let detail = serde_json::json!({
                            "reason": "parse_or_frontend_failed",
                            "message": e.to_string(),
                            "source_preview": src.chars().take(200).collect::<String>(),
                        });
                        if failure_samples.len() < 32 {
                            failure_samples.push(detail.clone());
                        }
                        if quarantine.is_some() {
                            quarantine_rows.push(serde_json::json!({
                                "reason": "compiler_rejected",
                                "detail": detail,
                                "record": record,
                            }));
                        }
                        continue;
                    }
                }
            }
        } else {
            let code = record.get("code").and_then(|v| v.as_str()).unwrap_or("");
            if !code.is_empty() {
                let dummy_path = Path::new("__validate__.vox");
                match crate::pipeline::run_frontend_str(code, dummy_path, false) {
                    Ok(result) if !result.has_errors() => {}
                    _ => {
                        rejected_compiler += 1;
                        continue;
                    }
                }
            }
        }

        // Assign difficulty if missing
        let mut record = record;
        if record.get("difficulty").is_none() {
            if let Some(constructs) = record.get("constructs").and_then(|v| v.as_array()) {
                let diff = constructs
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(crate::training::construct_difficulty)
                    .max()
                    .unwrap_or(5);
                record
                    .as_object_mut()
                    .unwrap()
                    .insert("difficulty".to_string(), serde_json::json!(diff));
            }
        }

        // Count constructs
        let mut count_for_record = 0u32;
        if let Some(constructs) = record.get("constructs").and_then(|v| v.as_array()) {
            count_for_record = constructs.len() as u32;
            for c in constructs {
                if let Some(s) = c.as_str() {
                    *construct_counts.entry(s.to_string()).or_insert(0) += 1;
                }
            }
        }

        record.as_object_mut().unwrap().insert(
            "construct_count".to_string(),
            serde_json::json!(count_for_record),
        );

        valid.push(record);
    }

    let accepted_pre_dedup = valid.len();

    // Dedup by ast_hash
    let mut seen: HashSet<String> = HashSet::new();
    let mut deduped: Vec<serde_json::Value> = Vec::new();
    for record in valid {
        let hash = record
            .get("ast_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !hash.is_empty() && seen.contains(&hash) {
            continue;
        }
        if !hash.is_empty() {
            seen.insert(hash);
        }
        deduped.push(record);
    }

    if let Some(parent) = output.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut body = String::new();
    for record in &deduped {
        body.push_str(&serde_json::to_string(record)?);
        body.push('\n');
    }
    tokio::fs::write(output, body).await?;

    if let Some(qpath) = quarantine {
        if let Some(parent) = qpath.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut qbody = String::new();
        for row in &quarantine_rows {
            qbody.push_str(&serde_json::to_string(row)?);
            qbody.push('\n');
        }
        tokio::fs::write(qpath, qbody).await?;
    }

    let rejected_total = rejected_malformed + rejected_compiler;
    let report_json = serde_json::json!({
        "input": input.to_string_lossy(),
        "output": output.to_string_lossy(),
        "recheck": recheck,
        "strict_env": strict,
        "total_input_lines": total,
        "accepted_after_dedup": deduped.len(),
        "rejected_malformed_json": rejected_malformed,
        "rejected_compiler": rejected_compiler,
        "rejected_total": rejected_total,
        "failure_samples": failure_samples,
    });

    if let Some(rpath) = report {
        if let Some(parent) = rpath.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(
            rpath,
            serde_json::to_string_pretty(&report_json).unwrap_or_default(),
        )
        .await
        .with_context(|| format!("write report {}", rpath.display()))?;
    }

    #[cfg(feature = "database")]
    {
        if let Ok(db) = vox_db::VoxDb::connect_default().await {
            for record in &deduped {
                let hash = record
                    .get("ast_hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let source = record.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let parse_valid = true;
                let ast_depth = record
                    .get("difficulty")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as u32;
                let count = record
                    .get("construct_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let split = record
                    .get("split")
                    .and_then(|v| v.as_str())
                    .unwrap_or("train");

                let _ = db
                    .upsert_corpus_quality(
                        hash,
                        source,
                        parse_valid,
                        ast_depth as usize,
                        count as usize,
                        0.0,
                        split,
                    )
                    .await;
            }
        }
    }

    // Coverage report
    let taxonomy: HashSet<&str> = crate::training::TAXONOMY.iter().copied().collect();
    let covered: HashSet<&str> = construct_counts
        .keys()
        .map(|s| s.as_str())
        .filter(|s| taxonomy.contains(s))
        .collect();
    let uncovered: Vec<&&str> = taxonomy.iter().filter(|s| !covered.contains(**s)).collect();
    let coverage_pct = if taxonomy.is_empty() {
        0.0
    } else {
        100.0 * covered.len() as f64 / taxonomy.len() as f64
    };

    println!("╔══════════════════════════════════════════════════╗");
    println!("║       Vox Training Data Validation Report       ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Input records:     {:<28}║", total);
    println!("║  Accepted (pre-dedup):{:<26}║", accepted_pre_dedup);
    println!("║  After dedup:       {:<28}║", deduped.len());
    println!("║  Rejected (json):   {:<28}║", rejected_malformed);
    println!("║  Rejected (compiler):{:<26}║", rejected_compiler);
    let cov_text = format!(
        "{:.1}% ({}/{})",
        coverage_pct,
        covered.len(),
        taxonomy.len()
    );
    println!("║  Construct coverage:{:<28}║", cov_text);
    println!("╠══════════════════════════════════════════════════╣");
    if uncovered.is_empty() {
        println!("║  ✅ All constructs covered!                      ║");
    } else {
        println!("║  Missing constructs:                             ║");
        for c in uncovered.iter().take(10) {
            println!("║    - {:<43}║", c);
        }
        if uncovered.len() > 10 {
            println!(
                "║    ... and {} more                               ║",
                uncovered.len() - 10
            );
        }
    }
    println!("╚══════════════════════════════════════════════════╝");

    if strict && rejected_total > 0 {
        anyhow::bail!(
            "VOX_MENS_TRAIN_JSONL_STRICT: rejected {} rows (malformed {} compiler {}). See --quarantine / --report.",
            rejected_total,
            rejected_malformed,
            rejected_compiler
        );
    }

    Ok(())
}
