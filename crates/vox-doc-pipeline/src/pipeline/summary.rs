//! `SUMMARY.md` generation from `docs/src/` markdown frontmatter.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::types::Page;
use anyhow::Context;

pub(crate) const SECTION_ORDER: &[&str] = &[
    "Getting Started",
    "Journeys",
    "Tutorials",
    "How-To Guides",
    "Language Reference",
    "API Reference \u{2014} Keywords",
    "API Reference \u{2014} Decorators",
    "API Reference \u{2014} Crates",
    "Examples",
    "Explanations",
    "Architecture Decisions (ADRs)",
    "Architecture SSOTs",
    "Contributors",
    "CI & Quality",
    "Operations",
    "Reference",
];

pub(crate) fn walk_dir(
    root: &Path,
    dir: &Path,
    sections: &mut BTreeMap<String, Vec<Page>>,
    root_pages: &mut Vec<Page>,
    all_pages: &mut Vec<Page>,
) -> anyhow::Result<()> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(root, &path, sections, root_pages, all_pages)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let rel_path = path
                    .strip_prefix(root)
                    .with_context(|| {
                        format!(
                            "doc path {} is not under root {}",
                            path.display(),
                            root.display()
                        )
                    })?
                    .to_str()
                    .with_context(|| format!("non-UTF-8 doc path: {}", path.display()))?
                    .replace('\\', "/");
                if rel_path == "SUMMARY.md" {
                    continue;
                }

                let content =
                    vox_bounded_fs::read_utf8_path_capped(&path).unwrap_or_else(|_| String::new());
                let (
                    title,
                    category,
                    sort_order,
                    description,
                    _manual_last_updated,
                    status,
                    schema_type,
                ) = parse_frontmatter(&content, &path)?;

                let last_updated = get_git_last_updated(&path);

                let page = Page {
                    title: title.clone(),
                    path: rel_path.clone(),
                    sort_order,
                    description,
                    last_updated,
                    status: status.clone(),
                    schema_type: schema_type.clone(),
                };
                let page2 = Page {
                    title,
                    path: rel_path.clone(),
                    sort_order,
                    description: None,
                    last_updated: None,
                    status,
                    schema_type: None,
                };
                all_pages.push(page);
                let inferred_category = category.or_else(|| infer_category_from_path(&rel_path));
                if let Some(cat) = inferred_category {
                    sections.entry(cat).or_default().push(page2);
                } else {
                    root_pages.push(page2);
                }
            }
        }
    }
    Ok(())
}

fn parse_frontmatter(
    content: &str,
    path: &Path,
) -> anyhow::Result<(
    String,
    Option<String>,
    i32,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    let mut title = path
        .file_stem()
        .with_context(|| format!("path has no file stem: {}", path.display()))?
        .to_str()
        .with_context(|| format!("non-UTF-8 file stem: {}", path.display()))?
        .replace(['-', '_'], " ");
    let mut category = None;
    let mut sort_order = 100i32;
    let mut description: Option<String> = None;
    let mut last_updated: Option<String> = None;
    let mut status: Option<String> = None;
    let mut schema_type: Option<String> = None;
    let mut saw_title = false;

    if let Some(after_dash) = content.strip_prefix("---") {
        if let Some(end) = after_dash.find("---") {
            let yaml = &after_dash[..end];
            for line in yaml.lines() {
                let line = line.trim();
                if let Some(t) = line.strip_prefix("title:") {
                    title = t.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    saw_title = true;
                } else if let Some(d) = line.strip_prefix("description:") {
                    let raw = d.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    if !raw.is_empty() {
                        description = Some(raw);
                    }
                } else if let Some(lu) = line.strip_prefix("last_updated:") {
                    let raw = lu
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string();
                    if !raw.is_empty() {
                        last_updated = Some(raw);
                    }
                } else if let Some(c) = line.strip_prefix("category:") {
                    let cat = c.trim().trim_matches(|c| c == '"' || c == '\'');
                    category = Some(normalize_category(cat)?);
                } else if let Some(s) = line.strip_prefix("sort_order:") {
                    sort_order = s.trim().parse().unwrap_or(100);
                } else if let Some(st) = line.strip_prefix("status:") {
                    let raw = st
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string();
                    if !raw.is_empty() {
                        status = Some(raw);
                    }
                } else if let Some(st) = line.strip_prefix("schema_type:") {
                    let raw = st
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string();
                    if !raw.is_empty() {
                        schema_type = Some(raw);
                    }
                }
            }
        }
    } else {
        title = title_case(&title);
    }

    if !saw_title {
        if let Some(h1) = first_h1(content) {
            title = h1;
        }
    }

    Ok((
        title,
        category,
        sort_order,
        description,
        last_updated,
        status,
        schema_type,
    ))
}

