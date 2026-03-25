//! `SUMMARY.md` generation from `docs/src/` markdown frontmatter.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::types::Page;

pub(crate) const SECTION_ORDER: &[&str] = &[
    "Getting Started",
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
    "CI & Quality",
    "Reference",
];

pub(crate) fn walk_dir(
    root: &Path,
    dir: &Path,
    sections: &mut BTreeMap<String, Vec<Page>>,
    root_pages: &mut Vec<Page>,
    all_pages: &mut Vec<Page>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(root, &path, sections, root_pages, all_pages);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let rel_path = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");
                if rel_path == "SUMMARY.md" {
                    continue;
                }

                let content = fs::read_to_string(&path).unwrap_or_default();
                let (title, category, sort_order, description, last_updated) =
                    parse_frontmatter(&content, &path);

                let page = Page {
                    title,
                    path: rel_path.clone(),
                    sort_order,
                    description,
                    last_updated,
                };
                all_pages.push(page);

                let page2 = Page {
                    title: all_pages.last().unwrap().title.clone(),
                    path: rel_path,
                    sort_order: all_pages.last().unwrap().sort_order,
                    description: None,
                    last_updated: None,
                };
                if let Some(cat) = category {
                    sections.entry(cat).or_default().push(page2);
                } else {
                    root_pages.push(page2);
                }
            }
        }
    }
}

fn parse_frontmatter(
    content: &str,
    path: &Path,
) -> (String, Option<String>, i32, Option<String>, Option<String>) {
    let mut title = path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .replace(['-', '_'], " ");
    let mut category = None;
    let mut sort_order = 100i32;
    let mut description: Option<String> = None;
    let mut last_updated: Option<String> = None;

    if let Some(after_dash) = content.strip_prefix("---") {
        if let Some(end) = after_dash.find("---") {
            let yaml = &after_dash[..end];
            for line in yaml.lines() {
                let line = line.trim();
                if let Some(t) = line.strip_prefix("title:") {
                    title = t.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
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
                    category = Some(
                        match cat {
                            "getting-started" => "Getting Started",
                            "tutorial" => "Tutorials",
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
                            "ci" | "quality" => "CI & Quality",
                            _ => cat,
                        }
                        .to_string(),
                    );
                } else if let Some(s) = line.strip_prefix("sort_order:") {
                    sort_order = s.trim().parse().unwrap_or(100);
                }
            }
        }
    } else {
        title = title_case(&title);
    }

    (title, category, sort_order, description, last_updated)
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
