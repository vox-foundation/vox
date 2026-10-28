//! Deterministic Mermaid diagram syntax validator and structural auto-fixer.
//!
//! Validates fenced ```` ```mermaid ```` blocks across `docs/src/` and provides
//! deterministic auto-repair for common LLM Mermaid failure modes:
//! - Missing or invalid diagram header keyword (`flowchart`, `graph`, `sequenceDiagram`, etc.)
//! - Unbalanced `subgraph` / `end` blocks
//! - Unquoted node labels containing parentheses, braces, or HTML tags (`A[Foo (Bar)]` -> `A["Foo (Bar)"]`)
//! - Unquoted inline edge labels containing parentheses or comparison operators (`A -- Yes (x < y) --> B` -> `A -- "Yes (x < y)" --> B`)

use serde::Serialize;

/// Supported Mermaid diagram header keywords.
pub const SUPPORTED_DIAGRAM_HEADERS: &[&str] = &[
    "flowchart",
    "graph",
    "sequenceDiagram",
    "stateDiagram-v2",
    "stateDiagram",
    "classDiagram",
    "erDiagram",
    "xychart-beta",
    "gantt",
    "pie",
    "mindmap",
    "timeline",
    "gitGraph",
    "journey",
    "quadrantChart",
    "C4Context",
    "sankey-beta",
    "block-beta",
    "architecture-beta",
];

/// A single diagnostic finding inside a Mermaid diagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MermaidDiagnostic {
    /// 1-based line offset within the Mermaid block (or markdown file when adjusted).
    pub line: usize,
    /// Stable machine-readable diagnostic code.
    pub code: &'static str,
    /// Human- and LLM-readable explanation with remediation hint.
    pub message: String,
}

/// Result of validating and optionally repairing a Mermaid diagram block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MermaidValidationResult {
    pub valid: bool,
    pub diagram_type: Option<String>,
    pub diagnostics: Vec<MermaidDiagnostic>,
    pub repaired_code: String,
    pub was_repaired: bool,
}

