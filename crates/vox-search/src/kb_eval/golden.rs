//! BEIR-format golden-set loader (`corpus.jsonl`, `queries.jsonl`, `qrels/{split}[.chunks].tsv`).
//!
//! Split membership is stored, not computed: a query belongs to the split whose qrels file
//! lists it. Loading [`Split::Dev`] never opens the test qrels files.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::metrics::Qrels;

/// Which stored split to load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Dev,
    Test,
}

impl Split {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Split::Dev => "dev",
            Split::Test => "test",
        }
    }
}

/// Who wrote the query wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Origin {
    #[serde(rename = "human")]
    Human,
    #[serde(rename = "llm-expansion")]
    LlmExpansion,
}

impl Origin {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Human => "human",
            Origin::LlmExpansion => "llm-expansion",
        }
    }
}

/// Query intent category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Intent {
    Where,
    HowTo,
    Concept,
    Policy,
    Why,
    Troubleshoot,
    MultiHop,
}

/// One `corpus.jsonl` line.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusDoc {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub text: String,
}

/// `queries.jsonl` per-query metadata.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryMeta {
    pub origin: Origin,
    pub author: String,
    pub approved_by: String,
    pub created: String,
    pub intent: Intent,
    pub canary: String,
}

/// One `queries.jsonl` line.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryLine {
    #[serde(rename = "_id")]
    pub id: String,
    pub text: String,
    pub metadata: QueryMeta,
}

/// A loaded golden set restricted to one split.
#[derive(Debug, Clone)]
pub struct GoldenSet {
    pub split: Split,
    pub corpus: Vec<CorpusDoc>,
    /// Only the queries that have qrels in this split, sorted by id.
    pub queries: Vec<QueryLine>,
    pub doc_qrels: BTreeMap<String, Qrels>,
    pub chunk_qrels: BTreeMap<String, Qrels>,
}

fn read_jsonl<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let text = vox_bounded_fs::read_utf8_path_capped(path)?;
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            serde_json::from_str(l).with_context(|| format!("{}:{}", path.display(), i + 1))
        })
        .collect()
}

const QRELS_HEADER: &str = "query-id\tcorpus-id\tscore";

fn read_qrels(path: &Path) -> Result<BTreeMap<String, Qrels>> {
    let text = vox_bounded_fs::read_utf8_path_capped(path)?;
    let mut out: BTreeMap<String, Qrels> = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let at = || format!("{}:{}", path.display(), i + 1);
        if i == 0 {
            if line != QRELS_HEADER {
                bail!("{}: expected header {QRELS_HEADER:?}", at());
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let [qid, cid, score] = f[..] else {
            bail!("{}: expected 3 tab-separated fields", at());
        };
        let score: u8 = score.parse().with_context(|| format!("{}: score", at()))?;
        out.entry(qid.to_string())
            .or_default()
            .insert(cid.to_string(), score);
    }
    Ok(out)
}

impl GoldenSet {
    /// Load `split` from `dir`. Opens only that split's two qrels files.
    pub fn load(dir: &Path, split: Split) -> Result<Self> {
        let corpus: Vec<CorpusDoc> = read_jsonl(&dir.join("corpus.jsonl"))?;
        let all_queries: Vec<QueryLine> = read_jsonl(&dir.join("queries.jsonl"))?;
        let s = split.as_str();
        let doc_qrels = read_qrels(&dir.join(format!("qrels/{s}.tsv")))?;
        let chunk_qrels = read_qrels(&dir.join(format!("qrels/{s}.chunks.tsv")))?;
        let mut queries: Vec<QueryLine> = all_queries
            .into_iter()
            .filter(|q| doc_qrels.contains_key(&q.id))
            .collect();
        queries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self {
            split,
            corpus,
            queries,
            doc_qrels,
            chunk_qrels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_names_are_stable() {
        assert_eq!(Split::Dev.as_str(), "dev");
        assert_eq!(Split::Test.as_str(), "test");
        assert_eq!(Origin::LlmExpansion.as_str(), "llm-expansion");
    }

    #[test]
    fn loads_fixture_dev_split_sorted_by_id() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kb_eval");
        let g = GoldenSet::load(&dir, Split::Dev).unwrap();
        let ids: Vec<&str> = g.queries.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids, ["q001", "q002", "q003"]);
        assert_eq!(g.corpus.len(), 3);
    }
}
