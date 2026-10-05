//! BEIR-format golden-set loader (`corpus.jsonl`, `queries.jsonl`, `qrels/{split}[.chunks].tsv`).
//!
//! Split membership is stored, not computed: a query belongs to the split whose qrels file
//! lists it. Loading [`Split::Dev`] never opens the test qrels files. Every validation error
//! names the file (relative to the golden-set dir) and the 1-based line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::metrics::Qrels;
use crate::kb_ids::{doc_heading_paths, doc_title, is_kb_corpus_path, split_chunk_id};

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
    /// Agent-authored seed wording under recorded user delegation (01-CONTEXT D-02);
    /// never human.
    #[serde(rename = "agent-seed")]
    AgentSeed,
    #[serde(rename = "llm-expansion")]
    LlmExpansion,
}

impl Origin {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Human => "human",
            Origin::AgentSeed => "agent-seed",
            Origin::LlmExpansion => "llm-expansion",
        }
    }

    /// Seed queries are `human` ∪ `agent-seed` (the EVAL-02 count and the `*/seed` report key).
    #[must_use]
    pub fn is_seed(self) -> bool {
        matches!(self, Origin::Human | Origin::AgentSeed)
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
    /// `YYYY-MM-DD`.
    pub created: String,
    pub intent: Intent,
    pub canary: String,
    /// For a paraphrase: the seed query it rewords. It must share that seed's split
    /// (checked by [`validate_splits`]), so dev tuning never sees a held-out question reworded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paraphrase_of: Option<String>,
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
    /// The canary GUID read from `README.md`.
    pub canary: String,
}

/// Per-split, per-origin query counts from [`validate_splits`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplitSummary {
    pub dev: BTreeMap<Origin, usize>,
    pub test: BTreeMap<Origin, usize>,
}

const QRELS_HEADER: &str = "query-id\tcorpus-id\tscore";
const CANARY_MARK: &str = "vox-kb-eval canary GUID ";

fn read(dir: &Path, rel: &str) -> Result<String> {
    vox_bounded_fs::read_utf8_path_capped(&dir.join(rel))
}

/// `(1-based line, value)` per non-blank JSONL line; errors name `rel:line`.
fn jsonl<T: DeserializeOwned>(dir: &Path, rel: &str) -> Result<Vec<(usize, T)>> {
    let text = read(dir, rel)?;
    let mut out = Vec::new();
    for (i, l) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let v = serde_json::from_str(l).with_context(|| format!("{rel}:{}", i + 1))?;
        out.push((i + 1, v));
    }
    Ok(out)
}

struct QrelRow {
    line: usize,
    qid: String,
    cid: String,
    score: u8,
}

/// Syntax-strict qrels rows: header, 3 fields, integer score 0..=2, no duplicate (query, id).
fn qrel_rows(dir: &Path, rel: &str) -> Result<Vec<QrelRow>> {
    let text = read(dir, rel)?;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let at = format!("{rel}:{}", i + 1);
        if i == 0 {
            if line != QRELS_HEADER {
                bail!("{at}: expected header {QRELS_HEADER:?}");
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let [qid, cid, score] = f[..] else {
            bail!("{at}: expected 3 tab-separated fields");
        };
        let score: u8 = score
            .parse()
            .ok()
            .filter(|s| *s <= 2)
            .with_context(|| format!("{at}: score must be an integer in 0..=2"))?;
        if !seen.insert((qid.to_string(), cid.to_string())) {
            bail!("{at}: duplicate row for ({qid}, {cid})");
        }
        out.push(QrelRow {
            line: i + 1,
            qid: qid.into(),
            cid: cid.into(),
            score,
        });
    }
    Ok(out)
}

/// Only the query-id column of a qrels file.
fn qrel_query_ids(dir: &Path, rel: &str) -> Result<BTreeSet<String>> {
    let text = read(dir, rel)?;
    let mut lines = text.lines();
    if lines.next() != Some(QRELS_HEADER) {
        bail!("{rel}:1: expected header {QRELS_HEADER:?}");
    }
    Ok(lines
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| l.split('\t').next())
        .map(str::to_string)
        .collect())
}

fn group(rows: &[QrelRow]) -> BTreeMap<String, Qrels> {
    let mut m: BTreeMap<String, Qrels> = BTreeMap::new();
    for r in rows {
        m.entry(r.qid.clone())
            .or_default()
            .insert(r.cid.clone(), r.score);
    }
    m
}

fn is_date_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

