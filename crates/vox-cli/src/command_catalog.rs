use clap::{Command, CommandFactory, ValueEnum};
use serde::Serialize;

use crate::command_contract;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CatalogFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogTier {
    Recommended,
    Advanced,
    FeatureGated,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandCatalogEntry {
    pub path: Vec<String>,
    pub command: String,
    pub about: String,
    pub aliases: Vec<String>,
    pub has_subcommands: bool,
    pub compiled_in: bool,
    pub source_group: String,
    pub feature_gate: Option<String>,
    pub tier: CatalogTier,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandCatalog {
    pub generated_from: String,
    pub entries: Vec<CommandCatalogEntry>,
}

pub fn build_catalog() -> CommandCatalog {
    let root = crate::VoxCliRoot::command();
    let mut entries = Vec::new();
    for sub in root.get_subcommands() {
        walk_command(sub, &[], &mut entries);
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    CommandCatalog {
        generated_from: "clap::CommandFactory(VoxCliRoot)".to_string(),
        entries,
    }
}

pub fn render_text(entries: &[CommandCatalogEntry]) -> String {
    let mut out = String::new();
    out.push_str("Vox command catalog (clap-derived)\n");
    out.push_str("Command | Tier | Description\n");
    out.push_str("------- | ---- | -----------\n");
    for e in entries {
        let tier = match e.tier {
            CatalogTier::Recommended => "recommended",
            CatalogTier::Advanced => "advanced",
            CatalogTier::FeatureGated => "feature_gated",
        };
        out.push_str(&format!(
            "{} | {} | {}\n",
            e.command,
            tier,
            sanitize_about(&e.about)
        ));
    }
    out
}

pub fn select_entries(
    all_entries: Vec<CommandCatalogEntry>,
    recommended_only: bool,
    include_nested: bool,
) -> Vec<CommandCatalogEntry> {
    all_entries
        .into_iter()
        .filter(|e| include_nested || e.path.len() == 1)
        .filter(|e| !recommended_only || e.tier == CatalogTier::Recommended)
        .collect()
}

fn walk_command(cmd: &Command, prefix: &[String], out: &mut Vec<CommandCatalogEntry>) {
    let mut path = prefix.to_vec();
    path.push(cmd.get_name().to_string());
    let feature_gate = command_contract::merged_feature_gate(&path);
    let tier = tier_for_path(&path, feature_gate.is_some());
    out.push(CommandCatalogEntry {
        command: format!("vox {}", path.join(" ")),
        about: cmd
            .get_about()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(no description)".to_string()),
        aliases: collect_aliases(cmd),
        has_subcommands: cmd.has_subcommands(),
        compiled_in: true,
        source_group: command_contract::catalog_source_group(&path),
        feature_gate,
        path: path.clone(),
        tier,
    });
    for sub in cmd.get_subcommands() {
        walk_command(sub, &path, out);
    }
}

fn collect_aliases(cmd: &Command) -> Vec<String> {
    let mut aliases: Vec<String> = cmd.get_all_aliases().map(ToString::to_string).collect();
    aliases.sort();
    aliases.dedup();
    aliases
}

fn tier_for_path(path: &[String], feature_gated: bool) -> CatalogTier {
    if feature_gated {
        return CatalogTier::FeatureGated;
    }
    let top = path.first().map(String::as_str).unwrap_or_default();
    if path.len() == 1
        && matches!(
            top,
            "build" | "check" | "run" | "test" | "bundle" | "dev" | "doctor" | "completions"
        )
    {
        return CatalogTier::Recommended;
    }
    CatalogTier::Advanced
}

fn sanitize_about(about: &str) -> String {
    about.replace('\n', " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_catalog_has_expected_top_level_commands() {
        let catalog = build_catalog();
        let commands: Vec<&str> = catalog
            .entries
            .iter()
            .filter(|e| e.path.len() == 1)
            .map(|e| e.path[0].as_str())
            .collect();
        for required in ["build", "check", "run", "doctor", "commands", "ci"] {
            assert!(
                commands.contains(&required),
                "missing top-level `{required}`; got {commands:?}"
            );
        }
    }

    #[test]
    fn select_entries_recommended_only_yields_known_starter_set() {
        let catalog = build_catalog();
        let selected = select_entries(catalog.entries, true, false);
        assert!(!selected.is_empty());
        for e in &selected {
            assert_eq!(
                e.tier,
                CatalogTier::Recommended,
                "{} should be recommended tier",
                e.command
            );
            assert_eq!(e.path.len(), 1, "{} should be top-level only", e.command);
        }
        let names: Vec<&str> = selected.iter().map(|e| e.path[0].as_str()).collect();
        for starter in [
            "build",
            "check",
            "run",
            "test",
            "bundle",
            "dev",
            "doctor",
            "completions",
        ] {
            assert!(
                names.contains(&starter),
                "recommended set should include `{starter}`; got {names:?}"
            );
        }
    }

    #[test]
    fn select_entries_top_level_excludes_nested_paths() {
        let catalog = build_catalog();
        let flat = select_entries(catalog.entries.clone(), false, false);
        assert!(flat.iter().all(|e| e.path.len() == 1));

        let nested = select_entries(catalog.entries, false, true);
        assert!(
            nested.iter().any(|e| e.path.len() > 1),
            "with include_nested, expect at least one multi-segment path"
        );
    }

    #[test]
    fn feature_gated_commands_marked_tier() {
        let catalog = build_catalog();
        let mens = catalog
            .entries
            .iter()
            .find(|e| e.path == ["mens"])
            .expect("default vox-cli build includes `mens`");
        assert_eq!(mens.tier, CatalogTier::FeatureGated);
        assert_eq!(mens.feature_gate.as_deref(), Some("mens-base|gpu"));
    }

    #[test]
    fn render_text_contains_command_and_tier_labels() {
        let catalog = build_catalog();
        let subset: Vec<_> = catalog
            .entries
            .into_iter()
            .filter(|e| e.path == ["build"])
            .collect();
        assert_eq!(subset.len(), 1);
        let text = render_text(&subset);
        assert!(text.contains("vox build"));
        assert!(text.contains("recommended"));
    }
}
