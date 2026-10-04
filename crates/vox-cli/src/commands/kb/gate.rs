//! `vox kb eval` ratchet modes: `--gate`, `--tighten`, `--write-baseline`.
//!
//! The CLI only gathers inputs; every decision is a pure function in
//! `vox_search::kb_eval::{baseline, canary}`. Ratchet modes always score the dev split.

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use vox_corpus::corpus::extract_docs::{ExtractDocsConfig, walk_and_extract_docs};
use vox_git::read_cmd::{GitReadError, read_only};
use vox_search::kb_eval::baseline::{
    Baseline, check_regression, check_tighten_only, improvements, new_epoch, ratchet_split,
    tightened,
};
use vox_search::kb_eval::bm25::Bm25Index;
use vox_search::kb_eval::canary::{
    assert_corpus_clean, check_roots_disjoint, query_fingerprint_leaks, scan_paths_for_guid,
    texts_containing_guid,
};
use vox_search::kb_eval::golden::{CorpusDoc, GoldenSet, load_query_texts, read_canary_guid};
use vox_search::kb_eval::{Report, run_eval};
use vox_search::kb_ids::KB_CORPUS_ROOTS;

use super::eval::EVAL_DIR;

pub(crate) const BASELINE_REL: &str = "contracts/kb/retrieval-baseline.v1.json";

/// A corpus doc carries the canary GUID. Lets the gate tag it `[canary]`, not `[regression]`.
#[derive(Debug)]
pub(crate) struct CorpusHasCanary(Vec<String>);

impl std::fmt::Display for CorpusHasCanary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "canary GUID found in corpus doc(s) {:?}; refusing to build an index over the golden set",
            self.0
        )
    }
}

impl std::error::Error for CorpusHasCanary {}

/// Index-time canary assert (EVAL-03): never build an index over a corpus carrying the GUID.
/// Names doc ids only, never the GUID.
pub(crate) fn ensure_clean(corpus: &[CorpusDoc], guid: &str) -> Result<()> {
    let hits = assert_corpus_clean(corpus, guid);
    if hits.is_empty() {
        Ok(())
    } else {
        Err(CorpusHasCanary(hits).into())
    }
}

/// Dev-split report: load with `ratchet_split`, assert the corpus is clean, index, evaluate.
pub(crate) fn dev_report(eval_dir: &Path) -> Result<Report> {
    let golden = GoldenSet::load(eval_dir, ratchet_split())?;
    ensure_clean(&golden.corpus, &golden.canary)?;
    let idx = Bm25Index::build(&golden.corpus);
    run_eval(&golden, &idx.doc_chunk_ids(), |q| idx.rank(q))
}

pub(crate) fn read_baseline(path: &Path) -> Result<Option<Baseline>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(
            Baseline::from_json_str(&s).with_context(|| format!("parse {}", path.display()))?,
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}

const BASE_BASELINE_PATHSPEC: [&str; 3] = [
    "contracts/kb/eval/queries.jsonl",
    "contracts/kb/eval/qrels",
    "contracts/kb/eval/corpus.jsonl",
];

/// Tracked markdown whose text may carry a pasted query: `.md` under `docs/` or `crates/`,
/// minus `archive` dirs and the golden set itself.
pub(crate) fn fingerprint_doc_paths(ls_files: &[String]) -> Vec<String> {
    ls_files
        .iter()
        .filter(|p| p.ends_with(".md") && (p.starts_with("docs/") || p.starts_with("crates/")))
        .filter(|p| !p.split('/').any(|c| c == "archive") && !p.starts_with("contracts/kb/eval/"))
        .cloned()
        .collect()
}

/// Run the real MENS docs extractor (same root as the training mix) and name every source
/// file whose extracted pair text carries the GUID.
pub(crate) fn mens_extract_guid_hits(root: &Path, guid: &str) -> Result<Vec<String>> {
    let cfg = ExtractDocsConfig {
        root: root.join("docs"),
        ..Default::default()
    };
    let mut hits = Vec::new();
    for p in walk_and_extract_docs(&cfg)? {
        let label = p.source_path.strip_prefix(root).unwrap_or(&p.source_path);
        let text = format!("{}\n{}\n{}", p.prompt, p.response, p.metadata);
        hits.extend(texts_containing_guid(
            &[(label.display().to_string(), text)],
            guid,
        ));
    }
    hits.sort();
    hits.dedup();
    Ok(hits)
}