fn is_guid_shape(g: &str) -> bool {
    g.len() == 36
        && g.bytes().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(&c),
        })
}

/// Read the canary GUID from `<eval_dir>/README.md` (the token after the canary marker).
/// This is the ONLY way code obtains the GUID; it is never a literal in Rust source.
pub fn read_canary_guid(eval_dir: &Path) -> Result<String> {
    let text = read(eval_dir, "README.md")?;
    let rest = text
        .split_once(CANARY_MARK)
        .map(|(_, r)| r)
        .context("README.md: canary line missing")?;
    let guid: String = rest.chars().take(36).collect();
    if !is_guid_shape(&guid) {
        bail!("README.md: canary GUID is not a lowercase 8-4-4-4-12 hex string");
    }
    Ok(guid)
}

/// Validated `corpus.jsonl` docs only (kb-corpus paths, unique ids).
pub fn load_corpus(dir: &Path) -> Result<Vec<CorpusDoc>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (line, d) in jsonl::<CorpusDoc>(dir, "corpus.jsonl")? {
        if !is_kb_corpus_path(&d.id) {
            bail!(
                "corpus.jsonl:{line}: _id {:?} is not a KB corpus path",
                d.id
            );
        }
        if !seen.insert(d.id.clone()) {
            bail!("corpus.jsonl:{line}: duplicate _id {:?}", d.id);
        }
        out.push(d);
    }
    Ok(out)
}

/// `(id, text)` of every `queries.jsonl` line; opens no qrels file.
pub fn load_query_texts(dir: &Path) -> Result<Vec<(String, String)>> {
    Ok(jsonl::<QueryLine>(dir, "queries.jsonl")?
        .into_iter()
        .map(|(_, q)| (q.id, q.text))
        .collect())
}

fn load_queries(dir: &Path, canary: &str) -> Result<Vec<QueryLine>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (line, q) in jsonl::<QueryLine>(dir, "queries.jsonl")? {
        let at = format!("queries.jsonl:{line}");
        let m = &q.metadata;
        if m.author.trim().is_empty() || m.approved_by.trim().is_empty() {
            bail!("{at}: author and approved_by must be non-empty");
        }
        if !is_date_shape(&m.created) {
            bail!("{at}: created must be YYYY-MM-DD");
        }
        if m.canary != canary {
            bail!("{at}: metadata.canary does not match the README canary GUID");
        }
        if !seen.insert(q.id.clone()) {
            bail!("{at}: duplicate _id {:?}", q.id);
        }
        out.push(q);
    }
    Ok(out)
}

impl GoldenSet {
    /// Load and strictly validate `split` from `dir`. Opens only that split's two qrels files.
    pub fn load(dir: &Path, split: Split) -> Result<Self> {
        let canary = read_canary_guid(dir)?;
        let corpus = load_corpus(dir)?;
        let all_queries = load_queries(dir, &canary)?;
        let known: BTreeSet<&str> = all_queries.iter().map(|q| q.id.as_str()).collect();
        let s = split.as_str();
        let (doc_rel, chunk_rel) = (format!("qrels/{s}.tsv"), format!("qrels/{s}.chunks.tsv"));
        let doc_rows = qrel_rows(dir, &doc_rel)?;
        let chunk_rows = qrel_rows(dir, &chunk_rel)?;
        let texts: BTreeMap<&str, &str> = corpus
            .iter()
            .map(|d| (d.id.as_str(), d.text.as_str()))
            .collect();
        for r in &doc_rows {
            let at = format!("{doc_rel}:{}", r.line);
            if !known.contains(r.qid.as_str()) {
                bail!("{at}: unknown query id {:?}", r.qid);
            }
            if !texts.contains_key(r.cid.as_str()) {
                bail!("{at}: corpus-id {:?} is not in corpus.jsonl", r.cid);
            }
        }
        let mut headings: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        for r in &chunk_rows {
            let at = format!("{chunk_rel}:{}", r.line);
            if !known.contains(r.qid.as_str()) {
                bail!("{at}: unknown query id {:?}", r.qid);
            }
            let (doc, hp) = split_chunk_id(&r.cid)
                .with_context(|| format!("{at}: chunk id {:?} has no '#'", r.cid))?;
            let Some(text) = texts.get(doc) else {
                bail!("{at}: doc {doc:?} is not in corpus.jsonl");
            };
            let paths = headings
                .entry(doc)
                .or_insert_with(|| doc_heading_paths(doc, text));
            if !paths.contains(hp) {
                bail!("{at}: heading_path {hp:?} does not exist in {doc}");
            }
        }
        let (doc_qrels, chunk_qrels) = (group(&doc_rows), group(&chunk_rows));
        for (qid, rel) in &doc_qrels {
            if !rel.values().any(|&v| v > 0) {
                bail!("{doc_rel}: query {qid} has no doc qrel with score > 0");
            }
            if !chunk_qrels
                .get(qid)
                .is_some_and(|c| c.values().any(|&v| v == 2))
            {
                bail!("{chunk_rel}: query {qid} has no answer-bearing chunk qrel (score 2)");
            }
        }
        if let Some(qid) = chunk_qrels.keys().find(|q| !doc_qrels.contains_key(*q)) {
            bail!("{chunk_rel}: query {qid} has chunk qrels but no doc qrels in {doc_rel}");
        }
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
            canary,
        })
    }
}

