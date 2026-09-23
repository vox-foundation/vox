//! Append-only JSONL store for repo history (spec §4). Validity comes from the first-parent
//! chain: `load` walks `parent` back from `state.last_sha`, so rows from interrupted appends,
//! rolled-back commits, or overlapping writers are ignored regardless of file order.
use super::model::{ChangeRec, CommitRec, HistoryData, LineageRec, State};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::hash::{BuildHasher, RandomState};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const COMMITS: &str = "commits.jsonl";
const CHANGES: &str = "changes.jsonl";
const LINEAGE: &str = "lineage.jsonl";
const STATE: &str = "state.json";
const LOCK: &str = "ingest.lock";
/// A lock untouched for this long is from a crashed ingest; `touch` runs after every commit.
const STALE_LOCK: Duration = Duration::from_secs(600);

pub struct HistoryStore {
    dir: PathBuf,
}

/// Held for one catch-up. The file stores a token, and `drop` removes it only when the token
/// is still ours, so a stalled owner never undoes a later takeover.
pub struct IngestLock {
    path: PathBuf,
    token: String,
}

impl IngestLock {
    /// Refresh the lock's mtime so a long backfill is not mistaken for a crashed one.
    pub fn touch(&self) -> io::Result<()> {
        OpenOptions::new()
            .write(true)
            .open(&self.path)?
            .set_modified(SystemTime::now())
    }
}

