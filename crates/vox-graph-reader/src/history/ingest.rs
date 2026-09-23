//! Walk the tip's first-parent line into the store (spec §5).
use super::classify::{classify, subject_hint};
use super::git::{BlobReader, Git};
use super::lineage::{self, FileDelta};
use super::model::{ChangeRec, CommitRec, LineageRec, SymbolDelta};
use super::store::{HistoryStore, IngestLock};
// AMENDED: #4 — `classify` takes the status so renames/adds are never whitespace/symbol-neutral.
use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Blobs above this are never read (vendored bindings, generated schemas).
const MAX_BLOB_BYTES: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq)]
pub enum CatchUp {
    Done {
        ingested: usize,
        rebuilt: bool,
    },
    OverBudget {
        ingested: usize,
        behind: usize,
    },
    /// Another process holds `ingest.lock`; answer from what is on disk.
    Busy,
    /// `brief` never ingests.
    Skipped,
}

/// `Claude Opus 5 <noreply@anthropic.com>` -> `Claude Opus 5`.
pub fn co_author_name(trailer: &str) -> String {
    trailer
        .split('<')
        .next()
        .unwrap_or(trailer)
        .trim()
        .to_string()
}

fn symbol_delta(before: &HashSet<String>, after: &HashSet<String>) -> SymbolDelta {
    let mut added: Vec<String> = after.difference(before).cloned().collect();
    let mut removed: Vec<String> = before.difference(after).cloned().collect();
    added.sort();
    removed.sort();
    SymbolDelta { added, removed }
}

pub fn ingest_commit(
    git: &Git,
    blobs: &mut BlobReader,
    sha: &str,
) -> io::Result<(CommitRec, Vec<ChangeRec>, Vec<LineageRec>)> {
    let meta = git.commit_meta(sha)?;
    let parent = meta.parents.first().cloned();
    let stats = git.file_deltas(parent.as_deref(), sha)?;
    let generated = git.generated(
        sha,
        &stats.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
    )?;

    let mut contents = Vec::with_capacity(stats.len());
    for f in &stats {
        let before = match (&parent, f.binary || f.status == "A") {
            (Some(p), false) => {
                blobs.read(p, f.old_path.as_deref().unwrap_or(&f.path), MAX_BLOB_BYTES)?
            }
            _ => None,
        };
        let after = if f.binary || f.status == "D" {
            None
        } else {
            blobs.read(sha, &f.path, MAX_BLOB_BYTES)?
        };
        contents.push((before, after));
    }

    // Parse each file once; the symbol sets feed both ChangeRec and lineage.
    let mut changes = Vec::with_capacity(stats.len());
    let mut deltas = Vec::with_capacity(stats.len());
    for (f, (before, after)) in stats.iter().zip(&contents) {
        let syms = lineage::file_symbols(&f.path, before.as_deref(), after.as_deref());
        let is_generated = generated.contains(&f.path);
        let reasons = classify(
            &f.path,
            &f.status,
            is_generated,
            f.whitespace_only,
            before.as_deref(),
            after.as_deref(),
        );
        let mechanical = !reasons.is_empty();
        changes.push(ChangeRec {
            sha: sha.to_string(),
            path: f.path.clone(),
            old_path: f.old_path.clone(),
            status: f.status.clone(),
            added: f.added,
            deleted: f.deleted,
            symbols: syms.as_ref().map(|(b, a)| symbol_delta(b, a)),
            mechanical,
            reasons,
        });
        deltas.push(FileDelta {
            path: &f.path,
            old_path: f.old_path.as_deref(),
            status: &f.status,
            score: f.score,
            added: f.added,
            deleted: f.deleted,
            before: before.as_deref(),
            after: after.as_deref(),
            skip: is_generated || (f.status == "M" && mechanical),
            syms,
        });
    }
    let lineage = lineage::detect(sha, &deltas);

    let is_merge = meta.parents.len() > 1;
    let commit = CommitRec {
        sha: sha.to_string(),
        parent,
        ts: meta.ts,
        author: meta.author.clone(),
        agent: meta.co_authors.first().map(|c| co_author_name(c)),
        subject: meta.subject.clone(),
        body: meta.body.clone(),
        is_merge,
        inner_subjects: if is_merge {
            git.inner_subjects(sha, None)?
        } else {
            Vec::new()
        },
        mechanical: !changes.is_empty() && changes.iter().all(|c| c.mechanical),
        subject_hint: subject_hint(&meta.subject).map(str::to_string),
    };
    Ok((commit, changes, lineage))
}

/// Bring the store up to `tip`. If `tip` no longer contains `last_sha`, roll back to the
/// newest ingested commit that is still an ancestor; reset only when none is, the store is
/// corrupt (unparseable state, version mismatch, broken chain), or the next commit does not
/// sit on `last_sha`. With `budget`, stops at the deadline and keeps everything appended so far.
pub fn catch_up(
    repo_root: &Path,
    store: &HistoryStore,
    lock: &IngestLock,
    tip: &str,
    budget: Option<Duration>,
    progress: &mut dyn FnMut(usize, usize),
) -> io::Result<CatchUp> {
    let start = Instant::now();
    let git = Git::new(repo_root);
    let tip_sha = git.rev_parse(tip)?;
    // AMENDED: #16 — an unparseable state.json is corruption: reset, don't error forever.
    let state = store.state().ok();
    let healthy = match &state {
        Some(s) if s.is_current() => store.load_checked()?.1,
        _ => false,
    };
    let mut rebuilt = false;
    if !healthy {
        store.reset()?;
        rebuilt = true;
    } else if let Some(last) = state.as_ref().and_then(|s| s.last_sha.as_deref()) {
        if !git.is_ancestor(last, &tip_sha) {
            let data = store.load()?;
            match data
                .commits
                .iter()
                .rev()
                .find(|c| git.is_ancestor(&c.sha, &tip_sha))
            {
                // Rows past it stay on disk but are off the chain, so `load` ignores them.
                Some(c) => store.commit_state(&c.sha)?, // AMENDED: #19 — no rewrite (`repair`) under a possible takeover
                None => {
                    store.reset()?;
                    rebuilt = true;
                }
            }
        }
    }
    let since = store.state()?.last_sha;
    let mut todo = git.first_parent_range(since.as_deref(), &tip_sha)?;
    // AMENDED: #8 — `last..tip` can reach `last` through a non-first parent (foxtrot merge); the
    // first new commit must sit directly on `last` or the chain would break silently.
    if let (Some(s), Some(first)) = (since.as_deref(), todo.first()) {
        if git.commit_meta(first)?.parents.first().map(String::as_str) != Some(s) {
            store.reset()?;
            rebuilt = true;
            todo = git.first_parent_range(None, &tip_sha)?;
        }
    }
    let mut blobs = BlobReader::spawn(repo_root)?;
    for (i, sha) in todo.iter().enumerate() {
        if budget.is_some_and(|b| start.elapsed() >= b) {
            return Ok(CatchUp::OverBudget {
                ingested: i,
                behind: todo.len() - i,
            });
        }
        let (c, ch, li) = ingest_commit(&git, &mut blobs, sha)?;
        store.append(&c, &ch, &li)?;
        store.commit_state(sha)?;
        lock.touch()?;
        progress(i + 1, todo.len());
    }
    Ok(CatchUp::Done {
        ingested: todo.len(),
        rebuilt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn co_author_name_strips_email() {
        assert_eq!(
            co_author_name("Claude Opus 5 <noreply@anthropic.com>"),
            "Claude Opus 5"
        );
        assert_eq!(co_author_name("Cursor"), "Cursor");
    }
}
