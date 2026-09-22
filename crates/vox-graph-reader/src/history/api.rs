//! Single entry point for CLI and MCP: resolve the shared store, catch up under the ingest
//! lock (except `brief`), answer, and say whether the answer is complete.
use super::git::Git;
use super::ingest::{CatchUp, catch_up};
use super::model::{HistoryData, store_dir_name};
use super::query::{self, AreaBy, DAY, LogEntry};
use super::search::search as search_history;
use super::store::{HistoryStore, prune_old_versions};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CORPUS_ID: &str = "repo-history";
/// Sibling version stores unused this long are deleted.
const VERSION_MAX_AGE: Duration = Duration::from_secs(30 * 86_400);
const ALLOWLIST: &str = "contracts/ci/crate-edges.allow.v1.json";

fn d7() -> i64 {
    7
}
fn d20() -> usize {
    20
}
fn d30() -> i64 {
    30
}
fn d60() -> i64 {
    60
}
fn d90() -> i64 {
    90
}
fn by_crate() -> AreaBy {
    AreaBy::Crate
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case")]
pub enum HistoryQuery {
    Log {
        path: String,
        #[serde(default)]
        include_mechanical: bool,
        #[serde(default = "d20")]
        limit: usize,
    },
    Focus {
        #[serde(default = "d30")]
        since_days: i64,
        #[serde(default = "by_crate")]
        by: AreaBy,
        #[serde(default)]
        include_mechanical: bool,
        #[serde(default = "d20")]
        limit: usize,
    },
    Forgotten {
        #[serde(default = "d60")]
        min_age_days: i64,
        #[serde(default = "by_crate")]
        by: AreaBy,
        #[serde(default = "d20")]
        limit: usize,
    },
    Search {
        text: String,
        #[serde(default = "d20")]
        limit: usize,
    },
    Timeline {
        #[serde(default = "d90")]
        since_days: i64,
        #[serde(default = "d7")]
        bucket_days: i64,
        #[serde(default = "by_crate")]
        by: AreaBy,
    },
    Brief,
}

pub struct Paths<'a> {
    pub repo_root: &'a Path,
    /// `repo-code-graph` graph.json, for `forgotten` fan-in. Missing -> fan-in 0.
    pub code_graph: Option<&'a Path>,
}

#[derive(Debug)]
pub struct Answer {
    /// `{"complete": bool, "behind": n, "rows": …}`
    pub value: serde_json::Value,
    pub text: String,
    pub catch_up: CatchUp,
    pub complete: bool,
    pub behind: usize,
}

/// The store shared by every worktree, for this binary's schema/extractor version.
pub fn store_dir(git: &Git) -> io::Result<PathBuf> {
    Ok(git
        .common_dir()?
        .join("vox-cache")
        .join(CORPUS_ID)
        .join(store_dir_name()))
}

pub fn answer(
    p: &Paths,
    q: &HistoryQuery,
    now_ts: i64,
    budget: Option<Duration>,
    progress: &mut dyn FnMut(usize, usize),
) -> io::Result<Answer> {
    let git = Git::new(p.repo_root);
    let tip = git.default_tip();
    let dir = store_dir(&git)?;
    let store = HistoryStore::open(&dir)?;
    let brief = *q == HistoryQuery::Brief;
    let cu = if brief {
        CatchUp::Skipped
    } else {
        match store.try_lock()? {
            Some(lock) => {
                if let Some(root) = dir.parent() {
                    prune_old_versions(root, &dir, VERSION_MAX_AGE); // best effort, never fails the query
                }
                catch_up(p.repo_root, &store, &lock, &tip, budget, progress)?
            }
            None => CatchUp::Busy,
        }
    };
    // AMENDED: #8 — a broken chain is never reported as complete.
    let (d, intact) = store.load_checked()?;
    let behind = git
        .first_parent_range(d.commits.last().map(|c| c.sha.as_str()), &tip)?
        .len();
    let complete = intact && behind == 0;
    let (current, fan_in) = match q {
        HistoryQuery::Forgotten { by, .. } => live_and_fan_in(p, &git, &tip, &dir, *by)?,
        HistoryQuery::Brief => live_and_fan_in(p, &git, &tip, &dir, AreaBy::Crate)?,
        _ => Default::default(),
    };
    let (rows, text) = match q {
        HistoryQuery::Log {
            path,
            include_mechanical,
            limit,
        } => {
            let mut rows = query::log(&d, path, *include_mechanical, *limit);
            fill_inner(&git, &d, &mut rows)?;
            render(rows, |e| {
                let via = if e.path == *path {
                    String::new()
                } else {
                    format!(" via {} (w={:.2})", e.path, e.weight)
                };
                let inner: String = e.inner.iter().map(|s| format!("\n    · {s}")).collect();
                format!(
                    "{} {} +{}/-{} {}{via}{inner}",
                    short(&e.sha),
                    ymd(e.ts),
                    e.added,
                    e.deleted,
                    e.subject
                )
            })
        }
        HistoryQuery::Focus {
            since_days,
            by,
            include_mechanical,
            limit,
        } => render(
            query::focus_between(
                &d,
                now_ts - since_days * DAY,
                i64::MAX,
                *by,
                *include_mechanical,
                *limit,
            ),
            |a| format!("{:>5} commits {:>7} lines  {}", a.commits, a.lines, a.area),
        ),
        HistoryQuery::Forgotten {
            min_age_days,
            by,
            limit,
        } => render(
            query::forgotten(&d, now_ts, *min_age_days, &current, &fan_in, *by, *limit),
            |f| {
                format!(
                    "{:>5}d  fan-in {:>4}  {}  — {}",
                    f.days_since, f.fan_in, f.area, f.last_subject
                )
            },
        ),
        HistoryQuery::Search { text, limit } => render(search_history(&d, text, *limit), |h| {
            format!("{} {} {}", short(&h.sha), ymd(h.ts), h.subject)
        }),
        HistoryQuery::Timeline {
            since_days,
            bucket_days,
            by,
        } => render(
            query::timeline(&d, now_ts - since_days * DAY, now_ts, *bucket_days, *by, 3),
            |b| {
                let areas = b
                    .top
                    .iter()
                    .map(|a| format!("{} ({})", a.area, a.commits))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{}  {areas}{}",
                    ymd(b.start_ts),
                    b.highlights
                        .iter()
                        .map(|h| format!("\n    · {h}"))
                        .collect::<String>()
                )
            },
        ),
        HistoryQuery::Brief => {
            let s = query::brief(&d, now_ts, &current, &fan_in);
            (serde_json::Value::String(s.clone()), s)
        }
    };
    let text = match (complete, brief) {
        (true, _) => text,
        (false, true) => {
            format!("{text} · {behind} commits behind (run `vox graph history focus` to catch up)")
        }
        (false, false) => {
            format!("partial: {behind} commits not yet ingested; call again.\n{text}")
        }
    };
    let value = serde_json::json!({ "complete": complete, "behind": behind, "rows": rows });
    Ok(Answer {
        value,
        text,
        catch_up: cu,
        complete,
        behind,
    })
}