/// Local MENS jsonl inputs, if present: any `*.jsonl` carrying the GUID.
fn jsonl_guid_hits(root: &Path, guid: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for dir in ["mens/data", "target/dogfood"] {
        for e in walkdir::WalkDir::new(root.join(dir)).into_iter().flatten() {
            if e.path().extension().is_some_and(|x| x == "jsonl")
                && let Ok(bytes) = std::fs::read(e.path())
            {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let label = e.path().strip_prefix(root).unwrap_or(e.path());
                hits.extend(texts_containing_guid(
                    &[(label.display().to_string(), text)],
                    guid,
                ));
            }
        }
    }
    hits
}

/// Every `[canary]` failure, each source tagged. `tracked` is `git ls-files`.
pub(crate) fn canary_failures(root: &Path, tracked: &[String]) -> Vec<String> {
    let eval = root.join(EVAL_DIR);
    let guid = match read_canary_guid(&eval) {
        Ok(g) => g,
        Err(e) => return vec![format!("[canary] cannot read the canary GUID: {e:#}")],
    };
    let mut out = Vec::new();
    for p in scan_paths_for_guid(root, tracked, &guid, "contracts/kb/eval/") {
        out.push(format!("[canary] repo-scan: {p} contains the canary GUID"));
    }
    if let Err(e) = check_roots_disjoint(KB_CORPUS_ROOTS, EVAL_DIR) {
        out.push(format!("[canary] roots: {e:#}"));
    }
    match mens_extract_guid_hits(root, &guid) {
        Ok(hits) => out.extend(hits.into_iter().map(|h| {
            format!("[canary] mens-extractor: docs pair from {h} contains the canary GUID")
        })),
        Err(e) => out.push(format!("[canary] mens-extractor failed to run: {e:#}")),
    }
    for p in jsonl_guid_hits(root, &guid) {
        out.push(format!("[canary] mens-jsonl: {p} contains the canary GUID"));
    }
    match load_query_texts(&eval) {
        Ok(queries) => {
            let docs: Vec<(String, String)> = fingerprint_doc_paths(tracked)
                .into_iter()
                .filter_map(|p| Some((p.clone(), std::fs::read_to_string(root.join(&p)).ok()?)))
                .collect();
            out.extend(
                query_fingerprint_leaks(&queries, &docs, 40)
                    .into_iter()
                    .map(|l| format!("[canary] query-fingerprint: {l}")),
            );
        }
        Err(e) => out.push(format!(
            "[canary] cannot load query texts to fingerprint: {e:#}"
        )),
    }
    out
}

/// Fail closed: the base ref must resolve to a commit.
pub(crate) fn resolve_base(root: &Path, base: &str) -> Result<()> {
    let spec = format!("{base}^{{commit}}");
    if base.starts_with('-')
        || read_only(root, &["rev-parse", "--verify", "--quiet", &spec]).is_err()
    {
        bail!("cannot resolve base ref {base}; fetch origin or pass --base");
    }
    Ok(())
}

/// `[tighten-only]` failures: HEAD's baseline vs the one at `base` (read via `git show`).
fn tighten_failures(root: &Path, base: &str, head: Option<&Baseline>) -> Vec<String> {
    if let Err(e) = resolve_base(root, base) {
        return vec![format!("[tighten-only] {e}")];
    }
    let shown = read_only(root, &["show", &format!("{base}:{BASELINE_REL}")]);
    let base_baseline = match shown {
        Ok(s) => match Baseline::from_json_str(&s) {
            Ok(b) => Some(b),
            Err(e) => {
                return vec![format!(
                    "[tighten-only] baseline at {base} unreadable: {e:#}"
                )];
            }
        },
        Err(GitReadError::NonZero { .. }) => None, // absent at base: bootstrap
        Err(e) => return vec![format!("[tighten-only] git show failed: {e}")],
    };
    let range = format!("{base}...HEAD");
    let mut args = vec!["diff", "--name-only", range.as_str(), "--"];
    args.extend(BASE_BASELINE_PATHSPEC);
    match read_only(root, &args) {
        Ok(changed) => check_tighten_only(base_baseline.as_ref(), head, !changed.trim().is_empty()),
        Err(e) => vec![format!(
            "[tighten-only] cannot diff the golden set against {base}: {e}"
        )],
    }
}