fn get_git_last_updated(path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["log", "-1", "--format=%as", "--"])
        .arg(path)
        .output()
        .ok()?;

    if output.status.success() {
        let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !date.is_empty() {
            return Some(date);
        }
    }
    None
}

fn normalize_category(cat: &str) -> anyhow::Result<String> {
    let normalized = match cat {
        "getting-started" => "Getting Started",
        "journey" | "journeys" => "Journeys",
        "tutorial" | "tutorials" => "Tutorials",
        "how-to" => "How-To Guides",
        "ref" | "reference" => "Reference",
        "lang-ref" | "language-reference" => "Language Reference",
        "api-keyword" => "API Reference \u{2014} Keywords",
        "api-decorator" => "API Reference \u{2014} Decorators",
        "api-crate" => "API Reference \u{2014} Crates",
        "example" => "Examples",
        "explanation" => "Explanations",
        "adr" => "Architecture Decisions (ADRs)",
        "architecture" | "ssot" => "Architecture SSOTs",
        "contributor" | "contributors" => "Contributors",
        "ci" | "quality" => "CI & Quality",
        "operations" | "ops" => "Operations",
        other => anyhow::bail!(
            "unsupported docs category {:?}; use the canonical frontmatter vocabulary",
            other
        ),
    };
    Ok(normalized.to_string())
}

fn infer_category_from_path(rel_path: &str) -> Option<String> {
    let category = if rel_path == "index.md" {
        "Getting Started"
    } else if rel_path.starts_with("journeys/") {
        "Journeys"
    } else if rel_path.starts_with("tutorials/") {
        "Tutorials"
    } else if rel_path.starts_with("how-to/") {
        "How-To Guides"
    } else if rel_path.starts_with("explanation/") {
        "Explanations"
    } else if rel_path.starts_with("reference/") || rel_path.starts_with("ref/") {
        "Reference"
    } else if rel_path.starts_with("api/keywords/") {
        "API Reference \u{2014} Keywords"
    } else if rel_path.starts_with("api/decorators/") {
        "API Reference \u{2014} Decorators"
    } else if rel_path.starts_with("api/") {
        "API Reference \u{2014} Crates"
    } else if rel_path.starts_with("examples/") {
        "Examples"
    } else if rel_path.starts_with("adr/") {
        "Architecture Decisions (ADRs)"
    } else if rel_path.starts_with("architecture/") {
        "Architecture SSOTs"
    } else if rel_path.starts_with("contributors/") {
        "Contributors"
    } else if rel_path.starts_with("ci/") {
        "CI & Quality"
    } else if rel_path.starts_with("operations/") {
        "Operations"
    } else {
        return None;
    };
    Some(category.to_string())
}

fn first_h1(content: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

/// Fail fast when mdBook would error: each `](path)` may appear at most once in `SUMMARY.md`.
pub(crate) fn assert_summary_link_targets_unique(summary: &str) -> anyhow::Result<()> {
    use std::collections::HashMap;

    let mut seen: HashMap<&str, usize> = HashMap::new();
    for (idx, raw_line) in summary.lines().enumerate() {
        let line_no = idx + 1;
        let s = raw_line.trim_start();
        let Some(rest) = s.strip_prefix("- [") else {
            continue;
        };
        let Some(title_end) = rest.find("](") else {
            continue;
        };
        let after = &rest[title_end + 2..];
        let Some(path_end) = after.find(')') else {
            continue;
        };
        let target = after[..path_end].trim();
        if target.is_empty() {
            continue;
        }
        if let Some(&first_line) = seen.get(target) {
            anyhow::bail!(
                "SUMMARY.md: duplicate mdBook chapter path {target:?} (line {first_line}, again line {line_no})"
            );
        }
        seen.insert(target, line_no);
    }
    Ok(())
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>()
                        + chars.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod summary_path_tests {
    use super::{assert_summary_link_targets_unique, first_h1};

    #[test]
    fn duplicate_targets_error() {
        let s = r"# Summary
- [A](foo.md)
- [B](foo.md)
";
        assert!(assert_summary_link_targets_unique(s).is_err());
    }

    #[test]
    fn unique_targets_ok() {
        let s = r"# Summary
- [A](a.md)
- [B](b.md)
";
        assert_summary_link_targets_unique(s).unwrap();
    }

    #[test]
    fn inline_code_line_not_confused_with_fence() {
        let s = "`x` ok";
        // Smoke: parser does not treat as summary links
        assert_summary_link_targets_unique(s).unwrap();
    }

    #[test]
    fn extracts_first_h1_when_present() {
        let s = "---\ncategory: \"reference\"\n---\n\n# Crate API: `vox-cli`\n";
        assert_eq!(first_h1(s).as_deref(), Some("Crate API: `vox-cli`"));
    }
}
