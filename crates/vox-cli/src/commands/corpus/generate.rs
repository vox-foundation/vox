use anyhow::Result;
use owo_colors::OwoColorize;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

pub(super) async fn run_generate(
    output: std::path::PathBuf,
    force_regen: bool,
    dry_run: bool,
) -> Result<()> {
    let workspace_root = vox_corpus::training::contract::find_workspace_root();
    let mut current_fp = String::new();
    if let Some(ref root) = workspace_root {
        current_fp = vox_corpus::corpus::preflight::compute_corpus_fingerprint(root);
    }

    let is_fresh = if !force_regen && !current_fp.is_empty() {
        if let Ok(db) = vox_db::VoxDb::connect_default().await {
            db.is_corpus_fresh(&current_fp).await.unwrap_or(false)
        } else if let Some(ref root) = workspace_root {
            let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
            vox_corpus::corpus::preflight::corpus_is_fresh(root, &fp_file)
        } else {
            false
        }
    } else {
        false
    };

    if is_fresh && !force_regen {
        eprintln!(
            "  {} Corpus is fresh (fingerprint: {}). Use --force-regen to rebuild.",
            "✓".green(),
            current_fp
        );
        return Ok(());
    }

    if dry_run {
        eprintln!("  {} Dry-run mode: would regenerate corpus", "ℹ".blue());
        return Ok(());
    }

    // Cleanup stale targets before regeneration
    if let Some(ref root) = workspace_root {
        vox_corpus::corpus::preflight::clean_corpus_targets(root)?;
    }

    let cfg = vox_corpus::synthetic_gen::SyntheticGenConfig::default();
    let count = vox_corpus::synthetic_gen::generate_all(&cfg, &output)?;
    eprintln!("  {} Synthesized {} pairs", "✓".green(), count);

    // Record snapshot in Arca and local file
    if !current_fp.is_empty() {
        if let Ok(db) = vox_db::VoxDb::connect_default().await {
            let _ = db
                .record_corpus_snapshot(
                    &current_fp,
                    env!("CARGO_PKG_VERSION"),
                    count as i64,
                    None,
                )
                .await;
        }
        if let Some(ref root) = workspace_root {
            let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
            let _ = vox_corpus::corpus::preflight::write_fingerprint_snapshot(root, &fp_file);
        }
    }

    Ok(())
}

