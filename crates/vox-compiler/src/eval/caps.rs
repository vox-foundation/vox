//! Receiver-imposed capabilities for the interpreter (spec §3.2). The grammar is a public
//! CLI surface once shipped; keep it small. Roots are canonicalised here so `allows_path`
//! compares like with like — on macOS `/tmp` is `/private/tmp`, on Windows `canonicalize`
//! yields `\\?\C:\…`, and comparing a canonical path against a raw root denies everything.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// Never gated. `path.resolve` is the one method in a pure namespace that touches disk;
/// the fs arm gates it as a read (Task 5). `db`/`repo` are in-memory stores.
pub const PURE: &[&str] = &[
    "path", "json", "csv", "toml", "yaml", "regex", "log", "db", "repo",
];
/// Gated. `io` is here for classification only — its authority comes from `fs:`.
/// `crypto` is gated by the `random` axis.
pub const GATED: &[&str] = &[
    "fs", "io", "process", "env", "secrets", "http", "time", "agentos", "crypto",
];

pub fn is_classified(ns: &str) -> bool {
    PURE.contains(&ns) || GATED.contains(&ns)
}

/// A parsed, canonicalised set of receiver-imposed capabilities. Every filesystem root
/// stored here has already been through `std::fs::canonicalize`, so `allows_path` never
/// has to worry about `/tmp` vs `/private/tmp` or relative-vs-absolute mismatches — it
/// just compares canonical `Component`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySet {
    allowed: BTreeSet<String>,
    env_write: bool,
    /// `None` = unscoped (developer default / legacy directive).
    fs_ro: Option<Vec<PathBuf>>,
    fs_rw: Option<Vec<PathBuf>>,
    frozen_time_ms: Option<i64>,
    random_seed: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid --caps token {token:?}: {why}")]
pub struct CapsParseError {
    pub token: String,
    pub why: &'static str,
}

impl CapabilitySet {
    fn restrictive() -> Self {
        Self {
            allowed: BTreeSet::new(),
            env_write: false,
            fs_ro: Some(Vec::new()),
            fs_rw: Some(Vec::new()),
            frozen_time_ms: None,
            random_seed: None,
        }
    }

