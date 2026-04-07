//! Ignore rules and semantic group table for diff-based CodeRabbit planning.

/// Files per PR soft target (stays comfortably under 300-file Pro cap).
pub(crate) const DEFAULT_MAX_FILES_PER_PR: usize = 250;

/// Directories / filename suffixes that should never land in a review PR.
///
/// The `"target-"` prefix catches any `target-<name>/` variation (target-agent/, target-doc-inv2/,
/// target-ci/, etc.), mirroring the `.gitignore` `target-*/` rule.
pub(crate) static IGNORED_DIRS: &[&str] = &[
    "target/",
    "target-", // catches target-agent/, target-doc-inv2/, target-ci/, target-toestub/, etc.
    "target_", // catches target_debug/, target_release/, etc.
    ".cargo-targets/",
    "docs/book/",          // generated mdBook HTML (in .gitignore but sometimes tracked)
    ".vox-research-data/", // local SQLite cache
    ".vox/cache/",
    ".gemini/",
    ".cursor/",
    "node_modules/",
    "dist/",
    "dist1/",
    ".next/",
    "tmp_tools/", // temporary scaffolding directories
    "vendor/",
    "vox-vscode/out/",
    // CodeRabbit tooling — worktrees, run-state, manifests.
    // Without this, a bare `.coderabbit` entry in the manifest triggers copy_dir_recursive
    // which recurses inside worktrees and hits Windows OS error 206 (path too long).
    ".coderabbit/",
    ".coderabbit",
];

pub(crate) static IGNORED_EXTENSIONS: &[&str] = &[
    ".db", ".db-wal", ".db-shm", ".png", ".jpg", ".jpeg", ".webp", ".ico", ".dll", ".exe", ".so",
    ".dylib", ".bin", ".svg", ".woff", ".woff2",
    ".csv", // data export / error dumps (errors.csv etc.) — not source code
    ".md", // documentation — excluded from code review PRs (overridable via allow_markdown_prefixes)
    ".txt", // text — excluded from code review PRs (overridable via allow_markdown_prefixes)
    ".hbs", // handlebars templates — excluded from code review PRs
];

/// Root-level files (no `/`) that are always excluded — scratch/agent artefacts.
pub(crate) static IGNORED_ROOT_EXACT: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    // known agent-generated scratch scripts
    "autofix.py",
    "check_lex.py",
    "debug_offset.py",
    "debug_sync.py",
    "find_missing_cli_paths.py",
    "gen_json.py",
    "list_tools.rs",
    "parse_toestub.py",
    "patch2.py",
    "summary.py",
    "sync_registry_final.py",
    "sync_registry_v2.py",
    "sync_registry_v3.py",
    "sync_registry.py",
    "test_clone.rs",
    "test_lexer.rs",
    "test_parse.rs",
    "tmp_parse_check.rs",
    "update_tool_entries.py",
    "update_trust_policy.py",
];

/// Root-level file suffix patterns that are excluded.
pub(crate) static IGNORED_ROOT_PATTERNS: &[&str] = &[
    ".log", ".txt", // only matched for root-level files (no slash)
    ".py",  // root-level Python scripts are scratch; sub-directory ones go to their group
];

