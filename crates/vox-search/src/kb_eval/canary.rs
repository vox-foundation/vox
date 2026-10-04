//! Pure canary checks (EVAL-03): the golden set must never flow into indexed or trained text.
//!
//! None of these functions read the canary GUID from anywhere but their argument; callers
//! pass the output of [`super::golden::read_canary_guid`]. An empty GUID matches every text,
//! so a caller bug fails closed (everything is flagged) rather than open.

use std::path::Path;

use anyhow::{Result, bail};

use super::golden::CorpusDoc;

/// Repo-path scan: the GUID must appear in no file outside `allowed_prefix` (a repo-relative
/// path prefix such as `contracts/kb/eval/`). Closes the Rust/docs extractor leak path: MENS
/// harvests repo files, so any repo file carrying the GUID outside the eval dir is a leak.
///
/// Files are read as bytes; invalid UTF-8 is decoded lossily, unreadable files are skipped.
/// Returns the sorted offending rel paths.
#[must_use]
pub fn scan_paths_for_guid(
    root: &Path,
    rel_paths: &[String],
    guid: &str,
    allowed_prefix: &str,
) -> Vec<String> {
    let mut out: Vec<String> = rel_paths
        .iter()
        .filter(|rel| !rel.starts_with(allowed_prefix))
        .filter(|rel| {
            std::fs::read(root.join(rel))
                .is_ok_and(|bytes| String::from_utf8_lossy(&bytes).contains(guid))
        })
        .cloned()
        .collect();
    out.sort();
    out
}

/// Labels of `(label, text)` items whose text contains the GUID. Closes the MENS docs
/// extractor and local-jsonl leak paths (the CLI feeds extractor pairs and jsonl lines).
#[must_use]
pub fn texts_containing_guid(items: &[(String, String)], guid: &str) -> Vec<String> {
    items
        .iter()
        .filter(|(_, text)| text.contains(guid))
        .map(|(label, _)| label.clone())
        .collect()
}

fn components(p: &str) -> Vec<&str> {
    p.split(['/', '\\'])
        .filter(|c| !c.is_empty() && *c != ".")
        .collect()
}

/// Indexer-root leak path: no corpus root may equal, contain or sit inside `eval_dir`
/// (path-component aware: `contracts/kbx` does not overlap `contracts/kb/eval`).
pub fn check_roots_disjoint(roots: &[&str], eval_dir: &str) -> Result<()> {
    let eval = components(eval_dir);
    let bad: Vec<&str> = roots
        .iter()
        .copied()
        .filter(|r| {
            let r = components(r);
            let n = r.len().min(eval.len());
            r[..n] == eval[..n]
        })
        .collect();
    if !bad.is_empty() {
        bail!(
            "corpus roots {bad:?} overlap the eval dir {eval_dir:?}; the golden set would be indexed"
        );
    }
    Ok(())
}

/// Indexer-corpus leak path: ids of corpus docs whose text contains the GUID.
#[must_use]
pub fn assert_corpus_clean(corpus: &[CorpusDoc], guid: &str) -> Vec<String> {
    corpus
        .iter()
        .filter(|d| d.text.contains(guid))
        .map(|d| d.id.clone())
        .collect()
}

