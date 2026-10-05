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
    assert_corpus_clean, check_roots_disjoint, decode_for_scan, query_fingerprint_leaks,
    scan_paths_for_guid, texts_containing_guid,
};
use vox_search::kb_eval::golden::{
    CorpusDoc, GoldenSet, load_query_texts, read_canary_guid, validate_splits,
};
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

/// Tracked files whose text may carry a pasted query (docs, plans, code comments, contracts):
/// everything but `archive` dirs and the golden set itself. Binary files are skipped at read.
pub(crate) fn fingerprint_paths(ls_files: &[String]) -> Vec<String> {
    ls_files
        .iter()
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
                let text = decode_for_scan(&bytes);
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

/// Paths from `git ls-files -z` output: NUL-separated and never C-quoted, so non-ASCII or
/// special-character paths are neither skipped nor mangled.
pub(crate) fn split_nul(out: &str) -> Vec<String> {
    out.split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Every `[canary]` failure, each source tagged. `tracked` is `git ls-files -z`.
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
            let docs = fingerprint_paths(tracked).into_iter().filter_map(|p| {
                let bytes = std::fs::read(root.join(&p)).ok()?;
                // NUL: binary, no pasted prose to find.
                let text =
                    (!bytes.contains(&0)).then(|| String::from_utf8_lossy(&bytes).into_owned())?;
                Some((p, text))
            });
            match query_fingerprint_leaks(&queries, docs, 40) {
                Ok(leaks) => out.extend(
                    leaks
                        .into_iter()
                        .map(|l| format!("[canary] query-fingerprint: {l}")),
                ),
                Err(e) => out.push(format!("[canary] query-fingerprint failed: {e:#}")),
            }
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

/// The baseline at `base`. `Ok(None)` (bootstrap) only when `ls-tree` succeeds and lists no
/// such path; any git or parse failure is an error, never a bootstrap (fail closed).
pub(crate) fn baseline_at(
    git: impl Fn(&[&str]) -> Result<String, GitReadError>,
    base: &str,
) -> Result<Option<Baseline>, String> {
    let listed = git(&["ls-tree", "--name-only", base, "--", BASELINE_REL])
        .map_err(|e| format!("cannot list {BASELINE_REL} at {base}: {e}"))?;
    if listed.trim().is_empty() {
        return Ok(None);
    }
    let shown = git(&["show", &format!("{base}:{BASELINE_REL}")])
        .map_err(|e| format!("cannot read the baseline at {base}: {e}"))?;
    Baseline::from_json_str(&shown)
        .map(Some)
        .map_err(|e| format!("baseline at {base} unreadable: {e:#}"))
}

/// Whether `git diff --name-only <range>` touches the golden set (`range` may be a single ref,
/// diffing the working tree).
fn golden_changed(root: &Path, range: &str) -> Result<bool, GitReadError> {
    let mut args = vec!["diff", "--name-only", range, "--"];
    args.extend(BASE_BASELINE_PATHSPEC);
    Ok(!read_only(root, &args)?.trim().is_empty())
}

/// `[tighten-only]` failures: HEAD's baseline vs the one at `base`.
fn tighten_failures(root: &Path, base: &str, head: Option<&Baseline>) -> Vec<String> {
    if let Err(e) = resolve_base(root, base) {
        return vec![format!("[tighten-only] {e}")];
    }
    let base_baseline = match baseline_at(|a| read_only(root, a), base) {
        Ok(b) => b,
        Err(e) => return vec![format!("[tighten-only] {e}")],
    };
    match golden_changed(root, &format!("{base}...HEAD")) {
        Ok(changed) => check_tighten_only(base_baseline.as_ref(), head, changed),
        Err(e) => vec![format!(
            "[tighten-only] cannot diff the golden set against {base}: {e}"
        )],
    }
}

/// Stable output: sorted (`[canary]` < `[golden]` < `[regression]` < `[tighten-only]`),
/// each line once.
pub(crate) fn sorted_unique(mut failures: Vec<String>) -> Vec<String> {
    failures.sort();
    failures.dedup();
    failures
}

/// `[golden]` failures: dev/test disjointness, coverage and the paraphrase-follows-seed rule.
/// `validate_splits` reads only the query-id column of the test qrels, never its judgments.
fn golden_failures(eval_dir: &Path) -> Vec<String> {
    match validate_splits(eval_dir) {
        Ok(_) => Vec::new(),
        Err(e) => vec![format!("[golden] split rule: {e:#}")],
    }
}

/// Every gate failure (unsorted) and the number of tracked files scanned.
pub(crate) fn gate_failures(root: &Path, base: &str) -> (Vec<String>, usize) {
    let mut fails = golden_failures(&root.join(EVAL_DIR));
    // `None`: unreadable (reported once here); `Some(None)`: absent at HEAD.
    let head = match read_baseline(&root.join(BASELINE_REL)) {
        Ok(h) => Some(h),
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
            Some(None) => fails.push(
                "[regression] no baseline at HEAD; bootstrap with --write-baseline".to_string(),
            ),
            None => {}
            Some(Some(h)) => {
                fails.extend(check_regression(&report, h));
                for i in improvements(&report, h) {
                    eprintln!(
                        "kb eval gate: {i} improved beyond tolerance; run `vox kb eval --tighten`"
                    );
                }
            }
        },
    }
    if let Some(h) = &head {
        fails.extend(tighten_failures(root, base, h.as_ref()));
    }
    let tracked: Vec<String> = match read_only(root, &["ls-files", "-z"]) {
        Ok(s) => split_nul(&s),
        Err(e) => {
            fails.push(format!("[canary] git ls-files failed: {e}"));
            Vec::new()
        }
    };
    fails.extend(canary_failures(root, &tracked));
    (fails, tracked.len())
}

/// Golden split rules + regression + tighten-only vs `base` + canary sweep, all failures in
/// one report.
pub fn run_gate(root: &Path, base: &str) -> Result<()> {
    let started = Instant::now();
    let (fails, tracked) = gate_failures(root, base);
    let fails = sorted_unique(fails);
    eprintln!("kb eval gate: {:.2}s", started.elapsed().as_secs_f64());
    if fails.is_empty() {
        println!("kb eval gate: OK ({tracked} tracked files scanned)");
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

/// A warning when `--gate` will reject this epoch bump: a baseline exists, and neither the
/// golden set (vs `origin/main`, working tree included) nor the retrieval config changed.
pub(crate) fn reepoch_warning(
    golden_changed: Result<bool, GitReadError>,
    prev: Option<&Baseline>,
    next: &Baseline,
) -> Option<String> {
    let prev = prev?;
    let cfg = |b: &Baseline| {
        let (r, c, e) = (&b.retriever, &b.chunker, &b.token_estimator);
        format!("{r}/{c}/{}/{e}", b.budget_tokens)
    };
    if cfg(prev) != cfg(next) {
        return None;
    }
    match golden_changed {
        Ok(true) => None,
        Ok(false) => Some(
            "kb eval: warning: golden set and retrieval config unchanged vs origin/main; \
             `vox kb eval --gate` will reject this epoch bump"
                .to_string(),
        ),
        Err(e) => Some(format!(
            "kb eval: warning: cannot diff the golden set against origin/main ({e}); \
             `--gate` rejects an epoch bump without a golden-set or retrieval-config change"
        )),
    }
}

/// Bootstrap or re-epoch: the only way the baseline may be loosened. Needs a non-empty note.
pub fn run_write_baseline(root: &Path, note: &str) -> Result<()> {
    let path = root.join(BASELINE_REL);
    let prev = read_baseline(&path)?;
    let next = new_epoch(prev.as_ref(), &dev_report(&root.join(EVAL_DIR))?, note)?;
    let changed = golden_changed(root, "origin/main");
    if let Some(w) = reepoch_warning(changed, prev.as_ref(), &next) {
        eprintln!("{w}");
    }
    std::fs::write(&path, next.to_json_string()?)?;
    eprintln!("kb eval: wrote {BASELINE_REL} epoch {}", next.epoch);
    Ok(())
}

#[cfg(test)]
mod tests;