/// Extract all ```` ```mermaid ```` fenced blocks from a Markdown document.
/// Returns `(fence_start_line_1_based, body_start_line_1_based, body_str)`.
#[must_use]
pub fn extract_mermaid_blocks(markdown: &str) -> Vec<(usize, usize, String)> {
    let mut blocks = Vec::new();
    let mut in_fence = false;
    let mut in_mermaid = false;
    let mut fence_backticks = 0usize;
    let mut fence_start_line = 0usize;
    let mut current_body = String::new();

    for (idx, line) in markdown.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();
        let ticks = trimmed.chars().take_while(|&c| c == '`').count();
        let after = trimmed[ticks..].trim();

        if !in_fence && ticks >= 3 {
            let lang = after
                .split(|c: char| c.is_whitespace() || c == ',')
                .next()
                .unwrap_or("");
            in_fence = true;
            fence_backticks = ticks;
            if lang.eq_ignore_ascii_case("mermaid") {
                in_mermaid = true;
                fence_start_line = line_no;
                current_body.clear();
            }
        } else if in_fence && ticks >= fence_backticks && after.is_empty() {
            if in_mermaid {
                blocks.push((
                    fence_start_line,
                    fence_start_line + 1,
                    std::mem::take(&mut current_body),
                ));
                in_mermaid = false;
            }
            in_fence = false;
            fence_backticks = 0;
        } else if in_mermaid {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    blocks
}

/// Validate a single Mermaid diagram source string and compute a deterministic repaired version.
#[must_use]
pub fn validate_and_repair_mermaid(code: &str) -> MermaidValidationResult {
    let mut diagnostics = Vec::new();
    let mut repaired_lines = Vec::new();
    let mut diagram_type: Option<String> = None;
    let mut subgraph_depth = 0usize;
    let mut last_subgraph_line = 1usize;

    for (idx, raw_line) in code.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();

        if trimmed.is_empty() || trimmed.starts_with("%%") {
            repaired_lines.push(raw_line.to_string());
            continue;
        }

        if diagram_type.is_none() {
            let first_token = trimmed.split_whitespace().next().unwrap_or("");
            if SUPPORTED_DIAGRAM_HEADERS.contains(&first_token) {
                diagram_type = Some(first_token.to_string());
                repaired_lines.push(raw_line.to_string());
            } else if first_token.eq_ignore_ascii_case("flowchart")
                || first_token.eq_ignore_ascii_case("graph")
            {
                let norm = first_token.to_ascii_lowercase();
                diagram_type = Some(norm.clone());
                let rest = &trimmed[first_token.len()..];
                diagnostics.push(MermaidDiagnostic {
                    line: line_no,
                    code: "mermaid-header-case",
                    message: format!("Mermaid header `{first_token}` must be lowercase `{norm}`."),
                });
                repaired_lines.push(format!("{norm}{rest}"));
            } else {
                diagnostics.push(MermaidDiagnostic {
                    line: line_no,
                    code: "mermaid-unknown-header",
                    message: format!(
                        "Unknown or missing Mermaid diagram type `{first_token}`; expected one of: flowchart, graph, sequenceDiagram, stateDiagram-v2, classDiagram, erDiagram, xychart-beta."
                    ),
                });
                // Default repair for headerless flowchart-like lines containing `-->`
                if trimmed.contains("-->") || trimmed.contains("---") {
                    diagram_type = Some("flowchart".to_string());
                    repaired_lines.push("flowchart TD".to_string());
                    let (fixed_line, mut line_diags) =
                        check_and_repair_flowchart_line(raw_line, line_no);
                    diagnostics.append(&mut line_diags);
                    repaired_lines.push(fixed_line);
                } else {
                    repaired_lines.push(raw_line.to_string());
                }
            }
            continue;
        }

        let is_flow = matches!(diagram_type.as_deref(), Some("flowchart" | "graph"));
        if is_flow {
            let first_word = trimmed.split_whitespace().next().unwrap_or("");
            if first_word == "subgraph" {
                subgraph_depth += 1;
                last_subgraph_line = line_no;
            } else if first_word == "end" {
                if subgraph_depth == 0 {
                    diagnostics.push(MermaidDiagnostic {
                        line: line_no,
                        code: "mermaid-unmatched-end",
                        message: "Encountered `end` without a matching `subgraph`.".to_string(),
                    });
                    // Omit unmatched `end` in repaired output
                    continue;
                }
                subgraph_depth -= 1;
            }

            let (fixed_line, mut line_diags) = check_and_repair_flowchart_line(raw_line, line_no);
            diagnostics.append(&mut line_diags);
            repaired_lines.push(fixed_line);
        } else {
            repaired_lines.push(raw_line.to_string());
        }
    }

    if diagram_type.is_none() {
        diagnostics.push(MermaidDiagnostic {
            line: 1,
            code: "mermaid-empty-diagram",
            message: "Mermaid diagram block is empty.".to_string(),
        });
    }

    if subgraph_depth > 0 {
        diagnostics.push(MermaidDiagnostic {
            line: last_subgraph_line,
            code: "mermaid-unclosed-subgraph",
            message: format!(
                "{subgraph_depth} unclosed `subgraph` block(s); add matching `end` line(s)."
            ),
        });
        for _ in 0..subgraph_depth {
            repaired_lines.push("  end".to_string());
        }
    }

    let mut repaired_code = repaired_lines.join("\n");
    if code.ends_with('\n') && !repaired_code.ends_with('\n') {
        repaired_code.push('\n');
    }
    let was_repaired = repaired_code != code;

    MermaidValidationResult {
        valid: diagnostics.is_empty(),
        diagram_type,
        diagnostics,
        repaired_code,
        was_repaired,
    }
}

/// Inspect and repair a single flowchart/graph line for:
/// 1. Unquoted `-- label -->` edge labels containing `(`, `)`, `<`, `>`, `[`, `]`, `{`, `}`
/// 2. Unquoted `id[label]` node labels containing `(`, `)`, `{`, `}`, or `<...>` HTML tags
fn check_and_repair_flowchart_line(
    raw_line: &str,
    line_no: usize,
) -> (String, Vec<MermaidDiagnostic>) {
    let mut diags = Vec::new();
    let mut line = raw_line.to_string();

    // 1. Check inline edge labels: ` -- text -->`
    if let Some((before_edge, edge_label, after_edge)) = split_inline_edge_label(&line) {
        let trimmed_label = edge_label.trim();
        let is_quoted = trimmed_label.starts_with('"') && trimmed_label.ends_with('"');
        if !is_quoted
            && trimmed_label
                .chars()
                .any(|c| matches!(c, '(' | ')' | '<' | '>' | '[' | ']' | '{' | '}'))
        {
            diags.push(MermaidDiagnostic {
                line: line_no,
                code: "mermaid-unquoted-edge-label",
                message: format!(
                    "Inline edge label `-- {trimmed_label} -->` contains special characters `()/[]/{{}}/<>`; quote it as `-- \"{}\" -->`.",
                    trimmed_label.replace('"', "'")
                ),
            });
            let safe_label = trimmed_label.replace('"', "'");
            line = format!("{before_edge}-- \"{safe_label}\" -->{after_edge}");
        }
    }

    // 2. Check square-bracket node labels: `NodeId[Label]` where `Label` is unquoted
    //    and contains `(`, `)`, `{`, `}`, or `<` / `>` HTML tags.
    let (repaired_nodes, mut node_diags) = repair_unquoted_square_node_labels(&line, line_no);
    diags.append(&mut node_diags);
    line = repaired_nodes;

    (line, diags)
}

