//! `.vox` source code corpus extractor for Populi training data.
//!
//! Walks `crates/vox-parser/tests/golden/**/*.vox`, `crates/vox-integration-tests/**/*.vox`,
//! and any other `.vox` files under the repo root, extracting complete file contents as
//! `prompt`/`response` training pairs.
//!
//! ## Extraction strategy
//! - **Per-file extraction**: each `.vox` file becomes one training pair.
//! - **Prompt**: derived from file-level `#` comments or auto-generated from filename.
//! - **Response**: the complete file contents (valid Vox syntax).
//! - **Category**: inferred from the file path and content constructs.
//! - **Per-block extraction**: each top-level construct (fn, actor, workflow, etc.) becomes
//!   a separate training pair with a targeted prompt.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde_json::json;

/// Configuration for Vox source extraction.
#[derive(Debug, Clone)]
pub struct ExtractVoxConfig {
    /// Root directory to walk (usually the repo root).
    pub root: PathBuf,
    /// Minimum number of non-comment, non-empty lines to include a file.
    pub min_content_lines: usize,
    /// Maximum number of pairs to emit (0 = unlimited).
    pub limit: usize,
    /// Default quality rating for Vox source pairs.
    pub default_rating: u8,
}

impl Default for ExtractVoxConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            min_content_lines: 2,
            limit: 0,
            default_rating: 5, // higher than Rust source — Vox is the target language
        }
    }
}

/// One extracted training pair from a `.vox` source.
#[derive(Debug, Clone)]
pub struct VoxTrainingPair {
    /// Source file path (relative to root).
    pub source_path: PathBuf,
    /// Inferred construct category.
    pub category: String,
    /// The prompt (doc comment text or generated imperative).
    pub prompt: String,
    /// The Vox source code block.
    pub response: String,
    /// Quality rating.
    pub rating: u8,
}

impl VoxTrainingPair {
    /// Serialize to a JSONL row compatible with `vox_tensor::data::TrainingPair`.
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        let v = json!({
            "prompt": self.prompt,
            "response": self.response,
            "category": self.category,
            "rating": self.rating,
            "source": self.source_path.display().to_string(),
            "format": "vox_source",
        });
        v.to_string()
    }
}

/// Prompt templates for different construct types discovered in Vox files.
const CONSTRUCT_PROMPTS: &[(&str, &[&str])] = &[
    ("fn", &[
        "Write a Vox function called `{name}`",
        "Implement the `{name}` function in Vox",
        "Show me a Vox function named `{name}`",
    ]),
    ("actor", &[
        "Define a Vox actor called `{name}`",
        "Create an actor named `{name}` in Vox with state and message handlers",
    ]),
    ("workflow", &[
        "Write a durable Vox workflow called `{name}`",
        "Implement the `{name}` workflow with retry semantics in Vox",
    ]),
    ("activity", &[
        "Define a Vox activity called `{name}`",
        "Write an activity function named `{name}` in Vox",
    ]),
    ("component", &[
        "Create a Vox UI component called `{name}`",
        "Write a component function named `{name}` that returns Element",
    ]),
    ("table", &[
        "Define a Vox @table schema called `{name}`",
        "Write a database table definition named `{name}` in Vox",
    ]),
    ("type", &[
        "Define a Vox type called `{name}`",
        "Create a tagged union type named `{name}` in Vox",
    ]),
    ("query", &[
        "Write a Vox @query function called `{name}`",
        "Implement a read-only data query named `{name}` in Vox",
    ]),
    ("mutation", &[
        "Write a Vox @mutation function called `{name}`",
        "Implement a data mutation named `{name}` in Vox",
    ]),
    ("mcp_tool", &[
        "Define an MCP tool called `{name}` in Vox",
        "Write a @mcp.tool function named `{name}`",
    ]),
    ("test", &[
        "Write a Vox test called `{name}`",
        "Create a unit test named `{name}` in Vox",
    ]),
];

