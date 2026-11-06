//! Recursive symbol extraction and Markdown table rendering for OpenWiki Crate API pages (`OPENWIKI-API-01..04`).

use std::fs;
use std::path::Path;

use super::openwiki::{
    DeepCrateSymbol, LayerCrateEntry, SUBSYSTEM_LAYERS, extract_deep_crate_symbols,
    render_crate_dependency_diagram,
};

fn escape_table_cell(s: &str) -> String {
    s.replace('\r', "")
        .replace('\n', " ")
        .replace('|', "\\|")
        .trim()
        .to_string()
}

/// Extract top-level `pub mod <name>;` declarations from a crate root (`lib.rs` or `main.rs`).
pub fn extract_declared_pub_mods(source: &str) -> Vec<String> {
    let mut mods = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub mod ")
            && rest.ends_with(';')
            && !rest.contains('{')
        {
            let name = rest.trim_end_matches(';').trim();
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                mods.push(name.to_string());
            }
        }
    }
    mods.sort();
    mods.dedup();
    mods
}

/// Walk `entry_file` (`src/lib.rs` or `src/main.rs`) and its declared `pub mod` submodule files
/// (`src/<mod>.rs` or `src/<mod>/mod.rs`), returning up to `max_symbols` source-anchored public symbols.
pub fn extract_crate_deep_symbols_from_dir(
    crate_dir: &Path,
    entry_file: &Path,
    repo_root: &Path,
    max_symbols: usize,
) -> Vec<DeepCrateSymbol> {
    let mut all_symbols = Vec::new();
    let Ok(entry_src) = fs::read_to_string(entry_file) else {
        return all_symbols;
    };

    let entry_rel = entry_file
        .strip_prefix(repo_root)
        .unwrap_or(entry_file)
        .to_string_lossy()
        .replace('\\', "/");
    all_symbols.extend(extract_deep_crate_symbols(&entry_src, &entry_rel));

    let src_dir = crate_dir.join("src");
    for mod_name in extract_declared_pub_mods(&entry_src) {
        let flat_mod = src_dir.join(format!("{mod_name}.rs"));
        let nested_mod = src_dir.join(&mod_name).join("mod.rs");
        let mod_path = if flat_mod.is_file() {
            Some(flat_mod)
        } else if nested_mod.is_file() {
            Some(nested_mod)
        } else {
            None
        };
        if let Some(mp) = mod_path
            && let Ok(mod_src) = fs::read_to_string(&mp)
        {
            let mod_rel = mp
                .strip_prefix(repo_root)
                .unwrap_or(&mp)
                .to_string_lossy()
                .replace('\\', "/");
            for mut sym in extract_deep_crate_symbols(&mod_src, &mod_rel) {
                sym.name = format!("{mod_name}::{}", sym.name);
                all_symbols.push(sym);
            }
        }
    }

    all_symbols.truncate(max_symbols);
    all_symbols
}

/// Render the OpenWiki `## Dependency Topology`, `## Reverse Dependents (Used By)`, and
/// `### Deep Public Symbols (Source-Anchored)` Markdown sections (`OPENWIKI-API-01..04`).
pub fn render_deep_crate_api_sections(
    crate_name: &str,
    forward_deps: &[String],
    used_by: &[String],
    symbols: &[DeepCrateSymbol],
) -> Result<String, String> {
    let mut out = String::new();

    if !forward_deps.is_empty() || !used_by.is_empty() {
        let chart = render_crate_dependency_diagram(crate_name, forward_deps, used_by)?;
        out.push_str("## Dependency Topology\n\n```vox-diagram\n");
        out.push_str(&chart);
        out.push_str("```\n\n");
    }

    if !used_by.is_empty() {
        out.push_str("## Reverse Dependents (Used By)\n\n");
        out.push_str("| Consumer Crate | Role |\n|---|---|\n");
        for consumer in used_by {
            out.push_str(&format!(
                "| [`{consumer}`](./{consumer}.generated.md) | Direct workspace dependent |\n"
            ));
        }
        out.push('\n');
    }

    if !symbols.is_empty() {
        out.push_str("### Deep Public Symbols (Source-Anchored)\n\n");
        out.push_str("| Kind | Symbol | Signature | Source | Description |\n");
        out.push_str("|---|---|---|---|---|\n");
        for sym in symbols {
            let desc = if sym.doc_summary.is_empty() {
                "—".to_string()
            } else {
                escape_table_cell(&sym.doc_summary)
            };
            let sig = escape_table_cell(&sym.signature);
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} | {} |\n",
                sym.kind,
                escape_table_cell(&sym.name),
                sig,
                sym.source_anchor_markdown(),
                desc
            ));
        }
        out.push('\n');
    }

    Ok(out)
}

