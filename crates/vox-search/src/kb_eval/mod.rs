//! Deterministic retrieval evaluation over a BEIR golden set (EVAL-01).
//!
//! A retriever is any `Fn(&str) -> Vec<RankedChunk>`; [`run_eval`] scores it against a
//! [`golden::GoldenSet`] on doc ids (best chunk per doc, first occurrence = MaxP) and chunk
//! ids. No environment, clock, DB or network is read and only ordered collections are
//! iterated, so two evaluations of the same inputs are byte-identical.

pub mod baseline;
pub mod bm25;
pub mod canary;
pub mod golden;
pub mod metrics;

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::kb_ids::{
    HEADING_PATH_SEP, chunk_id, heading_path_is_ancestor_or_equal, split_chunk_id,
};
use golden::GoldenSet;
use metrics::{
    BUDGET_TOKENS, Qrels, TOKEN_ESTIMATOR, answer_in_budget, mrr_at_k, ndcg_at_k, recall_at_k,
    round6,
};

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub answer_in_budget: f64,
}

/// Deterministic evaluation report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub split: String,
    pub retriever: &'static str,
    pub chunker: &'static str,
    pub budget_tokens: usize,
    pub token_estimator: &'static str,
    /// Total queries evaluated in the split.
    pub queries: usize,
    pub keys: BTreeMap<String, Block>,
}

impl Report {
    /// Fixed `{:.4}` text; no timestamps, durations, absolute paths or run ids.
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut s = format!(
            "split={} retriever={} chunker={} budget_tokens={} token_estimator={} queries={}\n",
            self.split,
            self.retriever,
            self.chunker,
            self.budget_tokens,
            self.token_estimator,
            self.queries
        );
        for (key, b) in &self.keys {
            s.push_str(&format!(
                "{key} queries={} answer_in_budget={:.4}\n",
                b.queries, b.answer_in_budget
            ));
            for (lvl, m) in [("doc", &b.doc), ("chunk", &b.chunk)] {
                s.push_str(&format!(
                    "  {lvl:<5} recall@5={:.4} recall@10={:.4} mrr@10={:.4} ndcg@10={:.4}\n",
                    m.recall_at_5, m.recall_at_10, m.mrr_at_10, m.ndcg_at_10
                ));
            }
        }
        s
    }

    /// Pretty JSON of the whole report (ordered maps, values already rounded).
    pub fn to_json_string(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Clone, Copy, Default)]
struct Scores {
    r5: f64,
    r10: f64,
    mrr: f64,
    ndcg: f64,
}

/// One query's doc scores, chunk scores and answer-in-budget.
type QueryScores = (Scores, Scores, f64);

fn score(rel: &Qrels, ranked: &[String]) -> Scores {
    Scores {
        r5: recall_at_k(rel, ranked, 5),
        r10: recall_at_k(rel, ranked, 10),
        mrr: mrr_at_k(rel, ranked, 10),
        ndcg: ndcg_at_k(rel, ranked, 10),
    }
}

fn mean_of(xs: impl Iterator<Item = Scores> + Clone) -> LevelMetrics {
    let n = xs.clone().count().max(1) as f64;
    let m = |f: fn(&Scores) -> f64| round6(xs.clone().map(|s| f(&s)).sum::<f64>() / n);
    LevelMetrics {
        recall_at_5: m(|s| s.r5),
        recall_at_10: m(|s| s.r10),
        mrr_at_10: m(|s| s.mrr),
        ndcg_at_10: m(|s| s.ndcg),
    }
}

fn block(rows: &[&QueryScores]) -> Block {
    let n = rows.len().max(1) as f64;
    Block {
        queries: rows.len(),
        doc: mean_of(rows.iter().map(|r| r.0)),
        chunk: mean_of(rows.iter().map(|r| r.1)),
        answer_in_budget: round6(rows.iter().map(|r| r.2).sum::<f64>() / n),
    }
}