/// Extract the first comment block from the top of a `.vox` file as the prompt.
fn extract_file_doc(source: &str) -> Option<String> {
    let mut doc_lines = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            let text = trimmed
                .trim_start_matches('#')
                .trim_start_matches("//")
                .trim();
            if !text.is_empty() {
                doc_lines.push(text.to_string());
            }
        } else if trimmed.is_empty() {
            if !doc_lines.is_empty() {
                break; // End of leading comment block
            }
        } else {
            break; // Non-comment, non-empty line
        }
    }
    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join(" "))
    }
}

/// Infer a category from filename and content keywords.
fn infer_vox_category(path: &Path, source: &str) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

    // Check for specific construct keywords in source
    let content_lower = source.to_lowercase();
    if content_lower.contains("@workflow") || content_lower.contains("workflow fn") {
        return "workflow".to_string();
    }
    if content_lower.contains("actor ") {
        return "actor".to_string();
    }
    if content_lower.contains("@mcp.tool") {
        return "mcp_tool".to_string();
    }
    if content_lower.contains("@table") {
        return "table".to_string();
    }
    if content_lower.contains("@component") || content_lower.contains("component fn") {
        return "component".to_string();
    }
    if content_lower.contains("@query") {
        return "query".to_string();
    }
    if content_lower.contains("@mutation") {
        return "mutation".to_string();
    }

    // Fall back to filename heuristics
    match stem {
        s if s.contains("test") => "test".to_string(),
        s if s.contains("workflow") || s.contains("durable") => "workflow".to_string(),
        s if s.contains("agent") => "agent_def".to_string(),
        s if s.contains("component") || s.contains("dashboard") => "component".to_string(),
        s if s.contains("server") => "server_fn".to_string(),
        s if s.contains("route") => "http_route".to_string(),
        _ => "function".to_string(), // Default: most .vox files contain functions
    }
}

/// Extract individual construct blocks from Vox source.
/// Returns (construct_type, name, source_block) triples.
fn extract_construct_blocks(source: &str) -> Vec<(&'static str, String, String)> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Detect construct starts
        let (construct_type, name) = if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            let n = extract_vox_name(trimmed, "fn ");
            ("fn", n)
        } else if trimmed.starts_with("actor ") {
            let n = extract_vox_name(trimmed, "actor ");
            ("actor", n)
        } else if trimmed.starts_with("@workflow") || trimmed.contains("workflow fn") {
            let n = extract_vox_name(trimmed, "fn ");
            ("workflow", n)
        } else if trimmed.starts_with("@table") {
            let n = extract_vox_type_name(trimmed);
            ("table", n)
        } else if trimmed.starts_with("type ") {
            let n = extract_vox_name(trimmed, "type ");
            ("type", n)
        } else if trimmed.starts_with("@component") || trimmed.starts_with("component fn") {
            let n = extract_vox_name(trimmed, "fn ");
            ("component", n)
        } else if trimmed.starts_with("@mcp.tool") {
            let n = extract_vox_name(trimmed, "fn ");
            ("mcp_tool", n)
        } else if trimmed.starts_with("@query") {
            let n = extract_vox_name(trimmed, "fn ");
            ("query", n)
        } else if trimmed.starts_with("@mutation") {
            let n = extract_vox_name(trimmed, "fn ");
            ("mutation", n)
        } else if trimmed.starts_with("@test") || trimmed.starts_with("test fn") {
            let n = extract_vox_name(trimmed, "fn ");
            ("test", n)
        } else {
            i += 1;
            continue;
        };

        // Collect the construct block by brace depth
        let block_start = i;
        let mut depth: i32 = 0;
        let mut found_open = false;
        let mut block_lines: Vec<&str> = Vec::new();

        while i < lines.len() {
            let line = lines[i];
            block_lines.push(line);
            for c in line.chars() {
                if c == '{' {
                    depth += 1;
                    found_open = true;
                } else if c == '}' {
                    depth -= 1;
                }
            }
            i += 1;
            if found_open && depth <= 0 {
                break;
            }
            // Also stop if we hit a new top-level construct (no braces found after 2 lines)
            if !found_open && i > block_start + 2 && !lines.get(i).map_or(true, |l| l.starts_with(' ') || l.starts_with('\t') || l.trim().is_empty()) {
                break;
            }
        }

        if block_lines.len() >= 2 {
            blocks.push((construct_type, name, block_lines.join("\n")));
        }
    }

    blocks
}

