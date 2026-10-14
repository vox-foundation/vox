//! Deterministic ground truth resolvers for extracted documentation mentions.

use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{ExtractedMention, MentionKind, ResolutionStatus, ResolvedMention};

#[derive(Debug, Deserialize)]
struct CommandRegistryFile {
    operations: Vec<CommandOperation>,
}

#[derive(Debug, Deserialize)]
struct CommandOperation {
    #[allow(dead_code)]
    surface: String,
    path: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EnvVarsFile {
    variables: Vec<EnvVarEntry>,
}

#[derive(Debug, Deserialize)]
struct EnvVarEntry {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ConfigRegistryFile {
    knobs: Vec<ConfigKnobEntry>,
}

#[derive(Debug, Deserialize)]
struct ConfigKnobEntry {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RetiredSymbolsFile {
    symbols: Vec<RetiredSymbolEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetiredSymbolEntry {
    pub id: String,
    pub pattern: String,
    pub replacement: Option<String>,
    pub rationale: Option<String>,
}

pub struct CompiledRetiredSymbol {
    pub id: String,
    pub regex: Regex,
    pub replacement: Option<String>,
    pub rationale: Option<String>,
}

/// The GroundTruthRegistry loads repository contracts, manifests, and filesystem structures
/// to deterministically resolve mentions.
pub struct GroundTruthRegistry {
    repo_root: PathBuf,
    workspace_crates: HashSet<String>,
    cli_commands: HashSet<String>,
    env_vars: HashSet<String>,
    config_keys: HashSet<String>,
    retired_symbols: Vec<CompiledRetiredSymbol>,
}

impl GroundTruthRegistry {
    /// Load all ground truth sources from the repository root.
    pub fn load(repo_root: &Path) -> Result<Self> {
        let repo_root = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());

        // 1. Workspace Crates
        let mut workspace_crates = HashSet::new();
        let crates_dir = repo_root.join("crates");
        if crates_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&crates_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() && p.join("Cargo.toml").is_file() {
                        if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                            workspace_crates.insert(name.to_string());
                        }
                    }
                }
            }
        }

        // 2. CLI Commands
        let mut cli_commands = HashSet::new();
        let cmd_reg_path = repo_root.join("contracts/cli/command-registry.yaml");
        if cmd_reg_path.is_file() {
            if let Ok(content) = fs::read_to_string(&cmd_reg_path) {
                if let Ok(parsed) = serde_yaml::from_str::<CommandRegistryFile>(&content) {
                    for op in parsed.operations {
                        let full_cmd = op.path.join(" ");
                        cli_commands.insert(full_cmd);
                        if let Some(first) = op.path.first() {
                            cli_commands.insert(first.clone());
                        }
                    }
                }
            }
        }
        // Baseline built-in commands
        cli_commands.insert("check".to_string());
        cli_commands.insert("run".to_string());
        cli_commands.insert("test".to_string());
        cli_commands.insert("build".to_string());
        cli_commands.insert("audit".to_string());
        cli_commands.insert("audit docs".to_string());
        cli_commands.insert("ci".to_string());
        cli_commands.insert("help".to_string());
        cli_commands.insert("version".to_string());
        cli_commands.insert("skill".to_string());

        // 3. Environment Variables
        let mut env_vars = HashSet::new();
        let env_vars_path = repo_root.join("contracts/config/env-vars.v1.yaml");
        if env_vars_path.is_file() {
            if let Ok(content) = fs::read_to_string(&env_vars_path) {
                if let Ok(parsed) = serde_yaml::from_str::<EnvVarsFile>(&content) {
                    for v in parsed.variables {
                        env_vars.insert(v.name);
                    }
                }
            }
        }
        env_vars.insert("RUST_LOG".to_string());
        env_vars.insert("RUST_BACKTRACE".to_string());
        env_vars.insert("PATH".to_string());
        env_vars.insert("HOME".to_string());

        // 4. Config Keys
        let mut config_keys = HashSet::new();
        let cfg_path = repo_root.join("contracts/config/registry.v1.yaml");
        if cfg_path.is_file() {
            if let Ok(content) = fs::read_to_string(&cfg_path) {
                if let Ok(parsed) = serde_yaml::from_str::<ConfigRegistryFile>(&content) {
                    for k in parsed.knobs {
                        config_keys.insert(k.name);
                    }
                }
            }
        }

        // 5. Retired Symbols
        let mut retired_symbols = Vec::new();
        let retired_path = repo_root.join("contracts/documentation/retired-symbols.v1.yaml");
        if retired_path.is_file() {
            if let Ok(content) = fs::read_to_string(&retired_path) {
                if let Ok(parsed) = serde_yaml::from_str::<RetiredSymbolsFile>(&content) {
                    for s in parsed.symbols {
                        if let Ok(re) = Regex::new(&s.pattern) {
                            retired_symbols.push(CompiledRetiredSymbol {
                                id: s.id,
                                regex: re,
                                replacement: s.replacement,
                                rationale: s.rationale,
                            });
                        }
                    }
                }
            }
        }

