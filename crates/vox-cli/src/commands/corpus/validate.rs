use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped_async;

pub(super) async fn run_validate(input: &Path, output: &Path, recheck: bool) -> Result<()> {
    if tokio::fs::metadata(input).await.is_err() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    let content = read_utf8_path_capped_async(input).await?;
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    let total = lines.len();
    let mut valid: Vec<serde_json::Value> = Vec::new();
    let mut rejected = 0u32;
    let mut construct_counts: HashMap<String, u32> = HashMap::new();

    for line in &lines {
        let record: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                rejected += 1;
                continue;
            }
        };

        let code = record.get("code").and_then(|v| v.as_str()).unwrap_or("");

        // Re-validate through compiler if requested
        if recheck && !code.is_empty() {
            let dummy_path = Path::new("__validate__.vox");
            match crate::pipeline::run_frontend_str(code, dummy_path, false) {
                Ok(result) if !result.has_errors() => {}
                _ => {
                    rejected += 1;
                    continue;
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

    #[cfg(feature = "database")]
    {
        if let Ok(db) = vox_db::VoxDb::connect_default().await {
            for record in &deduped {
                let hash = record
                    .get("ast_hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let source = record.get("source").and_then(|v| v.as_str()).unwrap_or("");
                // validate-batch implies successful compiler parse or generated.
                // We map this into upsert_corpus_quality.
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
                        0.0, // default reward
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
    println!(
        "║  Valid (post-check):{:<28}║",
        deduped.len() + rejected as usize
    );
    println!("║  After dedup:       {:<28}║", deduped.len());
    println!("║  Rejected:          {:<28}║", rejected);
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

    Ok(())
}