impl Drop for IngestLock {
    fn drop(&mut self) {
        if fs::read_to_string(&self.path).is_ok_and(|t| t == self.token) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn new_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // RandomState is seeded per process from OS randomness: no RNG dependency needed.
    format!(
        "{}-{:016x}",
        std::process::id(),
        RandomState::new().hash_one(nanos)
    )
}

impl HistoryStore {
    pub fn open(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    pub fn state(&self) -> io::Result<State> {
        match fs::read_to_string(self.dir.join(STATE)) {
            Ok(s) => serde_json::from_str(&s).map_err(io::Error::other),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(State::fresh()),
            Err(e) => Err(e),
        }
    }

    /// Delete all rows and the state (corrupt store, or no ingested commit survives a rewrite).
    pub fn reset(&self) -> io::Result<()> {
        for f in [COMMITS, CHANGES, LINEAGE, STATE] {
            match fs::remove_file(self.dir.join(f)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }

    /// Append one commit's rows. Follow with `commit_state`.
    pub fn append(
        &self,
        commit: &CommitRec,
        changes: &[ChangeRec],
        lineage: &[LineageRec],
    ) -> io::Result<()> {
        append_rows(&self.dir.join(CHANGES), changes)?;
        append_rows(&self.dir.join(LINEAGE), lineage)?;
        append_rows(&self.dir.join(COMMITS), std::slice::from_ref(commit))
    }

    /// Atomically record `sha` as the last fully ingested commit.
    pub fn commit_state(&self, sha: &str) -> io::Result<()> {
        let st = State {
            last_sha: Some(sha.to_string()),
            ..State::fresh()
        };
        let tmp = self.dir.join("state.json.tmp");
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(&st).map_err(io::Error::other)?,
        )?;
        fs::rename(tmp, self.dir.join(STATE))
    }

    pub fn load(&self) -> io::Result<HistoryData> {
        Ok(self.load_checked()?.0)
    }

    /// Rows on the first-parent chain ending at `state.last_sha`, oldest first, deduplicated.
    /// The flag is `false` when `last_sha` is set but the chain never reaches a root commit.
    pub fn load_checked(&self) -> io::Result<(HistoryData, bool)> {
        let Some(last) = self.state()?.last_sha else {
            return Ok((HistoryData::default(), true));
        };
        let rows: Vec<CommitRec> = read_rows(&self.dir.join(COMMITS))?;
        let by_sha: HashMap<&str, &CommitRec> = rows.iter().map(|c| (c.sha.as_str(), c)).collect();
        let mut commits: Vec<CommitRec> = Vec::new();
        let mut cur = Some(last.as_str());
        let mut intact = false;
        while let Some(sha) = cur {
            let Some(c) = by_sha.get(sha) else { break };
            if commits.len() > rows.len() {
                break; // a parent cycle can only come from a corrupt file
            }
            commits.push((*c).clone());
            cur = c.parent.as_deref();
            intact = cur.is_none();
        }
        commits.reverse();
        let valid: HashSet<String> = commits.iter().map(|c| c.sha.clone()).collect();
        let mut seen = HashSet::new();
        let changes: Vec<ChangeRec> = read_rows::<ChangeRec>(&self.dir.join(CHANGES))?
            .into_iter()
            .filter(|r| valid.contains(&r.sha) && seen.insert((r.sha.clone(), r.path.clone())))
            .collect();
        let mut seen = HashSet::new();
        let lineage: Vec<LineageRec> = read_rows::<LineageRec>(&self.dir.join(LINEAGE))?
            .into_iter()
            .filter(|r| {
                valid.contains(&r.sha) && seen.insert((r.sha.clone(), r.from.clone(), r.to.clone()))
            })
            .collect();
        Ok((
            HistoryData {
                commits,
                changes,
                lineage,
            },
            intact,
        ))
    }

    /// `None` when another live process is ingesting.
    pub fn try_lock(&self) -> io::Result<Option<IngestLock>> {
        let path = self.dir.join(LOCK);
        for _ in 0..2 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    let token = new_token();
                    f.write_all(token.as_bytes())?;
                    return Ok(Some(IngestLock { path, token }));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    match fs::metadata(&path).and_then(|m| m.modified()) {
                        Ok(t)
                            if SystemTime::now().duration_since(t).unwrap_or_default()
                                <= STALE_LOCK =>
                        {
                            return Ok(None);
                        }
                        Ok(_) => {
                            let _ = fs::remove_file(&path);
                        }
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e),
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(None)
    }
}

/// `s<digits>-e<alnum/.>`: the only directory names `prune_old_versions` may delete.
fn is_version_dir(name: &str) -> bool {
    let Some((schema, ext)) = name.strip_prefix('s').and_then(|r| r.split_once("-e")) else {
        return false;
    };
    !schema.is_empty()
        && schema.bytes().all(|b| b.is_ascii_digit())
        && !ext.is_empty()
        && ext.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
}

/// Best-effort delete of sibling version directories under `root`, other than `keep`,
/// whose `state.json` has not been written for `max_age`. Returns how many were removed.
/// Errors are skipped: housekeeping must never fail a query.
/// ponytail: age-based cleanup; move under `vox graph gc` if it ever misfires.
pub fn prune_old_versions(root: &Path, keep: &Path, max_age: Duration) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    let mut removed = 0;
    for e in entries.flatten() {
        let p = e.path();
        if p == keep || !p.is_dir() || !is_version_dir(&e.file_name().to_string_lossy()) {
            continue;
        }
        let Ok(stamp) = fs::metadata(p.join(STATE))
            .or_else(|_| fs::metadata(&p))
            .and_then(|m| m.modified())
        else {
            continue;
        };
        if SystemTime::now().duration_since(stamp).unwrap_or_default() > max_age
            && fs::remove_dir_all(&p).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

fn append_rows<T: Serialize>(path: &Path, rows: &[T]) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut f = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    let mut buf = Vec::new();
    // A torn final line (crash mid-write) must not swallow the first new row.
    if ends_without_newline(&mut f)? {
        buf.push(b'\n');
    }
    for r in rows {
        serde_json::to_writer(&mut buf, r).map_err(io::Error::other)?;
        buf.push(b'\n');
    }
    f.write_all(&buf)
}

fn ends_without_newline(f: &mut File) -> io::Result<bool> {
    let len = f.metadata()?.len();
    if len == 0 {
        return Ok(false);
    }
    f.seek(SeekFrom::Start(len - 1))?;
    let mut last = [0u8; 1];
    f.read_exact(&mut last)?;
    Ok(last[0] != b'\n')
}

/// JSONL rows; unparseable lines (a torn final write) are skipped.
fn read_rows<T: DeserializeOwned>(path: &Path) -> io::Result<Vec<T>> {
    let f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for line in BufReader::new(f).lines() {
        if let Ok(row) = serde_json::from_str(&line?) {
            out.push(row);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::model::*;
    use std::time::{Duration, SystemTime};

    fn commit(sha: &str, parent: Option<&str>) -> CommitRec {
        CommitRec {
            sha: sha.into(),
            parent: parent.map(Into::into),
            ts: 1,
            author: "a".into(),
            agent: None,
            subject: "s".into(),
            body: String::new(),
            is_merge: false,
            inner_subjects: vec![],
            mechanical: false,
            subject_hint: None,
        }
    }
    fn change(sha: &str, path: &str) -> ChangeRec {
        ChangeRec {
            sha: sha.into(),
            path: path.into(),
            old_path: None,
            status: "M".into(),
            added: 1,
            deleted: 0,
            symbols: None,
            mechanical: false,
            reasons: vec![],
        }
    }

    #[test]
    fn load_follows_the_parent_chain_not_file_order() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        s.append(&commit("a", None), &[change("a", "x.rs")], &[])
            .unwrap();
        s.commit_state("a").unwrap();
        // Local cherry-picks p1, p2 get ingested, then the tip is rewritten.
        s.append(&commit("p1", Some("a")), &[change("p1", "p.rs")], &[])
            .unwrap();
        s.append(&commit("p2", Some("p1")), &[change("p2", "p.rs")], &[])
            .unwrap();
        s.commit_state("p2").unwrap();
        // Rollback to `a` without compaction, then the upstream commit u1 lands on `a`.
        s.commit_state("a").unwrap();
        s.append(&commit("u1", Some("a")), &[change("u1", "u.rs")], &[])
            .unwrap();
        s.commit_state("u1").unwrap();
        let d = s.load().unwrap();
        assert_eq!(
            d.commits.iter().map(|c| c.sha.as_str()).collect::<Vec<_>>(),
            ["a", "u1"]
        );
        assert_eq!(
            d.changes
                .iter()
                .map(|c| c.path.as_str())
                .collect::<Vec<_>>(),
            ["x.rs", "u.rs"]
        );
    }

    #[test]
    fn overlapping_writer_duplicates_collapse_and_orphans_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        for _ in 0..2 {
            s.append(&commit("a", None), &[change("a", "x.rs")], &[])
                .unwrap();
        }
        s.commit_state("a").unwrap();
        s.append(&commit("b", Some("a")), &[change("b", "y.rs")], &[])
            .unwrap(); // state never advanced
        let d = s.load().unwrap();
        assert_eq!((d.commits.len(), d.changes.len()), (1, 1));
    }

    // AMENDED: #6 — the next append must not be glued onto a torn fragment.
    #[test]
    fn a_torn_line_does_not_swallow_the_next_append() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        s.append(&commit("a", None), &[change("a", "x.rs")], &[])
            .unwrap();
        s.commit_state("a").unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(dir.path().join("changes.jsonl"))
            .unwrap()
            .write_all(b"{\"sha\":\"a\",\"pa")
            .unwrap();
        s.append(&commit("b", Some("a")), &[change("b", "y.rs")], &[])
            .unwrap();
        s.commit_state("b").unwrap();
        assert_eq!(
            s.load().unwrap().changes.len(),
            2,
            "b's row survives the torn line before it"
        );
    }

    // AMENDED: #20 — the stale-lock takeover branch is otherwise untested.
    #[test]
    fn a_stale_lock_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        std::mem::forget(s.try_lock().unwrap().expect("lock")); // a crashed owner never releases
        let old = SystemTime::now() - Duration::from_secs(700);
        std::fs::File::options()
            .write(true)
            .open(dir.path().join("ingest.lock"))
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(s.try_lock().unwrap().is_some());
    }