    /// Parses the comma-separated `--caps` grammar (also used for the legacy
    /// `// vox:caps` comment's single-string form). One directory per `fs:ro=`/`fs:rw=`
    /// token — repeat the token to grant more than one root. `--caps` itself is
    /// repeatable at the CLI layer (`ArgAction::Append`), so a multi-root grant there
    /// is one flag per root rather than commas inside a single value; this parser's
    /// comma-splitting exists for the comment-directive and single-string embedding
    /// cases, which is also why it still rejects a literal `|` inside one token's value.
    pub fn parse(spec: &str) -> Result<Self, CapsParseError> {
        let mut out = Self::restrictive();
        for tok in spec.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            let err = |why| CapsParseError {
                token: tok.to_string(),
                why,
            };
            if tok == "deterministic" {
                out.frozen_time_ms = Some(0);
                out.random_seed = Some(0);
                out.allowed.insert("time".into());
                out.allowed.insert("crypto".into());
                continue;
            }
            let (ns, rest) = tok.split_once(':').ok_or(err("expected ns:value"))?;
            match (ns, rest) {
                ("fs", r) if r.starts_with("ro=") || r.starts_with("rw=") => {
                    let raw = &r[3..];
                    if raw.is_empty() {
                        return Err(err("fs needs a directory"));
                    }
                    if raw.contains('|') {
                        return Err(err("one directory per token; repeat fs:ro=/fs:rw="));
                    }
                    let root = std::fs::canonicalize(raw)
                        .map_err(|_| err("fs root does not exist or is unreadable"))?;
                    let target = if r.starts_with("ro=") {
                        &mut out.fs_ro
                    } else {
                        &mut out.fs_rw
                    };
                    target.get_or_insert_with(Vec::new).push(root);
                    out.allowed.insert("fs".into());
                    out.allowed.insert("io".into());
                }
                ("fs", _) => return Err(err("fs takes ro=<dir> or rw=<dir>")),
                ("io", _) => return Err(err("io is covered by fs:ro= / fs:rw=")),
                ("net" | "http", "allow") => {
                    out.allowed.insert("http".into());
                }
                ("net" | "http", "none") => {}
                ("net" | "http", _) => return Err(err("net takes none|allow")),
                ("env", "ro") => {
                    out.allowed.insert("env".into());
                }
                ("env", "rw") => {
                    out.allowed.insert("env".into());
                    out.env_write = true;
                }
                ("env", "none") => {}
                ("env", _) => return Err(err("env takes none|ro|rw")),
                ("time", "real") => {
                    out.allowed.insert("time".into());
                }
                ("time", r) if r.starts_with("frozen=") => {
                    let ms: i64 = r[7..]
                        .parse()
                        .map_err(|_| err("frozen= needs integer ms"))?;
                    if ms < 0 {
                        return Err(err("frozen= must be non-negative"));
                    }
                    out.frozen_time_ms = Some(ms);
                    out.allowed.insert("time".into());
                }
                ("time", _) => return Err(err("time takes real|frozen=<ms>")),
                ("random", r) if r.starts_with("seed=") => {
                    out.random_seed = Some(r[5..].parse().map_err(|_| err("seed= needs a u64"))?);
                    out.allowed.insert("crypto".into());
                }
                ("random", "deny") => {}
                ("random", _) => return Err(err("random takes seed=<u64>|deny")),
                ("process" | "secrets" | "agentos", "allow") => {
                    out.allowed.insert(ns.into());
                }
                ("process" | "secrets" | "agentos", "none") => {}
                ("process" | "secrets" | "agentos", _) => return Err(err("expected none|allow")),
                _ => return Err(err("unknown namespace")),
            }
        }
        Ok(out)
    }

    /// The typed constructor used by production embedders (the six sites documented on
    /// `production_embedders_assign_caps_explicitly` and the mesh executor). Roots are
    /// `PathBuf`s, not strings, so a directory containing a grammar separator (`,` `=`
    /// `|`) is perfectly fine here — unlike `parse`, this never round-trips the paths
    /// through the comma-split string grammar. `extra` tokens (e.g. `"env:ro"`,
    /// `"time:real"`) go through `parse` individually and are merged in.
    pub fn from_roots(
        ro: Vec<PathBuf>,
        rw: Vec<PathBuf>,
        extra: &[&str],
    ) -> Result<Self, CapsParseError> {
        let mut out = Self::restrictive();
        for d in ro {
            let root = std::fs::canonicalize(&d).map_err(|_| CapsParseError {
                token: d.display().to_string(),
                why: "fs root does not exist or is unreadable",
            })?;
            out.fs_ro.get_or_insert_with(Vec::new).push(root);
            out.allowed.insert("fs".into());
            out.allowed.insert("io".into());
        }
        for d in rw {
            let root = std::fs::canonicalize(&d).map_err(|_| CapsParseError {
                token: d.display().to_string(),
                why: "fs root does not exist or is unreadable",
            })?;
            out.fs_rw.get_or_insert_with(Vec::new).push(root);
            out.allowed.insert("fs".into());
            out.allowed.insert("io".into());
        }
        for tok in extra {
            let piece = Self::parse(tok)?;
            out.merge(piece);
        }
        Ok(out)
    }

    /// Tokens suitable for `vox run --caps <token> --caps <token>` (one root per token,
    /// matching the CLI's `ArgAction::Append` grammar — no comma-joining, unlike the
    /// legacy `to_spec`-style single string this replaces).
    pub fn to_tokens(&self) -> Vec<String> {
        let mut parts = Vec::new();
        for d in self.fs_ro.iter().flatten() {
            parts.push(format!("fs:ro={}", d.display()));
        }
        for d in self.fs_rw.iter().flatten() {
            parts.push(format!("fs:rw={}", d.display()));
        }
        if self.allowed.contains("http") {
            parts.push("net:allow".into());
        }
        if self.allowed.contains("process") {
            parts.push("process:allow".into());
        }
        if self.allowed.contains("env") {
            parts.push(if self.env_write {
                "env:rw".into()
            } else {
                "env:ro".into()
            });
        }
        if self.allowed.contains("secrets") {
            parts.push("secrets:allow".into());
        }
        if self.allowed.contains("agentos") {
            parts.push("agentos:allow".into());
        }
        match self.frozen_time_ms {
            Some(ms) => parts.push(format!("time:frozen={ms}")),
            None if self.allowed.contains("time") => parts.push("time:real".into()),
            None => {}
        }
        if let Some(s) = self.random_seed {
            parts.push(format!("random:seed={s}"));
        }
        parts
    }

    /// Additively combines two capability sets (used by `from_roots` to fold in
    /// `extra` tokens, and by callers reassembling a set from `to_tokens`). Roots and
    /// allowed namespaces union; the last non-`None` frozen-time / random-seed wins.
    pub fn merge(&mut self, other: Self) {
        self.allowed.extend(other.allowed);
        self.env_write |= other.env_write;
        match (&mut self.fs_ro, other.fs_ro) {
            (Some(a), Some(b)) => a.extend(b),
            (None, Some(b)) => self.fs_ro = Some(b),
            _ => {}
        }
        match (&mut self.fs_rw, other.fs_rw) {
            (Some(a), Some(b)) => a.extend(b),
            (None, Some(b)) => self.fs_rw = Some(b),
            _ => {}
        }
        if other.frozen_time_ms.is_some() {
            self.frozen_time_ms = other.frozen_time_ms;
        }
        if other.random_seed.is_some() {
            self.random_seed = other.random_seed;
        }
    }

    /// Maps the legacy `// vox:caps <word>...` comment directive (`vox-cli`/`vox-langtool`
    /// `run`) onto the new namespace grammar. Unscoped: legacy callers never provided
    /// per-directory roots, so `fs_ro`/`fs_rw` stay `None` (unrestricted within `fs`).
    pub fn from_legacy_directive(words: &[String]) -> Self {
        let mut allowed = BTreeSet::new();
        for w in words {
            match w.as_str() {
                "fs" => {
                    allowed.insert("fs".into());
                    allowed.insert("io".into());
                }
                "process" | "subprocess" => {
                    allowed.insert("process".into());
                }
                "net" | "http" => {
                    allowed.insert("http".into());
                }
                other => {
                    allowed.insert(other.into());
                }
            }
        }
        let env_write = allowed.contains("env");
        Self {
            allowed,
            env_write,
            fs_ro: None,
            fs_rw: None,
            frozen_time_ms: None,
            random_seed: None,
        }
    }

    /// The permissive default for interactive / trusted embedders (CLI `run`/`repl`/
    /// `play`, `vox-langtool` `run` absent a directive): every gated namespace allowed,
    /// filesystem unscoped.
    pub fn developer_default() -> Self {
        Self {
            allowed: GATED.iter().map(|s| s.to_string()).collect(),
            env_write: true,
            fs_ro: None,
            fs_rw: None,
            frozen_time_ms: None,
            random_seed: None,
        }
    }

    pub fn allows_namespace(&self, ns: &str) -> bool {
        PURE.contains(&ns) || self.allowed.contains(ns)
    }

    pub fn allows_env_write(&self) -> bool {
        self.env_write
    }

    /// `path` MUST be canonical (the fs builtins canonicalize before calling this).
    /// rw roots satisfy reads too — a writable directory is always readable.
    pub fn allows_path(&self, path: &Path, write: bool) -> bool {
        if !self.allows_namespace("fs") {
            return false;
        }
        let roots = if write { &self.fs_rw } else { &self.fs_ro };
        match roots {
            None => true,
            Some(rs) => {
                let extra = if write { None } else { self.fs_rw.as_deref() };
                rs.iter()
                    .chain(extra.into_iter().flatten())
                    .any(|r| is_under(path, r))
            }
        }
    }

    pub fn frozen_time_ms(&self) -> Option<i64> {
        self.frozen_time_ms
    }

    pub fn random_seed(&self) -> Option<u64> {
        self.random_seed
    }

    /// `@versioned` auto-snapshot is not covered by PURE `repo` — it is a
    /// write that restrictive embedders (`parse("")`, MCP) must not perform.
    /// Local / native runs (`developer_default`, or any grant of `fs` or
    /// `process`) still snapshot.
    pub fn allows_versioned_snapshot(&self) -> bool {
        self.allowed.contains("fs") || self.allowed.contains("process")
    }
}

