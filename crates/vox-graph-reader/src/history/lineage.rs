//! Rename / split / merge lineage between files within one commit (spec §5.2):
//! symbol migration where the extractor parses the file, normalized-line overlap otherwise.
use super::model::{LineageKind, LineageMethod, LineageRec};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Lines an `M` file must lose (source) or gain (target) to be considered.
pub const MIN_CHURN: u32 = 40;
const SYMBOL_MIN_MOVED: usize = 2;
const SYMBOL_MIN_WEIGHT: f32 = 0.2;
// ponytail: fixed thresholds; tune against the real-history regressions if precision is poor.
const LINE_MIN_MATCHED: usize = 15;
const LINE_MIN_WEIGHT: f32 = 0.3;
/// A normalized line present in this many eligible files is boilerplate.
const COMMON_LINE_FILES: usize = 3;

pub struct FileDelta<'a> {
    pub path: &'a str,
    pub old_path: Option<&'a str>,
    pub status: &'a str,
    pub score: Option<u8>,
    pub added: u32,
    pub deleted: u32,
    pub before: Option<&'a str>,
    pub after: Option<&'a str>,
    /// Generated, or a mechanical modification: never a lineage source or target.
    pub skip: bool,
    /// `(before, after)` symbol suffixes for parseable files, computed once by the caller.
    pub syms: Option<(HashSet<String>, HashSet<String>)>,
}

pub fn parseable(path: &str) -> bool {
    matches!(ext(path), "rs" | "ts" | "tsx" | "js" | "jsx" | "py")
}

fn ext(path: &str) -> &str {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
}

/// Symbol ids minus the file path and any `#n` suffix (`Type::method`, `free_fn`):
/// the part that survives a move between files.
pub fn symbol_suffixes(path: &str, content: &str) -> HashSet<String> {
    let g = crate::ast::extract_ast_in_module(Path::new(path), content, path);
    g.nodes
        .iter()
        .filter_map(|n| {
            n.id.strip_prefix(path)?
                .strip_prefix("::")
                .map(strip_dup_suffix)
        })
        .collect()
}

/// Before/after symbol sets for a parseable file; `None` otherwise.
pub fn file_symbols(
    path: &str,
    before: Option<&str>,
    after: Option<&str>,
) -> Option<(HashSet<String>, HashSet<String>)> {
    parseable(path).then(|| {
        let s = |c: Option<&str>| c.map(|c| symbol_suffixes(path, c)).unwrap_or_default();
        (s(before), s(after))
    })
}

fn strip_dup_suffix(s: &str) -> String {
    match s.rsplit_once('#') {
        Some((head, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
            head.to_string()
        }
        _ => s.to_string(),
    }
}

/// Trimmed lines without blanks, bracket-only lines, imports, derives, and `//` comments.
pub fn normalized_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|l| {
            !(l.is_empty()
                || l.chars().all(|c| "{}()[];,".contains(c))
                || l.starts_with("use ")
                || l.starts_with("pub use ")
                || l.starts_with("import ")
                || l.starts_with("from ")
                || l.starts_with("#[derive")
                || l.starts_with("//"))
        })
        .map(str::to_string)
        .collect()
}

type Bag = HashMap<String, usize>;

fn bag(lines: Vec<String>) -> Bag {
    let mut m = Bag::new();
    for l in lines {
        *m.entry(l).or_insert(0) += 1;
    }
    m
}

fn bag_minus(a: &Bag, b: &Bag) -> Bag {
    a.iter()
        .filter_map(|(k, &n)| {
            let r = n.saturating_sub(b.get(k).copied().unwrap_or(0));
            (r > 0).then(|| (k.clone(), r))
        })
        .collect()
}

fn bag_overlap(a: &Bag, b: &Bag) -> usize {
    a.iter()
        .map(|(k, &n)| n.min(b.get(k).copied().unwrap_or(0)))
        .sum()
}

