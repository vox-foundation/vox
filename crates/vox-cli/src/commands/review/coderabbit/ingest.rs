//! Fetch PR review comments and normalize CodeRabbit-shaped markdown.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;
use vox_git::GitBridge;

use super::github::{github_token, parse_github_owner_repo};

/// Normalized review item schema (ingestion output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedReviewItem {
    pub source_pr: u64,
    pub comment_id: u64,
    pub file_path: String,
    pub line: usize,
    pub line_end: Option<usize>,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub details: String,
    pub llm_prompt: Option<String>,
    pub suggested_fix: Option<String>,
    pub dedup_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhReviewComment {
    id: u64,
    path: Option<String>,
    line: Option<u64>,
    #[serde(rename = "original_line")]
    original_line: Option<u64>,
    body: Option<String>,
}

const GITHUB_PER_PAGE_DEFAULT: u32 = 100;

fn github_per_page() -> u32 {
    std::env::var("CODERABBIT_GITHUB_PER_PAGE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(GITHUB_PER_PAGE_DEFAULT)
        .clamp(1, 100)
}

async fn fetch_github_paginated(
    client: &reqwest::Client,
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
) -> Result<Vec<GhReviewComment>> {
    let per_page = github_per_page();
    let mut all = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "https://api.github.com/repos/{owner}/{repo}/{path}?per_page={per_page}&page={page}"
        );
        let resp = client
            .get(&url)
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            // required User-Agent for GitHub APIs
            .header("User-Agent", "vox-cli")
            .send()
            .await
            .context(format!("GET {path}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API {status}: {text}");
        }

        let comments: Vec<GhReviewComment> = resp.json().await.context("Parse JSON")?;
        let n = comments.len();
        if n == 0 {
            break;
        }
        all.extend(comments);
        if n < per_page as usize {
            break;
        }
        page += 1;
    }

    Ok(all)
}

async fn fetch_all_pr_comments(
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<Vec<GhReviewComment>> {
    let client = reqwest::Client::new();

    // 1. Inline review comments
    let mut all = fetch_github_paginated(
        &client,
        token,
        owner,
        repo,
        &format!("pulls/{pr_number}/comments"),
    )
    .await?;

    // 2. Issue comments (Main PR discussions)
    let issues = fetch_github_paginated(
        &client,
        token,
        owner,
        repo,
        &format!("issues/{pr_number}/comments"),
    )
    .await?;
    all.extend(issues);

    // 3. Top-level submitted reviews (Body content)
    let reviews = fetch_github_paginated(
        &client,
        token,
        owner,
        repo,
        &format!("pulls/{pr_number}/reviews"),
    )
    .await?;
    all.extend(reviews);

    Ok(all)
}

fn parse_coderabbit_body(body: &str) -> (String, String, Option<String>, String) {
    let mut category = "style".to_string();
    let mut title = String::new();
    let mut details = String::new();
    let mut suggested_fix: Option<String> = None;

    let body = body.trim();
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("**") {
            let rest = line.trim_start_matches('*').trim_end_matches('*').trim();
            let lower = rest.to_lowercase();
            if lower == "suggested fix" || lower == "suggestion" || lower == "suggested change" {
                i += 1;
                let mut fix_lines = Vec::new();
                while i < lines.len() && !lines[i].trim().starts_with("**") {
                    fix_lines.push(lines[i]);
                    i += 1;
                }
                let fix = fix_lines.join("\n").trim().to_string();
                if !fix.is_empty() {
                    suggested_fix = Some(fix);
                }
                continue;
            }
            if matches!(
                lower.as_str(),
                "security"
                    | "performance"
                    | "logic"
                    | "error handling"
                    | "style"
                    | "dead code"
                    | "dependencies"
            ) {
                category = rest.to_lowercase().replace(' ', "_");
                i += 1;
                if i < lines.len() && !lines[i].trim().starts_with("**") {
                    let t = lines[i].trim();
                    if !t.is_empty() {
                        title = t.to_string();
                    }
                    i += 1;
                }
                let mut detail_lines = Vec::new();
                while i < lines.len() && !lines[i].trim().starts_with("**") {
                    detail_lines.push(lines[i]);
                    i += 1;
                }
                let d = detail_lines.join("\n").trim().to_string();
                if !d.is_empty() {
                    details = d;
                }
                continue;
            }
            if rest.is_empty() {
                i += 1;
                continue;
            }
        }
        if title.is_empty() && !line.trim().is_empty() && !line.trim().starts_with('#') {
            title = line.trim().to_string();
        } else if !line.trim().is_empty() {
            if !details.is_empty() {
                details.push('\n');
            }
            details.push_str(line);
        }
        i += 1;
    }

    if title.is_empty() && !details.is_empty() {
        title = details.lines().next().unwrap_or("").trim().to_string();
        if title.len() > 80 {
            title = format!("{}...", &title[..77]);
        }
    }
    if title.is_empty() {
        title = "Code review finding".to_string();
    }

    (category, title, suggested_fix, details)
}