pub(super) async fn run_extract(dir: &Path, output: &Path) -> Result<()> {
    let entries = crate::training::walk_vox_files(dir);

    if entries.is_empty() {
        eprintln!(
            "{}",
            format!("No .vox files found in {}", dir.display()).yellow()
        );
        return Ok(());
    }

    // ── 8.2: Incremental — load existing hashes to skip unchanged files ──
    let existing_hashes: std::collections::HashSet<String> = if output.exists() {
        let content = std::fs::read_to_string(output).unwrap_or_default();
        content
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter_map(|v| v.get("ast_hash").and_then(|h| h.as_str()).map(String::from))
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    let incremental_skipped = existing_hashes.len();
    if incremental_skipped > 0 {
        println!(
            "{}",
            format!(
                "  ↻ Incremental mode: {} known entries, skipping unchanged files",
                incremental_skipped
            )
            .cyan()
        );
    }

    // Ensure output dir exists; open in append mode for incremental
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // For a clean run (no file yet) create it; otherwise append
    if !output.exists() {
        std::fs::File::create(output)?;
    }

    let total = entries.len();

    // ── 8.1: Parallel extraction using tokio::spawn ──────────────────────
    use std::sync::{Arc, Mutex};
    let output_arc = Arc::new(Mutex::new(output.to_path_buf()));
    let existing_arc = Arc::new(existing_hashes);

    let mut handles = Vec::with_capacity(entries.len());
    for path in entries {
        let output_path = Arc::clone(&output_arc);
        let known = Arc::clone(&existing_arc);
        let handle = tokio::spawn(async move {
            match crate::pipeline::run_frontend(&path, false).await {
                Ok(result) if !result.has_errors() => {
                    // Build record first to check the hash
                    match crate::training::build_training_record(&path, &result) {
                        Ok(record) => {
                            let hash = record
                                .get("ast_hash")
                                .and_then(|h| h.as_str())
                                .unwrap_or("")
                                .to_string();
                            // Skip if already in corpus
                            if !hash.is_empty() && known.contains(&hash) {
                                return (path, true, true); // (path, ok, skipped)
                            }
                            // Append to file under lock
                            let out = output_path.lock().unwrap();
                            if let Ok(line) = serde_json::to_string(&record) {
                                use std::io::Write;
                                if let Ok(mut f) = std::fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(&*out)
                                {
                                    let _ = writeln!(f, "{}", line);
                                    return (path, true, false);
                                }
                            }
                            (path, false, false)
                        }
                        Err(_) => (path, false, false),
                    }
                }
                _ => (path, false, false),
            }
        });
        handles.push(handle);
    }

    let mut success = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    for handle in handles {
        match handle.await {
            Ok((_, true, true)) => skipped += 1,
            Ok((_, true, false)) => success += 1,
            _ => failed += 1,
        }
    }

    println!(
        "{}",
        format!(
            "✓ Corpus extraction: {}/{} new ({} skipped, {} failed) → {}",
            success,
            total,
            skipped + incremental_skipped as u32,
            failed,
            output.display()
        )
        .green()
    );
    Ok(())
}

pub(super) async fn run_pairs(input: &Path, output: &Path, docs_dir: Option<&Path>) -> Result<()> {
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    let content = std::fs::read_to_string(input)?;
    let mut all_pairs: Vec<serde_json::Value> = Vec::new();
    let mut pair_hashes: HashSet<String> = HashSet::new();

    for line in content.lines().filter(|l| !l.is_empty()) {
        let record: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let code = record.get("code").and_then(|v| v.as_str()).unwrap_or("");
        let source = record
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let constructs: Vec<String> = record
            .get("constructs")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if code.is_empty() {
            continue;
        }

        let name = crate::training::extract_name_from_source(code);

        for construct in &constructs {
            let templates = crate::training::instruction_templates(construct);
            for template in templates {
                let instruction = template.replace("{name}", &name);

                // Dedup by content hash (XXH3)
                let combined = format!("{}|||{}", instruction, code);
                let h = vox_runtime::builtins::vox_hash_fast(&combined);
                if pair_hashes.contains(&h) {
                    continue;
                }
                pair_hashes.insert(h);

                let pair = serde_json::json!({
                    "prompt": instruction,
                    "response": code,
                    "instruction": instruction,
                    "output": code,
                    "category": construct,
                    "source": source,
                    "rating": 5,
                    "schema_version": crate::training::SCHEMA_VERSION,
                });
                all_pairs.push(pair);

                // Multi-turn: generate follow-up refinement pairs
                let multi = crate::training::generate_multiturn_pairs(
                    construct,
                    &name,
                    &instruction,
                    code,
                    crate::training::SCHEMA_VERSION,
                    source,
                );
                all_pairs.extend(multi);
            }
        }

        // Generate negative (broken code) examples for this record
        let neg_examples = crate::training::generate_negative_examples(code);
        for (broken_code, error_desc) in neg_examples {
            let fix_instruction = format!("Fix this broken Vox code. Error: {}", error_desc);
            let fix_pair = serde_json::json!({
                "prompt": format!("{}\n\n```vox\n{}\n```", fix_instruction, broken_code),
                "response": code,
                "instruction": fix_instruction,
                "output": code,
                "category": "error_correction",
                "source": source,
                "rating": 4,
                "schema_version": crate::training::SCHEMA_VERSION,
            });
            all_pairs.push(fix_pair);
        }
    }

    // Extract documentation pairs
    if let Some(docs) = docs_dir {
        let doc_pairs = extract_doc_pairs(docs);
        println!("  Extracted {} pairs from documentation", doc_pairs.len());
        all_pairs.extend(doc_pairs);
    }

    // ── Curriculum ordering: sort by construct difficulty ──────────
    all_pairs.sort_by(|a, b| {
        let cat_a = a.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let cat_b = b.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let diff_a = crate::training::construct_difficulty(cat_a);
        let diff_b = crate::training::construct_difficulty(cat_b);
        diff_a.cmp(&diff_b)
    });

    // Write output
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(output)?;
    for pair in &all_pairs {
        writeln!(f, "{}", serde_json::to_string(pair)?)?;
    }

    // Stats
    let mut cats: HashMap<String, u32> = HashMap::new();
    for p in &all_pairs {
        let cat = p
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        *cats.entry(cat.to_string()).or_insert(0) += 1;
    }
    let neg_count = cats.get("error_correction").copied().unwrap_or(0);
    println!(
        "\n{}",
        format!(
            "Generated {} training pairs ({} negative examples):",
            all_pairs.len(),
            neg_count
        )
        .green()
    );
    let mut sorted_cats: Vec<_> = cats.into_iter().collect();
    sorted_cats.sort_by(|a, b| b.1.cmp(&a.1));
    for (cat, count) in &sorted_cats {
        let diff = crate::training::construct_difficulty(cat);
        println!("  {:<20} {:>4} pairs  (difficulty: {})", cat, count, diff);
    }

    // Metadata
    let meta_path = output
        .parent()
        .unwrap_or(Path::new("."))
        .join("metadata.json");
    let meta = serde_json::json!({
        "schema_version": crate::training::SCHEMA_VERSION,
        "total_pairs": all_pairs.len(),
        "negative_pairs": neg_count,
        "curriculum_ordered": true,
        "generated_by": "vox corpus pairs",
        "compiler_version": env!("CARGO_PKG_VERSION"),
    });
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;

    println!(
        "\n✓ Wrote {} pairs to {} (curriculum-ordered)",
        all_pairs.len(),
        output.display()
    );
    println!("✓ Metadata written to {}", meta_path.display());

    Ok(())
}

/// Recursively walk a directory for markdown files.
fn walk_md_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    walk_md_recursive(dir, &mut result);
    result.sort();
    result
}

