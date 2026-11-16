//! Durable, secret-scrubbed terminal command history (`TERM-WARP-02`).

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub const DEFAULT_HISTORY_CAP: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub command: String,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub ts_ms: u64,
}

pub struct HistoryStore {
    path: Option<PathBuf>,
    cap: usize,
    entries: Mutex<Vec<HistoryEntry>>,
}

impl HistoryStore {
    pub fn open(path: impl Into<PathBuf>, cap: usize) -> anyhow::Result<Self> {
        let path = path.into();
        let cap = cap.max(1);
        let mut entries = Vec::new();
        if path.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            }
            if let Ok(raw) = fs::read_to_string(&path) {
                for line in raw.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Ok(entry) = serde_json::from_str::<HistoryEntry>(trimmed)
                        && !entry.command.trim().is_empty()
                    {
                        entries.push(entry);
                    }
                }
            }
            if entries.len() > cap {
                let excess = entries.len() - cap;
                entries.drain(0..excess);
            }
        }
        Ok(Self {
            path: Some(path),
            cap,
            entries: Mutex::new(entries),
        })
    }

    pub fn in_memory(cap: usize) -> Self {
        Self {
            path: None,
            cap: cap.max(1),
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn record_with_scrubber<F>(
        &self,
        raw_command: &str,
        cwd: Option<&str>,
        exit_code: Option<i32>,
        now_ms: u64,
        extra_scrub: F,
    ) -> anyhow::Result<bool>
    where
        F: Fn(&str) -> String,
    {
        // HISTCONTROL=ignorespace: MUST evaluate leading space/tab BEFORE trimming.
        if raw_command.starts_with(' ') || raw_command.starts_with('\t') {
            return Ok(false);
        }
        let normalized = crate::block::strip_ansi_and_normalize(raw_command);
        if normalized.starts_with(' ') || normalized.starts_with('\t') {
            return Ok(false);
        }
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }

        let after_extra = extra_scrub(trimmed);
        let scrubbed = vox_redact::redact_owned(&after_extra);
        if scrubbed.trim().is_empty() {
            return Ok(false);
        }

        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(last) = guard.last_mut()
            && last.command == scrubbed
        {
            last.cwd = cwd.map(str::to_owned);
            last.exit_code = exit_code;
            last.ts_ms = now_ms;
        } else {
            guard.push(HistoryEntry {
                command: scrubbed,
                cwd: cwd.map(str::to_owned),
                exit_code,
                ts_ms: now_ms,
            });
            if guard.len() > self.cap {
                let excess = guard.len() - self.cap;
                guard.drain(0..excess);
            }
        }

        if let Some(ref path) = self.path {
            persist_atomic(path, &guard)?;
        }
        Ok(true)
    }

    pub fn record(
        &self,
        raw_command: &str,
        cwd: Option<&str>,
        exit_code: Option<i32>,
        now_ms: u64,
    ) -> anyhow::Result<bool> {
        self.record_with_scrubber(raw_command, cwd, exit_code, now_ms, |s| s.to_owned())
    }

    pub fn query(&self, needle: &str, active_cwd: Option<&str>, limit: usize) -> Vec<HistoryEntry> {
        if limit == 0 {
            return Vec::new();
        }
        let guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let q = needle.trim().to_ascii_lowercase();
        let norm_active_cwd = active_cwd.map(trim_slash);

        if q.is_empty() {
            let mut seen = HashSet::new();
            let mut out = Vec::with_capacity(limit.min(guard.len()));
            for entry in guard.iter().rev() {
                if seen.insert(entry.command.clone()) {
                    out.push(entry.clone());
                    if out.len() >= limit {
                        break;
                    }
                }
            }
            return out;
        }

        let mut scored: Vec<(i64, u64, usize, HistoryEntry)> = Vec::new();
        for (idx, entry) in guard.iter().enumerate() {
            let cmd_lower = entry.command.to_ascii_lowercase();
            if let Some(mut score) = subsequence_score(&cmd_lower, &q) {
                if let (Some(ac), Some(ec)) = (norm_active_cwd, entry.cwd.as_deref()) {
                    score += cwd_affinity_bonus(ac, trim_slash(ec));
                }
                scored.push((score, entry.ts_ms, idx, entry.clone()));
            }
        }

        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| b.2.cmp(&a.2))
        });

        let mut seen = HashSet::new();
        let mut out = Vec::with_capacity(limit.min(scored.len()));
        for (_, _, _, entry) in scored {
            if seen.insert(entry.command.clone()) {
                out.push(entry);
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        guard.clear();
        if let Some(ref path) = self.path {
            persist_atomic(path, &guard)?;
        }
        Ok(())
    }
}

fn trim_slash(s: &str) -> &str {
    let trimmed = s.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() { s } else { trimmed }
}

fn cwd_affinity_bonus(active: &str, entry_cwd: &str) -> i64 {
    if active == entry_cwd {
        return 25;
    }
    let is_parent = |parent: &str, child: &str| {
        child
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('\\'))
    };
    if is_parent(entry_cwd, active) || is_parent(active, entry_cwd) {
        10
    } else {
        0
    }
}

fn subsequence_score(cmd: &str, needle: &str) -> Option<i64> {
    let mut score: i64 = 0;
    if cmd.starts_with(needle) {
        score += 200;
    } else if cmd.contains(needle) {
        score += 120;
    }

    let cmd_chars: Vec<char> = cmd.chars().collect();
    let mut pos = 0usize;
    let mut prev_match: Option<usize> = None;

    for qc in needle.chars() {
        let found = (pos..cmd_chars.len()).find(|&i| cmd_chars[i] == qc)?;
        score += 10;
        if prev_match == Some(found.saturating_sub(1)) && found > 0 {
            score += 15;
        }
        if found == 0
            || matches!(
                cmd_chars[found - 1],
                ' ' | '/' | '\\' | '-' | '_' | ':' | '.'
            )
        {
            score += 12;
        }
        if let Some(prev) = prev_match {
            let gap = found.saturating_sub(prev + 1);
            score -= (gap.min(10)) as i64;
        }
        prev_match = Some(found);
        pos = found + 1;
    }
    Some(score)
}

