//! Curated static CLI subcommand, flag, and shell builtin completion tables (`TERM-WARP-03`).

pub struct CliSpec {
    pub cmd: &'static str,
    pub subcommands: &'static [(&'static str, &'static str)],
    pub flags: &'static [(&'static str, &'static str)],
    pub git_ref_subcommands: &'static [&'static str],
}

pub const SHELL_BUILTINS: &[(&str, &str)] = &[
    ("alias", "Define or list command aliases"),
    ("cd", "Change working directory"),
    ("clear", "Clear terminal screen"),
    ("echo", "Print arguments to stdout"),
    ("exit", "Exit current shell"),
    ("export", "Export environment variable"),
    ("history", "Show command history"),
    ("pwd", "Print working directory"),
    ("source", "Execute script in current shell"),
    ("type", "Describe how command name is interpreted"),
    ("which", "Locate executable on PATH"),
];

pub const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/clear", "Clear console block stream"),
    ("/help", "Show Console shortcuts and command guide"),
    ("/history", "Search redacted command history"),
    ("/runbooks", "Open workspace Runbooks panel"),
    (
        "/voxify",
        "Inject OSC 633 + OSC 7 hooks into active subshell",
    ),
];

const VOX_SUBCOMMANDS: &[(&str, &str)] = &[
    (
        "audit",
        "Run code, architecture, retirement, and effort audits",
    ),
    ("build", "Compile Vox project"),
    ("check", "Type-check Vox files"),
    ("ci", "Run repository CI and SSOT verification gates"),
    ("doctor", "Check toolchain, environment, and secrets health"),
    ("explain", "Explain a Vox capability, command, or surface"),
    ("features", "List capabilities filtered by domain"),
    ("fmt", "Format Vox source files"),
    ("mens", "MENS training and evaluation workflows"),
    ("orchestrator", "Manage Vox orchestrator daemon and tasks"),
    ("run", "Execute a .vox script or entrypoint"),
    ("secrets", "Manage and verify Clavis secrets"),
    ("shell", "Check or launch policy-aware shell tools"),
    ("skill", "List, search, and discover Agent Skills"),
    (
        "telemetry",
        "Inspect local telemetry trust and upload settings",
    ),
    ("test", "Run Vox test suites"),
];

const VOX_FLAGS: &[(&str, &str)] = &[
    ("--caps", "Grant explicit script capability"),
    ("--domain", "Filter capabilities by domain"),
    ("--help", "Print command help"),
    ("--interp", "Run in pure interpreter mode"),
    ("--json", "Emit machine-readable JSON output"),
    ("--mode", "Select execution mode"),
    ("--verbose", "Enable verbose logging"),
    ("--version", "Print Vox version"),
    ("--write", "Write regenerated outputs to disk"),
];

const CARGO_SUBCOMMANDS: &[(&str, &str)] = &[
    ("bench", "Execute benchmarks"),
    ("build", "Compile local packages and dependencies"),
    ("check", "Analyze current package and report errors"),
    ("clean", "Remove artifacts from target directory"),
    ("clippy", "Run Clippy lints"),
    ("doc", "Build package documentation"),
    ("metadata", "Output resolved workspace metadata JSON"),
    ("run", "Run a binary or example of the local package"),
    ("test", "Execute unit and integration tests"),
    ("tree", "Display dependency graph"),
];

const CARGO_FLAGS: &[(&str, &str)] = &[
    ("-p", "Package to build or test"),
    ("--package", "Package to build or test"),
    ("--bin", "Build or test only the specified binary"),
    ("--lib", "Build or test only this package's library"),
    (
        "--test",
        "Build or test only the specified integration test",
    ),
    ("--features", "Space or comma separated list of features"),
    ("--all-features", "Activate all available features"),
    ("--no-default-features", "Do not activate default features"),
    ("--release", "Build artifacts in release mode"),
    ("--locked", "Assert Cargo.lock remains unchanged"),
    ("--quiet", "Do not print cargo log messages"),
    ("--verbose", "Use verbose output"),
];