/// Re-key each chunk qrel to the deepest indexed chunk of its doc whose heading_path is an
/// ancestor-or-equal of the qrel's (keeping the max score on collisions). A qrel with no such
/// chunk is an error.
fn resolve_chunk_qrels(
    qid: &str,
    rel: &Qrels,
    doc_chunks: &BTreeMap<String, BTreeSet<String>>,
) -> Result<Qrels> {
    let mut out = Qrels::new();
    for (id, &score) in rel {
        let target = split_chunk_id(id).and_then(|(doc, hp)| {
            doc_chunks
                .get(doc)?
                .iter()
                .filter_map(|c| split_chunk_id(c))
                .filter(|(_, chp)| heading_path_is_ancestor_or_equal(chp, hp))
                .max_by_key(|(_, chp)| chp.split(HEADING_PATH_SEP).count())
                .map(|(d, chp)| chunk_id(d, chp))
        });
        let Some(target) = target else {
            bail!("query {qid}: chunk qrel {id:?} has no ancestor-or-equal indexed chunk");
        };
        let e = out.entry(target).or_insert(0);
        *e = (*e).max(score);
    }
    Ok(out)
}

/// Evaluate `retrieve` on every query of `golden`. `doc_chunks` is the retriever's indexed
/// chunk ids per doc (e.g. [`bm25::Bm25Index::doc_chunk_ids`]), used to resolve chunk qrels.
pub fn run_eval<F>(
    golden: &GoldenSet,
    doc_chunks: &BTreeMap<String, BTreeSet<String>>,
    retrieve: F,
) -> Result<Report>
where
    F: Fn(&str) -> Vec<RankedChunk>,
{
    let mut rows: Vec<(golden::Origin, QueryScores)> = Vec::new();
    for q in &golden.queries {
        // Dedupe by chunk id, keeping the first (best-ranked) occurrence.
        let mut seen = BTreeSet::new();
        let ranked: Vec<RankedChunk> = retrieve(&q.text)
            .into_iter()
            .filter(|c| seen.insert(c.id.clone()))
            .collect();
        let chunk_ids: Vec<String> = ranked.iter().map(|c| c.id.clone()).collect();
        let mut seen_docs = BTreeSet::new();
        let doc_ids: Vec<String> = chunk_ids
            .iter()
            .filter_map(|id| split_chunk_id(id).map(|(d, _)| d.to_string()))
            .filter(|d| seen_docs.insert(d.clone()))
            .collect();
        let empty = Qrels::new();
        let doc_rel = golden.doc_qrels.get(&q.id).unwrap_or(&empty);
        let raw_chunk_rel = golden.chunk_qrels.get(&q.id).unwrap_or(&empty);
        let chunk_rel = resolve_chunk_qrels(&q.id, raw_chunk_rel, doc_chunks)?;
        let answers: BTreeSet<String> = chunk_rel
            .iter()
            .filter(|(_, s)| **s == 2)
            .map(|(id, _)| id.clone())
            .collect();
        rows.push((
            q.metadata.origin,
            (
                score(doc_rel, &doc_ids),
                score(&chunk_rel, &chunk_ids),
                answer_in_budget(&answers, &ranked, BUDGET_TOKENS),
            ),
        ));
    }
    let split = golden.split.as_str();
    let mut keys = BTreeMap::new();
    keys.insert(
        format!("{split}/all"),
        block(&rows.iter().map(|r| &r.1).collect::<Vec<_>>()),
    );
    let origins: BTreeSet<golden::Origin> = rows.iter().map(|r| r.0).collect();
    for o in origins {
        let sel: Vec<&QueryScores> = rows.iter().filter(|r| r.0 == o).map(|r| &r.1).collect();
        keys.insert(format!("{split}/{}", o.as_str()), block(&sel));
    }
    Ok(Report {
        split: split.to_string(),
        retriever: RETRIEVER,
        chunker: CHUNKER,
        budget_tokens: BUDGET_TOKENS,
        token_estimator: TOKEN_ESTIMATOR,
        queries: rows.len(),
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
                answer_in_budget: 0.5,
            },
        );
        let r = Report {
            split: "dev".into(),
            retriever: RETRIEVER,
            chunker: CHUNKER,
            budget_tokens: BUDGET_TOKENS,
            token_estimator: TOKEN_ESTIMATOR,
            queries: 1,
            keys,
        };
        let text = r.render_text();
        assert!(text.contains("dev/all queries=1 answer_in_budget=0.5000\n"));
        assert!(text.contains("recall@5=0.3333 recall@10=0.0000"));
    }

    use golden::{Intent, Origin, QueryLine, QueryMeta, Split};

    const ALPHA: &str = "docs/src/alpha.md";

    fn rc(id: &str) -> RankedChunk {
        RankedChunk {
            id: id.to_string(),
            bytes: 400,
        }
    }

    /// One query `q1` with the given chunk qrels (doc qrel: `ALPHA` = 2).
    fn golden_with(origin: Origin, chunk_qrels: &[(&str, u8)]) -> GoldenSet {
        let qrels = |pairs: &[(&str, u8)]| -> Qrels {
            pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
        };
        GoldenSet {
            split: Split::Dev,
            corpus: Vec::new(),
            queries: vec![QueryLine {
                id: "q1".into(),
                text: "anything".into(),
                metadata: QueryMeta {
                    origin,
                    author: "t".into(),
                    approved_by: "t".into(),
                    created: "2026-10-04".into(),
                    intent: Intent::Where,
                    canary: String::new(),
                },
            }],
            doc_qrels: [("q1".to_string(), qrels(&[(ALPHA, 2)]))].into(),
            chunk_qrels: [("q1".to_string(), qrels(chunk_qrels))].into(),
            canary: String::new(),
        }
    }

    fn alpha_chunks() -> BTreeMap<String, BTreeSet<String>> {
        let ids = [
            "docs/src/alpha.md#Alpha Guide",
            "docs/src/alpha.md#Alpha Guide › Install",
            "docs/src/alpha.md#Alpha Guide › Usage",
        ];
        [(
            ALPHA.to_string(),
            ids.iter().map(|s| s.to_string()).collect(),
        )]
        .into()
    }

    #[test]
    fn chunk_qrel_resolves_to_deepest_ancestor() {
        let g = golden_with(
            Origin::Human,
            &[("docs/src/alpha.md#Alpha Guide › Usage › Flags", 2)],
        );
        // Only the Usage chunk is retrieved: the deepest ancestor earns the credit.
        let r = run_eval(&g, &alpha_chunks(), |_| {
            vec![rc("docs/src/alpha.md#Alpha Guide › Usage")]
        })
        .unwrap();
        let b = &r.keys["dev/all"];
        assert_eq!(b.chunk.mrr_at_10, 1.0);
        assert_eq!(b.answer_in_budget, 1.0);
        // The shallower root chunk is NOT credited while a deeper ancestor exists.
        let r = run_eval(&g, &alpha_chunks(), |_| {
            vec![rc("docs/src/alpha.md#Alpha Guide")]
        })
        .unwrap();
        assert_eq!(r.keys["dev/all"].chunk.mrr_at_10, 0.0);
    }

    #[test]
    fn unresolvable_chunk_qrel_is_error() {
        let g = golden_with(Origin::Human, &[("docs/src/alpha.md#Nope › Elsewhere", 2)]);
        let err = run_eval(&g, &alpha_chunks(), |_| Vec::new()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("q1") && msg.contains("Nope › Elsewhere"),
            "{msg}"
        );
    }

    #[test]
    fn duplicate_chunk_ids_count_once() {
        let g = golden_with(
            Origin::Human,
            &[("docs/src/alpha.md#Alpha Guide › Install", 2)],
        );
        let r = run_eval(&g, &alpha_chunks(), |_| {
            vec![
                rc("docs/src/alpha.md#Alpha Guide"),
                rc("docs/src/alpha.md#Alpha Guide"),
                rc("docs/src/alpha.md#Alpha Guide › Install"),
            ]
        })
        .unwrap();
        // Deduped run is [root, Install]: rank 2, not 3.
        assert_eq!(r.keys["dev/all"].chunk.mrr_at_10, 0.5);
    }

    #[test]
    fn origin_keys_and_json_are_deterministic() {
        let g = golden_with(
            Origin::LlmExpansion,
            &[("docs/src/alpha.md#Alpha Guide › Install", 2)],
        );
        let run = || {
            run_eval(&g, &alpha_chunks(), |_| {
                vec![rc("docs/src/alpha.md#Alpha Guide › Install")]
            })
            .unwrap()
        };
        let r = run();
        assert!(r.keys.contains_key("dev/all") && r.keys.contains_key("dev/llm-expansion"));
        assert!(!r.keys.contains_key("dev/human"));
        assert_eq!(r.to_json_string().unwrap(), run().to_json_string().unwrap());
        assert!(r.to_json_string().unwrap().contains("\"answer_in_budget\""));
    }
}