/// Locate an inline `-- label -->` segment if present.
fn split_inline_edge_label(line: &str) -> Option<(&str, &str, &str)> {
    let arrow_pos = line.find("-->")?;
    let prefix = &line[..arrow_pos];
    // Find ` -- ` preceding `-->`
    let dash_pos = prefix.rfind(" -- ")?;
    let before = &line[..dash_pos + 1]; // includes leading space
    let label = &line[dash_pos + 4..arrow_pos];
    let after = &line[arrow_pos + 3..];
    if label.trim().is_empty() {
        return None;
    }
    Some((before, label, after))
}

/// Scan `line` for `Identifier[unquoted label]` where the label contains parentheses,
/// curly braces, or HTML tags (`<br/>`, etc.), and quote it as `Identifier["label"]`.
fn repair_unquoted_square_node_labels(
    line: &str,
    line_no: usize,
) -> (String, Vec<MermaidDiagnostic>) {
    let trimmed = line.trim_start();
    // Skip `subgraph id [Label]` headers — Mermaid allows `[Label]` on subgraph headers without quotes
    if trimmed.starts_with("subgraph ") {
        return (line.to_string(), Vec::new());
    }

    let mut diags = Vec::new();
    let mut out = String::with_capacity(line.len() + 8);
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] == '[' {
            // Check that the preceding character is part of a node identifier (alphanumeric or `_`)
            let prev_is_id = i > 0
                && (chars[i - 1].is_ascii_alphanumeric()
                    || chars[i - 1] == '_'
                    || chars[i - 1] == '-');
            if prev_is_id {
                // Find matching `]` on this line (respecting nested quotes if any)
                let mut j = i + 1;
                let mut in_quotes = false;
                let mut depth = 1usize;
                while j < chars.len() {
                    if chars[j] == '"' {
                        in_quotes = !in_quotes;
                    } else if !in_quotes {
                        if chars[j] == '[' {
                            depth += 1;
                        } else if chars[j] == ']' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                    j += 1;
                }

                if depth == 0 && j < chars.len() {
                    let inner: String = chars[i + 1..j].iter().collect();
                    let inner_trimmed = inner.trim();
                    let already_quoted = (inner_trimmed.starts_with('"')
                        && inner_trimmed.ends_with('"'))
                        || (inner_trimmed.starts_with("(\"") && inner_trimmed.ends_with("\")"))
                        || (inner_trimmed.starts_with('[') && inner_trimmed.ends_with(']'));

                    let has_hazard = !already_quoted
                        && inner_trimmed
                            .chars()
                            .any(|c| matches!(c, '(' | ')' | '{' | '}' | '<' | '>'));

                    if has_hazard {
                        let safe_inner = inner_trimmed.replace('"', "'");
                        diags.push(MermaidDiagnostic {
                            line: line_no,
                            code: "mermaid-unquoted-node-label",
                            message: format!(
                                "Node label `[{inner_trimmed}]` contains unquoted parentheses, braces, or HTML tags; quote it as `[\"{safe_inner}\"]`."
                            ),
                        });
                        out.push_str(&format!("[\"{safe_inner}\"]"));
                        i = j + 1;
                        continue;
                    }
                } else if depth > 0 {
                    diags.push(MermaidDiagnostic {
                        line: line_no,
                        code: "mermaid-unclosed-bracket",
                        message: "Unclosed `[` in node label on this line.".to_string(),
                    });
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    (out, diags)
}

/// Repair all broken ```` ```mermaid ```` blocks inside a full Markdown document.
/// Returns `(repaired_markdown, all_diagnostics_with_document_line_numbers)`.
#[must_use]
pub fn validate_and_repair_markdown_mermaid(
    markdown: &str,
) -> (String, Vec<MermaidDiagnostic>, usize) {
    let blocks = extract_mermaid_blocks(markdown);
    if blocks.is_empty() {
        return (markdown.to_string(), Vec::new(), 0);
    }

    let mut all_diags = Vec::new();
    let mut in_fence = false;
    let mut in_mermaid = false;
    let mut fence_backticks = 0usize;
    let mut fence_start_line = 0usize;
    let mut current_body = String::new();
    let mut out_lines: Vec<String> = Vec::new();

    for (idx, line) in markdown.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();
        let ticks = trimmed.chars().take_while(|&c| c == '`').count();
        let after = trimmed[ticks..].trim();

        if !in_fence && ticks >= 3 {
            let lang = after
                .split(|c: char| c.is_whitespace() || c == ',')
                .next()
                .unwrap_or("");
            in_fence = true;
            fence_backticks = ticks;
            if lang.eq_ignore_ascii_case("mermaid") {
                in_mermaid = true;
                fence_start_line = line_no;
                current_body.clear();
            }
            out_lines.push(line.to_string());
        } else if in_fence && ticks >= fence_backticks && after.is_empty() {
            if in_mermaid {
                let res = validate_and_repair_mermaid(&current_body);
                for mut d in res.diagnostics {
                    d.line = fence_start_line + d.line;
                    all_diags.push(d);
                }
                for rep_line in res.repaired_code.lines() {
                    out_lines.push(rep_line.to_string());
                }
                in_mermaid = false;
            }
            out_lines.push(line.to_string());
            in_fence = false;
            fence_backticks = 0;
        } else if in_mermaid {
            current_body.push_str(line);
            current_body.push('\n');
        } else {
            out_lines.push(line.to_string());
        }
    }

    let mut joined = out_lines.join("\n");
    if markdown.ends_with('\n') && !joined.ends_with('\n') {
        joined.push('\n');
    }
    (joined, all_diags, blocks.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_flowchart_passes_without_diagnostics() {
        let src = r#"flowchart LR
  A["`.vox` source"] --> B["Lexer / Parser"]
  B --> C["HIR (Lowered)"]
"#;
        let res = validate_and_repair_mermaid(src);
        assert!(res.valid, "diagnostics: {:?}", res.diagnostics);
        assert!(!res.was_repaired);
    }

    #[test]
    fn detects_and_repairs_unquoted_parens_and_edge_labels() {
        let src = r#"flowchart TD
    A[Start (Init)] --> B{Check?}
    B -- Yes (Iter < Max) --> C[Run <br/> Step]
"#;
        let res = validate_and_repair_mermaid(src);
        assert!(!res.valid);
        assert_eq!(res.diagnostics.len(), 3);
        assert!(res.was_repaired);

        let recheck = validate_and_repair_mermaid(&res.repaired_code);
        assert!(
            recheck.valid,
            "repaired code should be valid: {}\ndiags: {:?}",
            res.repaired_code, recheck.diagnostics
        );
        assert!(res.repaired_code.contains(r#"A["Start (Init)"]"#));
        assert!(res.repaired_code.contains(r#"-- "Yes (Iter < Max)" -->"#));
        assert!(res.repaired_code.contains(r#"C["Run <br/> Step"]"#));
    }

    #[test]
    fn detects_and_repairs_unclosed_subgraph() {
        let src = "flowchart TD\n  subgraph G [Group]\n    A --> B\n";
        let res = validate_and_repair_mermaid(src);
        assert!(!res.valid);
        assert_eq!(res.diagnostics[0].code, "mermaid-unclosed-subgraph");
        let recheck = validate_and_repair_mermaid(&res.repaired_code);
        assert!(recheck.valid);
    }

    #[test]
    fn repairs_markdown_document_with_mermaid_blocks() {
        let md = "# Title\n\n```mermaid\nflowchart LR\n  A[Node (1)] --> B[Node 2]\n```\n";
        let (repaired, diags, count) = validate_and_repair_markdown_mermaid(md);
        assert_eq!(count, 1);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 5);
        assert!(repaired.contains(r#"A["Node (1)"] --> B[Node 2]"#));
    }
}