const GIT_SUBCOMMANDS: &[(&str, &str)] = &[
    ("add", "Add file contents to the index"),
    ("branch", "List, create, or delete branches"),
    ("checkout", "Switch branches or restore working tree files"),
    (
        "cherry-pick",
        "Apply changes introduced by existing commits",
    ),
    ("commit", "Record changes to the repository"),
    ("diff", "Show changes between commits or working tree"),
    ("fetch", "Download objects and refs from another repository"),
    ("log", "Show commit logs"),
    ("merge", "Join two or more development histories"),
    ("pull", "Fetch from and integrate with another repository"),
    ("push", "Update remote refs along with associated objects"),
    ("rebase", "Reapply commits on top of another base tip"),
    ("reset", "Reset current HEAD to the specified state"),
    ("rev-parse", "Pick out and massage parameters"),
    ("show", "Show various types of objects"),
    ("stash", "Stash changes in a dirty working directory"),
    ("status", "Show the working tree status"),
    ("switch", "Switch branches"),
    ("tag", "Create, list, or verify tag objects"),
    ("worktree", "Manage multiple working trees"),
];

const GIT_FLAGS: &[(&str, &str)] = &[
    ("-b", "Create and checkout a new branch"),
    ("-C", "Run as if git was started in path"),
    ("--all", "Fetch or list all remotes/branches"),
    ("--amend", "Amend previous commit"),
    ("--branch", "Target branch name"),
    ("--cached", "View staged changes"),
    (
        "--force-with-lease",
        "Safer force push guarding upstream ref",
    ),
    ("--no-verify", "Bypass pre-commit and commit-msg hooks"),
    ("--oneline", "Condense each commit to a single line"),
    ("--staged", "View staged index diff"),
    ("--stat", "Generate diffstat summary"),
];

const GIT_REF_SUBCOMMANDS: &[&str] = &[
    "checkout",
    "switch",
    "merge",
    "rebase",
    "branch",
    "log",
    "diff",
    "reset",
    "cherry-pick",
];

const PNPM_SUBCOMMANDS: &[(&str, &str)] = &[
    ("add", "Install a package and any packages it depends on"),
    ("build", "Run build script"),
    (
        "dlx",
        "Fetch and execute a package binary without installing",
    ),
    ("exec", "Execute a shell command in scope of the project"),
    ("install", "Install all dependencies for a project"),
    ("lint", "Run lint script"),
    (
        "remove",
        "Remove packages from node_modules and package.json",
    ),
    ("run", "Run a script defined in package.json"),
    ("test", "Run package test script"),
    (
        "why",
        "Show all packages that depend on the specified package",
    ),
];

const PNPM_FLAGS: &[(&str, &str)] = &[
    ("-C", "Run pnpm in the specified directory"),
    ("-D", "Save package to devDependencies"),
    ("-F", "Select packages in workspace scope"),
    ("-w", "Run pnpm on the workspace root"),
    ("--dir", "Run pnpm in the specified directory"),
    ("--filter", "Select packages in workspace scope"),
    ("--frozen-lockfile", "Fail if pnpm-lock.yaml needs updates"),
    ("--save-dev", "Save package to devDependencies"),
    ("--workspace-root", "Run pnpm on the workspace root"),
];

const NPM_SUBCOMMANDS: &[(&str, &str)] = &[
    ("audit", "Run a security vulnerability audit"),
    ("ci", "Clean install from lockfile"),
    ("exec", "Run a command from a local or remote npm package"),
    ("init", "Create a package.json file"),
    ("install", "Install a package"),
    ("outdated", "Check for outdated packages"),
    ("publish", "Publish a package to the registry"),
    ("run", "Run arbitrary package scripts"),
    ("test", "Test a package"),
];

