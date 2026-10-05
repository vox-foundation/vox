//! `vox kb eval` — retrieval report, BM25 pooling (`--query`), `--headings`, `--snapshot-corpus`.
//!
//! Stdout carries only deterministic content; timings and paths go to stderr.

use std::io::Write;
use std::path::Path;

use super::gate::ensure_clean;
use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use vox_search::kb_eval::bm25::Bm25Index;
use vox_search::kb_eval::golden::{
    GoldenSet, Split, load_corpus, read_canary_guid, snapshot_corpus_jsonl, validate_splits,
};
use vox_search::kb_eval::run_eval;
use vox_search::kb_ids::doc_heading_paths;

/// Golden-set split to score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SplitArg {
    Dev,
    Test,
}

#[derive(Args, Debug)]
pub struct KbEvalArgs {
    /// Split to score.
    #[arg(long, value_enum, default_value = "dev")]
    pub split: SplitArg,
    /// Emit the report as JSON.
    #[arg(long)]
    pub json: bool,
    /// Print the top-N BM25 chunk ids for this text (qrel pooling).
    #[arg(long, conflicts_with_all = ["headings", "snapshot_corpus"])]
    pub query: Option<String>,
    /// Number of chunks to print with --query.
    #[arg(long, default_value_t = 20, requires = "query")]
    pub top: usize,
    /// Print every valid heading_path of this snapshot doc id.
    #[arg(long, value_name = "DOC_ID", conflicts_with = "snapshot_corpus")]
    pub headings: Option<String>,
    /// Rewrite contracts/kb/eval/corpus.jsonl from tracked non-archive docs/src markdown.
    #[arg(long)]
    pub snapshot_corpus: bool,
    /// Ratchet gate: report every golden-split, regression, tighten-only and canary failure;
    /// exit non-zero on any.
    #[arg(long, conflicts_with_all = ["tighten", "write_baseline", "split", "query", "headings", "snapshot_corpus"])]
    pub gate: bool,
    /// Base ref the baseline must only tighten against.
    #[arg(
        long,
        value_name = "REF",
        default_value = "origin/main",
        requires = "gate"
    )]
    pub base: String,
    /// Raise baseline values to the current dev report; never lowers anything.
    #[arg(long, conflicts_with_all = ["gate", "write_baseline", "split", "query", "headings", "snapshot_corpus"])]
    pub tighten: bool,
    /// Write a new baseline epoch from the current dev report (human-reviewed).
    #[arg(long, requires = "epoch_note", conflicts_with_all = ["gate", "tighten", "split", "query", "headings", "snapshot_corpus"])]
    pub write_baseline: bool,
    /// Why a new epoch is being written (required with --write-baseline).
    #[arg(long, value_name = "TEXT", requires = "write_baseline")]
    pub epoch_note: Option<String>,
}

pub(crate) const EVAL_DIR: &str = "contracts/kb/eval";

pub fn run(args: &KbEvalArgs) -> Result<()> {
    let root = crate::commands::ci::repo_root();
    let dir = root.join(EVAL_DIR);
    let mut out = std::io::stdout().lock();
    if args.gate {
        super::gate::run_gate(&root, &args.base)
    } else if args.tighten {
        super::gate::run_tighten(&root)
    } else if args.write_baseline {
        super::gate::run_write_baseline(&root, args.epoch_note.as_deref().unwrap_or_default())
    } else if args.snapshot_corpus {
        run_snapshot(&root, &dir)
    } else if let Some(doc) = &args.headings {
        run_headings(&dir, doc, &mut out)
    } else if let Some(text) = &args.query {
        run_query(&dir, text, args.top, &mut out)
    } else {
        let split = match args.split {
            SplitArg::Dev => Split::Dev,
            SplitArg::Test => Split::Test,
        };
        run_report(&dir, split, args.json, &mut out)
    }
}

pub fn run_report(eval_dir: &Path, split: Split, json: bool, out: &mut impl Write) -> Result<()> {
    let golden = GoldenSet::load(eval_dir, split)?;
    ensure_clean(&golden.corpus, &golden.canary)?;
    if split == Split::Test {
        validate_splits(eval_dir)?;
        eprintln!("held-out split: milestone-close use only; never tune on it");
    }
    let idx = Bm25Index::build(&golden.corpus);
    let report = run_eval(&golden, &idx.doc_chunk_ids(), |q| idx.rank(q))?;
    if json {
        writeln!(out, "{}", report.to_json_string()?)?;
    } else {
        write!(out, "{}", report.render_text())?;
    }
    Ok(())
}