/// Check dev/test are disjoint and together cover every query in `queries.jsonl`; returns
/// per-split/per-origin counts. Reads only the query-id column of the two doc qrels files.
/// The Dev load path never calls this.
pub fn validate_splits(dir: &Path) -> Result<SplitSummary> {
    let lines = jsonl::<QueryLine>(dir, "queries.jsonl")?;
    let origins: BTreeMap<String, Origin> = lines
        .iter()
        .map(|(_, q)| (q.id.clone(), q.metadata.origin))
        .collect();
    let dev = qrel_query_ids(dir, "qrels/dev.tsv")?;
    let test = qrel_query_ids(dir, "qrels/test.tsv")?;
    if let Some(q) = dev.intersection(&test).next() {
        bail!("query {q} is in both the dev and test splits");
    }
    if let Some(q) = dev.iter().chain(&test).find(|q| !origins.contains_key(*q)) {
        bail!("qrels reference unknown query id {q:?}");
    }
    if let Some(q) = origins
        .keys()
        .find(|q| !dev.contains(*q) && !test.contains(*q))
    {
        bail!("query {q} is in neither the dev nor the test split");
    }
    for (_, q) in &lines {
        let Some(seed) = &q.metadata.paraphrase_of else {
            continue;
        };
        if !origins.contains_key(seed) {
            bail!("query {} is a paraphrase of unknown query {seed:?}", q.id);
        }
        if dev.contains(&q.id) != dev.contains(seed) {
            bail!(
                "paraphrase {} is not in the same split as its seed {seed}",
                q.id
            );
        }
    }
    let count = |ids: &BTreeSet<String>| {
        let mut m = BTreeMap::new();
        for id in ids {
            *m.entry(origins[id]).or_insert(0) += 1;
        }
        m
    };
    Ok(SplitSummary {
        dev: count(&dev),
        test: count(&test),
    })
}

/// Build the full `corpus.jsonl` text for `rel_paths` (e.g. from `git ls-files`): paths failing
/// [`is_kb_corpus_path`] are skipped, the rest are read capped, BOM/CRLF-normalized and
/// emitted as compact lines sorted by `_id`, trailing newline.
pub fn snapshot_corpus_jsonl(root: &Path, rel_paths: &[String]) -> Result<String> {
    let paths: BTreeSet<&str> = rel_paths
        .iter()
        .map(String::as_str)
        .filter(|p| is_kb_corpus_path(p))
        .collect();
    let mut out = String::new();
    for rel in paths {
        let raw = vox_bounded_fs::read_utf8_path_capped(&root.join(rel))?;
        let text = vox_bounded_fs::normalize_text(raw);
        let doc = CorpusDoc {
            id: rel.to_string(),
            title: doc_title(rel, &text),
            text,
        };
        out.push_str(&serde_json::to_string(&doc)?);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod validation_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_names_are_stable() {
        assert_eq!(Split::Dev.as_str(), "dev");
        assert_eq!(Split::Test.as_str(), "test");
        assert_eq!(Origin::LlmExpansion.as_str(), "llm-expansion");
        assert_eq!(Origin::AgentSeed.as_str(), "agent-seed");
    }

    #[test]
    fn seed_is_human_or_agent_seed_only() {
        assert!(Origin::Human.is_seed() && Origin::AgentSeed.is_seed());
        assert!(!Origin::LlmExpansion.is_seed());
    }

    #[test]
    fn loads_checked_in_fixture() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kb_eval");
        let g = GoldenSet::load(&dir, Split::Dev).unwrap();
        let ids: Vec<&str> = g.queries.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids, ["q001", "q002", "q003"]);
    }
}