const NPM_FLAGS: &[(&str, &str)] = &[
    ("--dry-run", "Report changes without modifying disk"),
    ("--legacy-peer-deps", "Ignore peerDependencies conflicts"),
    ("--prefix", "Operate in specified directory"),
    ("--save-dev", "Package will appear in devDependencies"),
    ("--workspace", "Enable running a command in a workspace"),
    ("--workspaces", "Run across all configured workspaces"),
];

const DOCKER_SUBCOMMANDS: &[(&str, &str)] = &[
    ("build", "Build an image from a Dockerfile"),
    (
        "compose",
        "Define and run multi-container Docker applications",
    ),
    ("exec", "Execute a command in a running container"),
    ("images", "List images"),
    ("inspect", "Return low-level information on Docker objects"),
    ("logs", "Fetch the logs of a container"),
    ("network", "Manage networks"),
    ("ps", "List containers"),
    ("pull", "Download an image from a registry"),
    ("rm", "Remove one or more containers"),
    ("rmi", "Remove one or more images"),
    ("run", "Create and run a new container from an image"),
    ("stop", "Stop one or more running containers"),
    ("volume", "Manage volumes"),
];

const DOCKER_FLAGS: &[(&str, &str)] = &[
    ("-d", "Run container in background"),
    ("-e", "Set environment variables"),
    ("-f", "Follow output or specify file"),
    ("-it", "Interactive TTY session"),
    ("-p", "Publish container port(s) to host"),
    ("-v", "Bind mount a volume"),
    ("--entrypoint", "Overwrite default ENTRYPOINT of image"),
    ("--name", "Assign a name to the container"),
    ("--platform", "Set target platform"),
    ("--rm", "Automatically remove container when it exits"),
];

pub const CLI_SPECS: &[CliSpec] = &[
    CliSpec {
        cmd: "vox",
        subcommands: VOX_SUBCOMMANDS,
        flags: VOX_FLAGS,
        git_ref_subcommands: &[],
    },
    CliSpec {
        cmd: "cargo",
        subcommands: CARGO_SUBCOMMANDS,
        flags: CARGO_FLAGS,
        git_ref_subcommands: &[],
    },
    CliSpec {
        cmd: "git",
        subcommands: GIT_SUBCOMMANDS,
        flags: GIT_FLAGS,
        git_ref_subcommands: GIT_REF_SUBCOMMANDS,
    },
    CliSpec {
        cmd: "pnpm",
        subcommands: PNPM_SUBCOMMANDS,
        flags: PNPM_FLAGS,
        git_ref_subcommands: &[],
    },
    CliSpec {
        cmd: "npm",
        subcommands: NPM_SUBCOMMANDS,
        flags: NPM_FLAGS,
        git_ref_subcommands: &[],
    },
    CliSpec {
        cmd: "docker",
        subcommands: DOCKER_SUBCOMMANDS,
        flags: DOCKER_FLAGS,
        git_ref_subcommands: &[],
    },
];

pub fn find_cli_spec(cmd: &str) -> Option<&'static CliSpec> {
    let base = cmd
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(cmd)
        .to_ascii_lowercase();
    let name = base.strip_suffix(".exe").unwrap_or(&base);
    CLI_SPECS.iter().find(|s| s.cmd == name)
}

