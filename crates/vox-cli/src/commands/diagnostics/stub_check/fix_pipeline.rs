//! Staged fix pipeline: Pass A (frontmatter), B (placeholders), C (unwired), D (sprawl).

use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use vox_code_audit::Finding;

/// Verify frontmatter parses and body is unchanged; restore original on failure.
fn verify_and_restore_on_failure(full: &Path, original: &str, op: &str) -> anyhow::Result<()> {
    let written = read_utf8_path_capped(full)?;
    let (_, body) = parse_frontmatter(&written);
    let (_, orig_body) = parse_frontmatter(original);
    if body.trim_start() != orig_body.trim_start() {
        std::fs::write(full, original)?;
        anyhow::bail!(
            "Pass A {}: body changed unexpectedly, restored original",
            op
        );
    }
    Ok(())
}

fn parse_frontmatter(s: &str) -> (Option<&str>, &str) {
    let trimmed = s.trim_start();
    if !trimmed.starts_with("---") {
        return (None, s);
    }
    let after_open = trimmed
        .strip_prefix("---")
        .unwrap_or(trimmed)
        .trim_start_matches('\n');
    let close_pos = after_open
        .find("\n---")
        .or_else(|| after_open.find("---\n"));
    let (fm_end, body_start) = match close_pos {
        Some(i) => (
            i,
            i + if after_open[i..].starts_with("\n---\n") {
                5
            } else {
                4
            },
        ),
        None => return (None, s),
    };
    let fm = after_open[..fm_end].trim_end();
    let body = after_open.get(body_start..).unwrap_or("");
    (Some(fm), body)
}

/// Normalize file path for baseline key.
pub(crate) fn norm_key(file: &Path, line: usize, rule_id: &str) -> (String, usize, String) {
    let f = file.to_string_lossy().replace('\\', "/");
    let f = f.trim_start_matches("./");
    (f.to_string(), line, rule_id.to_string())
}