/// Stable output: sorted (`[canary]` < `[regression]` < `[tighten-only]`), each line once.
pub(crate) fn sorted_unique(mut failures: Vec<String>) -> Vec<String> {
    failures.sort();
    failures.dedup();
    failures
}

/// Regression + tighten-only vs `base` + canary sweep, all failures in one report.
pub fn run_gate(root: &Path, base: &str) -> Result<()> {
    let started = Instant::now();
    let mut fails = Vec::new();
    let head = match read_baseline(&root.join(BASELINE_REL)) {
        Ok(h) => h,
        Err(e) => {
            fails.push(format!("[regression] baseline unreadable: {e:#}"));
            None
        }
    };
    match dev_report(&root.join(EVAL_DIR)) {
        Err(e) => fails.push(match e.downcast_ref::<CorpusHasCanary>() {
            Some(c) => format!("[canary] {c}"),
            None => format!("[regression] golden set failed to load: {e:#}"),
        }),
        Ok(report) => match &head {
            None if fails.is_empty() => fails.push(
                "[regression] no baseline at HEAD; bootstrap with --write-baseline".to_string(),
            ),
            None => {}
            Some(h) => {
                fails.extend(check_regression(&report, h));
                for i in improvements(&report, h) {
                    eprintln!(
                        "kb eval gate: {i} improved beyond tolerance; run `vox kb eval --tighten`"
                    );
                }
            }
        },
    }
    fails.extend(tighten_failures(root, base, head.as_ref()));
    let tracked: Vec<String> = match read_only(root, &["ls-files"]) {
        Ok(s) => s.lines().map(str::to_owned).collect(),
        Err(e) => {
            fails.push(format!("[canary] git ls-files failed: {e}"));
            Vec::new()
        }
    };
    fails.extend(canary_failures(root, &tracked));
    let fails = sorted_unique(fails);
    eprintln!("kb eval gate: {:.2}s", started.elapsed().as_secs_f64());
    if fails.is_empty() {
        println!("kb eval gate: OK ({} tracked files scanned)", tracked.len());
        return Ok(());
    }
    for f in &fails {
        println!("{f}");
    }
    Err(anyhow!("kb eval gate: {} failure(s)", fails.len()))
}

/// Raise baseline values to the dev report; never lowers a value. Writes only on change.
pub fn run_tighten(root: &Path) -> Result<()> {
    let path = root.join(BASELINE_REL);
    let Some(prev) = read_baseline(&path)? else {
        bail!("no baseline at {BASELINE_REL}; bootstrap with --write-baseline --epoch-note");
    };
    let next = tightened(&prev, &dev_report(&root.join(EVAL_DIR))?);
    if next == prev {
        eprintln!("kb eval tighten: nothing to raise");
        return Ok(());
    }
    for (key, block) in &next.metrics {
        if prev.metrics.get(key) != Some(block) {
            eprintln!("kb eval tighten: raised {key}");
        }
    }
    std::fs::write(&path, next.to_json_string()?)?;
    Ok(())
}

