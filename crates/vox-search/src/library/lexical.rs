//! Indexer-owned Tantivy index over library chunks. Rebuilt wholesale from DB rows (AP-16);
//! the only writer is [`LibraryLexicalIndex::rebuild_from_rows`].

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{
    IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions, Value,
};
use tantivy::{Index, TantivyDocument};
use vox_db::LibraryChunkRow;

/// Recorded in `kb_index_state.lexical_backend`.
pub const LEXICAL_BACKEND: &str = "tantivy-0.22";

/// Field boosts, tuned on the KB eval dev split (02-09). `title` is the document title repeated
/// on every chunk of the doc, so a high boost lets one matching title crowd the whole result
/// list; at 2.0/1.5 doc-level nDCG@10 was 0.358 vs 0.549 at 0.25/0.25 (BM25 baseline: 0.460).
const TITLE_BOOST: f32 = 0.25;
const HEADING_BOOST: f32 = 0.25;

/// Where the library index lives: `<override>/library` when `VOX_SEARCH_TANTIVY_ROOT` is set and
/// non-blank, else `<data_dir>/kb/library`; `None` when neither exists (AP-14).
pub fn resolve_index_dir(
    override_root: Option<&str>,
    data_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    match override_root.map(str::trim).filter(|s| !s.is_empty()) {
        Some(root) => Some(PathBuf::from(root).join("library")),
        None => data_dir.map(|d| d.join("kb").join("library")),
    }
}

/// [`resolve_index_dir`] fed from the secrets registry and `vox_config::paths::data_dir`.
pub fn default_index_dir() -> Option<PathBuf> {
    let root = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxSearchTantivyRoot);
    resolve_index_dir(root.expose(), vox_config::paths::data_dir())
}

pub struct LibraryLexicalIndex {
    index: Index,
}

fn build_schema() -> Schema {
    let text = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("en_stem")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    let mut b = Schema::builder();
    b.add_text_field("chunk_hash", STRING | STORED);
    b.add_text_field("doc_path", STRING | STORED);
    b.add_text_field("kind", STRING | STORED);
    b.add_text_field("title", text.clone());
    b.add_text_field("heading", text.clone());
    b.add_text_field("body", text);
    b.build()
}

impl LibraryLexicalIndex {
    /// Open an existing index; errors when `dir` holds none.
    pub fn open(dir: &Path) -> Result<Self> {
        let index = Index::open_in_dir(dir)
            .with_context(|| format!("open library index at {}", dir.display()))?;
        Ok(Self { index })
    }

    /// Build a fresh index from `rows` in a sibling temp dir, then swap it into `dir`.
    ///
    /// The writer heap is fixed; Tantivy derives its thread count itself (never hardcoded).
    pub fn rebuild_from_rows(dir: &Path, rows: &[LibraryChunkRow]) -> Result<Self> {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .context("library index dir has no file name")?;
        let tmp = dir.with_file_name(format!("{name}.tmp"));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        std::fs::create_dir_all(&tmp)?;
        let schema = build_schema();
        let index = Index::create_in_dir(&tmp, schema.clone())?;
        let mut writer = index.writer::<TantivyDocument>(50_000_000)?;
        let f = |n: &str| schema.get_field(n).expect("field in own schema");
        for r in rows {
            let mut doc = TantivyDocument::default();
            doc.add_text(f("chunk_hash"), &r.chunk_hash);
            doc.add_text(f("doc_path"), &r.doc_path);
            doc.add_text(f("kind"), &r.chunk_kind);
            doc.add_text(f("title"), &r.title);
            doc.add_text(f("heading"), &r.heading_path);
            doc.add_text(f("body"), &r.body);
            writer.add_document(doc)?;
        }
        writer.commit()?;
        writer.wait_merging_threads()?;
        drop(index);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        std::fs::rename(&tmp, dir)?;
        Self::open(dir)
    }

    /// Documents currently indexed.
    pub fn num_docs(&self) -> Result<u64> {
        Ok(self.index.reader()?.searcher().num_docs())
    }