fn last_updated_today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Run staged fix pipeline: Pass A (frontmatter), B (placeholders), C (unwired), D (sprawl).
pub(crate) fn run_fix_pipeline(
    findings: &[Finding],
    root: &Path,
    apply_pass_a: bool,
) -> anyhow::Result<()> {
    use std::io::Write;

    // Pass A: doc/missing-frontmatter, doc/missing-frontmatter-field
    let pass_a: Vec<_> = findings
        .iter()
        .filter(|f| {
            f.rule_id == "doc/missing-frontmatter" || f.rule_id == "doc/missing-frontmatter-field"
        })
        .collect();
    if !pass_a.is_empty() {
        println!(
            "\n--- Fix pipeline Pass A: docs/frontmatter ({} findings) ---",
            pass_a.len()
        );
        let mut seen = std::collections::HashSet::new();
        for f in &pass_a {
            let path_str = f.file.to_string_lossy();
            if seen.insert(path_str.to_string()) {
                let full = root.join(&f.file);
                if full.exists() && apply_pass_a {
                    let content = read_utf8_path_capped(&full)?;
                    let trimmed = content.trim_start();
                    if !trimmed.starts_with("---") {
                        let title = f
                            .file
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Page");
                        let today = last_updated_today();
                        let front = format!(
                            "---\ntitle: \"{}\"\ncategory: reference\nlast_updated: {}\ntraining_eligible: false\n---\n\n",
                            title.replace('_', " "),
                            today
                        );
                        let mut file = std::fs::File::create(&full)?;
                        file.write_all(front.as_bytes())?;
                        file.write_all(content.as_bytes())?;
                        drop(file);
                        verify_and_restore_on_failure(&full, &content, "frontmatter-add")?;
                        println!("  Applied frontmatter: {}", path_str);
                    } else {
                        let needs_title = pass_a.iter().any(|x| {
                            x.file == f.file
                                && x.rule_id == "doc/missing-frontmatter-field"
                                && x.message.contains("'title'")
                        });
                        let needs_training_eligible = pass_a.iter().any(|x| {
                            x.file == f.file
                                && x.rule_id == "doc/missing-frontmatter-field"
                                && x.message.contains("training_eligible")
                        });
                        if needs_title || needs_training_eligible {
                            let after_first = trimmed
                                .strip_prefix("---")
                                .unwrap_or(trimmed)
                                .trim_start_matches('\n');
                            let fm_end = after_first
                                .find("\n---")
                                .or_else(|| after_first.find("---"))
                                .unwrap_or(after_first.len());
                            let fm_content = after_first[..fm_end].trim_end();
                            let rest = &after_first[fm_end..];
                            let mut additions = Vec::new();
                            if needs_title && !fm_content.contains("title:") {
                                let title = f
                                    .file
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Page");
                                additions.push(format!("title: \"{}\"", title.replace('_', " ")));
                            }
                            if needs_training_eligible && !fm_content.contains("training_eligible:")
                            {
                                additions.push("training_eligible: false".to_string());
                            }
                            if !additions.is_empty() {
                                let new_fm = format!("{}\n{}\n", fm_content, additions.join("\n"));
                                let prefix = &content[..content.len() - trimmed.len()];
                                let new_content = format!("{}---\n{}\n---{}", prefix, new_fm, rest);
                                std::fs::write(&full, &new_content)?;
                                verify_and_restore_on_failure(
                                    &full,
                                    &content,
                                    "frontmatter-fields",
                                )?;
                                println!("  Applied missing fields: {}", path_str);
                            }
                        }
                    }
                } else {
                    println!(
                        "  {} (use --fix-pipeline-apply to add frontmatter)",
                        path_str
                    );
                }
            }
        }
    }

    // Pass B: rule IDs for stub/placeholder, stub/todo, etc. (toestub-ignore: legitimate rule names)
    let pass_b: Vec<_> = findings
        .iter()
        .filter(|f| {
            f.rule_id.starts_with("stub/placeholder") // toestub-ignore(stub)
                || f.rule_id.starts_with("stub/todo") // toestub-ignore(stub)
                || f.rule_id == "stub/unimplemented" // toestub-ignore(stub)
                || f.rule_id == "stub/panic-not-impl" // toestub-ignore(stub)
        })
        .collect();
    if !pass_b.is_empty() {
        println!(
            "\n--- Fix pipeline Pass B: placeholders / stubs ({} findings) ---",
            pass_b.len()
        );
        for f in pass_b.iter().take(30) {
            println!("  {}:{} — {}", f.file.display(), f.line, f.rule_id);
        }
        if pass_b.len() > 30 {
            println!("  ... and {} more", pass_b.len() - 30);
        }
        println!("  → Replace with real implementation or tracked issue.");
    }

    // Pass C: unwired/module
    let pass_c: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "unwired/module")
        .collect();
    if !pass_c.is_empty() {
        println!(
            "\n--- Fix pipeline Pass C: unwired modules ({} findings) ---",
            pass_c.len()
        );
        for f in pass_c.iter().take(20) {
            println!("  {}:{} — {}", f.file.display(), f.line, f.message);
        }
        if pass_c.len() > 20 {
            println!("  ... and {} more", pass_c.len() - 20);
        }
        println!("  → Wire with use or remove the module.");
    }

    // Pass D: arch/sprawl
    let pass_d: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "arch/sprawl")
        .collect();
    if !pass_d.is_empty() {
        println!(
            "\n--- Fix pipeline Pass D: sprawl ({} findings) ---",
            pass_d.len()
        );
        for f in pass_d.iter().take(15) {
            println!("  {} — {}", f.file.display(), f.message);
        }
        if pass_d.len() > 15 {
            println!("  ... and {} more", pass_d.len() - 15);
        }
        println!("  → Group files into domain subdirs (e.g. commands/build/, commands/fmt/).");
    }

    Ok(())
}