fn persist_atomic(path: &Path, entries: &[HistoryEntry]) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent_existed = parent.exists();
    fs::create_dir_all(parent).context("creating terminal history parent directory")?;
    #[cfg(unix)]
    if !parent_existed {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .context("setting 0o700 on terminal history directory")?;
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("history.jsonl");
    let tmp_path = parent.join(format!(".{file_name}.tmp"));

    let mut file = fs::File::create(&tmp_path).context("creating temporary history file")?;
    for entry in entries {
        serde_json::to_writer(&mut file, entry).context("serializing history entry")?;
        file.write_all(b"\n")?;
    }
    file.flush()?;
    file.sync_all()?;
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))
            .context("setting 0o600 on temporary history file")?;
    }

    fs::rename(&tmp_path, path).context("atomically replacing terminal history file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignorespace_and_blank_commands_are_skipped_before_trim() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history.jsonl");
        let store = HistoryStore::open(&path, 100).expect("open history");

        assert!(
            !store
                .record(" export SECRET_KEY=abc", Some("/tmp"), Some(0), 100)
                .unwrap()
        );
        assert!(
            !store
                .record("\texport SECRET_KEY=abc", Some("/tmp"), Some(0), 101)
                .unwrap()
        );
        assert!(
            !store
                .record("   \n\t  ", Some("/tmp"), Some(0), 102)
                .unwrap()
        );
        assert!(!store.record("", None, None, 103).unwrap());

        assert!(store.query("", None, 10).is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn scrubs_three_stages_dedups_caps_persists_0600_and_clears() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("history.jsonl");
        let store = HistoryStore::open(&path, 3).expect("open history");

        let scrub_clavis = |s: &str| s.replace("clavis-super-secret-val", "[CLAVIS_REDACTED]");
        let raw = "\x1b[32mcurl -H 'X-Custom: clavis-super-secret-val' --key sk-live-1234567890abcdef1234567890\x1b[0m";
        assert!(
            store
                .record_with_scrubber(raw, Some("/work/a"), Some(0), 1_000, scrub_clavis)
                .unwrap()
        );

        let rows = store.query("", None, 10);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].command.contains("\x1b["));
        assert!(!rows[0].command.contains("clavis-super-secret-val"));
        assert!(
            !rows[0]
                .command
                .contains("sk-live-1234567890abcdef1234567890")
        );
        assert!(rows[0].command.contains("[CLAVIS_REDACTED]"));
        assert!(rows[0].command.contains("[REDACTED_KEY]"));

        assert!(
            store
                .record_with_scrubber(raw, Some("/work/b"), Some(2), 2_000, scrub_clavis)
                .unwrap()
        );
        let rows = store.query("", None, 10);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cwd.as_deref(), Some("/work/b"));
        assert_eq!(rows[0].exit_code, Some(2));
        assert_eq!(rows[0].ts_ms, 2_000);

        store
            .record("cargo check", Some("/work"), Some(0), 3_000)
            .unwrap();
        store
            .record("cargo test", Some("/work"), Some(0), 4_000)
            .unwrap();
        store
            .record("vox doctor", Some("/work"), Some(0), 5_000)
            .unwrap();

        let disk_raw = std::fs::read_to_string(&path).expect("read jsonl");
        assert!(!disk_raw.contains("clavis-super-secret-val"));
        assert!(!disk_raw.contains("sk-live-1234567890abcdef1234567890"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dir_mode = std::fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            let file_mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(dir_mode, 0o700);
            assert_eq!(file_mode, 0o600);
        }

        let reopened = HistoryStore::open(&path, 3).expect("reopen");
        let reloaded = reopened.query("", None, 10);
        assert_eq!(reloaded.len(), 3);
        assert_eq!(reloaded[0].command, "vox doctor");
        assert_eq!(reloaded[1].command, "cargo test");
        assert_eq!(reloaded[2].command, "cargo check");

        reopened.clear().expect("clear");
        assert!(reopened.query("", None, 10).is_empty());
        assert!(
            HistoryStore::open(&path, 3)
                .unwrap()
                .query("", None, 10)
                .is_empty()
        );
    }

    #[test]
    fn query_ranks_subsequence_recency_and_cwd_affinity() {
        let store = HistoryStore::in_memory(50);
        store
            .record(
                "cargo test -p vox-other",
                Some("/repo/crates/vox-other"),
                Some(0),
                1_000,
            )
            .unwrap();
        store
            .record(
                "cargo test -p vox-terminal-core",
                Some("/repo/crates/vox-terminal-core"),
                Some(0),
                1_100,
            )
            .unwrap();
        store
            .record("echo terminal cargounrelated", Some("/tmp"), Some(0), 1_200)
            .unwrap();

        let hits = store.query("cargo terminal", Some("/repo/crates/vox-terminal-core"), 20);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].command, "cargo test -p vox-terminal-core");

        let parent_hits = store.query("cargo test", Some("/repo/crates/vox-terminal-core/src"), 20);
        assert_eq!(parent_hits[0].command, "cargo test -p vox-terminal-core");

        let recent = store.query("", None, 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].ts_ms, 1_200);
        assert_eq!(recent[1].ts_ms, 1_100);
    }
}