fn walk_md_recursive(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_md_recursive(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

/// Extract instruction-response pairs from Markdown documentation files.
fn extract_doc_pairs(docs_dir: &Path) -> Vec<serde_json::Value> {
    let mut pairs = Vec::new();
    let entries = walk_md_files(docs_dir);

    for md_file in &entries {
        let content = match std::fs::read_to_string(md_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Find ```vox ... ``` blocks
        let mut in_vox_block = false;
        let mut code_lines: Vec<String> = Vec::new();
        let mut context_line = String::new();

        for line in content.lines() {
            if line.trim_start().starts_with("```vox") {
                in_vox_block = true;
                code_lines.clear();
                continue;
            }
            if in_vox_block && line.trim_start().starts_with("```") {
                in_vox_block = false;
                let code = code_lines.join("\n");
                if code.len() >= 20 {
                    let instruction = if context_line.is_empty() || context_line.starts_with('#') {
                        format!(
                            "Write Vox code as shown in {} documentation",
                            md_file.file_stem().unwrap_or_default().to_string_lossy()
                        )
                    } else {
                        context_line.trim_end_matches(':').to_string()
                    };
                    pairs.push(serde_json::json!({
                        "prompt": instruction,
                        "response": code,
                        "instruction": instruction,
                        "output": code,
                        "category": "documentation",
                        "source": format!("docs/{}", md_file.file_name().unwrap_or_default().to_string_lossy()),
                        "rating": 4,
                        "schema_version": crate::training::SCHEMA_VERSION,
                    }));
                }
                continue;
            }
            if in_vox_block {
                code_lines.push(line.to_string());
            } else if !line.trim().is_empty() {
                context_line = line.to_string();
            }
        }
    }

    pairs
}

pub(super) fn run_prompt(output: &Path) -> Result<()> {
    let prompt = crate::training::generate_system_prompt();

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, &prompt)?;

    println!("✓ System prompt written to {}", output.display());
    println!(
        "  {} characters, {} lines",
        prompt.len(),
        prompt.lines().count()
    );

    Ok(())
}