/// Render the top-level `docs/src/architecture/subsystems/index.generated.md` page (`DOCS-PROD-DB-01`).
pub fn render_subsystem_index_markdown(all_entries: &[LayerCrateEntry]) -> Result<String, String> {
    let mut md = String::new();
    md.push_str("---\n");
    md.push_str("title: \"Subsystem Architecture Atlas (L0–L5)\"\n");
    md.push_str("description: \"Auto-generated L0–L5 architectural layer topology and domain subsystem map derived from docs/src/architecture/layers.toml.\"\n");
    md.push_str("category: \"Architecture SSOTs\"\n");
    md.push_str("status: \"current\"\n");
    md.push_str("sort_order: 9\n");
    md.push_str("code_anchors:\n  - \"docs/src/architecture/layers.toml\"\n");
    md.push_str("---\n\n");
    md.push_str("# Subsystem Architecture Atlas (L0–L5)\n\n");
    md.push_str("Machine-generated overview of the 6 architectural layers (`L0`–`L5`) and domain subsystem SSoTs in the Vox workspace.\n\n");
    md.push_str("## Layer Dependency Hierarchy\n\n```vox-diagram\ngraph {\n    direction: down\n");
    md.push_str("    node L5: \"L5 — Product Surfaces (CLI, Axis GUI, Daemon)\"\n");
    md.push_str("    node L4: \"L4 — Native Plugins & Terminal TUI\"\n");
    md.push_str("    node L3: \"L3 — Compiler, VoxDb, Orchestrator, Search & Runtimes\"\n");
    md.push_str("    node L2: \"L2 — Domain Libraries & Graph Reader\"\n");
    md.push_str("    node L1: \"L1 — Primitives, Crypto, Secrets & Transport\"\n");
    md.push_str("    node L0: \"L0 — Pure Types & Contracts\"\n");
    md.push_str("    L5 -> L4\n    L4 -> L3\n    L3 -> L2\n    L2 -> L1\n    L1 -> L0\n}\n");
    md.push_str("```\n\n## Architectural Layers (`L0`–`L5`)\n\n");
    md.push_str("| Layer | Subsystem Hub | Crate Count | Summary |\n|---|---|---|---|\n");
    for spec in SUBSYSTEM_LAYERS {
        let count = all_entries.iter().filter(|e| e.layer == spec.layer).count();
        md.push_str(&format!(
            "| `L{}` | [{}](./{}.generated.md) | {} | {} |\n",
            spec.layer, spec.title, spec.slug, count, spec.summary
        ));
    }
    md.push_str("\n## Domain Subsystem Authority Map\n\n");
    md.push_str("| Domain Subsystem | Primary Layer | Canonical SSoT |\n|---|---|---|\n");
    md.push_str(
        "| Compiler & Web IR | `L3` | [`vox-compiler`](../../api/vox-compiler.generated.md) |\n",
    );
    md.push_str("| VoxDb / Arca / Codex | `L3` | [`vox-db`](../../api/vox-db.generated.md) |\n");
    md.push_str("| Multi-Agent Orchestrator & DEI | `L3` | [`vox-orchestrator`](../../api/vox-orchestrator.generated.md) |\n");
    md.push_str(
        "| Hybrid Search & Tantivy | `L3` | [`vox-search`](../../api/vox-search.generated.md) |\n",
    );
    md.push_str("| Populi / Mens / Speech (Oratio) | `L3` / `L4` | [`vox-populi`](../../api/vox-populi.generated.md) |\n");
    md.push_str("| Axis Desktop Workbench & TUI | `L5` / `L4` | [`vox-gui`](../../api/vox-gui.generated.md) |\n");
    md.push_str("| Cryptography & Clavis Secrets | `L1` | [`cryptography-ssot-2026`](../cryptography-ssot-2026.md) |\n");
    Ok(md)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_pub_mods_are_sorted_deduped_and_skip_inline_or_private() {
        let src =
            "pub mod zeta;\nmod private;\npub mod alpha;\npub mod inline { }\npub mod alpha;\n";
        assert_eq!(extract_declared_pub_mods(src), vec!["alpha", "zeta"]);
    }

    #[test]
    fn deep_symbols_from_dir_walks_declared_submodules_and_truncates() {
        let root = std::env::temp_dir().join(format!("vox-openwiki-sym-{}", std::process::id()));
        let src = root.join("demo").join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("lib.rs"),
            "pub mod sub;\n/// Root fn.\npub fn root_fn() {}\n",
        )
        .unwrap();
        fs::write(src.join("sub.rs"), "/// Sub fn.\npub fn sub_fn() {}\n").unwrap();

        let syms =
            extract_crate_deep_symbols_from_dir(&root.join("demo"), &src.join("lib.rs"), &root, 10);
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"root_fn"), "{names:?}");
        assert!(names.contains(&"sub::sub_fn"), "{names:?}");
        assert_eq!(
            extract_crate_deep_symbols_from_dir(&root.join("demo"), &src.join("lib.rs"), &root, 1)
                .len(),
            1
        );
        assert!(
            extract_crate_deep_symbols_from_dir(&root, &root.join("missing.rs"), &root, 10)
                .is_empty()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn deep_api_sections_empty_inputs_render_nothing_and_cells_are_escaped() {
        assert_eq!(
            render_deep_crate_api_sections("c", &[], &[], &[]).unwrap(),
            ""
        );
        let sym = DeepCrateSymbol {
            kind: "fn".into(),
            name: "a|b".into(),
            signature: "fn a()".into(),
            doc_summary: String::new(),
            rel_path: "crates/c/src/lib.rs".into(),
            start_line: 1,
            end_line: 1,
        };
        let out = render_deep_crate_api_sections("c", &[], &["user".into()], &[sym]).unwrap();
        assert!(out.contains("## Reverse Dependents (Used By)"));
        assert!(out.contains("[`user`](./user.generated.md)"));
        assert!(out.contains("`a\\|b`"));
        assert!(out.contains("| — |"));
    }

    #[test]
    fn subsystem_index_has_frontmatter_and_one_row_per_layer() {
        let md = render_subsystem_index_markdown(&[]).unwrap();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("# Subsystem Architecture Atlas (L0–L5)"));
        assert!(!md.contains("```mermaid"));
        let fences = vox_diagram::fence::extract(&md, "vox-diagram");
        assert_eq!(fences.len(), 1);
        assert!(
            vox_diagram::check(&fences[0].body).valid,
            "{}",
            fences[0].body
        );
        for spec in SUBSYSTEM_LAYERS {
            assert!(md.contains(&format!("| `L{}` |", spec.layer)));
        }
    }
}