/// Merge rows: up to 5 inner subjects that touched the row's path.
fn fill_inner(git: &Git, d: &HistoryData, rows: &mut [LogEntry]) -> io::Result<()> {
    let merges: HashSet<&str> = d
        .commits
        .iter()
        .filter(|c| c.is_merge)
        .map(|c| c.sha.as_str())
        .collect();
    for e in rows.iter_mut().filter(|e| merges.contains(e.sha.as_str())) {
        let subjects = git.inner_subjects(&e.sha, Some(&e.path))?;
        let extra = subjects.len().saturating_sub(5);
        e.inner = subjects.into_iter().take(5).collect();
        if extra > 0 {
            e.inner.push(format!("+{extra} more"));
        }
    }
    Ok(())
}

fn live_and_fan_in(
    p: &Paths,
    git: &Git,
    tip: &str,
    dir: &Path,
    by: AreaBy,
) -> io::Result<(HashSet<String>, HashMap<String, u32>)> {
    let current = git.ls_tree(tip)?;
    let fan_in = match p.code_graph {
        Some(g) => cached_fan_in(dir, g, &p.repo_root.join(ALLOWLIST), by),
        None => HashMap::new(),
    };
    Ok((current, fan_in))
}

/// Fan-in per area, cached in `<store>/fan_in.json`. The cache is keyed by the size and mtime
/// of graph.json and the allowlist, plus the grouping, so a warm `brief` never parses the 14 MB graph.
fn cached_fan_in(dir: &Path, graph: &Path, allow: &Path, by: AreaBy) -> HashMap<String, u32> {
    let stamp = |p: &Path| {
        std::fs::metadata(p).ok().map(|m| {
            let t = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos());
            (m.len(), t)
        })
    };
    let key = format!("{by:?}:{:?}:{:?}", stamp(graph), stamp(allow));
    let cache = dir.join("fan_in.json");
    let cached: Option<(String, HashMap<String, u32>)> = std::fs::read(&cache)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    if let Some((k, m)) = cached {
        if k == key {
            return m;
        }
    }
    let read = |p: &Path| {
        std::fs::read(p)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
    };
    let allowed = read(allow)
        .map(|v| query::allowed_crate_edges(&v))
        .unwrap_or_default();
    let m = read(graph)
        .map(|v| query::fan_in_by_area(&v, by, &allowed))
        .unwrap_or_default();
    let _ = std::fs::write(&cache, serde_json::to_vec(&(key, &m)).unwrap_or_default());
    m
}

fn render<T: serde::Serialize>(
    rows: Vec<T>,
    line: impl Fn(&T) -> String,
) -> (serde_json::Value, String) {
    let text = rows.iter().map(&line).collect::<Vec<_>>().join("\n");
    (serde_json::to_value(&rows).unwrap_or_default(), text)
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(9)]
}

/// Unix seconds -> `YYYY-MM-DD` (UTC), Hinnant's days-to-civil.
fn ymd(ts: i64) -> String {
    let z = ts.div_euclid(DAY) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_json_defaults_and_ymd() {
        let q: HistoryQuery = serde_json::from_str(r#"{"query":"focus"}"#).unwrap();
        assert_eq!(
            q,
            HistoryQuery::Focus {
                since_days: 30,
                by: AreaBy::Crate,
                include_mechanical: false,
                limit: 20
            }
        );
        let q: HistoryQuery = serde_json::from_str(r#"{"query":"log","path":"a.rs"}"#).unwrap();
        assert!(matches!(q, HistoryQuery::Log { ref path, limit: 20, .. } if path == "a.rs"));
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(1_758_499_200), "2025-09-22");
    }
}
