//! Deterministic ground truth resolvers for extracted documentation mentions.

use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;

use super::types::{ExtractedMention, MentionKind, ResolutionStatus, ResolvedMention};

#[derive(Debug, Deserialize)]
struct CommandRegistryFile {
    operations: Vec<CommandOperation>,
}

#[derive(Debug, Deserialize)]
struct CommandOperation {
    surface: String,
    path: Vec<String>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoCatalogFile {
    #[serde(default)]
    repositories: Vec<RepoCatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct RepoCatalogEntry {
    #[serde(default)]
    repository_id: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

/// Retired or renamed workspace crates with their canonical replacements.
const RETIRED_CRATE_REPLACEMENTS: &[(&str, &str, &str)] = &[
    ("vox-arca", "vox-db", "Retired crate; merged into vox-db"),
    ("vox-codex", "vox-db", "Retired crate; merged into vox-db"),
    (
        "vox-codex-api",
        "vox-db",
        "Retired crate; merged into vox-db",
    ),
    ("vox-db-client", "vox-db", "Retired crate alias; use vox-db"),
    (
        "vox-schola",
        "vox-ml-cli",
        "Retired crate; merged into vox-ml-cli",
    ),
    (
        "vox-schola-cuda",
        "vox-ml",
        "Retired bundle/crate alias; use vox-ml",
    ),
    (
        "vox-schola-dev",
        "vox-dev",
        "Retired bundle/crate alias; use vox-dev",
    ),
    (
        "vox-oratio",
        "vox-speech",
        "Retired crate; replaced by vox-speech",
    ),
    (
        "vox-audio-ingress",
        "vox-speech",
        "Retired crate; merged into vox-speech",
    ),
    (
        "vox-ludus",
        "vox-gamify",
        "Retired crate; replaced by vox-gamify",
    ),
    (
        "vox-plugin-gamify",
        "vox-gamify",
        "Retired crate alias; use vox-gamify",
    ),
    (
        "vox-codegen-rust",
        "vox-codegen",
        "Retired crate; unified under vox-codegen",
    ),
    (
        "vox-codegen-html",
        "vox-codegen-ts",
        "Retired crate; replaced by vox-codegen-ts",
    ),
    (
        "vox-ssg",
        "vox-codegen-ts",
        "Retired crate; replaced by vox-codegen-ts",
    ),
    (
        "vox-pm",
        "vox-package",
        "Retired crate; renamed to vox-package",
    ),
    (
        "vox-toestub",
        "vox-code-audit",
        "Retired crate; renamed to vox-code-audit",
    ),
    (
        "vox-ci-guards",
        "vox-cli-ci",
        "Retired crate; consolidated into vox-cli-ci",
    ),
    (
        "vox-py",
        "vox-python-bridge",
        "Retired crate; replaced by vox-python-bridge",
    ),
    (
        "vox-mcp",
        "vox-orchestrator-mcp",
        "Retired crate; moved to vox-orchestrator-mcp",
    ),
    (
        "vox-share",
        "vox-cli-share",
        "Retired crate alias; use vox-cli-share",
    ),
    (
        "vox-install-policy",
        "vox-cli",
        "Retired crate; absorbed into vox-cli",
    ),
    (
        "vox-socrates-policy",
        "vox-eval",
        "Retired crate; moved to vox-eval",
    ),
    (
        "vox-claim-extractor",
        "vox-doc-pipeline",
        "Retired crate; moved to vox-doc-pipeline",
    ),
    (
        "vox-gateway",
        "vox-orchestrator-d",
        "Retired crate; replaced by vox-orchestrator-d",
    ),
    (
        "vox-browser",
        "vox-skills",
        "Retired crate; replaced by vox-skills / browser plugin",
    ),
    (
        "vox-spool",
        "vox-orchestrator-queue",
        "Retired crate; replaced by vox-orchestrator-queue",
    ),
    (
        "vox-webhook",
        "vox-orchestrator-mcp",
        "Retired crate; moved to vox-orchestrator-mcp",
    ),
    (
        "vox-http-envelope",
        "vox-http-client",
        "Retired crate; replaced by vox-http-client",
    ),
    (
        "vox-lang",
        "vox-compiler",
        "Retired crate; replaced by vox-compiler",
    ),
    (
        "vox-runtime-v1",
        "vox-runtime",
        "Retired crate alias; use vox-runtime",
    ),
];

/// Cached per-crate module, AST, and re-export symbol index.
#[derive(Debug, Default)]
struct CrateSymbolIndex {
    exact_paths: HashSet<String>,
    type_paths: HashSet<String>,
    external_reexports: HashMap<String, String>,
    glob_prefixes: Vec<String>,
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
    #[serde(default)]
    env_var: Option<String>,
}

#[derive(Debug, Deserialize)]
struct McpToolRegistryFile {
    #[serde(default)]
    tools: Vec<McpToolEntry>,
}

#[derive(Debug, Deserialize)]
struct McpToolEntry {
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
    crate_dirs: HashMap<String, PathBuf>,
    known_vox_identifiers: HashSet<String>,
    known_mcp_tools: HashSet<String>,
    retired_crates: HashMap<String, (&'static str, &'static str)>,
    cli_commands: HashSet<String>,
    cli_parent_groups: HashSet<String>,
    retired_cli_commands: HashMap<String, String>,
    env_vars: HashSet<String>,
    config_keys: HashSet<String>,
    retired_symbols: Vec<CompiledRetiredSymbol>,
    symbol_cache: RwLock<HashMap<String, Arc<CrateSymbolIndex>>>,
}

fn parse_cargo_package_and_bins(cargo_toml: &str) -> (Option<String>, Vec<String>) {
    enum Section {
        None,
        Package,
        Bin,
    }
    let mut section = Section::None;
    let mut pkg_name = None;
    let mut bin_names = Vec::new();
    for raw_line in cargo_toml.lines() {
        let line = raw_line.split('#').next().unwrap_or(raw_line).trim();
        if line.starts_with('[') {
            section = if line == "[package]" {
                Section::Package
            } else if line == "[[bin]]" {
                Section::Bin
            } else {
                Section::None
            };
            continue;
        }
        if let Some(rest) = line.strip_prefix("name") {
            let trimmed = rest.trim_start();
            if let Some(val) = trimmed.strip_prefix('=') {
                let name = val.trim().trim_matches('"').trim_matches('\'');
                if !name.is_empty() {
                    match section {
                        Section::Package if pkg_name.is_none() => {
                            pkg_name = Some(name.to_string());
                        }
                        Section::Bin => {
                            bin_names.push(name.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    (pkg_name, bin_names)
}

/// True when the repo's own `.gitignore` files (root and every ancestor dir of
/// `rel`, each rooted at its own dir) ignore `rel`. In-process via the `ignore`
/// crate already in this crate's deps — no git subprocess, no new edge. The
/// target may not exist, so it is tested as both a file and a directory.
fn is_gitignored(repo_root: &Path, rel: &str) -> bool {
    use ignore::gitignore::GitignoreBuilder;
    let rel_path = Path::new(rel);
    let mut dir = repo_root.to_path_buf();
    let mut comps = rel_path.components().peekable();
    loop {
        let gi = dir.join(".gitignore");
        if gi.is_file() {
            let mut b = GitignoreBuilder::new(&dir);
            if b.add(&gi).is_none()
                && let Ok(m) = b.build()
            {
                let sub = rel_path
                    .strip_prefix(dir.strip_prefix(repo_root).unwrap_or(Path::new("")))
                    .unwrap_or(rel_path);
                if [true, false]
                    .iter()
                    .any(|&d| m.matched_path_or_any_parents(sub, d).is_ignore())
                {
                    return true;
                }
            }
        }
        match comps.next() {
            Some(c) if comps.peek().is_some() => dir.push(c),
            _ => return false,
        }
    }
}

fn load_catalog_bundle_ids(repo_root: &Path, ids: &mut HashSet<String>) {
    let catalog_path = repo_root.join("crates/vox-plugin-catalog/catalog.toml");
    if catalog_path.is_file()
        && let Ok(content) = fs::read_to_string(&catalog_path)
    {
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if let Some(rest) = line.strip_prefix("id") {
                let after = rest.trim_start();
                if let Some(val) = after.strip_prefix('=') {
                    let id = val.trim().trim_matches('"').trim_matches('\'');
                    if !id.is_empty() {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
    }
}

fn load_repository_catalog_ids(repo_root: &Path, ids: &mut HashSet<String>) {
    let repo_yaml = repo_root.join(".vox/repositories.yaml");
    if !repo_yaml.is_file() {
        return;
    }
    let Ok(content) = fs::read_to_string(&repo_yaml) else {
        return;
    };
    if let Ok(parsed) = serde_yaml::from_str::<RepoCatalogFile>(&content) {
        for entry in parsed.repositories {
            if let Some(id) = entry.repository_id
                && !id.is_empty()
            {
                ids.insert(id);
            }
            if let Some(name) = entry.display_name
                && !name.is_empty()
            {
                ids.insert(name);
            }
        }
    } else if let Ok(entries) = serde_yaml::from_str::<Vec<RepoCatalogEntry>>(&content) {
        for entry in entries {
            if let Some(id) = entry.repository_id
                && !id.is_empty()
            {
                ids.insert(id);
            }
            if let Some(name) = entry.display_name
                && !name.is_empty()
            {
                ids.insert(name);
            }
        }
    }
}

fn load_graph_corpora_ids(repo_root: &Path, ids: &mut HashSet<String>) {
    let corpora_yaml = repo_root.join("contracts/retrieval/vox-graph-corpora.v1.yaml");
    if !corpora_yaml.is_file() {
        return;
    }
    let Ok(content) = fs::read_to_string(&corpora_yaml) else {
        return;
    };
    if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&content)
        && let Some(corpora) = val.get("corpora").and_then(|v| v.as_sequence())
    {
        for entry in corpora {
            if let Some(id) = entry.get("id").and_then(|v| v.as_str())
                && !id.is_empty()
            {
                ids.insert(id.to_string());
            }
        }
    }
}

impl GroundTruthRegistry {
    /// Load all ground truth sources from the repository root.
    pub fn load(repo_root: &Path) -> Result<Self> {
        let repo_root = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());

        // 1. Workspace Crates (real crates/*/Cargo.toml packages) and [[bin]] targets
        let mut workspace_crates = HashSet::new();
        let mut crate_dirs = HashMap::new();
        let mut known_vox_identifiers = HashSet::new();
        let crates_dir = repo_root.join("crates");
        if crates_dir.is_dir()
            && let Ok(entries) = fs::read_dir(&crates_dir)
        {
            for entry in entries.flatten() {
                let p = entry.path();
                let manifest = p.join("Cargo.toml");
                if p.is_dir()
                    && manifest.is_file()
                    && let Some(dir_name) = p.file_name().and_then(|s| s.to_str())
                {
                    workspace_crates.insert(dir_name.to_string());
                    crate_dirs.insert(dir_name.to_string(), p.clone());
                    if let Ok(toml_text) = fs::read_to_string(&manifest) {
                        let (pkg_opt, bins) = parse_cargo_package_and_bins(&toml_text);
                        if let Some(pkg_name) = pkg_opt {
                            workspace_crates.insert(pkg_name.clone());
                            crate_dirs.insert(pkg_name, p);
                        }
                        for bin_name in bins {
                            known_vox_identifiers.insert(bin_name);
                        }
                    }
                }
            }
        }

        // Known non-crate `vox-*` identifiers (distribution bundles, repo IDs, graph corpora, keyring/HF repo names)
        load_catalog_bundle_ids(&repo_root, &mut known_vox_identifiers);
        load_repository_catalog_ids(&repo_root, &mut known_vox_identifiers);
        load_graph_corpora_ids(&repo_root, &mut known_vox_identifiers);
        for non_crate_id in [
            "vox-mens-index",
            "vox-mens-playground",
            "vox-secrets-vault",
            "vox-secrets-env",
            "vox-secrets-local",
            "vox-repository",
        ] {
            known_vox_identifiers.insert(non_crate_id.to_string());
        }

        let mut retired_crates = HashMap::new();
        for &(old_name, replacement, rationale) in RETIRED_CRATE_REPLACEMENTS {
            if !workspace_crates.contains(old_name) && !known_vox_identifiers.contains(old_name) {
                retired_crates.insert(old_name.to_string(), (replacement, rationale));
            }
        }

        // 2. CLI Commands (strictly `vox-cli` surface + live top-level clap aliases)
        let mut cli_commands = HashSet::new();
        let mut cli_parent_groups = HashSet::new();
        let mut retired_cli_commands = HashMap::new();
        for (old_cmd, repl) in [
            ("mens train-uv", "vox mens train --backend qlora"),
            ("schola", "vox mens"),
        ] {
            retired_cli_commands.insert(old_cmd.to_string(), repl.to_string());
        }

        let cmd_reg_path = repo_root.join("contracts/cli/command-registry.yaml");
        if cmd_reg_path.is_file()
            && let Ok(content) = fs::read_to_string(&cmd_reg_path)
            && let Ok(parsed) = serde_yaml::from_str::<CommandRegistryFile>(&content)
        {
            for op in parsed.operations {
                if op.surface != "vox-cli" || op.path.is_empty() {
                    continue;
                }
                let full_cmd = op.path.join(" ");
                let status = op.status.as_deref().unwrap_or("active");
                if matches!(status, "retired" | "deprecated") {
                    // The registry records no replacement; an empty string means "retired,
                    // replacement unknown" (explicit entries above keep theirs via `or_default`).
                    retired_cli_commands.entry(full_cmd).or_default();
                    continue;
                }
                cli_commands.insert(full_cmd);
                for k in 1..op.path.len() {
                    cli_parent_groups.insert(op.path[..k].join(" "));
                }
            }
            for builtin in ["help", "version", "exit", "quit", "visus"] {
                cli_commands.insert(builtin.to_string());
            }

            const CLAP_TOP_ALIASES: &[(&str, &str)] = &[
                ("fab", "fabrica"),
                ("clavis", "secrets"),
                ("rec", "recensio"),
                ("graphify", "graph"),
                ("search", "graph"),
                ("bapp", "bundle-app"),
                ("ludus", "gamify"),
                ("gsd", "plan"),
                ("axis", "gui"),
                ("orchestrator", "dei"),
                ("oc", "openclaw"),
                ("models", "model"),
                ("oratio", "speech"),
                ("plugin-bundle", "bundle"),
            ];
            let existing_cmds: Vec<String> = cli_commands.iter().cloned().collect();
            let existing_groups: Vec<String> = cli_parent_groups.iter().cloned().collect();
            for &(alias, canonical) in CLAP_TOP_ALIASES {
                for cmd in &existing_cmds {
                    if cmd == canonical {
                        cli_commands.insert(alias.to_string());
                    } else if let Some(rest) = cmd.strip_prefix(&format!("{canonical} ")) {
                        cli_commands.insert(format!("{alias} {rest}"));
                    }
                }
                for grp in &existing_groups {
                    if grp == canonical {
                        cli_parent_groups.insert(alias.to_string());
                    } else if let Some(rest) = grp.strip_prefix(&format!("{canonical} ")) {
                        cli_parent_groups.insert(format!("{alias} {rest}"));
                    }
                }
            }
        } else {
            for cmd in [
                "check",
                "run",
                "test",
                "build",
                "audit",
                "audit docs",
                "ci",
                "help",
                "version",
            ] {
                cli_commands.insert(cmd.to_string());
            }
            cli_parent_groups.insert("audit".to_string());
            cli_parent_groups.insert("ci".to_string());
        }

        // 3. Environment Variables
        let mut env_vars = HashSet::new();
        let env_vars_path = repo_root.join("contracts/config/env-vars.v1.yaml");
        if env_vars_path.is_file()
            && let Ok(content) = fs::read_to_string(&env_vars_path)
            && let Ok(parsed) = serde_yaml::from_str::<EnvVarsFile>(&content)
        {
            for v in parsed.variables {
                env_vars.insert(v.name);
            }
        }
        for v in [
            "RUST_LOG",
            "RUST_BACKTRACE",
            "PATH",
            "HOME",
            "VOX_REPO_ROOT",
            "VOX_WEB_PORT",
            "VOX_PROFILE",
            "VOX_LOG",
            "VOX_HOME",
            "VOX_CACHE_DIR",
            "VOX_CONFIG",
            "VOX_ENV",
            "VOX_FORMAT",
            "VOX_COLOR",
            "VOX_NO_COLOR",
            "VOX_OFFLINE",
            "VOX_TIMEOUT",
            "VOX_SEARCH_WEB_RESEARCH_DISABLED",
            "VOX_RESEARCH_TRIAL_SIGNING_KEY_HEX",
            "VOX_RESEARCH_TRIAL_SIGNER_ALLOWLIST",
            "VOX_RESEARCH_PREFER_FREE_TIER",
            "VOX_MCP_ORCHESTRATOR_RPC_WRITES",
            "VOX_MENS_HF_NAMESPACE",
            "VOX_GUI_PORT",
            "VOX_OPENALEX_API_KEY",
            "VOX_MINICHECK_MODEL",
        ] {
            env_vars.insert(v.to_string());
        }

        // 4. Config Keys
        let mut config_keys = HashSet::new();
        let cfg_path = repo_root.join("contracts/config/registry.v1.yaml");
        if cfg_path.is_file()
            && let Ok(content) = fs::read_to_string(&cfg_path)
            && let Ok(parsed) = serde_yaml::from_str::<ConfigRegistryFile>(&content)
        {
            for k in parsed.knobs {
                if let Some(ev) = k.env_var {
                    env_vars.insert(ev);
                }
                config_keys.insert(k.name);
            }
        }

        // 5. MCP Tools (`::`-namespaced names like `vox_compiler::ast_inspect`)
        let mut known_mcp_tools = HashSet::new();
        let mcp_tools_path = repo_root.join("contracts/mcp/tool-registry.canonical.yaml");
        if mcp_tools_path.is_file()
            && let Ok(content) = fs::read_to_string(&mcp_tools_path)
            && let Ok(parsed) = serde_yaml::from_str::<McpToolRegistryFile>(&content)
        {
            for t in parsed.tools {
                if !t.name.is_empty() {
                    known_mcp_tools.insert(t.name);
                }
            }
        }

        // 6. Retired Symbols
        let mut retired_symbols = Vec::new();
        let retired_path = repo_root.join("contracts/documentation/retired-symbols.v1.yaml");
        if retired_path.is_file()
            && let Ok(content) = fs::read_to_string(&retired_path)
            && let Ok(parsed) = serde_yaml::from_str::<RetiredSymbolsFile>(&content)
        {
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

        Ok(Self {
            repo_root,
            workspace_crates,
            crate_dirs,
            known_vox_identifiers,
            known_mcp_tools,
            retired_crates,
            cli_commands,
            cli_parent_groups,
            retired_cli_commands,
            env_vars,
            config_keys,
            retired_symbols,
            symbol_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Construct an empty registry for fallback / testing.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            repo_root: std::path::PathBuf::new(),
            workspace_crates: HashSet::new(),
            crate_dirs: HashMap::new(),
            known_vox_identifiers: HashSet::new(),
            known_mcp_tools: HashSet::new(),
            retired_crates: HashMap::new(),
            cli_commands: HashSet::new(),
            cli_parent_groups: HashSet::new(),
            retired_cli_commands: HashMap::new(),
            env_vars: HashSet::new(),
            config_keys: HashSet::new(),
            retired_symbols: Vec::new(),
            symbol_cache: RwLock::new(HashMap::new()),
        }
    }

    fn get_or_build_crate_index(&self, crate_name: &str) -> Option<Arc<CrateSymbolIndex>> {
        if let Ok(guard) = self.symbol_cache.read()
            && let Some(idx) = guard.get(crate_name)
        {
            return Some(Arc::clone(idx));
        }

        let crate_dir = self
            .crate_dirs
            .get(crate_name)
            .cloned()
            .unwrap_or_else(|| self.repo_root.join("crates").join(crate_name));
        let src_dir = crate_dir.join("src");
        if !src_dir.is_dir() {
            return None;
        }

        let built = Arc::new(build_crate_symbol_index(&src_dir, &self.workspace_crates));
        if let Ok(mut guard) = self.symbol_cache.write() {
            let entry = guard
                .entry(crate_name.to_string())
                .or_insert_with(|| Arc::clone(&built));
            return Some(Arc::clone(entry));
        }
        Some(built)
    }

    fn resolve_crate_symbol(&self, crate_name: &str, subpath: &str, depth: u8) -> bool {
        if depth > 2 {
            return false;
        }
        let Some(idx) = self.get_or_build_crate_index(crate_name) else {
            return false;
        };
        if subpath.is_empty() || idx.exact_paths.contains(subpath) {
            return true;
        }
        if let Some((parent, _)) = subpath.rsplit_once("::")
            && idx.type_paths.contains(parent)
        {
            return true;
        }
        if let Some((first, rest)) = subpath.split_once("::")
            && let Some(ext_crate) = idx.external_reexports.get(first)
            && self.resolve_crate_symbol(ext_crate, rest, depth + 1)
        {
            return true;
        }
        for glob_pref in &idx.glob_prefixes {
            let candidate = format!("{glob_pref}::{subpath}");
            if idx.exact_paths.contains(&candidate) {
                return true;
            }
            let as_crate = glob_pref.replace('_', "-");
            if self.workspace_crates.contains(&as_crate)
                && self.resolve_crate_symbol(&as_crate, subpath, depth + 1)
            {
                return true;
            }
        }
        false
    }

    /// The canonical replacement for `crate_name` when it is an *exact* entry of the retired
    /// crate rename table. Unlike the regex-based `retired-symbols.v1.yaml` patterns, this never
    /// matches a longer name (`vox-dei-d`) or a command containing the name, so it is the only
    /// retirement source safe for automatic rewriting.
    #[must_use]
    pub fn exact_retired_crate_replacement(&self, crate_name: &str) -> Option<&'static str> {
        self.retired_crates.get(crate_name).map(|&(r, _)| r)
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
                    .split(':')
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

                // Build/cache outputs (`dist`, `target`, ...) exist per-environment,
                // not per-docs; their absence is not a docs deficit.
                if is_gitignored(&self.repo_root, target) {
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
                let target = mention.normalized_target.as_str();
                if let Some(&(replacement, rationale)) = self.retired_crates.get(target) {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Retired,
                        replacement: Some(replacement.to_string()),
                        rationale: Some(rationale.to_string()),
                    }
                } else if self.workspace_crates.contains(target)
                    || self.known_vox_identifiers.contains(target)
                    || target.starts_with("vox-mens-qwen")
                {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    }
                } else if target.starts_with("vox-") {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Missing,
                        replacement: None,
                        rationale: Some(format!(
                            "Crate or bundle '{}' not found in workspace Cargo.toml or catalog.toml",
                            target
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
                let norm = mention.normalized_target.trim();
                if let Some(after_cargo) = norm.strip_prefix("cargo run -p ") {
                    let pkg = after_cargo.split_whitespace().next().unwrap_or("");
                    if pkg != "vox-cli" && self.workspace_crates.contains(pkg) {
                        return ResolvedMention {
                            extracted: mention.clone(),
                            status: ResolutionStatus::Resolved,
                            replacement: None,
                            rationale: None,
                        };
                    }
                }

                let cmd_str = norm
                    .strip_prefix("cargo run -p vox-cli")
                    .or_else(|| norm.strip_prefix("vox "))
                    .unwrap_or(norm)
                    .trim();

                let mut tokens = cmd_str.split_whitespace().peekable();
                while let Some(&tok) = tokens.peek() {
                    if matches!(
                        tok,
                        "--" | "--json" | "--quiet" | "-q" | "--release" | "--interp"
                    ) {
                        tokens.next();
                    } else if matches!(tok, "--features" | "--mode" | "--caps") {
                        tokens.next();
                        tokens.next();
                    } else {
                        break;
                    }
                }

                let subcmds: Vec<&str> = tokens.take_while(|w| !w.starts_with('-')).collect();
                if subcmds.is_empty() {
                    return ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    };
                }

                let full = subcmds.join(" ");
                let retired_repl = self
                    .retired_cli_commands
                    .get(&full)
                    .or_else(|| {
                        if subcmds.len() >= 2 {
                            self.retired_cli_commands.get(&subcmds[..2].join(" "))
                        } else {
                            None
                        }
                    })
                    .or_else(|| self.retired_cli_commands.get(subcmds[0]));
                if let Some(repl) = retired_repl {
                    return ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Retired,
                        replacement: (!repl.is_empty()).then(|| repl.clone()),
                        rationale: Some(format!("CLI command '{}' is retired/deprecated", cmd_str)),
                    };
                }

                let mut matched_len = 0usize;
                for k in (1..=subcmds.len()).rev() {
                    let cand = subcmds[..k].join(" ");
                    if self.cli_commands.contains(&cand) {
                        matched_len = k;
                        break;
                    }
                }

                if matched_len == 0 {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Missing,
                        replacement: None,
                        rationale: Some(format!(
                            "CLI subcommand '{}' not found in command registry",
                            cmd_str
                        )),
                    }
                } else if matched_len < subcmds.len()
                    && self
                        .cli_parent_groups
                        .contains(&subcmds[..matched_len].join(" "))
                {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Missing,
                        replacement: None,
                        rationale: Some(format!(
                            "CLI subcommand '{}' not found under '{}'",
                            subcmds[matched_len],
                            subcmds[..matched_len].join(" ")
                        )),
                    }
                } else {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
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
                        status: ResolutionStatus::Ambiguous,
                        replacement: None,
                        rationale: None,
                    }
                }
            }
            MentionKind::RustSymbol => {
                if self.known_mcp_tools.contains(&mention.normalized_target) {
                    return ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    };
                }
                let mut parts = mention.normalized_target.split("::");
                let root_mod = parts.next().unwrap_or("");
                let subpath = parts.collect::<Vec<_>>().join("::");
                let mod_as_crate = root_mod.replace('_', "-");

                if let Some(&(replacement, rationale)) = self.retired_crates.get(&mod_as_crate) {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Retired,
                        // Keep the symbol path: `vox_mcp::a::B` -> `vox_orchestrator_mcp::a::B`.
                        replacement: Some(if subpath.is_empty() {
                            replacement.replace('-', "_")
                        } else {
                            format!("{}::{subpath}", replacement.replace('-', "_"))
                        }),
                        rationale: Some(rationale.to_string()),
                    }
                } else if matches!(
                    root_mod,
                    "std" | "core" | "alloc" | "crate" | "Self" | "super"
                ) {
                    ResolvedMention {
                        extracted: mention.clone(),
                        status: ResolutionStatus::Resolved,
                        replacement: None,
                        rationale: None,
                    }
                } else if self.workspace_crates.contains(&mod_as_crate) {
                    if self.resolve_crate_symbol(&mod_as_crate, &subpath, 0) {
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
                                "Symbol '{}' not found in crate '{}'",
                                mention.normalized_target, mod_as_crate
                            )),
                        }
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

fn qualify_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

fn module_segments_for_rs_path(src_dir: &Path, path: &Path) -> Vec<String> {
    let Ok(rel) = path.strip_prefix(src_dir) else {
        return Vec::new();
    };
    let mut segs: Vec<String> = rel
        .iter()
        .filter_map(|s| s.to_str())
        .map(ToString::to_string)
        .collect();
    if let Some(last) = segs.pop() {
        let stem = last.strip_suffix(".rs").unwrap_or(&last);
        if stem != "lib" && stem != "main" && stem != "mod" {
            segs.push(stem.to_string());
        }
    }
    segs
}

fn record_pub_use_tree(
    use_expr: &str,
    mod_prefix: &str,
    workspace_crates: &HashSet<String>,
    idx: &mut CrateSymbolIndex,
) {
    let body = use_expr.trim().trim_end_matches(';').trim();
    if let Some(prefix) = body.strip_suffix("::*") {
        let cleaned = prefix
            .trim()
            .trim_start_matches("crate::")
            .trim_start_matches("self::");
        if mod_prefix.is_empty() && !cleaned.is_empty() {
            idx.glob_prefixes.push(cleaned.to_string());
        }
        return;
    }

    if let Some((before_brace, inside_brace)) = body.split_once('{') {
        let base = before_brace.trim().trim_end_matches("::");
        let inside = inside_brace
            .rsplit_once('}')
            .map(|(inner, _)| inner)
            .unwrap_or(inside_brace);
        for item in inside.split(',') {
            let entry = item.trim();
            if entry.is_empty() {
                continue;
            }
            if let Some((orig, alias)) = entry.split_once(" as ") {
                let name = alias.trim();
                let orig_path = if base.is_empty() {
                    orig.trim().to_string()
                } else {
                    format!("{base}::{}", orig.trim())
                };
                if !name.is_empty() && name != "_" {
                    idx.exact_paths.insert(qualify_path(mod_prefix, name));
                    let as_crate = orig_path.replace('_', "-");
                    if mod_prefix.is_empty() && workspace_crates.contains(&as_crate) {
                        idx.external_reexports.insert(name.to_string(), as_crate);
                    }
                }
            } else {
                let leaf = entry.split("::").last().unwrap_or(entry).trim();
                if !leaf.is_empty() && leaf != "self" && leaf != "*" {
                    idx.exact_paths.insert(qualify_path(mod_prefix, leaf));
                }
            }
        }
        return;
    }

    if let Some((orig, alias)) = body.split_once(" as ") {
        let name = alias.trim();
        let orig_trimmed = orig.trim();
        if !name.is_empty() && name != "_" {
            idx.exact_paths.insert(qualify_path(mod_prefix, name));
            let as_crate = orig_trimmed.replace('_', "-");
            if mod_prefix.is_empty() && workspace_crates.contains(&as_crate) {
                idx.external_reexports.insert(name.to_string(), as_crate);
            }
        }
        return;
    }

    if let Some(leaf) = body.split("::").last() {
        let leaf = leaf.trim();
        if !leaf.is_empty() && leaf != "self" {
            idx.exact_paths.insert(qualify_path(mod_prefix, leaf));
        }
    }
}

fn build_crate_symbol_index(
    src_dir: &Path,
    workspace_crates: &HashSet<String>,
) -> CrateSymbolIndex {
    let mut idx = CrateSymbolIndex::default();

    for entry in WalkDir::new(src_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };

        let segs = module_segments_for_rs_path(src_dir, path);
        for k in 1..=segs.len() {
            idx.exact_paths.insert(segs[..k].join("::"));
        }
        let mod_prefix = segs.join("::");

        let graph = vox_graph_reader::ast::extract_ast_in_module(path, &content, &mod_prefix);
        for node in graph.nodes {
            let clean_id = node.id.split('#').next().unwrap_or(&node.id).to_string();
            if !clean_id.is_empty() {
                if matches!(node.kind.as_str(), "struct" | "enum" | "trait") {
                    idx.type_paths.insert(clean_id.clone());
                }
                idx.exact_paths.insert(clean_id);
            }
        }

        let mut collecting_use: Option<String> = None;
        for raw_line in content.lines() {
            let trimmed = raw_line.trim();
            if let Some(mut buf) = collecting_use.take() {
                buf.push(' ');
                buf.push_str(trimmed);
                if trimmed.contains(';') {
                    record_pub_use_tree(&buf, &mod_prefix, workspace_crates, &mut idx);
                } else {
                    collecting_use = Some(buf);
                }
                continue;
            }

            if let Some(after_mod) = trimmed
                .strip_prefix("pub mod ")
                .or_else(|| trimmed.strip_prefix("pub(crate) mod "))
                .or_else(|| trimmed.strip_prefix("mod "))
            {
                let ident = after_mod
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("");
                if !ident.is_empty() {
                    idx.exact_paths.insert(qualify_path(&mod_prefix, ident));
                }
            } else if let Some(after_use) = trimmed
                .strip_prefix("pub use ")
                .or_else(|| trimmed.strip_prefix("pub(crate) use "))
            {
                if after_use.contains(';') {
                    record_pub_use_tree(after_use, &mod_prefix, workspace_crates, &mut idx);
                } else {
                    collecting_use = Some(after_use.to_string());
                }
            } else if let Some(after_item) = trimmed
                .strip_prefix("pub const ")
                .or_else(|| trimmed.strip_prefix("pub static "))
                .or_else(|| trimmed.strip_prefix("macro_rules! "))
            {
                let ident = after_item
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("");
                if !ident.is_empty() {
                    idx.exact_paths.insert(qualify_path(&mod_prefix, ident));
                }
            }
        }
    }

    idx
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

    fn mk_prescribes_mention(kind: MentionKind, target: &str) -> ExtractedMention {
        ExtractedMention {
            kind,
            role: MentionRole::Prescribes,
            raw_text: target.to_string(),
            normalized_target: target.to_string(),
            source_file: "docs/src/test.md".to_string(),
            line_number: 1,
            byte_offset: 0,
            fence_info: None,
        }
    }

    #[test]
    fn honest_crate_bundle_and_retired_resolution() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let registry = GroundTruthRegistry::load(repo_root).expect("load ground truth");

        for retired_crate in [
            "vox-arca",
            "vox-codex",
            "vox-schola",
            "vox-oratio",
            "vox-ludus",
            "vox-codegen-rust",
            "vox-pm",
        ] {
            let res = registry.resolve_mention(&mk_prescribes_mention(
                MentionKind::CrateName,
                retired_crate,
            ));
            assert_eq!(
                res.status,
                ResolutionStatus::Retired,
                "expected {retired_crate} to be Retired"
            );
            assert!(
                res.replacement.is_some(),
                "expected canonical replacement for {retired_crate}"
            );
        }

        for real_or_bundle in [
            "vox-db",
            "vox-codegen",
            "vox-package",
            "vox-speech",
            "vox-gamify",
            "vox-ml",
            "vox-dev",
            "vox-fullstack",
        ] {
            let res = registry.resolve_mention(&mk_prescribes_mention(
                MentionKind::CrateName,
                real_or_bundle,
            ));
            assert_eq!(
                res.status,
                ResolutionStatus::Resolved,
                "expected {real_or_bundle} to be Resolved"
            );
        }

        let unknown = registry.resolve_mention(&mk_prescribes_mention(
            MentionKind::CrateName,
            "vox-totally-unknown-crate",
        ));
        assert_eq!(unknown.status, ResolutionStatus::Missing);
    }

    /// A registry-derived retired command has no known replacement: it must not borrow the
    /// `mens train-uv` one. (The 3 retired ops today are all training commands, so the
    /// hardcoded text happened to fit; the next retired non-training command would not.)
    #[test]
    fn registry_retired_cli_command_has_no_invented_replacement() {
        let root = std::env::temp_dir().join(format!(
            "vox-resolver-retired-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(root.join("contracts/cli")).unwrap();
        fs::write(
            root.join("contracts/cli/command-registry.yaml"),
            "operations:\n- surface: vox-cli\n  path: [oldthing, run]\n  status: deprecated\n- surface: vox-cli\n  path: [newthing]\n  status: active\n",
        )
        .unwrap();
        let registry = GroundTruthRegistry::load(&root).expect("load");
        let res = registry.resolve_mention(&mk_prescribes_mention(
            MentionKind::CliCommand,
            "vox oldthing run",
        ));
        fs::remove_dir_all(&root).ok();
        assert_eq!(res.status, ResolutionStatus::Retired);
        assert_eq!(
            res.replacement, None,
            "no replacement is known for oldthing"
        );
    }

    #[test]
    fn retired_crate_symbol_replacement_keeps_the_symbol_path() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let registry = GroundTruthRegistry::load(repo_root).expect("load ground truth");
        let res = registry.resolve_mention(&mk_prescribes_mention(
            MentionKind::RustSymbol,
            "vox_arca::store::Foo",
        ));
        assert_eq!(res.status, ResolutionStatus::Retired);
        assert_eq!(res.replacement.as_deref(), Some("vox_db::store::Foo"));
    }

    #[test]
    fn honest_filepath_cli_and_rust_symbol_resolution() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let registry = GroundTruthRegistry::load(repo_root).expect("load ground truth");

        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::FilePath,
                    "crates/vox-cli/src/missing_module_xyz.rs",
                ))
                .status,
            ResolutionStatus::Missing
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::FilePath,
                    "docs/src/missing_page_xyz.md",
                ))
                .status,
            ResolutionStatus::Missing
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::FilePath,
                    "crates/vox-cli/src/main.rs",
                ))
                .status,
            ResolutionStatus::Resolved
        );

        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::CliCommand,
                    "vox audit docs-reality",
                ))
                .status,
            ResolutionStatus::Missing
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::CliCommand,
                    "vox mens imaginary-subcmd",
                ))
                .status,
            ResolutionStatus::Missing
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::CrateName,
                    "vox-compilerd",
                ))
                .status,
            ResolutionStatus::Resolved
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(MentionKind::CrateName, "vox-mcp",))
                .status,
            ResolutionStatus::Retired
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::CliCommand,
                    "vox mens watch",
                ))
                .status,
            ResolutionStatus::Missing
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::CliCommand,
                    "vox mens train-uv",
                ))
                .status,
            ResolutionStatus::Retired
        );
        for valid_cmd in [
            "vox audit docs",
            "vox ci ssot-drift",
            "vox check examples/hello.vox",
            "vox run scripts/setup.vox",
            "vox fab bundle",
            "vox orchestrator",
        ] {
            assert_eq!(
                registry
                    .resolve_mention(&mk_prescribes_mention(MentionKind::CliCommand, valid_cmd))
                    .status,
                ResolutionStatus::Resolved,
                "expected CLI command `{valid_cmd}` to resolve"
            );
        }

        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::RustSymbol,
                    "vox_compiler::codegen_rust",
                ))
                .status,
            ResolutionStatus::Missing
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::RustSymbol,
                    "vox_actor_runtime::missing_mod_xyz",
                ))
                .status,
            ResolutionStatus::Missing
        );
        assert_eq!(
            registry
                .resolve_mention(&mk_prescribes_mention(
                    MentionKind::RustSymbol,
                    "vox_codex::Store",
                ))
                .status,
            ResolutionStatus::Retired
        );
        for valid_sym in [
            "vox_actor_runtime::llm",
            "vox_compiler::ast",
            "vox_compiler::ast_inspect",
        ] {
            assert_eq!(
                registry
                    .resolve_mention(&mk_prescribes_mention(MentionKind::RustSymbol, valid_sym))
                    .status,
                ResolutionStatus::Resolved,
                "expected Rust symbol `{valid_sym}` to resolve"
            );
        }
    }

    /// Scratch repo with a `.gitignore` of `dist/`; `with_dist` also creates the dir.
    fn ignore_repo(tag: &str, with_dist: bool) -> (PathBuf, GroundTruthRegistry) {
        let root = std::env::temp_dir().join(format!(
            "vox-doc-pipeline-ignore-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates/x/ui")).unwrap();
        fs::write(root.join(".gitignore"), "dist/\n").unwrap();
        if with_dist {
            fs::create_dir_all(root.join("crates/x/ui/dist")).unwrap();
        }
        let reg = GroundTruthRegistry::load(&root).expect("load ground truth");
        (root, reg)
    }

    #[test]
    fn gitignored_missing_path_is_not_a_missing_deficit() {
        let (root, reg) = ignore_repo("missing", false);
        let r = reg.resolve_mention(&mk_prescribes_mention(
            MentionKind::FilePath,
            "crates/x/ui/dist",
        ));
        assert_ne!(
            r.status,
            ResolutionStatus::Missing,
            "build output is environment-dependent"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gitignored_existing_path_is_not_missing() {
        let (root, reg) = ignore_repo("existing", true);
        let r = reg.resolve_mention(&mk_prescribes_mention(
            MentionKind::FilePath,
            "crates/x/ui/dist",
        ));
        assert_eq!(r.status, ResolutionStatus::Resolved);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn non_ignored_missing_path_is_still_missing() {
        let (root, reg) = ignore_repo("control", false);
        let r = reg.resolve_mention(&mk_prescribes_mention(
            MentionKind::FilePath,
            "crates/x/ui/src/gone.rs",
        ));
        assert_eq!(r.status, ResolutionStatus::Missing);
        let _ = fs::remove_dir_all(root);
    }
}