fn extract_vox_name(line: &str, after_kw: &str) -> String {
    if let Some(pos) = line.find(after_kw) {
        let rest = &line[pos + after_kw.len()..];
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if end > 0 {
            return rest[..end].to_string();
        }
    }
    "unnamed".to_string()
}

fn extract_vox_type_name(line: &str) -> String {
    if let Some(pos) = line.find("type ") {
        let rest = &line[pos + 5..];
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if end > 0 {
            return rest[..end].to_string();
        }
    }
    "unnamed".to_string()
}

/// Get a prompt template for a construct type and name.
fn construct_prompt(construct_type: &str, name: &str, seed: usize) -> String {
    for &(ct, templates) in CONSTRUCT_PROMPTS {
        if ct == construct_type {
            let tmpl = templates[seed % templates.len()];
            return tmpl.replace("{name}", name);
        }
    }
    format!("Write a Vox {construct_type} called `{name}`")
}

/// Check if content contains frontmatter indicating it should be excluded from training.
fn is_eligible_for_training(content: &str) -> bool {
    // Vox golden examples have frontmatter commented out with //
    if content.contains("training_eligible: false") || content.contains("training_eligible:false") {
        return false;
    }
    if content.contains("status: deprecated") || content.contains("status: \"deprecated\"") || content.contains("status: 'deprecated'") {
        return false;
    }
    true
}

/// Extract all training pairs from a single `.vox` file.
pub fn extract_from_vox_file(
    path: &Path,
    config: &ExtractVoxConfig,
) -> anyhow::Result<Vec<VoxTrainingPair>> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;

    if !is_eligible_for_training(&source) {
        return Ok(Vec::new());
    }

    let content_lines = source
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && !t.starts_with("//")
        })
        .count();

    if content_lines < config.min_content_lines {
        return Ok(Vec::new());
    }

    let category = infer_vox_category(path, &source);
    let mut pairs = Vec::new();

    // 1. Whole-file pair
    let file_prompt = extract_file_doc(&source).unwrap_or_else(|| {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("program");
        format!(
            "Write a complete Vox program that implements {}",
            stem.replace('_', " ")
        )
    });

    pairs.push(VoxTrainingPair {
        source_path: path.to_path_buf(),
        category: format!("vox_{category}"),
        prompt: file_prompt,
        response: source.clone(),
        rating: config.default_rating,
    });

    // 2. Per-construct pairs
    let blocks = extract_construct_blocks(&source);
    for (i, (construct_type, name, block)) in blocks.iter().enumerate() {
        if block.lines().count() < 2 {
            continue;
        }
        let prompt = construct_prompt(construct_type, name, i);
        pairs.push(VoxTrainingPair {
            source_path: path.to_path_buf(),
            category: format!("vox_{construct_type}"),
            prompt: prompt.clone(),
            response: block.clone(),
            rating: config.default_rating,
        });

        // "explain-from-code" pair
        if i % 2 == 0 {
            let explain_prompt = format!("Explain the purpose and function of the following Vox {} snippet:\n```vox\n{}\n```", construct_type, block);
            let explain_response = format!("This is a Vox {} named `{}`. It demonstrates standard Vox syntax, explicit typing, and safe state management.", construct_type, name);
            pairs.push(VoxTrainingPair {
                source_path: path.to_path_buf(),
                category: format!("vox_{construct_type}_explain"),
                prompt: explain_prompt,
                response: explain_response,
                rating: config.default_rating,
            });
        }

        // Compact form pair
        if i % 3 == 0 {
            let compact_prompt = format!("{} (compact, no whitespace)", prompt);
            let compact_response = crate::corpus::preflight::to_compact(block);
            pairs.push(VoxTrainingPair {
                source_path: path.to_path_buf(),
                category: format!("vox_{construct_type}_compact"),
                prompt: compact_prompt,
                response: compact_response,
                rating: config.default_rating,
            });
        }
    }

    if config.limit > 0 {
        pairs.truncate(config.limit);
    }

    Ok(pairs)
}