        Ok(Self {
            repo_root,
            workspace_crates,
            cli_commands,
            env_vars,
            config_keys,
            retired_symbols,
        })
    }

    /// Construct an empty registry for fallback / testing.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            repo_root: std::path::PathBuf::new(),
            workspace_crates: HashSet::new(),
            cli_commands: HashSet::new(),
            env_vars: HashSet::new(),
            config_keys: HashSet::new(),
            retired_symbols: Vec::new(),
        }
    }

    /// Resolve an extracted mention against ground truth.
    #[must_use]
    pub fn resolve_mention(&self, mention: &ExtractedMention) -> ResolvedMention {
        // Step A: Check retired symbols contract first
        for retired in &self.retired_symbols {
            if retired.regex.is_match(&mention.raw_text) {
                return ResolvedMention {
                    extracted: mention.clone(),
                    status: ResolutionStatus::Retired,
                    replacement: retired.replacement.clone(),
                    rationale: retired.rationale.clone(),
                };
            }
        }

        // Step B: Resolve by kind
        match mention.kind {
            MentionKind::FilePath => {
                let target = mention
                    .normalized_target
                    .split('#')
                    .next()
                    .unwrap_or(&mention.normalized_target)
                    .trim_end_matches('/');
                let direct = self.repo_root.join(target);
                if direct.exists() {
                    return ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    };
                }

                // Relative to source file
                let source_parent = Path::new(&mention.source_file)
                    .parent()
                    .unwrap_or(Path::new(""));
                let rel = self.repo_root.join(source_parent).join(target);
                if rel.exists() {
                    return ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    };
                }

                ResolvedMention {
                    extracted: mention.clone(),
                    status: ResolutionStatus::Missing,
                    replacement: None,
                    rationale: Some(format!("Path {} does not exist in repo", target)),
                }
            }
            MentionKind::CrateName => {
                if self.workspace_crates.contains(&mention.normalized_target) {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    }
                } else if mention.normalized_target.starts_with("vox-") {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Missing,
                        replacement: None,
                        rationale: Some(format!(
                            "Crate {} is not in workspace members",
                            mention.normalized_target
                        )),
                    }
                } else {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Ambiguous,
                        replacement: None,
                        rationale: None,
                    }
                }
            }
            MentionKind::CliCommand => {
                let cmd_str = mention
                    .normalized_target
                    .trim_start_matches("vox ")
                    .trim_start_matches("cargo run -p vox-cli -- ")
                    .trim();
                let subcmds: Vec<&str> = cmd_str
                    .split_whitespace()
                    .take_while(|w| !w.starts_with('-'))
                    .collect();
                let full = subcmds.join(" ");

                if self.cli_commands.contains(&full)
                    || (!subcmds.is_empty() && self.cli_commands.contains(subcmds[0]))
                {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    }
                } else {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Missing,
                        replacement: None,
                        rationale: Some(format!(
                            "CLI subcommand '{}' not found in command registry",
                            cmd_str
                        )),
                    }
                }
            }
            MentionKind::EnvVar => {
                if self.env_vars.contains(&mention.normalized_target) {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    }
                } else if mention.normalized_target.starts_with("VOX_") {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Missing,
                        replacement: None,
                        rationale: Some(format!(
                            "Env var {} not found in env-vars contract",
                            mention.normalized_target
                        )),
                    }
                } else {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Ambiguous,
                        replacement: None,
                        rationale: None,
                    }
                }
            }
            MentionKind::ConfigKey => {
                if self.config_keys.contains(&mention.normalized_target) {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    }
                } else {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Missing,
                        replacement: None,
                        rationale: Some(format!(
                            "Config key {} not found in config registry",
                            mention.normalized_target
                        )),
                    }
                }
            }
            MentionKind::RustSymbol => {
                let root_mod = mention.normalized_target.split("::").next().unwrap_or("");
                let mod_as_crate = root_mod.replace('_', "-");

                if self.workspace_crates.contains(&mod_as_crate)
                    || root_mod == "crate"
                    || root_mod == "std"
                    || root_mod == "core"
                {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    }
                } else {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Ambiguous,
                        replacement: None,
                        rationale: Some(format!(
                            "Symbol prefix '{}' could not be resolved to a known crate",
                            root_mod
                        )),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claims::types::{MentionKind, MentionRole};

    #[test]
    fn test_resolves_known_and_missing_targets() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();

        let registry = GroundTruthRegistry::load(repo_root).expect("load ground truth");

        // 1. Known file path
        let file_mention = ExtractedMention {
            kind: MentionKind::FilePath,
            role: MentionRole::Prescribes,
            raw_text: "Cargo.toml".to_string(),
            normalized_target: "Cargo.toml".to_string(),
            source_file: "docs/src/test.md".to_string(),
            line_number: 1,
            byte_offset: 0,
            fence_info: None,
        };
        let res = registry.resolve_mention(&file_mention);
        assert_eq!(res.status, ResolutionStatus::Resolved);

        // 2. Missing file path
        let dead_file_mention = ExtractedMention {
            kind: MentionKind::FilePath,
            role: MentionRole::Prescribes,
            raw_text: "crates/vox-nonexistent-path-12345/foo.rs".to_string(),
            normalized_target: "crates/vox-nonexistent-path-12345/foo.rs".to_string(),
            source_file: "docs/src/test.md".to_string(),
            line_number: 2,
            byte_offset: 10,
            fence_info: None,
        };
        let res_dead = registry.resolve_mention(&dead_file_mention);
        assert_eq!(res_dead.status, ResolutionStatus::Missing);

        // 3. Retired symbol
        let retired_mention = ExtractedMention {
            kind: MentionKind::EnvVar,
            role: MentionRole::Mentions,
            raw_text: "TURSO_URL".to_string(),
            normalized_target: "TURSO_URL".to_string(),
            source_file: "docs/src/test.md".to_string(),
            line_number: 3,
            byte_offset: 20,
            fence_info: None,
        };
        let res_ret = registry.resolve_mention(&retired_mention);
        assert_eq!(res_ret.status, ResolutionStatus::Retired);
        assert_eq!(res_ret.replacement.as_deref(), Some("VOX_DB_URL"));
    }
}