pub fn run_query(eval_dir: &Path, text: &str, top: usize, out: &mut impl Write) -> Result<()> {
    let corpus = load_corpus(eval_dir)?;
    ensure_clean(&corpus, &read_canary_guid(eval_dir)?)?;
    let idx = Bm25Index::build(&corpus);
    for (i, (score, c)) in idx.rank_scored(text).into_iter().take(top).enumerate() {
        writeln!(out, "{}\t{score:.4}\t{}", i + 1, c.id)?;
    }
    Ok(())
}

pub fn run_headings(eval_dir: &Path, doc_id: &str, out: &mut impl Write) -> Result<()> {
    let corpus = load_corpus(eval_dir)?;
    let Some(doc) = corpus.iter().find(|d| d.id == doc_id) else {
        bail!(
            "unknown doc id {doc_id:?}: not in {}/corpus.jsonl",
            EVAL_DIR
        );
    };
    for h in doc_heading_paths(&doc.id, &doc.text) {
        writeln!(out, "{h}")?;
    }
    Ok(())
}

pub fn run_snapshot(root: &Path, eval_dir: &Path) -> Result<()> {
    let listing = vox_git::read_cmd::read_only(root, &["ls-files", "-z", "--", "docs/src"])
        .context("git ls-files docs/src")?;
    let paths = super::gate::split_nul(&listing);
    let body = snapshot_corpus_jsonl(root, &paths)?;
    std::fs::create_dir_all(eval_dir)?;
    std::fs::write(eval_dir.join("corpus.jsonl"), &body)?;
    eprintln!(
        "wrote {EVAL_DIR}/corpus.jsonl: {} docs, {} bytes",
        body.lines().count(),
        body.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../vox-search/tests/fixtures/kb_eval")
    }

    fn capture(f: impl FnOnce(&mut Vec<u8>) -> Result<()>) -> Result<String> {
        let mut buf = Vec::new();
        f(&mut buf)?;
        Ok(String::from_utf8(buf)?)
    }

    #[test]
    fn report_text_and_json_are_deterministic() {
        let text = capture(|o| run_report(&fixture(), Split::Dev, false, o)).unwrap();
        assert!(text.contains("dev/all"), "{text}");
        let again = capture(|o| run_report(&fixture(), Split::Dev, false, o)).unwrap();
        assert_eq!(text, again);
        let json = capture(|o| run_report(&fixture(), Split::Dev, true, o)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.to_string().contains("dev/all"));
    }

    #[test]
    fn query_prints_ranked_tab_separated_lines() {
        let s = capture(|o| run_query(&fixture(), "zephyrine installer setup", 3, o)).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert!(!lines.is_empty() && lines.len() <= 3, "{s}");
        let cols: Vec<&str> = lines[0].split('\t').collect();
        assert_eq!(cols[0], "1");
        assert_eq!(cols[1].split('.').nth(1).map(str::len), Some(4));
        assert!(cols[2].starts_with("docs/src/alpha.md#"), "{s}");
    }

    #[test]
    fn headings_lists_nested_paths_and_skips_fences() {
        let s = capture(|o| run_headings(&fixture(), "docs/src/alpha.md", o)).unwrap();
        for want in [
            "Alpha Guide",
            "Alpha Guide › Install",
            "Alpha Guide › Usage › Flags",
        ] {
            assert!(s.lines().any(|l| l == want), "missing {want}: {s}");
        }
        assert!(capture(|o| run_headings(&fixture(), "docs/src/nope.md", o)).is_err());
        let beta = capture(|o| run_headings(&fixture(), "docs/src/beta.md", o)).unwrap();
        assert!(!beta.to_lowercase().contains("fake"), "{beta}");
    }

    #[test]
    fn snapshot_is_sorted_and_repeatable() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("docs/src");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("b.md"), "# B\nbody\n").unwrap();
        std::fs::write(docs.join("a.md"), "# A\nbody\n").unwrap();
        let paths = vec!["docs/src/b.md".to_string(), "docs/src/a.md".to_string()];
        let one = snapshot_corpus_jsonl(tmp.path(), &paths).unwrap();
        assert_eq!(one, snapshot_corpus_jsonl(tmp.path(), &paths).unwrap());
        assert!(one.find("docs/src/a.md").unwrap() < one.find("docs/src/b.md").unwrap());
    }
}