/// (order, group_name, inclusion_test_as_path_prefix_or_suffix)
///
/// Order matters: lower numbers appear in earlier PRs.
/// Each group is ≤ DEFAULT_MAX_FILES_PER_PR files; oversized groups are sub-divided.
pub(crate) static SEMANTIC_GROUPS: &[(u32, &str, SemanticMatcher)] = &[
    // ── Infrastructure ──────────────────────────────────────────────────────
    (
        10,
        "02_github_agents",
        SemanticMatcher::Prefix(&[".github/", ".agents/"]),
    ),
    (
        15,
        "03_dotfiles_config",
        SemanticMatcher::Prefix(&[
            ".cargo/",
            ".config/",
            ".gitlab-ci.yml",
            ".gitignore",
            ".aiignore",
            ".cursorignore",
            ".cursorrules",
            ".dockerignore",
            ".git-hooks/",
            "config/",
            "rust-toolchain.toml",
        ]),
    ),
    (
        17,
        "04_opencode_retire",
        SemanticMatcher::Prefix(&[".opencode/"]),
    ),
    // Root-level files (no path separator): Cargo.toml, README.md, AGENTS.md, CLAUDE.md, etc.
    // Must come AFTER the dotfile/prefix groups so .gitignore lands in 03_dotfiles_config.
    (19, "01_scaffold", SemanticMatcher::RootFile),
    (25, "05_contracts", SemanticMatcher::Prefix(&["contracts/"])),
    // ── Docs ────────────────────────────────────────────────────────────────
    (30, "06_docs_src", SemanticMatcher::Prefix(&["docs/src/"])),
    (35, "07_docs_other", SemanticMatcher::Prefix(&["docs/"])),
    // ── Frontend / UI ───────────────────────────────────────────────────────
    (
        40,
        "08_frontend",
        SemanticMatcher::Prefix(&["frontend/", "islands/", "packages/"]),
    ),
    (45, "09_examples", SemanticMatcher::Prefix(&["examples/"])),
    (
        50,
        "10_vscode_ext",
        SemanticMatcher::Prefix(&["vox-vscode/"]),
    ),
    // ── Scripts / Tooling ───────────────────────────────────────────────────
    (
        55,
        "11_scripts_xtask",
        SemanticMatcher::Prefix(&[
            "scripts/",
            "xtask/",
            "fuzz/",
            "infra/",
            "tree-sitter-vox/",
            "tools/",
            "editors/",
        ]),
    ),
    // ── ML / Mens ─────────────────────────────────────────────────────────
    (60, "12_populi_ml", SemanticMatcher::Prefix(&["mens/"])),
    // ── Tests / Fixtures ────────────────────────────────────────────────────
    (65, "13_tests", SemanticMatcher::Prefix(&["tests/"])),
    // ── Compiler front-end ──────────────────────────────────────────────────
    (
        70,
        "14_crate_parser_lexer",
        SemanticMatcher::Prefix(&["crates/vox-parser/", "crates/vox-lexer/", "crates/vox-ast/"]),
    ),
    (
        75,
        "15_crate_hir",
        SemanticMatcher::Prefix(&["crates/vox-hir/"]),
    ),
    (
        80,
        "16_crate_typeck",
        SemanticMatcher::Prefix(&["crates/vox-typeck/"]),
    ),
    // ── Code generation ─────────────────────────────────────────────────────
    (
        85,
        "17_crate_codegen",
        SemanticMatcher::Prefix(&[
            "crates/vox-codegen-rust/",
            "crates/vox-ssg/",
            "crates/vox-codegen-ts/",
            "crates/vox-codegen-common/",
            "crates/vox-codegen-ts-sdk/",
        ]),
    ),
    // ── Runtime / LSP / Tooling crates ──────────────────────────────────────
    (
        90,
        "18_crate_runtime_lsp",
        SemanticMatcher::Prefix(&[
            "crates/vox-runtime/",
            "crates/vox-lsp/",
            "crates/vox-dap/",
            "crates/vox-fmt/",
            "crates/vox-wasm/",
            "crates/vox-toestub/",
            "crates/vox-doc-pipeline/",
        ]),
    ),
    // ── Agent / MCP ─────────────────────────────────────────────────────────
    (
        95,
        "19_crate_mcp_dei",
        SemanticMatcher::Prefix(&[
            "crates/vox-mcp/",
            "crates/vox-dei/",
            "crates/vox-codex/",
            "crates/vox-capability-registry/",
            "crates/vox-db/",
        ]),
    ),
    // ── CLI (big, always last of must-haves) ────────────────────────────────
    (
        100,
        "20_crate_cli",
        SemanticMatcher::Prefix(&["crates/vox-cli/"]),
    ),
    // ── All remaining crates (catch-all) ────────────────────────────────────
    (105, "21_crate_other", SemanticMatcher::Prefix(&["crates/"])),
    // ── Deployment / generated outputs / static assets ──────────────────────
    (
        110,
        "22_deploy_outputs",
        SemanticMatcher::Prefix(&["deploy/", "dist/", "dist1/", "lib/", "static/"]),
    ),
];

/// Pattern type for semantic group matching.
#[derive(Copy, Clone)]
pub enum SemanticMatcher {
    /// File path starts with any of the given prefixes.
    Prefix(&'static [&'static str]),
    /// File has no directory separator (root-level file: Cargo.toml, README.md, etc.).
    RootFile,
}

impl SemanticMatcher {
    pub(crate) fn matches(&self, path: &str) -> bool {
        match self {
            SemanticMatcher::Prefix(prefixes) => prefixes.iter().any(|p| path.starts_with(p)),
            SemanticMatcher::RootFile => {
                // True only for files directly in the repo root (no '/' in path)
                !path.contains('/')
            }
        }
    }
}
