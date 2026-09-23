//! Read-side views over loaded history (spec §6). Mechanical rows are excluded unless
//! `include_mechanical`. Everything is an in-memory scan.
//! ponytail: O(rows) per query; move to vox-db tables if a query passes ~200 ms.
use super::model::{CommitRec, HistoryData};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) const DAY: i64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AreaBy {
    Crate,
    Dir,
}

/// `crates/vox-cli/src/x.rs` -> `crates/vox-cli` (Crate) / `crates/vox-cli/src` (Dir).
pub fn area_of(path: &str, by: AreaBy) -> String {
    match by {
        AreaBy::Dir => path.rsplit_once('/').map_or(".", |(d, _)| d).to_string(),
        AreaBy::Crate => {
            let mut it = path.split('/');
            match (it.next(), it.next(), it.next()) {
                (
                    Some(top @ ("crates" | "apps" | "clients" | "docs" | "contracts")),
                    Some(second),
                    Some(_),
                ) => format!("{top}/{second}"),
                (Some(top), Some(_), _) => top.to_string(),
                _ => ".".to_string(),
            }
        }
    }
}

fn index(d: &HistoryData) -> HashMap<&str, &CommitRec> {
    d.commits.iter().map(|c| (c.sha.as_str(), c)).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AreaScore {
    pub area: String,
    pub commits: u32,
    pub lines: u32,
}

pub fn focus_between(
    d: &HistoryData,
    from_ts: i64,
    to_ts: i64,
    by: AreaBy,
    include_mechanical: bool,
    limit: usize,
) -> Vec<AreaScore> {
    let idx = index(d);
    let mut acc: HashMap<String, (HashSet<&str>, u32)> = HashMap::new();
    for ch in d
        .changes
        .iter()
        .filter(|c| include_mechanical || !c.mechanical)
    {
        let Some(c) = idx.get(ch.sha.as_str()) else {
            continue;
        };
        if c.ts < from_ts || c.ts >= to_ts {
            continue;
        }
        let e = acc.entry(area_of(&ch.path, by)).or_default();
        e.0.insert(ch.sha.as_str());
        e.1 += ch.added + ch.deleted;
    }
    let mut v: Vec<AreaScore> = acc
        .into_iter()
        .map(|(area, (shas, lines))| AreaScore {
            area,
            commits: shas.len() as u32,
            lines,
        })
        .collect();
    v.sort_by(|a, b| {
        b.commits
            .cmp(&a.commits)
            .then(b.lines.cmp(&a.lines))
            .then(a.area.cmp(&b.area))
    });
    v.truncate(limit);
    v
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LogEntry {
    pub sha: String,
    pub ts: i64,
    pub subject: String,
    pub agent: Option<String>,
    /// The path at the time (an ancestor path when reached through lineage).
    pub path: String,
    pub added: u32,
    pub deleted: u32,
    /// Product of lineage weights from `path` back to this row's file (1.0 = same file).
    pub weight: f32,
    pub mechanical: bool,
    /// For merge rows: inner subjects that touched `path` (filled by `api::answer`).
    pub inner: Vec<String>,
}

/// Changes to `path` and, through lineage, to what it was renamed/split/merged from.
/// An ancestor's rows count only up to the commit that created the edge.
pub fn log(d: &HistoryData, path: &str, include_mechanical: bool, limit: usize) -> Vec<LogEntry> {
    let idx = index(d);
    let ts_of = |sha: &str| idx.get(sha).map_or(i64::MIN, |c| c.ts);
    let mut queue = VecDeque::from([(path.to_string(), 1.0f32, i64::MAX)]);
    let mut seen = HashSet::new();
    let mut scope = Vec::new();
    while let Some((p, w, until)) = queue.pop_front() {
        if scope.len() >= 256 || !seen.insert(p.clone()) {
            continue;
        }
        for e in d.lineage.iter().filter(|e| e.to == p) {
            let t = ts_of(&e.sha);
            if t <= until {
                queue.push_back((e.from.clone(), w * e.weight, t));
            }
        }
        scope.push((p, w, until));
    }
    let mut out = Vec::new();
    for (p, w, until) in &scope {
        for ch in d
            .changes
            .iter()
            .filter(|c| &c.path == p && (include_mechanical || !c.mechanical))
        {
            let Some(c) = idx.get(ch.sha.as_str()) else {
                continue;
            };
            if c.ts <= *until {
                out.push(LogEntry {
                    sha: c.sha.clone(),
                    ts: c.ts,
                    subject: c.subject.clone(),
                    agent: c.agent.clone(),
                    path: p.clone(),
                    added: ch.added,
                    deleted: ch.deleted,
                    weight: *w,
                    mechanical: ch.mechanical,
                    inner: Vec::new(),
                });
            }
        }
    }
    out.sort_by(|a, b| b.ts.cmp(&a.ts).then(a.path.cmp(&b.path)));
    out.truncate(limit);
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Forgotten {
    pub area: String,
    pub days_since: i64,
    pub last_subject: String,
    pub fan_in: u32,
    pub score: f64,
}

/// Live areas ranked by days since their last meaningful change × ln(2 + fan-in), so a
/// dormant module others depend on outranks a dormant leaf. Areas with only mechanical
/// changes count from the start of recorded history.
pub fn forgotten(
    d: &HistoryData,
    now_ts: i64,
    min_age_days: i64,
    current_paths: &HashSet<String>,
    fan_in: &HashMap<String, u32>,
    by: AreaBy,
    limit: usize,
) -> Vec<Forgotten> {
    let live: HashSet<String> = current_paths.iter().map(|p| area_of(p, by)).collect();
    let idx = index(d);
    let mut last: HashMap<String, (i64, String)> = HashMap::new();
    let mut seen_areas = HashSet::new();
    for ch in &d.changes {
        let area = area_of(&ch.path, by);
        if !live.contains(&area) {
            continue;
        }
        seen_areas.insert(area.clone());
        let Some(c) = idx.get(ch.sha.as_str()) else {
            continue;
        };
        if ch.mechanical {
            continue;
        }
        let slot = last.entry(area).or_insert((i64::MIN, String::new()));
        if c.ts > slot.0 {
            *slot = (c.ts, c.subject.clone());
        }
    }
    let history_start = d.commits.iter().map(|c| c.ts).min().unwrap_or(now_ts);
    for area in seen_areas {
        last.entry(area).or_insert((
            history_start,
            "(no meaningful change in recorded history)".into(),
        ));
    }
    let mut v: Vec<Forgotten> = last
        .into_iter()
        .filter_map(|(area, (ts, subject))| {
            let days = (now_ts - ts) / DAY;
            let fi = fan_in.get(&area).copied().unwrap_or(0);
            (days >= min_age_days).then(|| Forgotten {
                score: days as f64 * (2.0 + f64::from(fi)).ln(),
                fan_in: fi,
                days_since: days,
                last_subject: subject,
                area,
            })
        })
        .collect();
    v.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.area.cmp(&b.area)));
    v.truncate(limit);
    v
}

/// `crates/<name>/…` -> `<name>` (directory name == package name for every crate).
fn crate_of(path: &str) -> Option<&str> {
    let mut it = path.split('/');
    (it.next() == Some("crates")).then(|| it.next()).flatten()
}

/// `{"edges": [[from, to], …]}` (`contracts/ci/crate-edges.allow.v1.json`) -> set of pairs.
pub fn allowed_crate_edges(json: &serde_json::Value) -> HashSet<(String, String)> {
    json.get("edges")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some((
                p.get(0)?.as_str()?.to_string(),
                p.get(1)?.as_str()?.to_string(),
            ))
        })
        .collect()
}

