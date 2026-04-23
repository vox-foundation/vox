//! On-disk Tantivy index for documentation / contract mirrors (`tantivy-lexical` feature).

use std::path::Path;

use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{STORED, Schema, TEXT, Value};
use tantivy::{Index, IndexWriter, ReloadPolicy, TantivyDocument};

/// One ranked lexical hit from the docs index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TantivyDocHit {
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

/// Lazily opened Tantivy index under `index_dir`.
pub struct TantivyDocsIndex {
    index: Index,
}

impl TantivyDocsIndex {
    /// Open existing index or create an empty schema if missing.
    pub fn open(index_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(index_dir).map_err(|e| e.to_string())?;
        let schema = build_schema();
        let mmap = tantivy::directory::MmapDirectory::open(index_dir).map_err(|e| e.to_string())?;
        let index = Index::open_or_create(mmap, schema).map_err(|e| e.to_string())?;
        Ok(Self { index })
    }

    /// BM25 search over `path` + `body` fields.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<TantivyDocHit>, String> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| e.to_string())?;
        let searcher = reader.searcher();
        let schema = self.index.schema();
        let path_field = schema.get_field("path").map_err(|e| e.to_string())?;
        let body_field = schema.get_field("body").map_err(|e| e.to_string())?;
        let parser = QueryParser::for_index(&self.index, vec![path_field, body_field]);
        let q = parser.parse_query(query).map_err(|e| e.to_string())?;
        let top_docs = searcher
            .search(&q, &TopDocs::with_limit(limit.max(1)))
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for (score, doc_addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_addr).map_err(|e| e.to_string())?;
            let p = doc
                .get_first(path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let body_snip = doc
                .get_first(body_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .chars()
                .take(400)
                .collect::<String>();
            out.push(TantivyDocHit {
                path: p,
                score,
                snippet: body_snip,
            });
        }
        Ok(out)
    }

    /// Replace the entire index with `documents` (path → full text).
    pub fn rebuild(index_dir: &Path, documents: &[(String, String)]) -> Result<(), String> {
        let _ = std::fs::remove_dir_all(index_dir);
        std::fs::create_dir_all(index_dir).map_err(|e| e.to_string())?;
        let schema = build_schema();
        let index = Index::create_in_dir(index_dir, schema.clone()).map_err(|e| e.to_string())?;
        let mut writer: IndexWriter = index.writer(50_000_000).map_err(|e| e.to_string())?;
        let path_f = schema.get_field("path").map_err(|e| e.to_string())?;
        let body_f = schema.get_field("body").map_err(|e| e.to_string())?;
        for (path, body) in documents {
            let mut doc = TantivyDocument::default();
            doc.add_text(path_f, path);
            doc.add_text(body_f, body);
            writer.add_document(doc).map_err(|e| e.to_string())?;
        }
        writer.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("path", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT);
    schema_builder.build()
}