fn norm(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Paraphrase-free pasting leak path: `"{qid} in {path}"` for each query of at least
/// `min_chars` normalized characters (lowercased, whitespace collapsed) found verbatim in a
/// `(path, text)` doc. Shorter queries are ignored (too generic to be a fingerprint).
#[must_use]
pub fn query_fingerprint_leaks(
    queries: &[(String, String)],
    docs: &[(String, String)],
    min_chars: usize,
) -> Vec<String> {
    let docs: Vec<(&str, String)> = docs.iter().map(|(p, t)| (p.as_str(), norm(t))).collect();
    let mut out = Vec::new();
    for (qid, text) in queries {
        let q = norm(text);
        if q.chars().count() < min_chars {
            continue;
        }
        out.extend(
            docs.iter()
                .filter(|(_, d)| d.contains(&q))
                .map(|(path, _)| format!("{qid} in {path}")),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb_ids::KB_CORPUS_ROOTS;
    use std::fs;

    // Obviously fake placeholder; never the real canary.
    const FAKE: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

    fn write(root: &Path, rel: &str, body: &[u8]) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    #[test]
    fn planted_guid_in_docs_is_detected_then_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let body = format!("canary {FAKE}\n");
        write(root, "contracts/kb/eval/README.md", body.as_bytes());
        write(root, "docs/src/planted.md", body.as_bytes());
        write(root, "docs/src/clean.md", b"nothing here\n");
        let rels: Vec<String> = [
            "contracts/kb/eval/README.md",
            "docs/src/planted.md",
            "docs/src/clean.md",
        ]
        .map(String::from)
        .to_vec();
        let prefix = "contracts/kb/eval/";

        assert!(
            root.join("docs/src/planted.md").exists(),
            "plant missing before scan 1"
        );
        assert_eq!(
            scan_paths_for_guid(root, &rels, FAKE, prefix),
            vec!["docs/src/planted.md".to_string()]
        );
        fs::remove_file(root.join("docs/src/planted.md")).unwrap();
        assert!(
            !root.join("docs/src/planted.md").exists(),
            "plant still present before scan 2"
        );
        assert!(scan_paths_for_guid(root, &rels, FAKE, prefix).is_empty());
    }

    #[test]
    fn scan_tolerates_non_utf8_and_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = vec![0xff, 0xfe, b' '];
        bytes.extend_from_slice(FAKE.as_bytes());
        write(dir.path(), "docs/src/binary.md", &bytes);
        let rels = vec![
            "docs/src/binary.md".to_string(),
            "docs/src/gone.md".to_string(),
        ];
        assert_eq!(
            scan_paths_for_guid(dir.path(), &rels, FAKE, "contracts/kb/eval/"),
            vec!["docs/src/binary.md".to_string()]
        );
    }

    #[test]
    fn roots_overlapping_eval_dir_rejected() {
        let eval = "contracts/kb/eval";
        assert!(check_roots_disjoint(KB_CORPUS_ROOTS, eval).is_ok());
        for bad in [
            "contracts/kb/eval",
            "contracts",
            "contracts/kb/eval/sub",
            "./contracts/kb/",
        ] {
            let mut roots = KB_CORPUS_ROOTS.to_vec();
            roots.push(bad);
            assert!(
                check_roots_disjoint(&roots, eval).is_err(),
                "{bad} should overlap"
            );
        }
        // Component-aware: a sibling that merely shares a string prefix is fine.
        assert!(check_roots_disjoint(&["contracts/kbx", "docs/src"], eval).is_ok());
    }

    #[test]
    fn corpus_doc_with_guid_is_reported() {
        let doc = |id: &str, text: String| CorpusDoc {
            id: id.into(),
            title: id.into(),
            text,
        };
        let corpus = vec![
            doc("docs/src/a.md", "fine".into()),
            doc("docs/src/b.md", format!("oops {FAKE}")),
        ];
        assert_eq!(
            assert_corpus_clean(&corpus, FAKE),
            vec!["docs/src/b.md".to_string()]
        );
        assert!(assert_corpus_clean(&corpus[..1], FAKE).is_empty());
    }

    #[test]
    fn long_pasted_query_is_fingerprinted_short_is_ignored() {
        let long = "How do I configure the retrieval baseline ratchet tolerance?"; // > 40 chars
        let queries = vec![
            ("q1".to_string(), long.to_string()),
            ("q2".to_string(), "short query text here".to_string()),
        ];
        let pasted =
            "Intro.\n\nHOW  do I configure\nthe retrieval baseline   ratchet tolerance? Yes.";
        let docs = vec![
            (
                "docs/src/leak.md".to_string(),
                format!("{pasted} short query text here"),
            ),
            ("docs/src/ok.md".to_string(), "unrelated".to_string()),
        ];
        assert_eq!(
            query_fingerprint_leaks(&queries, &docs, 40),
            vec!["q1 in docs/src/leak.md".to_string()]
        );
    }

    #[test]
    fn texts_containing_guid_returns_labels() {
        let items = vec![
            ("pair-1".to_string(), "clean".to_string()),
            ("line-7".to_string(), format!("{{\"x\":\"{FAKE}\"}}")),
        ];
        assert_eq!(
            texts_containing_guid(&items, FAKE),
            vec!["line-7".to_string()]
        );
    }
}