/// Incoming cross-area call edges per area from a graphify `graph.json`. A cross-crate edge
/// counts only when `allowed` declares that dependency, which drops stdlib homonyms that
/// name resolution bound to repo fns (e.g. `std::env::temp_dir` -> a test helper). Edges
/// touching files outside `crates/` are not counted.
pub fn fan_in_by_area(
    graph: &serde_json::Value,
    by: AreaBy,
    allowed: &HashSet<(String, String)>,
) -> HashMap<String, u32> {
    let edges = graph
        .get("links")
        .or_else(|| graph.get("edges"))
        .and_then(|v| v.as_array());
    let mut m = HashMap::new();
    for e in edges.into_iter().flatten() {
        let file = |k: &str| {
            e.get(k)
                .and_then(|v| v.as_str())
                .and_then(|id| id.split_once("::"))
                .map(|(p, _)| p)
        };
        let (Some(sp), Some(tp)) = (file("source"), file("target")) else {
            continue;
        };
        let (Some(sc), Some(tc)) = (crate_of(sp), crate_of(tp)) else {
            continue;
        };
        let (sa, ta) = (area_of(sp, by), area_of(tp, by));
        if sa != ta && (sc == tc || allowed.contains(&(sc.to_string(), tc.to_string()))) {
            *m.entry(ta).or_insert(0) += 1;
        }
    }
    m
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bucket {
    pub start_ts: i64,
    pub top: Vec<AreaScore>,
    pub highlights: Vec<String>,
}

/// Fixed-width buckets from `since_ts`: top areas plus up to 5 feat/merge subjects each.
pub fn timeline(
    d: &HistoryData,
    since_ts: i64,
    now_ts: i64,
    bucket_days: i64,
    by: AreaBy,
    per_bucket: usize,
) -> Vec<Bucket> {
    let width = bucket_days.max(1) * DAY;
    let mut out = Vec::new();
    let mut start = since_ts;
    while start < now_ts {
        let end = start + width;
        let highlights = d
            .commits
            .iter()
            .filter(|c| {
                c.ts >= start
                    && c.ts < end
                    && !c.mechanical
                    && (c.is_merge || c.subject.starts_with("feat"))
            })
            .take(5)
            .map(|c| c.subject.clone())
            .collect();
        out.push(Bucket {
            start_ts: start,
            top: focus_between(d, start, end, by, false, per_bucket),
            highlights,
        });
        start = end;
    }
    out
}

/// One line for SessionStart: top-3 focus areas over 7 days and the top forgotten area.
pub fn brief(
    d: &HistoryData,
    now_ts: i64,
    current_paths: &HashSet<String>,
    fan_in: &HashMap<String, u32>,
) -> String {
    let focus = focus_between(d, now_ts - 7 * DAY, i64::MAX, AreaBy::Crate, false, 3);
    let f = if focus.is_empty() {
        "none".to_string()
    } else {
        focus
            .iter()
            .map(|a| format!("{} ({})", a.area, a.commits))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let g = forgotten(d, now_ts, 30, current_paths, fan_in, AreaBy::Crate, 1)
        .into_iter()
        .next()
        .map_or("none".to_string(), |x| {
            format!("{} ({}d, fan-in {})", x.area, x.days_since, x.fan_in)
        });
    format!("history: focus 7d: {f} · forgotten: {g}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::model::*;
    use serde_json::json;

    const NOW: i64 = 200 * DAY;

    fn c(sha: &str, day: i64, subject: &str) -> CommitRec {
        CommitRec {
            sha: sha.into(),
            parent: None,
            ts: day * DAY,
            author: "a".into(),
            agent: None,
            subject: subject.into(),
            body: String::new(),
            is_merge: false,
            inner_subjects: vec![],
            mechanical: false,
            subject_hint: None,
        }
    }
    fn ch(sha: &str, path: &str, mech: bool) -> ChangeRec {
        ChangeRec {
            sha: sha.into(),
            path: path.into(),
            old_path: None,
            status: "M".into(),
            added: 5,
            deleted: 1,
            symbols: Some(SymbolDelta {
                added: vec!["Engine::run".into()],
                removed: vec![],
            }),
            mechanical: mech,
            reasons: vec![],
        }
    }
    fn data() -> HistoryData {
        HistoryData {
            commits: vec![
                c("s1", 10, "feat: old core work"),
                c("s2", 195, "feat: gui panel"),
                c("s3", 198, "style: fmt"),
                c("s4", 190, "refactor: split engine"),
            ],
            changes: vec![
                ch("s1", "crates/core/src/lib.rs", false),
                ch("s2", "crates/gui/src/a.rs", false),
                ch("s3", "crates/core/src/lib.rs", true),
                ch("s4", "crates/gui/src/host.rs", false),
            ],
            lineage: vec![LineageRec {
                sha: "s4".into(),
                from: "crates/gui/src/engine.rs".into(),
                to: "crates/gui/src/host.rs".into(),
                kind: LineageKind::Split,
                method: LineageMethod::Symbol,
                weight: 0.5,
                evidence: 3,
            }],
        }
    }

    #[test]
    fn area_of_groups_by_crate_or_dir() {
        assert_eq!(
            area_of("crates/vox-cli/src/x.rs", AreaBy::Crate),
            "crates/vox-cli"
        );
        assert_eq!(
            area_of("crates/vox-cli/src/x.rs", AreaBy::Dir),
            "crates/vox-cli/src"
        );
        assert_eq!(area_of("scripts/fmt.vox", AreaBy::Crate), "scripts");
        assert_eq!(area_of("README.md", AreaBy::Crate), ".");
    }

    #[test]
    fn focus_excludes_mechanical_by_default() {
        let f = focus_between(&data(), 180 * DAY, i64::MAX, AreaBy::Crate, false, 10);
        assert_eq!(f[0].area, "crates/gui");
        assert!(
            f.iter().all(|a| a.area != "crates/core"),
            "core only had an fmt change recently"
        );
        let all = focus_between(&data(), 180 * DAY, i64::MAX, AreaBy::Crate, true, 10);
        assert!(all.iter().any(|a| a.area == "crates/core"));
    }

    #[test]
    fn log_follows_lineage_with_weight() {
        let mut d = data();
        d.commits.push(c("s0", 5, "feat: engine"));
        d.changes.push(ch("s0", "crates/gui/src/engine.rs", false));
        let l = log(&d, "crates/gui/src/host.rs", false, 10);
        assert_eq!(
            l.iter().map(|e| e.sha.as_str()).collect::<Vec<_>>(),
            ["s4", "s0"]
        );
        assert_eq!(l[1].weight, 0.5);
        assert!(
            l.iter().all(|e| e.inner.is_empty()),
            "the facade fills inner subjects"
        );
    }

    #[test]
    fn forgotten_ranks_old_high_fan_in_live_areas() {
        let live: HashSet<String> = ["crates/core/src/lib.rs", "crates/gui/src/a.rs"]
            .map(String::from)
            .into();
        let fan_in = HashMap::from([("crates/core".to_string(), 40)]);
        let f = forgotten(&data(), NOW, 30, &live, &fan_in, AreaBy::Crate, 5);
        assert_eq!(f[0].area, "crates/core");
        assert_eq!(
            f[0].days_since, 190,
            "the fmt commit on day 198 does not count"
        );
        assert!(
            f.iter().all(|x| x.area != "crates/gui"),
            "gui was touched 5 days ago"
        );
    }

    #[test]
    fn fan_in_counts_only_declared_crate_dependencies() {
        let g = json!({"links": [
            {"source": "crates/a/src/x.rs::f", "target": "crates/b/src/y.rs::g"},
            {"source": "crates/c/src/x.rs::f", "target": "crates/b/src/y.rs::temp_dir"},
            {"source": "crates/b/src/z.rs::h", "target": "crates/b/src/y.rs::g"},
            {"source": "cmd:x", "target": "crates/b/src/y.rs::g"}]});
        let allowed = allowed_crate_edges(&json!({"schema_version": 1, "edges": [["a", "b"]]}));
        assert_eq!(
            fan_in_by_area(&g, AreaBy::Crate, &allowed),
            HashMap::from([("crates/b".to_string(), 1)]),
            "c->b is a stdlib homonym (c does not depend on b); b->b is the same area"
        );
        let g2 = json!({"links": [{"source": "crates/b/src/z.rs::h", "target": "crates/b/tests/y.rs::g"}]});
        assert_eq!(
            fan_in_by_area(&g2, AreaBy::Dir, &HashSet::new()),
            HashMap::from([("crates/b/tests".to_string(), 1)]),
            "same crate, different dirs needs no allowlist entry"
        );
    }

    #[test]
    fn timeline_buckets_and_brief_line() {
        let t = timeline(&data(), 186 * DAY, NOW, 7, AreaBy::Crate, 3);
        assert_eq!(t.len(), 2);
        assert!(
            t.iter()
                .flat_map(|b| &b.highlights)
                .any(|h| h == "feat: gui panel")
        );
        let live: HashSet<String> = ["crates/core/src/lib.rs"].map(String::from).into();
        let b = brief(&data(), NOW, &live, &HashMap::new());
        assert!(
            b.starts_with("history: focus 7d: crates/gui (1)")
                && b.contains("forgotten: crates/core"),
            "{b}"
        );
    }
}
