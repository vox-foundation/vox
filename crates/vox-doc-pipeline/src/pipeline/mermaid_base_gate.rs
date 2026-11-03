//! Base-ref compare for the shrink-only legacy-Mermaid ratchet (VoxDiagram Phase 87, R4).
//!
//! Pure over the two baseline texts plus the renames git detected between the base ref and head.
//! Rule: no baselined path may exceed its base count, and the total never grows. A path absent
//! from the base baseline is admitted only as the target of a git-detected rename (`git diff -M`)
//! of a baselined path whose entry is removed from head and whose count it does not exceed. A
//! plain delete-and-add therefore cannot move quota between unrelated docs.

use std::collections::BTreeSet;

use super::mermaid_gate::parse;

/// A git-detected rename, `(old, new)`, as repo-relative paths.
pub type Rename = (String, String);

/// Failures of the head baseline against the base baseline. `None` is an absent file. Pre-seed and
/// bootstrap (absent at base) pass; a baseline deleted after it existed at base, or an unparsable
/// side, fails. Each path is checked against its base count, and an unlisted path must be the
/// target of an unused rename, so the total is non-increasing by construction.
#[must_use]
pub fn mermaid_baseline_gate_failures(
    base_text: Option<&str>,
    head_text: Option<&str>,
    renames: &[Rename],
) -> Vec<String> {
    let side = |name: &str, text: &str| {
        parse(text).map_err(|e| vec![format!("[legacy-mermaid] {name} baseline invalid: {e}")])
    };
    let base = match base_text.map(|t| side("base", t)).transpose() {
        Ok(b) => b,
        Err(f) => return f,
    };
    let head = match head_text.map(|t| side("head", t)).transpose() {
        Ok(h) => h,
        Err(f) => return f,
    };
    let (Some(base), Some(head)) = (base, head) else {
        // pre-seed or bootstrap pass; a baseline deleted after it existed at base fails
        return match (base_text, head_text) {
            (Some(_), None) => vec!["[legacy-mermaid] baseline deleted".to_string()],
            _ => Vec::new(),
        };
    };
    let mut spent: BTreeSet<&str> = BTreeSet::new();
    let mut failures = Vec::new();
    for (path, &n) in &head.files {
        match base.files.get(path) {
            Some(&was) if n > was => failures.push(format!(
                "[legacy-mermaid] {path}: {was} -> {n} fences (the baseline only shrinks against the base ref)"
            )),
            Some(_) => {}
            None => {
                let from = renames.iter().find(|(old, new)| {
                    new == path
                        && !spent.contains(old.as_str())
                        && !head.files.contains_key(old)
                        && base.files.get(old).is_some_and(|&was| n <= was)
                });
                match from {
                    Some((old, _)) => {
                        spent.insert(old);
                    }
                    None => failures.push(format!(
                        "[legacy-mermaid] {path}: 0 -> {n} fences (a new path is admitted only as a git-detected rename of a baselined doc whose entry is removed; the baseline only shrinks against the base ref)"
                    )),
                }
            }
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bl(files: &[(&str, usize)]) -> String {
        let rows: String = files.iter().map(|(p, n)| format!("\n  {p}: {n}")).collect();
        let body = if rows.is_empty() { " {}".into() } else { rows };
        format!("schema_version: 1\nfiles:{body}\n")
    }

    fn mv(old: &str, new: &str) -> Rename {
        (old.to_string(), new.to_string())
    }

    #[test]
    fn gate_text_cases() {
        let a1 = bl(&[("docs/src/a.md", 1)]);
        let gate = |b: Option<&str>, h: Option<&str>| mermaid_baseline_gate_failures(b, h, &[]);
        assert!(gate(None, None).is_empty(), "pre-seed");
        assert!(gate(None, Some(&a1)).is_empty(), "bootstrap");
        assert!(gate(Some(&a1), Some(&a1)).is_empty(), "identical");
        let a2 = bl(&[("docs/src/a.md", 2)]);
        assert!(gate(Some(&a2), Some(&a1)).is_empty(), "lowered");
        assert!(gate(Some(&a1), Some(&bl(&[]))).is_empty(), "entry removed");

        let up = gate(Some(&a1), Some(&a2));
        assert_eq!(up.len(), 1, "{up:?}");
        assert!(up[0].contains("docs/src/a.md: 1 -> 2"), "{up:?}");

        let b = bl(&[("docs/src/a.md", 1), ("docs/src/b.md", 1)]);
        let added = gate(Some(&a1), Some(&b));
        assert_eq!(added.len(), 1, "{added:?}");
        assert!(added[0].contains("docs/src/b.md: 0 -> 1"), "{added:?}");

        assert_eq!(
            gate(Some(&a1), None),
            ["[legacy-mermaid] baseline deleted".to_string()]
        );
        let bad_head = gate(Some(&a1), Some("not: [valid"));
        assert_eq!(bad_head.len(), 1, "{bad_head:?}");
        assert!(
            bad_head[0].contains("head baseline invalid"),
            "{bad_head:?}"
        );
        let bad_base = gate(Some("schema_version: 2\nfiles: {}\n"), Some(&a1));
        assert_eq!(bad_base.len(), 1, "{bad_base:?}");
        assert!(
            bad_base[0].contains("base baseline invalid"),
            "{bad_base:?}"
        );
    }

    #[test]
    fn a_git_detected_rename_carries_the_quota_but_never_grows_it() {
        let base = bl(&[("docs/src/a.md", 2), ("docs/src/z.md", 1)]);
        let gate = |head: &[(&str, usize)], r: &[Rename]| {
            mermaid_baseline_gate_failures(Some(&base), Some(&bl(head)), r)
        };
        let ab = [mv("docs/src/a.md", "docs/src/b.md")];
        // a.md -> b.md: old entry gone, same count (or fewer)
        assert!(gate(&[("docs/src/b.md", 2), ("docs/src/z.md", 1)], &ab).is_empty());
        assert!(gate(&[("docs/src/b.md", 1), ("docs/src/z.md", 1)], &ab).is_empty());
        // growing while moving fails
        assert_eq!(
            gate(&[("docs/src/b.md", 3), ("docs/src/z.md", 1)], &ab).len(),
            1
        );
        // the same move without git seeing a rename is a laundered new path
        assert_eq!(
            gate(&[("docs/src/b.md", 2), ("docs/src/z.md", 1)], &[]).len(),
            1
        );
        // old entry still present: the quota would be spent twice
        let both = gate(
            &[
                ("docs/src/a.md", 2),
                ("docs/src/b.md", 2),
                ("docs/src/z.md", 1),
            ],
            &ab,
        );
        assert_eq!(both.len(), 1, "{both:?}");
        // rename of a path that never had a quota admits nothing
        let phantom = [mv("docs/src/q.md", "docs/src/b.md")];
        assert_eq!(
            gate(&[("docs/src/b.md", 1), ("docs/src/z.md", 1)], &phantom).len(),
            1
        );
        // one rename cannot feed two new paths
        let two = gate(
            &[
                ("docs/src/b.md", 1),
                ("docs/src/c.md", 1),
                ("docs/src/z.md", 1),
            ],
            &[
                mv("docs/src/a.md", "docs/src/b.md"),
                mv("docs/src/a.md", "docs/src/c.md"),
            ],
        );
        assert_eq!(two.len(), 1, "{two:?}");
    }
}