/// Walk a directory tree and extract pairs from all `.vox` files.
pub fn walk_and_extract_vox(config: &ExtractVoxConfig) -> anyhow::Result<Vec<VoxTrainingPair>> {
    let mut all = Vec::new();
    walk_vox_dir(&config.root, config, &mut all)?;
    Ok(all)
}

fn walk_vox_dir(
    dir: &Path,
    config: &ExtractVoxConfig,
    out: &mut Vec<VoxTrainingPair>,
) -> anyhow::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | "node_modules" | ".vox" | "vox-vscode") {
                continue;
            }
            walk_vox_dir(&path, config, out)?;
        } else if path.extension().is_some_and(|e| e == "vox") {
            match extract_from_vox_file(&path, config) {
                Ok(mut pairs) => {
                    if config.limit > 0 {
                        let remaining = config.limit.saturating_sub(out.len());
                        pairs.truncate(remaining);
                    }
                    out.extend(pairs);
                    if config.limit > 0 && out.len() >= config.limit {
                        return Ok(());
                    }
                }
                Err(e) => {
                    eprintln!("  [vox extract] skip {}: {e}", path.display());
                }
            }
        }
    }
    Ok(())
}

/// Write extracted Vox pairs to a JSONL file (truncating).
pub fn write_vox_to_jsonl(pairs: &[VoxTrainingPair], output: &Path) -> anyhow::Result<usize> {
    use std::io::Write;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output)
        .with_context(|| format!("open output {}", output.display()))?;
    for pair in pairs {
        writeln!(f, "{}", pair.to_jsonl())?;
    }
    Ok(pairs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_VOX: &str = r#"# agent.vox
# Example agent definition with tools and memory

@table type AgentMemory {
    session_id: str
    context: str
}

fn SupportBot(query: str, session: str) to str {
    let past = db.agent_memory.find(session)
    let response = "Based on " + past.context + " -> " + query
    db.agent_memory.insert(AgentMemory(session, query))
    ret response
}
"#;

    #[test]
    fn extracts_whole_file_pair() {
        let cfg = ExtractVoxConfig {
            min_content_lines: 1,
            ..Default::default()
        };
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("agent.vox");
        std::fs::write(&p, SAMPLE_VOX).unwrap();
        let pairs = extract_from_vox_file(&p, &cfg).unwrap();
        assert!(!pairs.is_empty(), "should extract at least one pair");
        assert!(pairs[0].response.contains("SupportBot"));
    }

    #[test]
    fn extracts_construct_blocks() {
        let blocks = extract_construct_blocks(SAMPLE_VOX);
        assert!(
            blocks.iter().any(|(_, name, _)| name == "AgentMemory"),
            "should find @table type AgentMemory"
        );
        assert!(
            blocks.iter().any(|(_, name, _)| name == "SupportBot"),
            "should find fn SupportBot"
        );
    }

    #[test]
    fn infers_category_from_content() {
        let category = infer_vox_category(Path::new("test.vox"), SAMPLE_VOX);
        assert_eq!(category, "table"); // @table is the first construct keyword found
    }

    #[test]
    fn extract_file_doc_works() {
        let doc = extract_file_doc(SAMPLE_VOX);
        assert!(doc.is_some());
        assert!(doc.unwrap().contains("agent"));
    }
}