    #[test]
    fn broken_chain_is_reported_and_torn_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        s.append(&commit("b", Some("missing")), &[], &[]).unwrap();
        s.commit_state("b").unwrap();
        assert!(
            !s.load_checked().unwrap().1,
            "chain does not reach a root commit"
        );
        s.reset().unwrap();
        s.append(&commit("a", None), &[change("a", "x.rs")], &[])
            .unwrap();
        s.commit_state("a").unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(dir.path().join("changes.jsonl"))
            .unwrap()
            .write_all(b"{\"sha\":\"a\",\"pa")
            .unwrap();
        let (d, intact) = s.load_checked().unwrap();
        assert!(intact);
        assert_eq!(d.changes.len(), 1);
    }

    #[test]
    fn missing_state_is_fresh_and_reset_clears_everything() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        assert_eq!(s.state().unwrap(), State::fresh());
        s.append(&commit("a", None), &[], &[]).unwrap();
        s.commit_state("a").unwrap();
        s.reset().unwrap();
        assert_eq!(s.state().unwrap().last_sha, None);
        assert!(s.load().unwrap().commits.is_empty());
    }

    #[test]
    fn lock_is_exclusive_and_only_its_owner_removes_it() {
        let dir = tempfile::tempdir().unwrap();
        let s = HistoryStore::open(dir.path()).unwrap();
        let lock_file = dir.path().join("ingest.lock");
        let a = s.try_lock().unwrap().expect("first lock");
        assert!(s.try_lock().unwrap().is_none());
        // Simulate a takeover after a stall: another process now owns the file.
        std::fs::write(&lock_file, "other-process-token").unwrap();
        drop(a);
        assert!(
            lock_file.exists(),
            "a stalled owner must not delete the new owner's lock"
        );
        std::fs::remove_file(&lock_file).unwrap();
        let b = s.try_lock().unwrap().expect("lock after release");
        b.touch().unwrap();
        drop(b);
        assert!(!lock_file.exists());
    }

    #[test]
    fn prune_removes_only_old_sibling_version_dirs() {
        let root = tempfile::tempdir().unwrap();
        let dir = |n: &str| {
            let d = root.path().join(n);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("state.json"), "{}").unwrap();
            d
        };
        let (keep, old, fresh) = (dir("s1-e4"), dir("s1-e3"), dir("s1-e2"));
        let (notes, lookalike) = (dir("notes"), dir("server-env")); // AMENDED: #15 — never prune non-version dirs
        let forty_days_ago = SystemTime::now() - Duration::from_secs(40 * 86_400);
        for d in [&old, &notes, &lookalike] {
            std::fs::File::options()
                .write(true)
                .open(d.join("state.json"))
                .unwrap()
                .set_modified(forty_days_ago)
                .unwrap();
        }
        assert_eq!(
            prune_old_versions(root.path(), &keep, Duration::from_secs(30 * 86_400)),
            1
        );
        assert!(
            !old.exists()
                && keep.exists()
                && fresh.exists()
                && notes.exists()
                && lookalike.exists()
        );
    }
}
