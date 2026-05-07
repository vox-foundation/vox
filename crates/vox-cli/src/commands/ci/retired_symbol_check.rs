use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct SymbolPolicy {
    #[serde(rename = "schema_version", default)]
    _schema_version: String,
    symbols: Vec<RetiredSymbol>,
}

#[derive(Deserialize, Debug)]
struct RetiredSymbol {
    id: String,
    pattern: String,
    replacement: String,
    rationale: String,
}

pub fn run(root: &Path) -> Result<()> {
    let policy_path = root.join("contracts/documentation/retired-symbols.v1.yaml");
    if !policy_path.exists() {
        return Err(anyhow!(
            "Policy file not found at {}",
            policy_path.display()
        ));
    }

    let content = fs::read_to_string(&policy_path)
        .with_context(|| format!("Failed to read {}", policy_path.display()))?;

    let policy: SymbolPolicy = serde_yaml::from_str(&content)
        .with_context(|| "Failed to parse retired-symbols.v1.yaml")?;

    let mut regexes = Vec::new();
    for sym in &policy.symbols {
        let re = Regex::new(&sym.pattern)
            .with_context(|| format!("Invalid regex pattern for {}: {}", sym.id, sym.pattern))?;
        regexes.push((sym, re));
    }

    let docs_dir = root.join("docs");
    let mut failures = Vec::new();

    let mut dirs_to_visit = vec![docs_dir];
    while let Some(dir) = dirs_to_visit.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs_to_visit.push(path);
            } else if path.extension().is_some_and(|e| e == "md" || e == "json") {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename == "SUMMARY.md" || filename == "doc-inventory.json" {
                    println!("DEBUG: skipping {}", filename);
                    continue;
                }
                println!("DEBUG: processing {}", filename);
                if let Ok(body) = fs::read_to_string(&path) {
                    for (line_idx, line) in body.lines().enumerate() {
                        for (sym, re) in &regexes {
                            if re.is_match(line) {
                                // Skip cases explicitly marked as deprecated or archived
                                if line.contains("DEPRECATED")
                                    || line.contains("Historical note")
                                    || line.contains("ARCHIVED")
                                {
                                    continue;
                                }

                                let filename =
                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                if filename.contains("-ARCHIVED.md") {
                                    continue;
                                }

                                failures.push(format!(
                                    "{}:{}: Found retired symbol '{}': Use {} instead. ({})",
                                    path.strip_prefix(root).unwrap_or(&path).display(),
                                    line_idx + 1,
                                    sym.id,
                                    sym.replacement,
                                    sym.rationale
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    if !failures.is_empty() {
        for f in &failures {
            eprintln!("{}", f);
        }
        return Err(anyhow!(
            "Found {} retired symbol violations in docs/",
            failures.len()
        ));
    }

    println!("retired-symbol-check OK");
    Ok(())
}
