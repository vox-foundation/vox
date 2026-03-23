//! Dynamic mdBook `SUMMARY.md` generator for Vox.
//! Recursively walks `docs/src/`, reads YAML frontmatter, and groups by section.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
struct Page {
    title: String,
    path: String,
    sort_order: i32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let check_mode = args.contains(&"--check".to_string());
    
    let docs_src = Path::new("docs/src");
    if !docs_src.exists() {
        eprintln!("Error: docs/src/ not found. Run from repo root.");
        std::process::exit(1);
    }

    let mut sections: BTreeMap<String, Vec<Page>> = BTreeMap::new();
    let mut root_pages = Vec::new();

    walk_dir(docs_src, docs_src, &mut sections, &mut root_pages);

    let mut output = String::from("# Summary\n\n");
    
    // Sort and add root pages (Home, CLI, etc)
    root_pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
    for page in root_pages {
        output.push_str(&format!("- [{}]({})\n", page.title, page.path));
    }
    output.push('\n');

    // Section ordering
    let section_order = ["Reference", "Architecture", "API Reference", "ADRs", "CI & Quality", "How-To Guides"];
    
    for section_name in section_order {
        if let Some(mut pages) = sections.remove(section_name) {
            output.push_str(&format!("# {}\n\n", section_name));
            pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
            for page in pages {
                output.push_str(&format!("- [{}]({})\n", page.title, page.path));
            }
            output.push('\n');
        }
    }

    // Any remaining sections
    for (name, mut pages) in sections {
        output.push_str(&format!("# {}\n\n", name));
        pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
        for page in pages {
            output.push_str(&format!("- [{}]({})\n", page.title, page.path));
        }
        output.push('\n');
    }

    let summary_path = docs_src.join("SUMMARY.md");
    if check_mode {
        let current = fs::read_to_string(&summary_path).unwrap_or_default();
        if current.trim() != output.trim() {
            println!("SUMMARY.md is out of date! Run vox-doc-pipeline to update.");
            std::process::exit(1);
        }
    } else {
        fs::write(&summary_path, output).expect("Failed to write SUMMARY.md");
        println!("Successfully generated SUMMARY.md with all pages.");
    }
}

fn walk_dir(root: &Path, dir: &Path, sections: &mut BTreeMap<String, Vec<Page>>, root_pages: &mut Vec<Page>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(root, &path, sections, root_pages);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let rel_path = path.strip_prefix(root).unwrap().to_str().unwrap().replace('\\', "/");
                if rel_path == "SUMMARY.md" { continue; }

                let content = fs::read_to_string(&path).unwrap_or_default();
                let (title, category, sort_order) = parse_frontmatter(&content, &path);

                let page = Page { title, path: rel_path, sort_order };
                if let Some(cat) = category {
                    sections.entry(cat).or_default().push(page);
                } else {
                    root_pages.push(page);
                }
            }
        }
    }
}

fn parse_frontmatter(content: &str, path: &Path) -> (String, Option<String>, i32) {
    let mut title = path.file_stem().unwrap().to_str().unwrap().replace('-', " ").replace('_', " ");
    let mut category = None;
    let mut sort_order = 100;

    if content.starts_with("---") {
        let after_dash = &content[3..];
        if let Some(end) = after_dash.find("---") {
            let yaml = &after_dash[..end];
            for line in yaml.lines() {
                let line = line.trim();
                if let Some(t) = line.strip_prefix("title:") {
                    title = t.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                } else if let Some(c) = line.strip_prefix("category:") {
                    let cat = c.trim().trim_matches(|c| c == '"' || c == '\'');
                    category = Some(match cat {
                        "architecture" => "Architecture",
                        "api" => "API Reference",
                        "adr" => "ADRs",
                        "ci" => "CI & Quality",
                        "how-to" => "How-To Guides",
                        "reference" => "Reference",
                        _ => cat,
                    }.to_string());
                } else if let Some(s) = line.strip_prefix("sort_order:") {
                    sort_order = s.trim().parse().unwrap_or(100);
                }
            }
        }
    } else {
        // Simple title case for unannotated files
        title = title_case(&title);
    }

    (title, category, sort_order)
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