fn is_under(path: &Path, root: &Path) -> bool {
    let p: Vec<Component> = path.components().collect();
    let r: Vec<Component> = root.components().collect();
    p.len() >= r.len() && p[..r.len()].iter().zip(&r).all(|(a, b)| component_eq(a, b))
}

/// Both sides are canonical, so only case (and, on Windows, drive-letter prefix
/// spelling) can differ — and NTFS/APFS-default are case-insensitive.
fn component_eq(a: &Component, b: &Component) -> bool {
    #[cfg(windows)]
    {
        if let (Component::Normal(x), Component::Normal(y)) = (a, b) {
            return x
                .to_string_lossy()
                .eq_ignore_ascii_case(&y.to_string_lossy());
        }
        if let (Component::Prefix(x), Component::Prefix(y)) = (a, b) {
            return x
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&y.as_os_str().to_string_lossy());
        }
    }
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn unmentioned_namespaces_are_denied() {
        let c = CapabilitySet::parse("env:ro").unwrap();
        assert!(c.allows_namespace("env"));
        for ns in ["fs", "process", "http", "secrets", "agentos", "crypto"] {
            assert!(!c.allows_namespace(ns), "{ns} must be denied");
        }
    }

    #[test]
    fn pure_namespaces_are_always_allowed() {
        let c = CapabilitySet::parse("").unwrap();
        for ns in [
            "path", "json", "csv", "toml", "yaml", "regex", "log", "db", "repo",
        ] {
            assert!(c.allows_namespace(ns), "{ns} is pure and must not be gated");
        }
    }

    #[test]
    fn roots_are_canonicalised_at_parse_time() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.txt"), "A").unwrap();
        let c = CapabilitySet::parse(&format!("fs:rw={}", d.path().display())).unwrap();
        let canon = std::fs::canonicalize(d.path().join("a.txt")).unwrap();
        assert!(c.allows_path(&canon, true), "root not canonicalised: {c:?}");
    }

    #[test]
    fn fs_roots_scope_reads_and_writes_separately_and_repeat_the_token() {
        let ro = tempfile::tempdir().unwrap();
        let ro2 = tempfile::tempdir().unwrap();
        let rw = tempfile::tempdir().unwrap();
        let c = CapabilitySet::parse(&format!(
            "fs:ro={},fs:ro={},fs:rw={}",
            ro.path().display(),
            ro2.path().display(),
            rw.path().display()
        ))
        .unwrap();
        let cro = std::fs::canonicalize(ro.path()).unwrap();
        let crw = std::fs::canonicalize(rw.path()).unwrap();
        assert!(c.allows_path(&cro.join("x"), false));
        assert!(!c.allows_path(&cro.join("x"), true));
        assert!(c.allows_path(&crw.join("x"), true));
        assert!(
            c.allows_path(&crw.join("x"), false),
            "rw roots satisfy reads"
        );
        // Component boundary: /tmp/job must not admit /tmp/jobx.
        let sibling = crw
            .parent()
            .unwrap()
            .join(format!("{}x", crw.file_name().unwrap().to_string_lossy()));
        assert!(!c.allows_path(&sibling.join("y"), true));
    }

    #[test]
    fn a_missing_root_is_a_receiver_error_not_a_silent_deny() {
        assert!(CapabilitySet::parse("fs:ro=/definitely/not/here").is_err());
    }

    #[test]
    fn pipe_separator_is_gone_and_io_token_is_rejected() {
        assert!(CapabilitySet::parse("fs:ro=/a|/b").is_err());
        assert!(CapabilitySet::parse("io:allow").is_err());
    }

    #[test]
    fn env_has_read_and_write_levels() {
        let ro = CapabilitySet::parse("env:ro").unwrap();
        assert!(ro.allows_namespace("env") && !ro.allows_env_write());
        let rw = CapabilitySet::parse("env:rw").unwrap();
        assert!(rw.allows_env_write());
    }

    #[test]
    fn frozen_time_and_seeded_random() {
        let c = CapabilitySet::parse("time:frozen=1700000000000,random:seed=7").unwrap();
        assert_eq!(c.frozen_time_ms(), Some(1_700_000_000_000));
        assert_eq!(c.random_seed(), Some(7));
        assert!(c.allows_namespace("crypto"));
        assert!(CapabilitySet::parse("time:frozen=-1").is_err());
        let d = CapabilitySet::parse("random:deny").unwrap();
        assert!(!d.allows_namespace("crypto"));
    }

    #[test]
    fn deterministic_shorthand_expands() {
        let c = CapabilitySet::parse("deterministic").unwrap();
        assert_eq!(c.frozen_time_ms(), Some(0));
        assert_eq!(c.random_seed(), Some(0));
        assert!(
            !c.allows_namespace("process")
                && !c.allows_namespace("http")
                && !c.allows_namespace("env")
        );
    }

    #[test]
    fn net_and_http_are_one_namespace() {
        assert!(
            CapabilitySet::parse("net:allow")
                .unwrap()
                .allows_namespace("http")
        );
        assert!(
            CapabilitySet::parse("http:allow")
                .unwrap()
                .allows_namespace("http")
        );
    }

    #[test]
    fn legacy_directive_maps_words_and_is_unscoped() {
        let c =
            CapabilitySet::from_legacy_directive(&["fs".into(), "subprocess".into(), "net".into()]);
        assert!(c.allows_namespace("fs") && c.allows_namespace("io"));
        assert!(c.allows_namespace("process"));
        assert!(
            c.allows_namespace("http"),
            "`net` was the legacy word for http (vox-langtool fixture)"
        );
        assert!(!c.allows_namespace("env"));
        assert!(c.allows_path(Path::new("/anything"), true));
    }

    #[test]
    fn is_classified_covers_pure_and_gated_only() {
        assert!(is_classified("fs") && is_classified("path"));
        assert!(!is_classified("nope"));
    }

    #[test]
    fn allows_versioned_snapshot_is_false_for_restrictive() {
        assert!(CapabilitySet::developer_default().allows_versioned_snapshot());
        assert!(
            !CapabilitySet::parse("")
                .unwrap()
                .allows_versioned_snapshot()
        );
        assert!(CapabilitySet::from_legacy_directive(&["fs".into()]).allows_versioned_snapshot());
        assert!(
            CapabilitySet::from_legacy_directive(&["process".into()]).allows_versioned_snapshot()
        );
        assert!(
            !CapabilitySet::parse("env:ro")
                .unwrap()
                .allows_versioned_snapshot()
        );
    }

    #[test]
    fn developer_default_allows_everything() {
        let c = CapabilitySet::developer_default();
        for ns in [
            "fs", "io", "process", "env", "secrets", "http", "time", "agentos", "crypto",
        ] {
            assert!(c.allows_namespace(ns));
        }
        assert!(c.allows_env_write() && c.allows_path(Path::new("/"), true));
    }

    #[test]
    fn from_roots_accepts_a_comma_in_the_directory() {
        let d = tempfile::Builder::new().prefix("a,b-").tempdir().unwrap();
        let c = CapabilitySet::from_roots(vec![], vec![d.path().to_path_buf()], &["time:real"])
            .unwrap();
        let canon = std::fs::canonicalize(d.path()).unwrap();
        assert!(c.allows_path(&canon.join("x"), true));
        let tokens = c.to_tokens();
        assert!(tokens.iter().any(|t| t.starts_with("fs:rw=")), "{tokens:?}");
        assert!(tokens.iter().any(|t| t == "time:real"), "{tokens:?}");
    }

    #[test]
    fn to_tokens_round_trips_through_parse_of_each_token() {
        let d = tempfile::tempdir().unwrap();
        let c = CapabilitySet::from_roots(
            vec![],
            vec![d.path().to_path_buf()],
            &["time:real", "env:ro"],
        )
        .unwrap();
        let mut again = CapabilitySet::parse("").unwrap();
        for t in c.to_tokens() {
            let piece = CapabilitySet::parse(&t).unwrap();
            again.merge(piece);
        }
        assert_eq!(c.frozen_time_ms(), again.frozen_time_ms());
        let canon = std::fs::canonicalize(d.path()).unwrap();
        assert!(again.allows_path(&canon.join("x"), true));
        assert!(again.allows_namespace("env") && !again.allows_env_write());
        assert_eq!(c, again);
    }

    #[test]
    fn bad_specs_name_the_offending_token() {
        for bad in [
            "fs",
            "fs:banana",
            "net:maybe",
            "time:frozen=abc",
            "nosuch:allow",
            "env:allow",
        ] {
            let e = CapabilitySet::parse(bad).unwrap_err();
            assert!(
                e.to_string().contains(bad.split('=').next().unwrap()),
                "{bad} → {e}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_drive_letters_survive_the_ns_separator_and_case() {
        let d = tempfile::tempdir().unwrap();
        let c = CapabilitySet::parse(&format!("fs:rw={}", d.path().display())).unwrap();
        let canon = std::fs::canonicalize(d.path()).unwrap();
        assert!(c.allows_path(&canon.join("a.txt"), true));
        let upper = PathBuf::from(canon.to_string_lossy().to_ascii_uppercase());
        assert!(
            c.allows_path(&upper.join("a.txt"), true),
            "NTFS is case-insensitive"
        );
    }

    /// Every production embedder must set `interp.caps` explicitly on the line right
    /// after `Interpreter::new(...)` — no site may rely on the constructor's default.
    /// Paths point at the real files (some differ from the plan's stale sketch, e.g.
    /// `commands/run.rs` not `run.rs`); the six sites are the complete grep result for
    /// `Interpreter::new` under `crates/` excluding `tests/` and `#[cfg(test)]` blocks.
    #[test]
    fn production_embedders_assign_caps_explicitly() {
        let patterns = [
            ("crates/vox-cli/src/commands/run.rs", "developer_default"),
            ("crates/vox-cli/src/commands/repl.rs", "developer_default"),
            ("crates/vox-cli/src/commands/play.rs", "developer_default"),
            (
                "crates/vox-langtool/src/commands/run.rs",
                "developer_default",
            ),
            (
                "crates/vox-terminal-core/src/vox_interp.rs",
                "CapabilitySet::parse",
            ),
            (
                "crates/vox-orchestrator-mcp/src/workspace_mcp/dispatch.rs",
                "from_roots",
            ),
        ];
        for (path, needle) in patterns {
            let full = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path);
            let src = std::fs::read_to_string(&full)
                .unwrap_or_else(|e| panic!("read {}: {e}", full.display()));
            assert!(
                src.contains("Interpreter::new") && src.contains(needle),
                "{path} missing explicit caps ({needle})"
            );
        }
    }
}
