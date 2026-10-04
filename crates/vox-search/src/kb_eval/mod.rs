//! Deterministic retrieval evaluation over a BEIR golden set (EVAL-01).
//!
//! A retriever is any `Fn(&str) -> Vec<RankedChunk>`; [`run_eval`] scores it against a
//! [`golden::GoldenSet`] on doc ids (best chunk per doc, first occurrence = MaxP) and chunk
//! ids. No environment, clock, DB or network is read and only ordered collections are
//! iterated, so two evaluations of the same inputs are byte-identical.

pub mod bm25;
pub mod golden;
pub mod metrics;

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde::Serialize;

use crate::kb_ids::split_chunk_id;
use golden::GoldenSet;
use metrics::{Qrels, mrr_at_k, ndcg_at_k, recall_at_k, round6};

/// Retriever name recorded in reports.
pub const RETRIEVER: &str = "bm25-inmem-v1";
/// Chunker name recorded in reports.
pub const CHUNKER: &str = "md-h1h2-4096-v1";

/// A ranked retrieval result: chunk id plus its byte length (for the budget estimate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedChunk {
    pub id: String,
    pub bytes: usize,
}

/// Recall/MRR/nDCG at one id level (doc or chunk), means rounded to 6 dp.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct LevelMetrics {
    #[serde(rename = "recall@5")]
    pub recall_at_5: f64,
    #[serde(rename = "recall@10")]
    pub recall_at_10: f64,
    #[serde(rename = "mrr@10")]
    pub mrr_at_10: f64,
    #[serde(rename = "ndcg@10")]
    pub ndcg_at_10: f64,
}

/// Metrics for one report key (e.g. `dev/all`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block {
    pub queries: usize,
    pub doc: LevelMetrics,
    pub chunk: LevelMetrics,
}

/// Deterministic evaluation report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub split: String,
    pub retriever: &'static str,
    pub chunker: &'static str,
    pub keys: BTreeMap<String, Block>,
}

impl Report {
    /// Fixed `{:.4}` text; no timestamps, durations, absolute paths or run ids.
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut s = format!(
            "split={} retriever={} chunker={}\n",
            self.split, self.retriever, self.chunker
        );
        for (key, b) in &self.keys {
            s.push_str(&format!("{key} queries={}\n", b.queries));
            for (lvl, m) in [("doc", &b.doc), ("chunk", &b.chunk)] {
                s.push_str(&format!(
                    "  {lvl:<5} recall@5={:.4} recall@10={:.4} mrr@10={:.4} ndcg@10={:.4}\n",
                    m.recall_at_5, m.recall_at_10, m.mrr_at_10, m.ndcg_at_10
                ));
            }
        }
        s
    }
}

#[derive(Clone, Copy, Default)]
struct Scores {
    r5: f64,
    r10: f64,
    mrr: f64,
    ndcg: f64,
}

fn score(rel: &Qrels, ranked: &[String]) -> Scores {
    Scores {
        r5: recall_at_k(rel, ranked, 5),
        r10: recall_at_k(rel, ranked, 10),
        mrr: mrr_at_k(rel, ranked, 10),
        ndcg: ndcg_at_k(rel, ranked, 10),
    }
}

fn mean(xs: &[Scores]) -> LevelMetrics {
    let n = xs.len().max(1) as f64;
    let sum = |f: fn(&Scores) -> f64| round6(xs.iter().map(f).sum::<f64>() / n);
    LevelMetrics {
        recall_at_5: sum(|s| s.r5),
        recall_at_10: sum(|s| s.r10),
        mrr_at_10: sum(|s| s.mrr),
        ndcg_at_10: sum(|s| s.ndcg),
    }
}

/// Keep the first occurrence of each id.
fn dedupe(ids: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    ids.filter(|id| seen.insert(id.clone())).collect()
}

/// Evaluate `retrieve` on every query of `golden`.
pub fn run_eval<F>(golden: &GoldenSet, retrieve: F) -> Result<Report>
where
    F: Fn(&str) -> Vec<RankedChunk>,
{
    let mut per_query: Vec<(Scores, Scores)> = Vec::new();
    for q in &golden.queries {
        let ranked = retrieve(&q.text);
        let chunk_ids = dedupe(ranked.iter().map(|c| c.id.clone()));
        let doc_ids = dedupe(
            chunk_ids
                .iter()
                .filter_map(|id| split_chunk_id(id).map(|(d, _)| d.to_string())),
        );
        let empty = Qrels::new();
        let doc_rel = golden.doc_qrels.get(&q.id).unwrap_or(&empty);
        let chunk_rel = golden.chunk_qrels.get(&q.id).unwrap_or(&empty);
        per_query.push((score(doc_rel, &doc_ids), score(chunk_rel, &chunk_ids)));
    }
    let docs: Vec<Scores> = per_query.iter().map(|p| p.0).collect();
    let chunks: Vec<Scores> = per_query.iter().map(|p| p.1).collect();
    let mut keys = BTreeMap::new();
    keys.insert(
        format!("{}/all", golden.split.as_str()),
        Block {
            queries: per_query.len(),
            doc: mean(&docs),
            chunk: mean(&chunks),
        },
    );
    Ok(Report {
        split: golden.split.as_str().to_string(),
        retriever: RETRIEVER,
        chunker: CHUNKER,
        keys,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_text_uses_fixed_four_decimals() {
        let m = LevelMetrics {
            recall_at_5: 1.0 / 3.0,
            ..Default::default()
        };
        let mut keys = BTreeMap::new();
        keys.insert(
            "dev/all".to_string(),
            Block {
                queries: 1,
                doc: m,
                chunk: m,
            },
        );
        let r = Report {
            split: "dev".into(),
            retriever: RETRIEVER,
            chunker: CHUNKER,
            keys,
        };
        let text = r.render_text();
        assert!(text.contains("dev/all queries=1\n"));
        assert!(text.contains("recall@5=0.3333 recall@10=0.0000"));
    }
}