pub(crate) fn scan_path_executables() -> Vec<String> {
    let Some(path_os) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for dir in std::env::split_paths(&path_os) {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten().take(1024) {
            if is_executable_entry(&entry) {
                let raw = strip_pathext(entry.file_name().to_string_lossy().into_owned());
                if !raw.is_empty() && !raw.starts_with('.') {
                    names.push(raw);
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

#[cfg(unix)]
fn is_executable_entry(entry: &std::fs::DirEntry) -> bool {
    use std::os::unix::fs::PermissionsExt;
    entry
        .metadata()
        .is_ok_and(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
}

#[cfg(not(unix))]
fn is_executable_entry(entry: &std::fs::DirEntry) -> bool {
    entry.metadata().is_ok_and(|m| m.is_file()) && {
        let n = entry.file_name().to_string_lossy().to_ascii_lowercase();
        [".exe", ".cmd", ".bat", ".ps1"]
            .iter()
            .any(|ext| n.ends_with(ext))
    }
}

fn strip_pathext(name: String) -> String {
    #[cfg(windows)]
    {
        let lower = name.to_ascii_lowercase();
        for ext in [".exe", ".cmd", ".bat", ".ps1"] {
            if lower.ends_with(ext) {
                return name[..name.len() - ext.len()].to_string();
            }
        }
    }
    name
}

pub(crate) fn collect_git_refs(start_cwd: &std::path::Path) -> Vec<(String, &'static str)> {
    let Some((git_dir, common_dir)) = locate_git_dirs(start_cwd) else {
        return Vec::new();
    };
    let mut refs = std::collections::BTreeMap::new();
    for root in [&common_dir, &git_dir] {
        walk_loose_refs(&root.join("refs/heads"), "", "branch", &mut refs);
        walk_loose_refs(&root.join("refs/tags"), "", "tag", &mut refs);
        walk_loose_refs(&root.join("refs/remotes"), "", "remote", &mut refs);
    }
    let packed = common_dir.join("packed-refs");
    if packed
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() <= 2 * 1024 * 1024)
        && let Ok(text) = std::fs::read_to_string(&packed)
    {
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with(['#', '^']) || refs.len() >= 256 {
                continue;
            }
            if let Some((_, full_ref)) = line.split_once(' ') {
                if let Some(b) = full_ref.strip_prefix("refs/heads/") {
                    refs.entry(b.to_string()).or_insert("branch");
                } else if let Some(t) = full_ref.strip_prefix("refs/tags/") {
                    refs.entry(t.to_string()).or_insert("tag");
                } else if let Some(r) = full_ref.strip_prefix("refs/remotes/")
                    && !r.ends_with("/HEAD")
                {
                    refs.entry(r.to_string()).or_insert("remote");
                }
            }
        }
    }
    refs.into_iter().take(256).collect()
}

fn locate_git_dirs(
    start_cwd: &std::path::Path,
) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let mut cur = Some(start_cwd);
    for _ in 0..32 {
        let dir = cur?;
        let dot_git = dir.join(".git");
        if dot_git.is_dir() {
            let common = resolve_commondir(&dot_git);
            return Some((dot_git, common));
        }
        if dot_git.is_file() {
            let raw = std::fs::read_to_string(&dot_git).ok()?;
            let p = std::path::PathBuf::from(raw.lines().next()?.strip_prefix("gitdir:")?.trim());
            let git_dir = if p.is_relative() { dir.join(p) } else { p };
            let common = resolve_commondir(&git_dir);
            return Some((git_dir, common));
        }
        cur = dir.parent();
    }
    None
}

fn resolve_commondir(git_dir: &std::path::Path) -> std::path::PathBuf {
    if let Ok(raw) = std::fs::read_to_string(git_dir.join("commondir"))
        && let Some(line) = raw.lines().next().map(str::trim).filter(|s| !s.is_empty())
    {
        let p = std::path::PathBuf::from(line);
        let joined = if p.is_relative() { git_dir.join(p) } else { p };
        return std::fs::canonicalize(&joined).unwrap_or(joined);
    }
    git_dir.to_path_buf()
}

fn walk_loose_refs(
    dir: &std::path::Path,
    prefix: &str,
    kind: &'static str,
    out: &mut std::collections::BTreeMap<String, &'static str>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten().take(256) {
        if out.len() >= 256 {
            break;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "HEAD" {
            continue;
        }
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let path = entry.path();
        if path.is_dir() {
            walk_loose_refs(&path, &rel, kind, out);
        } else if path.is_file() {
            out.entry(rel).or_insert(kind);
        }
    }
}
