//! `git` subprocess plumbing for repo history. argv only, never a shell.
//! `vox-graph-reader` is L0 and cannot depend on `vox-git` (L1); see spec §4.
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Git's well-known empty tree: the diff base of a root commit.
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

pub struct Git {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitMeta {
    pub sha: String,
    pub parents: Vec<String>,
    pub ts: i64,
    pub author: String,
    pub subject: String,
    pub body: String,
    pub co_authors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileStat {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub score: Option<u8>,
    pub added: u32,
    pub deleted: u32,
    pub binary: bool,
    pub whitespace_only: bool,
}

/// Config pinned on every call so porcelain output doesn't depend on the host.
/// AMENDED: #10 — `log.showSignature` corrupts `git show` parsing, and `diff.algorithm`, renames and the
/// rename limit change numstat/status, so ingest would not be deterministic per SHA.
const PINNED: &[&str] = &[
    "-c",
    "log.showSignature=false",
    "-c",
    "diff.algorithm=myers",
    "-c",
    "diff.renames=true",
    "-c",
    "diff.renameLimit=32767",
    "-c",
    "core.quotePath=false",
];

/// `git -C root`, isolated from the caller: every inherited `GIT_*` variable (hooks set
/// `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_CONFIG_PARAMETERS`, …) is removed, and `PINNED` config applies.
fn git_cmd(root: &Path) -> Command {
    let mut c = Command::new("git");
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("GIT_") {
            c.env_remove(&k);
        }
    }
    c.arg("-C").arg(root).args(PINNED);
    c
}

fn lines(s: String) -> Vec<String> {
    s.lines().map(str::to_string).collect()
}

fn range(since: Option<&str>, tip: &str) -> String {
    since.map_or(tip.to_string(), |s| format!("{s}..{tip}"))
}

impl Git {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    fn run(&self, args: &[&str]) -> io::Result<String> {
        let out = git_cmd(&self.root).args(args).output()?;
        if !out.status.success() {
            return Err(io::Error::other(format!(
                "git {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        String::from_utf8(out.stdout).map_err(io::Error::other)
    }

    pub fn rev_parse(&self, rev: &str) -> io::Result<String> {
        Ok(self
            .run(&["rev-parse", "--verify", &format!("{rev}^{{commit}}")])?
            .trim()
            .to_string())
    }

    pub fn is_ancestor(&self, a: &str, b: &str) -> bool {
        git_cmd(&self.root)
            .args(["merge-base", "--is-ancestor", a, b])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Absolute git common dir, shared by every worktree of the repository.
    pub fn common_dir(&self) -> io::Result<PathBuf> {
        Ok(PathBuf::from(
            self.run(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?
                .trim(),
        ))
    }

    /// `origin/main` when it exists (upstream history only grows), else local `main`.
    pub fn default_tip(&self) -> String {
        if self.rev_parse("refs/remotes/origin/main").is_ok() {
            "origin/main".into()
        } else {
            "main".into()
        }
    }

    /// First-parent commits after `since` (exclusive) through `tip`, oldest first.
    pub fn first_parent_range(&self, since: Option<&str>, tip: &str) -> io::Result<Vec<String>> {
        Ok(lines(self.run(&[
            "rev-list",
            "--first-parent",
            "--reverse",
            &range(since, tip),
        ])?))
    }

    pub fn commit_meta(&self, sha: &str) -> io::Result<CommitMeta> {
        let fmt = "--format=%H%x00%P%x00%ct%x00%an%x00%s%x00%b%x00%(trailers:key=Co-Authored-By,valueonly,separator=%x1f)";
        parse_commit_meta(&self.run(&["show", "-s", fmt, sha])?)
    }

    /// Subjects a merge brought in (`merge^1..merge^2`), oldest first; with `path`, only
    /// the commits that touched it.
    pub fn inner_subjects(&self, merge: &str, path: Option<&str>) -> io::Result<Vec<String>> {
        let span = format!("{merge}^1..{merge}^2");
        let mut args = vec!["log", "--reverse", "--format=%s", span.as_str()];
        if let Some(p) = path {
            args.extend(["--", p]);
        }
        Ok(lines(self.run(&args)?))
    }

    /// Per-file stats of `sha` against `parent` (empty tree for a root commit).
    pub fn file_deltas(&self, parent: Option<&str>, sha: &str) -> io::Result<Vec<FileStat>> {
        let base = parent.unwrap_or(EMPTY_TREE);
        let diff = |extra: &[&str]| -> io::Result<String> {
            let mut a = vec!["diff", "--no-ext-diff", "--no-textconv", "-M", "-z"];
            a.extend_from_slice(extra);
            a.extend_from_slice(&[base, sha]);
            self.run(&a)
        };
        let status = parse_name_status_z(&diff(&["--name-status"])?);
        let counts = parse_numstat_z(&diff(&["--numstat"])?);
        let ws = parse_numstat_z(&diff(&["--numstat", "-w"])?);
        Ok(merge_stats(status, &counts, &ws))
    }

    /// Paths whose `linguist-generated` attribute is set or `true` in `rev`'s own tree,
    /// so classification is deterministic per commit whichever worktree ingests it.
    pub fn generated(&self, rev: &str, paths: &[String]) -> io::Result<HashSet<String>> {
        if paths.is_empty() {
            return Ok(HashSet::new());
        }
        let mut child = git_cmd(&self.root)
            .args([
                "check-attr",
                &format!("--source={rev}"),
                "-z",
                "--stdin",
                "linguist-generated",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().expect("piped stdin");
        let input: Vec<u8> = paths.iter().flat_map(|p| p.bytes().chain([0])).collect();
        // Write on a thread: a large path list can fill stdout's pipe before stdin drains.
        let writer = std::thread::spawn(move || stdin.write_all(&input));
        let out = child.wait_with_output()?;
        writer
            .join()
            .map_err(|_| io::Error::other("check-attr writer panicked"))??;
        if !out.status.success() {
            // e.g. git < 2.40 has no `--source`: fail loudly instead of reporting nothing generated.
            return Err(io::Error::other(format!(
                "git check-attr: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(parse_check_attr_z(&String::from_utf8_lossy(&out.stdout)))
    }

    /// Paths present at `rev` (the history tip, not the current worktree).
    pub fn ls_tree(&self, rev: &str) -> io::Result<HashSet<String>> {
        Ok(self
            .run(&["ls-tree", "-r", "-z", "--name-only", rev])?
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect())
    }
}

pub(crate) fn parse_commit_meta(out: &str) -> io::Result<CommitMeta> {
    let f: Vec<&str> = out.trim_end_matches('\n').splitn(7, '\0').collect();
    if f.len() < 7 {
        return Err(io::Error::other(format!(
            "unexpected `git show` output: {out:?}"
        )));
    }
    Ok(CommitMeta {
        sha: f[0].to_string(),
        parents: f[1].split_whitespace().map(str::to_string).collect(),
        ts: f[2].trim().parse().map_err(io::Error::other)?,
        author: f[3].to_string(),
        subject: f[4].to_string(),
        body: f[5].trim().to_string(),
        co_authors: f[6]
            .split('\u{1f}')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    })
}

pub(crate) fn parse_name_status_z(out: &str) -> Vec<FileStat> {
    let mut v = Vec::new();
    let mut it = out.split('\0').filter(|t| !t.is_empty());
    while let Some(st) = it.next() {
        let (letter, score) = st.split_at(1);
        let old_path = matches!(letter, "R" | "C").then(|| it.next().unwrap_or("").to_string());
        let path = it.next().unwrap_or("").to_string();
        v.push(FileStat {
            path,
            old_path,
            status: letter.to_string(),
            score: score.parse().ok(),
            ..Default::default()
        });
    }
    v
}

/// path (new path for renames) -> (added, deleted, binary).
pub(crate) fn parse_numstat_z(out: &str) -> HashMap<String, (u32, u32, bool)> {
    let mut m = HashMap::new();
    let mut it = out.split('\0').filter(|t| !t.is_empty());
    while let Some(tok) = it.next() {
        let mut p = tok.splitn(3, '\t');
        let (a, d, path) = (
            p.next().unwrap_or(""),
            p.next().unwrap_or(""),
            p.next().unwrap_or(""),
        );
        let path = if path.is_empty() {
            let _old = it.next();
            it.next().unwrap_or("").to_string()
        } else {
            path.to_string()
        };
        m.insert(
            path,
            (a.parse().unwrap_or(0), d.parse().unwrap_or(0), a == "-"),
        );
    }
    m
}

pub(crate) fn merge_stats(
    mut status: Vec<FileStat>,
    counts: &HashMap<String, (u32, u32, bool)>,
    ws: &HashMap<String, (u32, u32, bool)>,
) -> Vec<FileStat> {
    for f in &mut status {
        if let Some(&(a, d, b)) = counts.get(&f.path) {
            (f.added, f.deleted, f.binary) = (a, d, b);
        }
        // `diff -w --numstat` omits (or zeroes) whitespace-only files.
        f.whitespace_only = f.status == "M"
            && !f.binary
            && ws.get(&f.path).is_none_or(|&(a, d, _)| a == 0 && d == 0);
    }
    status
}

pub(crate) fn parse_check_attr_z(out: &str) -> HashSet<String> {
    let t: Vec<&str> = out.split('\0').collect();
    t.as_chunks::<3>()
        .0
        .iter()
        .filter(|c| matches!(c[2], "set" | "true"))
        .map(|c| c[0].to_string())
        .collect()
}

/// One long-lived `git cat-file --batch` for all blob reads in an ingest.
pub struct BlobReader {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl BlobReader {
    pub fn spawn(root: &Path) -> io::Result<Self> {
        let mut child = git_cmd(root)
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    /// `rev:path` as UTF-8; `None` when missing, not a blob, non-UTF-8, or over `max_bytes`.
    pub fn read(&mut self, rev: &str, path: &str, max_bytes: usize) -> io::Result<Option<String>> {
        writeln!(self.stdin, "{rev}:{path}")?;
        self.stdin.flush()?;
        let mut header = String::new();
        self.stdout.read_line(&mut header)?;
        let mut parts = header.split_whitespace();
        let (_name, kind, size) = (parts.next(), parts.next(), parts.next());
        let Some(size) = size.and_then(|s| s.parse::<usize>().ok()) else {
            return Ok(None); // `missing` / `ambiguous`: header only, no body
        };
        let mut buf = vec![0u8; size + 1]; // body + trailing LF
        self.stdout.read_exact(&mut buf)?;
        buf.pop();
        if kind != Some("blob") || size > max_bytes {
            return Ok(None);
        }
        Ok(String::from_utf8(buf).ok())
    }
}

impl Drop for BlobReader {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numstat_z_handles_rename_and_binary() {
        let m = parse_numstat_z("1\t1\ta.rs\x001\t0\t\x00b.txt\x00c.txt\x00-\t-\timg.png\x00");
        assert_eq!(m["a.rs"], (1, 1, false));
        assert_eq!(m["c.txt"], (1, 0, false));
        assert_eq!(m["img.png"], (0, 0, true));
        assert!(!m.contains_key("b.txt"));
    }

    #[test]
    fn name_status_z_reads_score_and_old_path() {
        let v = parse_name_status_z("M\x00a.rs\x00R080\x00b.txt\x00c.txt\x00");
        assert_eq!(v[0].status, "M");
        assert_eq!(
            (
                v[1].status.as_str(),
                v[1].score,
                v[1].old_path.as_deref(),
                v[1].path.as_str()
            ),
            ("R", Some(80), Some("b.txt"), "c.txt")
        );
    }

    #[test]
    fn merge_stats_flags_whitespace_only_modifications() {
        let status = parse_name_status_z("M\x00ws.rs\x00M\x00real.rs\x00A\x00new.rs\x00");
        let counts = parse_numstat_z("1\t1\tws.rs\x001\t1\treal.rs\x003\t0\tnew.rs\x00");
        let ws = parse_numstat_z("1\t1\treal.rs\x003\t0\tnew.rs\x00");
        let v = merge_stats(status, &counts, &ws);
        assert!(v[0].whitespace_only && !v[1].whitespace_only && !v[2].whitespace_only);
    }

    #[test]
    fn check_attr_z_accepts_set_and_true() {
        let s = parse_check_attr_z(
            "a\x00linguist-generated\x00set\x00b\x00linguist-generated\x00unspecified\x00c\x00linguist-generated\x00true\x00",
        );
        assert!(s.contains("a") && s.contains("c") && !s.contains("b"));
    }

    #[test]
    fn every_call_pins_output_affecting_config() {
        let args: Vec<String> = git_cmd(Path::new("/r"))
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.contains(&"log.showSignature=false".to_string())
                && args.contains(&"core.quotePath=false".to_string())
        );
    }

    #[test]
    fn commit_meta_parses_fields() {
        let m = parse_commit_meta(
            "abc\x00p1 p2\x0042\x00Ann\x00feat: x\x00body\n\x00A <a@x>\x1fB <b@x>\n",
        )
        .unwrap();
        assert_eq!(m.parents, vec!["p1", "p2"]);
        assert_eq!(m.ts, 42);
        assert_eq!(m.co_authors, vec!["A <a@x>", "B <b@x>"]);
    }
}