fn infer_severity(category: &str, details: &str) -> (String, f64, String) {
    let cat = category.to_lowercase();
    let (base_severity, mut confidence, reason) = match cat.as_str() {
        "security" => ("error", 0.9, "category: security".to_string()),
        "logic" | "error_handling" => ("warning", 0.85, format!("category: {cat}")),
        "performance" => ("warning", 0.8, "category: performance".to_string()),
        "dead_code" => ("info", 0.75, "category: dead_code".to_string()),
        "dependencies" => ("warning", 0.7, "category: dependencies".to_string()),
        "style" => ("info", 0.6, "category: style".to_string()),
        _ => ("info", 0.4, format!("category: {cat} (unknown)")),
    };
    if details.contains("```") || details.contains("suggested") {
        confidence = (confidence + 0.1_f64).min(1.0);
    }
    let (severity, final_reason) = if confidence < 0.5 {
        (
            "needs_review".to_string(),
            "low confidence, manual review".to_string(),
        )
    } else {
        (base_severity.to_string(), reason)
    };
    (severity, confidence, final_reason)
}

fn dedup_hash(path: &str, line: usize, title: &str, details: &str) -> String {
    let mut data = Vec::new();
    data.extend_from_slice(path.as_bytes());
    data.extend_from_slice(&line.to_le_bytes());
    data.extend_from_slice(title.as_bytes());
    data.extend_from_slice(details.as_bytes());
    let h = blake3::hash(&data);
    h.to_hex().to_string()
}

fn extract_nitpicks(body: &str) -> Vec<(String, usize, String, String)> {
    let mut nitpicks = Vec::new();
    let mut in_nitpicks = false;
    let mut table_start = false;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Nitpicks") {
            in_nitpicks = true;
            continue;
        }
        if in_nitpicks && trimmed.starts_with("## ") {
            break; // Next section
        }
        if in_nitpicks && trimmed.starts_with('|') {
            if trimmed.contains("---") || trimmed.contains("File") {
                table_start = true;
                continue;
            }
            if table_start {
                let columns: Vec<&str> = trimmed.split('|').collect();
                if columns.len() >= 4 {
                    let file = columns[1].trim().trim_matches('`').to_string();
                    let line_num = columns[2].trim().parse::<usize>().unwrap_or(0);
                    let suggestion = columns[3].trim().to_string();
                    if !file.is_empty() {
                        nitpicks.push((
                            file,
                            line_num,
                            "CodeRabbit Nitpick".to_string(),
                            suggestion,
                        ));
                    }
                }
            }
        }
    }
    nitpicks
}

