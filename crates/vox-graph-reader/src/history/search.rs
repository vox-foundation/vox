//! Search over history (spec §6). Term-based matching with weighted scoring.
use super::model::HistoryData;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchHit {
    pub sha: String,
    pub ts: i64,
    pub subject: String,
    pub score: u32,
}

/// Term counting: subject ×3, merged-branch subjects ×2, touched paths/symbols ×2, body ×1.
/// ponytail: plain term matching; move to vox-search (tantivy + semantic) when it misses.
pub fn search(d: &HistoryData, query: &str, limit: usize) -> Vec<SearchHit> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() {
        return Vec::new();
    }
    let mut touched: HashMap<&str, String> = HashMap::new();
    for ch in &d.changes {
        let s = touched.entry(ch.sha.as_str()).or_default();
        s.push_str(&ch.path.to_lowercase());
        s.push('\n');
        for sym in ch
            .symbols
            .iter()
            .flat_map(|x| x.added.iter().chain(&x.removed))
        {
            s.push_str(&sym.to_lowercase());
            s.push('\n');
        }
    }
    let mut v: Vec<SearchHit> = d
        .commits
        .iter()
        .filter_map(|c| {
            let (subj, body, inner) = (
                c.subject.to_lowercase(),
                c.body.to_lowercase(),
                c.inner_subjects.join("\n").to_lowercase(),
            );
            let files = touched.get(c.sha.as_str()).map_or("", String::as_str);
            let score: u32 = terms
                .iter()
                .map(|t| {
                    let t = t.as_str();
                    3 * u32::from(subj.contains(t))
                        + 2 * u32::from(inner.contains(t))
                        + 2 * u32::from(files.contains(t))
                        + u32::from(body.contains(t))
                })
                .sum();
            (score > 0).then(|| SearchHit {
                sha: c.sha.clone(),
                ts: c.ts,
                subject: c.subject.clone(),
                score,
            })
        })
        .collect();
    v.sort_by(|a, b| b.score.cmp(&a.score).then(b.ts.cmp(&a.ts)));
    v.truncate(limit);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::model::*;

    const DAY: i64 = 86_400;

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
    fn search_matches_subject_paths_and_symbols() {
        assert_eq!(search(&data(), "panel", 5)[0].sha, "s2");
        assert!(
            search(&data(), "Engine::run", 5).len() >= 3,
            "symbol ids are searchable"
        );
        assert!(search(&data(), "", 5).is_empty());
    }
}
