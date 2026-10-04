//! `vox kb eval` ratchet modes: `--gate`, `--tighten`, `--write-baseline`.
//!
//! The CLI only gathers inputs; every decision is a pure function in
//! `vox_search::kb_eval::{baseline, canary}`. Ratchet modes always score the dev split.

use std::path::Path;

use anyhow::{Context, Result, bail};
use vox_search::kb_eval::baseline::{Baseline, new_epoch, ratchet_split, tightened};
use vox_search::kb_eval::bm25::Bm25Index;
use vox_search::kb_eval::canary::assert_corpus_clean;
use vox_search::kb_eval::golden::{CorpusDoc, GoldenSet};
use vox_search::kb_eval::{Report, run_eval};

use super::eval::EVAL_DIR;

pub(crate) const BASELINE_REL: &str = "contracts/kb/retrieval-baseline.v1.json";

/// Index-time canary assert (EVAL-03): never build an index over a corpus carrying the GUID.
/// Names doc ids only, never the GUID.
pub(crate) fn ensure_clean(corpus: &[CorpusDoc], guid: &str) -> Result<()> {
    let hits = assert_corpus_clean(corpus, guid);
    if !hits.is_empty() {
        bail!(
            "canary GUID found in corpus doc(s) {hits:?}; refusing to build an index over the golden set"
        );
    }
    Ok(())
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

/// Placeholder until the gate lands (next commit).
pub fn run_gate(_root: &Path, _base: &str) -> Result<()> {
    bail!("--gate is not implemented yet")
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
    use super::*;
    use std::path::PathBuf;
    use vox_search::kb_eval::LevelMetrics;
    use vox_search::kb_eval::baseline::BaselineBlock;
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

    fn all(v: f64) -> BaselineBlock {
        let l = LevelMetrics {
            recall_at_5: v,
            recall_at_10: v,
            mrr_at_10: v,
            ndcg_at_10: v,
        };
        BaselineBlock {
            doc: l,
            chunk: l,
            answer_in_budget: v,
        }
    }

    fn baseline_with(root: &Path, v: f64) -> PathBuf {
        let report = dev_report(&root.join(EVAL_DIR)).unwrap();
        let mut b = new_epoch(None, &report, "test").unwrap();
        b.metrics.values_mut().for_each(|m| *m = all(v));
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
        let b = read_baseline(&tmp.path().join(BASELINE_REL))
            .unwrap()
            .unwrap();
        assert_eq!((b.epoch, b.tolerance), (1, 0.01));
        assert!(b.metrics.contains_key("dev/all") && b.metrics.contains_key("dev/human"));
        run_write_baseline(tmp.path(), "second").unwrap();
        let b = read_baseline(&tmp.path().join(BASELINE_REL))
            .unwrap()
            .unwrap();
        assert_eq!(b.epoch, 2);
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
            super::super::eval::run_report(
                &dir,
                vox_search::kb_eval::golden::Split::Dev,
                false,
                &mut out,
            )
            .unwrap_err(),
            super::super::eval::run_query(&dir, "zephyrine", 3, &mut out).unwrap_err(),
            dev_report(&dir).unwrap_err(),
            run_write_baseline(tmp.path(), "x").unwrap_err(),
        ] {
            let msg = format!("{err:#}");
            assert!(msg.contains(&id), "{msg}");
            assert!(!msg.contains(&guid), "GUID must never be printed: {msg}");
        }
        assert!(out.is_empty(), "nothing scored before the assert");
    }
}