/// Content that left (`source`) or arrived (`!source`): (total symbols, moved set) and lines.
fn delta_side(f: &FileDelta, source: bool) -> (Option<(usize, HashSet<String>)>, Bag) {
    let whole = (source && f.status == "D") || (!source && matches!(f.status, "A" | "R"));
    let empty = HashSet::new();
    let syms = f.syms.as_ref().map(|(b, a)| {
        let (from, other) = if source { (b, a) } else { (a, b) };
        let other = if whole { &empty } else { other };
        (from.len(), from.difference(other).cloned().collect())
    });
    let (from, other) = if source {
        (f.before, f.after)
    } else {
        (f.after, f.before)
    };
    let other = if whole { "" } else { other.unwrap_or("") };
    (
        syms,
        bag_minus(
            &bag(normalized_lines(from.unwrap_or(""))),
            &bag(normalized_lines(other)),
        ),
    )
}

pub fn detect(sha: &str, files: &[FileDelta]) -> Vec<LineageRec> {
    let rec = |from: &str, to: &str, kind, method, weight, evidence| LineageRec {
        sha: sha.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        kind,
        method,
        weight,
        evidence,
    };
    let mut out: Vec<LineageRec> = files
        .iter()
        .filter(|f| f.status == "R")
        .filter_map(|f| {
            Some(rec(
                f.old_path?,
                f.path,
                LineageKind::Rename,
                LineageMethod::GitRename,
                f32::from(f.score.unwrap_or(100)) / 100.0,
                0,
            ))
        })
        .collect();

    let sources: Vec<&FileDelta> = files
        .iter()
        .filter(|f| {
            !f.skip
                && f.before.is_some()
                && (f.status == "D" || (f.status == "M" && f.deleted >= MIN_CHURN))
        })
        .collect();
    let targets: Vec<&FileDelta> = files
        .iter()
        .filter(|f| {
            !f.skip
                && f.after.is_some()
                && (matches!(f.status, "A" | "R") || (f.status == "M" && f.added >= MIN_CHURN))
        })
        .collect();
    let mut src_sides: Vec<_> = sources.iter().map(|s| delta_side(s, true)).collect();
    let mut tgt_sides: Vec<_> = targets.iter().map(|t| delta_side(t, false)).collect();

    // Lines held by 3+ eligible files (version pins, table rows) say nothing about lineage.
    let common: HashSet<String> = {
        let mut holders: HashMap<&str, HashSet<&str>> = HashMap::new();
        for (f, (_, b)) in sources
            .iter()
            .zip(&src_sides)
            .chain(targets.iter().zip(&tgt_sides))
        {
            for l in b.keys() {
                holders.entry(l.as_str()).or_default().insert(f.path);
            }
        }
        holders
            .into_iter()
            .filter(|(_, h)| h.len() >= COMMON_LINE_FILES)
            .map(|(l, _)| l.to_string())
            .collect()
    };
    for (_, b) in src_sides.iter_mut().chain(tgt_sides.iter_mut()) {
        b.retain(|l, _| !common.contains(l));
    }

    let mut edges: Vec<(usize, usize, LineageMethod, f32, u32)> = Vec::new();
    for (si, s) in sources.iter().enumerate() {
        let (s_syms, s_lines) = &src_sides[si];
        let s_total: usize = s_lines.values().sum();
        for (ti, t) in targets.iter().enumerate() {
            if t.path == s.path {
                continue;
            }
            let (t_syms, t_lines) = &tgt_sides[ti];
            if let (Some((total, removed)), Some((_, added))) = (s_syms, t_syms) {
                let moved = removed.intersection(added).count();
                let weight = if *total == 0 {
                    0.0
                } else {
                    moved as f32 / *total as f32
                };
                if moved >= SYMBOL_MIN_MOVED || (moved >= 1 && weight >= SYMBOL_MIN_WEIGHT) {
                    edges.push((si, ti, LineageMethod::Symbol, weight, moved as u32));
                    continue;
                }
            }
            if s_total == 0 || ext(s.path) != ext(t.path) {
                continue;
            }
            let t_total: usize = t_lines.values().sum();
            let matched = bag_overlap(s_lines, t_lines);
            let weight = matched as f32 / s_total as f32;
            let t_share = if t_total == 0 {
                0.0
            } else {
                matched as f32 / t_total as f32
            };
            if matched >= LINE_MIN_MATCHED
                && weight >= LINE_MIN_WEIGHT
                && t_share >= LINE_MIN_WEIGHT
            {
                edges.push((si, ti, LineageMethod::Line, weight, matched as u32));
            }
        }
    }

    // A symbol or line copied into several targets must not count twice: scale each source's
    // outgoing weights so they sum to at most 1.
    let mut sums: HashMap<usize, f32> = HashMap::new();
    for &(s, _, _, w, _) in &edges {
        *sums.entry(s).or_insert(0.0) += w;
    }
    for e in &mut edges {
        let total = sums[&e.0];
        if total > 1.0 {
            e.3 /= total;
        }
    }

    let mut per_source: HashMap<usize, usize> = HashMap::new();
    let mut per_target: HashMap<&str, usize> = HashMap::new();
    for r in &out {
        *per_target.entry(r.to.as_str()).or_insert(0) += 1;
    }
    for &(s, t, ..) in &edges {
        *per_source.entry(s).or_insert(0) += 1;
        *per_target.entry(targets[t].path).or_insert(0) += 1;
    }
    let mut detected = Vec::with_capacity(edges.len());
    for (s, t, method, weight, evidence) in edges {
        let kind = if per_source[&s] >= 2 {
            LineageKind::Split
        } else if per_target[targets[t].path] >= 2 {
            LineageKind::Merge
        } else {
            LineageKind::Move
        };
        detected.push(rec(
            sources[s].path,
            targets[t].path,
            kind,
            method,
            weight,
            evidence,
        ));
    }
    out.extend(detected);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fd<'a>(
        path: &'a str,
        status: &'a str,
        before: Option<&'a str>,
        after: Option<&'a str>,
    ) -> FileDelta<'a> {
        let n = |s: Option<&str>| s.map_or(0, |c| c.lines().count() as u32);
        FileDelta {
            path,
            old_path: None,
            status,
            score: None,
            added: if status == "D" { 0 } else { n(after) },
            deleted: if status == "A" { 0 } else { n(before) },
            before,
            after,
            skip: false,
            syms: file_symbols(path, before, after),
        }
    }

    const ABC: &str = "fn alpha() { 1 }\nfn beta() { 2 }\nfn gamma() { 3 }\n";

    #[test]
    fn symbol_split_into_two_files() {
        let files = [
            fd("a.rs", "D", Some(ABC), None),
            fd(
                "b.rs",
                "A",
                None,
                Some("fn alpha() { 1 }\nfn beta() { 2 }\n"),
            ),
            fd("c.rs", "A", None, Some("fn gamma() { 3 }\n")),
        ];
        let out = detect("s", &files);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| e.kind == LineageKind::Split
            && e.method == LineageMethod::Symbol
            && e.from == "a.rs"));
        let total: f32 = out.iter().map(|e| e.weight).sum();
        assert!((total - 1.0).abs() < 1e-4, "weights sum to 1, got {total}");
    }

    #[test]
    fn methods_keep_identity_across_files() {
        let s = symbol_suffixes("x/a.rs", "struct E;\nimpl E { fn run(&self) {} }\n");
        assert!(s.contains("E::run") && s.contains("E"), "{s:?}");
    }

    #[test]
    fn line_fallback_splits_a_toml_file() {
        let block = |p: &str| {
            (0..20)
                .map(|i| format!("{p}_{i} = {i}\n"))
                .collect::<String>()
        };
        let (x, y) = (block("x"), block("y"));
        let whole = format!("{x}{y}");
        let files = [
            fd("a.toml", "D", Some(&whole), None),
            fd("x.toml", "A", None, Some(&x)),
            fd("y.toml", "A", None, Some(&y)),
        ];
        let out = detect("s", &files);
        assert_eq!(out.len(), 2);
        assert!(
            out.iter()
                .all(|e| e.method == LineageMethod::Line && e.kind == LineageKind::Split)
        );
    }

    #[test]
    fn rename_plus_folded_duplicate_is_a_merge() {
        let body = "fn load() { 1 }\nfn save() { 2 }\n";
        let mut ren = fd("core/io.rs", "R", Some(body), Some(body));
        ren.old_path = Some("cuda/io.rs");
        ren.score = Some(100);
        let files = [ren, fd("metal/io.rs", "D", Some(body), None)];
        let out = detect("s", &files);
        let kinds: Vec<_> = out.iter().map(|e| (e.from.as_str(), e.kind)).collect();
        assert!(
            kinds.contains(&("cuda/io.rs", LineageKind::Rename)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&("metal/io.rs", LineageKind::Merge)),
            "{kinds:?}"
        );
    }

    // AMENDED: #5 — the same symbols copied into two targets must not give away 200% of the source.
    #[test]
    fn a_source_never_gives_away_more_than_all_of_itself() {
        let files = [
            fd("a.rs", "D", Some(ABC), None),
            fd("b.rs", "A", None, Some(ABC)),
            fd("c.rs", "A", None, Some(ABC)),
        ];
        let out = detect("s", &files);
        assert_eq!(out.len(), 2);
        let total: f32 = out.iter().map(|e| e.weight).sum();
        assert!(total <= 1.0 + 1e-4, "outgoing weight {total} > 1");
    }

    #[test]
    fn skipped_files_never_take_part() {
        let mut src = fd("a.rs", "D", Some(ABC), None);
        src.skip = true;
        assert!(detect("s", &[src, fd("b.rs", "A", None, Some(ABC))]).is_empty());
    }

    #[test]
    fn line_fallback_needs_same_extension_and_target_share() {
        let block = |p: &str, n: usize| {
            (0..n)
                .map(|i| format!("{p}_{i} = {i}\n"))
                .collect::<String>()
        };
        let moved = block("m", 20);
        assert!(
            detect(
                "s",
                &[
                    fd("a.toml", "D", Some(&moved), None),
                    fd("b.md", "A", None, Some(&moved))
                ]
            )
            .is_empty(),
            "different extensions never pair"
        );
        let big = format!("{moved}{}", block("other", 180));
        assert!(
            detect(
                "s",
                &[
                    fd("a.toml", "D", Some(&moved), None),
                    fd("b.toml", "A", None, Some(&big))
                ]
            )
            .is_empty(),
            "moved lines are only 10% of the new target"
        );
    }

    #[test]
    fn lines_shared_by_three_files_are_boilerplate() {
        let common = (0..20)
            .map(|i| format!("pin_{i} = \"1.0\"\n"))
            .collect::<String>();
        let src = format!("{common}unique_a = 1\n");
        let files = [
            fd("a.toml", "D", Some(&src), None),
            fd("x.toml", "A", None, Some(&common)),
            fd("z.toml", "A", None, Some(&common)),
        ];
        assert!(detect("s", &files).is_empty());
    }

    #[test]
    fn small_edits_and_unrelated_files_produce_no_lineage() {
        let files = [
            fd("a.rs", "M", Some(ABC), Some("fn alpha() { 9 }\n")),
            fd("z.rs", "A", None, Some("fn zeta() {}\n")),
        ];
        assert!(detect("s", &files).is_empty());
        assert_eq!(
            normalized_lines("use a::b;\n\n  }\n  let x = 1;\n"),
            vec!["let x = 1;"]
        );
        assert!(parseable("a.tsx") && !parseable("a.toml"));
    }
}