/// Ingest PR review comments and produce normalized items.
pub async fn ingest_pr(pr_number: u64, path: &Path) -> Result<Vec<NormalizedReviewItem>> {
    let bridge = GitBridge::open(path).context("Open git repo")?;
    let remote_url = bridge.remote_url().context("Get remote URL")?;
    let (owner, repo) =
        parse_github_owner_repo(&remote_url).context("Parse owner/repo from remote URL")?;

    let token = github_token()?;
    let comments = fetch_all_pr_comments(&token, &owner, &repo, pr_number).await?;

    let mut items = Vec::new();
    for c in comments {
        let body = c.body.as_deref().unwrap_or("").trim();
        if body.is_empty() {
            continue;
        }
        debug!(
            "parse_coderabbit_body input (comment_id={}): {:?}",
            c.id, body
        );

        // 1. Extract any table-based nitpicks
        let nitpicks = extract_nitpicks(body);
        for (file, ln, title, details) in nitpicks {
            let hash = dedup_hash(&file, ln, &title, &details);
            items.push(NormalizedReviewItem {
                source_pr: pr_number,
                comment_id: c.id,
                file_path: file,
                line: ln,
                line_end: None,
                severity: "info".to_string(),
                category: "nitpick".to_string(),
                title: title.clone(),
                details: details.clone(),
                llm_prompt: None,
                suggested_fix: None,
                dedup_hash: hash,
                raw_body: None,
                confidence: Some(0.9),
                severity_reason: Some("category: nitpick".to_string()),
            });
        }

        let file_path = c.path.unwrap_or_else(|| "global".to_string());
        let line = c.line.or(c.original_line).unwrap_or(0) as usize;

        // 2. Global walkthroughs / PR Summaries
        if file_path == "global" && body.contains("## Walkthrough") {
            let hash = dedup_hash(
                &file_path,
                line,
                "Walkthrough",
                "Walkthrough summary included",
            );
            items.push(NormalizedReviewItem {
                source_pr: pr_number,
                comment_id: c.id,
                file_path: file_path.clone(),
                line,
                line_end: None,
                severity: "info".to_string(),
                category: "walkthrough".to_string(),
                title: "CodeRabbit Walkthrough".to_string(),
                details: body.to_string(),
                llm_prompt: None,
                suggested_fix: None,
                dedup_hash: hash,
                raw_body: Some(body.to_string()),
                confidence: Some(1.0),
                severity_reason: Some("category: walkthrough".to_string()),
            });
            continue;
        }

        // 3. Standard parsing
        let (category, title, suggested_fix, details) = parse_coderabbit_body(body);
        let (severity, confidence, severity_reason) = infer_severity(&category, &details);

        let hash = dedup_hash(&file_path, line, &title, &details);

        items.push(NormalizedReviewItem {
            source_pr: pr_number,
            comment_id: c.id,
            file_path,
            line,
            line_end: None,
            severity,
            category,
            title,
            details,
            llm_prompt: None,
            suggested_fix,
            dedup_hash: hash,
            raw_body: None,
            confidence: Some(confidence),
            severity_reason: Some(severity_reason),
        });
    }

    Ok(items)
}

/// Run ingest command: fetch, normalize, output, optionally persist (stub unless Codex wired).
pub async fn run_ingest(
    pr_number: u64,
    output: Option<&Path>,
    persist: bool,
    path: &Path,
) -> Result<()> {
    let items = ingest_pr(pr_number, path).await?;

    let json = serde_json::to_string_pretty(&items).context("Serialize normalized items")?;

    match output {
        Some(p) => {
            std::fs::write(p, &json).with_context(|| format!("Write to {}", p.display()))?;
            eprintln!("Ingested {} items to {}", items.len(), p.display());
        }
        None => {
            println!("{json}");
        }
    }

    if persist {
        let cr_dir = path.join(".coderabbit");
        std::fs::create_dir_all(&cr_dir).ok();
        let cache_path = cr_dir.join("ingested_findings.json");
        std::fs::write(&cache_path, &json)
            .with_context(|| format!("Write local cache {}", cache_path.display()))?;
        eprintln!(
            "Persisted {} items to {}",
            items.len(),
            cache_path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coderabbit_body_suggested_fix() {
        let body = "**security**\n\nPotential XSS.\n\n**Suggested fix**\nUse `escape_html()`.";
        let (cat, title, fix, details) = parse_coderabbit_body(body);
        assert!(cat.contains("security") || cat == "security");
        assert!(title.contains("XSS") || details.contains("XSS"));
        assert!(fix.is_some());
        assert!(fix.unwrap().contains("escape_html"));
    }

    #[test]
    fn dedup_hash_deterministic() {
        let h1 = dedup_hash("a.rs", 10, "title", "details");
        let h2 = dedup_hash("a.rs", 10, "title", "details");
        assert_eq!(h1, h2);
    }
}
