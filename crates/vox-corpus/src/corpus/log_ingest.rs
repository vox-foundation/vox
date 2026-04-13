use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use anyhow::Context;
use serde_json::json;

pub fn ingest_training_logs(log_path: &Path, out: &mut impl Write) -> anyhow::Result<usize> {
    let file = std::fs::File::open(log_path).context("open log")?;
    let reader = BufReader::new(file);
    let mut actual = 0;
    
    let mut current_error = String::new();
    let mut snippet = String::new();
    let mut collecting = false;

    // Use a Vec to handle potential BOM or weird encoding issues by just looking for the substring
    for line in reader.lines() {
        let line = line?;
        if line.contains("error[") {
            if collecting && !snippet.is_empty() {
                emit_error(out, &current_error, &snippet, &mut actual)?;
            }
            collecting = true;
            current_error = line.trim().to_string();
            snippet.clear();
        } else if collecting {
            if let Some(pos) = line.find('|') {
                let content = line[pos+1..].trim();
                if !content.is_empty() && !content.contains("^^") && !content.contains("expected") {
                    snippet.push_str(content);
                    snippet.push('\n');
                }
            }
        }
    }

    if collecting && !snippet.is_empty() {
        emit_error(out, &current_error, &snippet, &mut actual)?;
    }

    Ok(actual)
}

fn emit_error(out: &mut impl Write, error: &str, snippet: &str, count: &mut usize) -> anyhow::Result<()> {
    let record = json!({
        "prompt": "Fix the following Vox compiler error.",
        "rejected": snippet.trim().to_string(),
        "chosen": format!("// Fix: {}", error),
        "category": "negative_telemetry",
        "lane": "vox_dogfood_flywheel",
    });
    writeln!(out, "{}", serde_json::to_string(&record)?)?;
    *count += 1;
    Ok(())
}
