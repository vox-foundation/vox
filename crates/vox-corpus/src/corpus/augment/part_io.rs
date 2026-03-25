/// Augment every instruction in a slice of JSONL lines and emit additional lines.
///
/// Lines that already have `record_format` or non-plain fields are emitted as-is.
/// For each parseable `{"prompt": ..., "response": ...}` pair, `config.variants_per_prompt`
/// augmented instruction variants are appended. The response is unchanged.
pub fn augment_jsonl_lines(lines: &[String], config: &AugmentConfig, seed: u64) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len() * (1 + config.variants_per_prompt));
    for line in lines {
        out.push(line.clone());
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let Some(prompt) = v.get("prompt").and_then(|x| x.as_str()) else {
            continue;
        };
        let Some(response) = v.get("response").and_then(|x| x.as_str()) else {
            continue;
        };

        let category = v.get("category").and_then(|x| x.as_str()).unwrap_or("");
        if category == "negative_preference" {
            continue;
        }

        let variants_to_gen = if category == "documentation" {
            1.min(config.variants_per_prompt)
        } else {
            config.variants_per_prompt
        };

        if variants_to_gen == 0 {
            continue;
        }

        let mut local_config = config.clone();
        local_config.variants_per_prompt = variants_to_gen;

        let variants = augment_prompt(prompt, &local_config, seed);
        for variant_prompt in variants {
            let mut row = v.as_object().cloned().unwrap_or_default();
            row.insert(
                "prompt".to_string(),
                serde_json::Value::String(variant_prompt),
            );
            // Mark augmented rows so they can be filtered during eval
            row.entry("augmented".to_string())
                .or_insert(serde_json::Value::Bool(true));
            let _ = response; // retained from parent
            if let Ok(s) = serde_json::to_string(&serde_json::Value::Object(row)) {
                out.push(s);
            }
        }
    }
    out
}

/// Apply augmentation to every eligible line in a JSONL file in-place.
///
/// Reads all lines, calls [`augment_jsonl_lines`] to expand them, then rewrites
/// the file with the augmented set. Returns the number of **new** lines added.
pub fn augment_corpus_file(
    path: &std::path::Path,
    config: &AugmentConfig,
    seed: u64,
) -> anyhow::Result<usize> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("augment_corpus_file: read {}", path.display()))?;
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect();
    let original_count = lines.len();
    let augmented = augment_jsonl_lines(&lines, config, seed);
    let added = augmented.len().saturating_sub(original_count);
    let mut writer = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .with_context(|| format!("augment_corpus_file: open for write {}", path.display()))?,
    );
    for line in &augmented {
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;
    Ok(added)
}