    /// BM25 over title, heading (both down-weighted, see the boosts above) and body. Lenient
    /// parse: a malformed query never fails the search. Order is (score rounded to 1e-6 desc, chunk_hash asc) so equal-scoring
    /// chunks come back in a stable order.
    // ponytail: over-fetch 3x (+16) before the tie-break sort; a tie run longer than that at
    // the cut line can still reorder. Fetch everything tied if that ever matters.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(String, f32)>> {
        let searcher = self.index.reader()?.searcher();
        let schema = self.index.schema();
        let f = |n: &str| schema.get_field(n).expect("field in own schema");
        let mut parser =
            QueryParser::for_index(&self.index, vec![f("title"), f("heading"), f("body")]);
        parser.set_field_boost(f("title"), TITLE_BOOST);
        parser.set_field_boost(f("heading"), HEADING_BOOST);
        let (q, _ignored) = parser.parse_query_lenient(query);
        let fetch = limit.saturating_mul(3).max(limit + 16);
        let mut hits: Vec<(String, f32)> = Vec::new();
        for (score, addr) in searcher.search(&q, &TopDocs::with_limit(fetch))? {
            let doc: TantivyDocument = searcher.doc(addr)?;
            if let Some(h) = doc.get_first(f("chunk_hash")).and_then(|v| v.as_str()) {
                hits.push((h.to_string(), score));
            }
        }
        hits.sort_by(|a, b| {
            let key = |s: f32| -((f64::from(s) * 1e6).round() as i64);
            key(a.1).cmp(&key(b.1)).then_with(|| a.0.cmp(&b.0))
        });
        hits.truncate(limit);
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(hash: &str, heading: &str, body: &str) -> LibraryChunkRow {
        LibraryChunkRow {
            chunk_hash: hash.into(),
            doc_path: "docs/src/a.md".into(),
            title: "Doc".into(),
            heading_path: heading.into(),
            anchor: None,
            chunk_kind: "section".into(),
            body: body.into(),
            token_estimate: 1,
        }
    }

    #[test]
    fn resolve_index_dir_prefers_override_then_data_dir() {
        let data = || Some(PathBuf::from("/data"));
        assert_eq!(
            resolve_index_dir(Some("/x"), data()),
            Some(PathBuf::from("/x/library"))
        );
        let default = Some(PathBuf::from("/data/kb/library"));
        assert_eq!(resolve_index_dir(None, data()), default);
        assert_eq!(resolve_index_dir(Some("  "), data()), default);
        assert_eq!(resolve_index_dir(None, None), None);
    }

    #[test]
    fn rebuild_search_and_swap() {
        let dir = tempfile::tempdir().unwrap();
        let idx_dir = dir.path().join("lib");
        let rows = vec![
            row("h2", "Doc › Two", "zebra zebra"),
            row("h1", "Doc › One", "zebra"),
            row("h3", "Doc › Three", "unrelated"),
        ];
        let idx = LibraryLexicalIndex::rebuild_from_rows(&idx_dir, &rows).unwrap();
        assert_eq!(idx.num_docs().unwrap(), 3);
        let hits = idx.search("zebra", 5).unwrap();
        assert_eq!(
            hits.iter().map(|h| h.0.as_str()).collect::<Vec<_>>(),
            ["h2", "h1"]
        );
        // Malformed query syntax degrades instead of erroring.
        assert!(idx.search("zebra AND (", 5).is_ok());
        // A second rebuild replaces the first and leaves no temp dir behind.
        let idx = LibraryLexicalIndex::rebuild_from_rows(&idx_dir, &rows[..1]).unwrap();
        assert_eq!(idx.num_docs().unwrap(), 1);
        assert!(!dir.path().join("lib.tmp").exists());
        assert_eq!(
            LibraryLexicalIndex::open(&idx_dir)
                .unwrap()
                .num_docs()
                .unwrap(),
            1
        );
        assert!(LibraryLexicalIndex::open(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn equal_scores_tie_break_on_hash() {
        let dir = tempfile::tempdir().unwrap();
        let rows = vec![row("b", "Doc", "same"), row("a", "Doc", "same")];
        let idx = LibraryLexicalIndex::rebuild_from_rows(&dir.path().join("lib"), &rows).unwrap();
        let hits = idx.search("same", 5).unwrap();
        assert_eq!(
            hits.iter().map(|h| h.0.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }
}