/// Bootstrap or re-epoch: the only way the baseline may be loosened. Needs a non-empty note.
pub fn run_write_baseline(root: &Path, note: &str) -> Result<()> {
    let path = root.join(BASELINE_REL);
    let prev = read_baseline(&path)?;
    let next = new_epoch(prev.as_ref(), &dev_report(&root.join(EVAL_DIR))?, note)?;
    std::fs::write(&path, next.to_json_string()?)?;
    eprintln!("kb eval: wrote {BASELINE_REL} epoch {}", next.epoch);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::eval::{run_query, run_report};
    use super::*;
    use vox_search::kb_eval::baseline::BaselineBlock;
    use vox_search::kb_eval::golden::Split;
    use vox_search::kb_eval::golden::read_canary_guid;

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let dst = to.join(e.file_name());
            if e.path().is_dir() {
                copy_dir(&e.path(), &dst);
            } else {
                std::fs::copy(e.path(), dst).unwrap();
            }
        }
    }

    /// Tempdir repo whose `contracts/kb/eval/` is a copy of the vox-search fixture.
    pub(crate) fn temp_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../vox-search/tests/fixtures/kb_eval");
        copy_dir(&fixture, &tmp.path().join(EVAL_DIR));
        tmp
    }

    /// Write a baseline whose every metric is `v`.
    fn baseline_with(root: &Path, v: f64) -> std::path::PathBuf {
        let mut b = new_epoch(None, &dev_report(&root.join(EVAL_DIR)).unwrap(), "t").unwrap();
        let l = serde_json::json!({"recall@5": v, "recall@10": v, "mrr@10": v, "ndcg@10": v});
        let all = serde_json::json!({"doc": l, "chunk": l, "answer_in_budget": v});
        let blk: BaselineBlock = serde_json::from_value(all).unwrap();
        b.metrics.values_mut().for_each(|m| *m = blk);
        let path = root.join(BASELINE_REL);
        std::fs::write(&path, b.to_json_string().unwrap()).unwrap();
        path
    }

    #[test]
    fn tighten_never_lowers_file() {
        let tmp = temp_repo();
        let report = dev_report(&tmp.path().join(EVAL_DIR)).unwrap();
        // Baseline below the computed values: raised to exactly the computed ones.
        let path = baseline_with(tmp.path(), 0.0);
        run_tighten(tmp.path()).unwrap();
        let got = read_baseline(&path).unwrap().unwrap();
        for (k, m) in &got.metrics {
            assert_eq!(m, &BaselineBlock::from(&report.keys[k]), "{k}");
        }
        // Baseline above the computed values: file left byte-identical.
        let path = baseline_with(tmp.path(), 2.0);
        let before = std::fs::read(&path).unwrap();
        run_tighten(tmp.path()).unwrap();
        assert_eq!(before, std::fs::read(&path).unwrap());
        // No baseline at all is an error, not a silent bootstrap.
        std::fs::remove_file(&path).unwrap();
        assert!(run_tighten(tmp.path()).is_err());
    }

    #[test]
    fn write_baseline_needs_note_then_bootstraps_epoch_one() {
        let tmp = temp_repo();
        assert!(run_write_baseline(tmp.path(), "  ").is_err());
        assert!(!tmp.path().join(BASELINE_REL).exists());
        run_write_baseline(tmp.path(), "fixture bootstrap").unwrap();
        let load = || {
            read_baseline(&tmp.path().join(BASELINE_REL))
                .unwrap()
                .unwrap()
        };
        let b = load();
        assert_eq!((b.epoch, b.tolerance), (1, 0.01));
        assert!(b.metrics.contains_key("dev/all") && b.metrics.contains_key("dev/seed"));
        run_write_baseline(tmp.path(), "second").unwrap();
        assert_eq!(load().epoch, 2);
    }

    #[test]
    fn index_build_rejects_corpus_with_guid() {
        let tmp = temp_repo();
        let dir = tmp.path().join(EVAL_DIR);
        let guid = read_canary_guid(&dir).unwrap();
        let p = dir.join("corpus.jsonl");
        let mut lines: Vec<serde_json::Value> = std::fs::read_to_string(&p)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let id = lines[0]["_id"].as_str().unwrap().to_string();
        let text = format!("{} {guid}", lines[0]["text"].as_str().unwrap());
        lines[0]["text"] = text.into();
        let body: String = lines.iter().map(|v| format!("{v}\n")).collect();
        std::fs::write(&p, body).unwrap();
        let mut out = Vec::new();
        for err in [
            run_report(&dir, Split::Dev, false, &mut out).unwrap_err(),
            run_query(&dir, "zephyrine", 3, &mut out).unwrap_err(),
            dev_report(&dir).unwrap_err(),
            run_write_baseline(tmp.path(), "x").unwrap_err(),
        ] {
            let msg = format!("{err:#}");
            assert!(msg.contains(&id), "{msg}");
            assert!(!msg.contains(&guid), "GUID must never be printed: {msg}");
        }
        assert!(out.is_empty(), "nothing scored before the assert");
    }

    fn strings(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    fn section(guid: &str) -> String {
        format!(
            "# Planted\n\n## Leaky section\n\n{}\nmarker {guid}\n",
            "This section body is deliberately long enough to be extracted as a pair. ".repeat(4)
        )
    }

    #[test]
    fn mens_extract_scan_flags_planted_guid_then_clears() {
        let tmp = temp_repo();
        let root = tmp.path();
        let guid = read_canary_guid(&root.join(EVAL_DIR)).unwrap();
        write(root, "docs/src/planted.md", &section(&guid));
        assert!(root.join("docs/src/planted.md").exists());
        let hits = mens_extract_guid_hits(root, &guid).unwrap();
        assert!(hits.iter().any(|h| h.ends_with("planted.md")), "{hits:?}");
        std::fs::remove_file(root.join("docs/src/planted.md")).unwrap();
        assert!(!root.join("docs/src/planted.md").exists());
        assert!(mens_extract_guid_hits(root, &guid).unwrap().is_empty());
    }

    #[test]
    fn fingerprint_doc_paths_filters_scope() {
        let ls = strings(
            "docs/src/a.md crates/x/README.md docs/src/archive/old.md crates/x/docs/archive/old.md \
             contracts/kb/eval/README.md docs/src/data.json scripts/notes.md",
        );
        assert_eq!(
            fingerprint_doc_paths(&ls),
            strings("docs/src/a.md crates/x/README.md")
        );
    }

    #[test]
    fn resolve_base_rejects_missing_ref() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let err = resolve_base(&repo, "refs/heads/definitely-missing-kb-eval").unwrap_err();
        assert!(err.to_string().contains("cannot resolve base ref"), "{err}");
        assert!(resolve_base(&repo, "--output=x").is_err());
        assert!(resolve_base(&repo, "HEAD").is_ok());
    }

    #[test]
    fn gate_failures_sorted() {
        let got = sorted_unique(strings(
            "[tighten-only]_b [canary]_z [regression]_a [canary]_z [canary]_a",
        ));
        assert_eq!(
            got,
            strings("[canary]_a [canary]_z [regression]_a [tighten-only]_b")
        );
    }

    #[test]
    fn canary_sweep_names_every_source_and_never_the_guid() {
        let tmp = temp_repo();
        let root = tmp.path();
        let eval = root.join(EVAL_DIR);
        let guid = read_canary_guid(&eval).unwrap();
        let pasted = "zephyrine installer setup walkthrough for the quokka daemon beta channel";
        let q = std::fs::read_to_string(eval.join("queries.jsonl")).unwrap();
        std::fs::write(
            eval.join("queries.jsonl"),
            q.replacen("zephyrine installer setup", pasted, 1),
        )
        .unwrap();
        write(root, "notes/leak.txt", &format!("oops {guid}\n"));
        write(root, "docs/src/planted.md", &section(&guid));
        write(root, "docs/src/pasted.md", &format!("# P\n\n{pasted}\n"));
        write(
            root,
            "mens/data/train.jsonl",
            &format!("{{\"t\":\"{guid}\"}}\n"),
        );
        let tracked = strings(
            "contracts/kb/eval/README.md notes/leak.txt docs/src/planted.md docs/src/pasted.md",
        );
        let out = canary_failures(root, &tracked).join("\n");
        for want in [
            "repo-scan: notes/leak.txt",
            "repo-scan: docs/src/planted.md",
            "mens-extractor: docs pair from docs/src/planted.md",
            "mens-jsonl: mens/data/train.jsonl",
            "query-fingerprint: q001 in docs/src/pasted.md",
        ] {
            assert!(out.contains(want), "missing {want:?} in:\n{out}");
        }
        assert!(!out.contains("contracts/kb/eval/README.md"), "{out}");
        assert!(!out.contains(&guid), "GUID must never be printed");
        for rel in
            "notes/leak.txt docs/src/planted.md docs/src/pasted.md mens/data/train.jsonl".split(' ')
        {
            std::fs::remove_file(root.join(rel)).unwrap();
            assert!(!root.join(rel).exists());
        }
        let clean = canary_failures(root, &["contracts/kb/eval/README.md".to_string()]);
        assert!(clean.is_empty(), "{clean:?}");
    }
}
