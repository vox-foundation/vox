//! Bounded-RAM `std.frame` chunk streaming endpoint and registry for Axis (`STRM-03`).
//!
//! Serves paginated slices via `GET /api/v2/research/frame-chunks/{stream_id}?offset=&limit=`.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::OnceLock;

use axum::Json;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::gui_ops::enforce_route;
use crate::http_gateway::GatewayState;
use crate::services::routes::{err, ok};

const RESEARCH_SURFACES: &[&str] = &["research"];

pub const MIN_FRAME_PAGE_LIMIT: usize = 1;
pub const DEFAULT_FRAME_PAGE_LIMIT: usize = 100;
pub const MAX_FRAME_PAGE_LIMIT: usize = 500;
pub const MAX_ACTIVE_FRAME_STREAMS: usize = 64;
pub const MAX_RETAINED_STREAM_ROWS: usize = 5_000;

#[must_use]
pub fn clamp_frame_page_limit(limit: Option<usize>) -> usize {
    match limit {
        None | Some(0) => DEFAULT_FRAME_PAGE_LIMIT,
        Some(n) => n.clamp(MIN_FRAME_PAGE_LIMIT, MAX_FRAME_PAGE_LIMIT),
    }
}

pub fn validate_stream_id(stream_id: &str) -> Result<(), String> {
    let valid = !stream_id.is_empty()
        && stream_id.len() <= 128
        && !stream_id.contains("..")
        && stream_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
    valid
        .then_some(())
        .ok_or_else(|| format!("invalid frame stream_id: {stream_id:?}"))
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FrameChunkQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameFieldSchemaDto {
    pub name: String,
    pub dtype: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameSchemaManifestDto {
    pub stream_id: String,
    pub columns: Vec<FrameFieldSchemaDto>,
    pub format: String,
    pub batch_size: usize,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameChunkPageDto {
    pub stream_id: String,
    pub columns: Vec<String>,
    pub schema: Vec<FrameFieldSchemaDto>,
    pub rows: Vec<Vec<Value>>,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
    pub total_rows: Option<usize>,
    pub content_hash: String,
}

pub type FrameChunkPage = FrameChunkPageDto;

struct StreamCursorState {
    columns: Vec<String>,
    schema: Vec<FrameFieldSchemaDto>,
    format: String,
    batch_size: usize,
    content_hash: String,
    window_start: usize,
    retained_rows: VecDeque<Vec<Value>>,
    pending_csv_lines: VecDeque<String>,
    total_emitted: usize,
    max_retained_rows: usize,
}

impl StreamCursorState {
    fn manifest(&self, stream_id: &str) -> FrameSchemaManifestDto {
        FrameSchemaManifestDto {
            stream_id: stream_id.to_string(),
            columns: self.schema.clone(),
            format: self.format.clone(),
            batch_size: self.batch_size,
            content_hash: self.content_hash.clone(),
        }
    }

    fn advance_to(&mut self, target_end: usize) -> Result<(), String> {
        let col_count = self.columns.len();
        while self.total_emitted < target_end {
            let Some(line) = self.pending_csv_lines.pop_front() else {
                break;
            };
            let fields = parse_csv_line(&line)?;
            if fields.len() != col_count {
                return Err(format!(
                    "csv row has {} columns, expected {col_count}",
                    fields.len()
                ));
            }
            let row = fields
                .into_iter()
                .map(|(t, q)| csv_cell_to_value(t, q))
                .collect();
            self.push_retained_row(row);
        }
        Ok(())
    }

    fn push_retained_row(&mut self, row: Vec<Value>) {
        if self.max_retained_rows > 0 {
            if self.retained_rows.len() >= self.max_retained_rows {
                self.retained_rows.pop_front();
                self.window_start += 1;
            }
            self.retained_rows.push_back(row);
        }
        self.total_emitted += 1;
    }
}

/// Bounded in-memory registry of active `std.frame` streams keyed by `stream_id`.
pub struct FrameStreamRegistry {
    max_streams: usize,
    max_rows_per_stream: usize,
    inner: Mutex<(VecDeque<String>, HashMap<String, StreamCursorState>)>,
}

impl Default for FrameStreamRegistry {
    fn default() -> Self {
        Self::new(MAX_ACTIVE_FRAME_STREAMS, MAX_RETAINED_STREAM_ROWS)
    }
}

impl FrameStreamRegistry {
    #[must_use]
    pub fn new(max_streams: usize, max_rows_per_stream: usize) -> Self {
        Self {
            max_streams: max_streams.max(1),
            max_rows_per_stream: max_rows_per_stream.clamp(1, MAX_RETAINED_STREAM_ROWS),
            inner: Mutex::new((VecDeque::new(), HashMap::new())),
        }
    }

    #[rustfmt::skip]
    pub fn register_csv_stream(&self, stream_id: &str, csv_text: &str, batch_size: usize) -> Result<FrameSchemaManifestDto, String> {
        validate_stream_id(stream_id)?;
        let mut lines: VecDeque<String> = csv_text.lines().map(|l| l.trim_end_matches('\r')).filter(|l| !l.trim().is_empty()).map(ToString::to_string).collect();
        let Some(header_line) = lines.pop_front() else { return Err("csv stream is empty".to_string()); };
        let cols: Vec<String> = parse_csv_line(&header_line)?.into_iter().map(|(n, _)| n).collect();
        if cols.is_empty() { return Err("csv stream header has zero columns".to_string()); }
        Ok(self.insert_stream(stream_id, cols, "utf8", "csv", batch_size, csv_text.as_bytes(), lines, Vec::new()))
    }

    #[rustfmt::skip]
    pub fn register_rows_stream(&self, stream_id: &str, columns: Vec<String>, rows: Vec<Vec<Value>>, batch_size: usize) -> Result<FrameSchemaManifestDto, String> {
        validate_stream_id(stream_id)?;
        if columns.is_empty() { return Err("frame stream requires at least one column".to_string()); }
        let digest = serde_json::to_vec(&(&columns, rows.len())).unwrap_or_default();
        Ok(self.insert_stream(stream_id, columns, "json", "rows", batch_size, &digest, VecDeque::new(), rows))
    }

    pub fn get_manifest(&self, stream_id: &str) -> Result<FrameSchemaManifestDto, String> {
        validate_stream_id(stream_id)?;
        self.inner
            .lock()
            .1
            .get(stream_id)
            .map(|s| s.manifest(stream_id))
            .ok_or_else(|| format!("frame stream {stream_id:?} not found"))
    }

    pub fn read_page(
        &self,
        stream_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<FrameChunkPageDto, String> {
        validate_stream_id(stream_id)?;
        let limit = clamp_frame_page_limit(Some(limit));
        let target_end = offset.saturating_add(limit);
        let mut guard = self.inner.lock();
        let state = guard
            .1
            .get_mut(stream_id)
            .ok_or_else(|| format!("frame stream {stream_id:?} not found"))?;
        state.advance_to(target_end)?;
        if offset < state.window_start {
            return Err(format!(
                "resync_required: offset {offset} precedes retained window start {}",
                state.window_start
            ));
        }
        let rows: Vec<Vec<Value>> = state
            .retained_rows
            .iter()
            .skip(offset - state.window_start)
            .take(limit)
            .cloned()
            .collect();
        let exhausted = state.pending_csv_lines.is_empty();
        Ok(FrameChunkPageDto {
            stream_id: stream_id.to_string(),
            columns: state.columns.clone(),
            schema: state.schema.clone(),
            rows,
            offset,
            limit,
            has_more: state.total_emitted > target_end || !exhausted,
            total_rows: exhausted.then_some(state.total_emitted),
            content_hash: state.content_hash.clone(),
        })
    }

    pub fn remove_stream(&self, stream_id: &str) -> bool {
        let mut guard = self.inner.lock();
        guard.0.retain(|id| id != stream_id);
        guard.1.remove(stream_id).is_some()
    }

    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    fn insert_stream(
        &self, stream_id: &str, columns: Vec<String>, dtype: &str, format: &str,
        batch_size: usize, hash_input: &[u8], pending_csv_lines: VecDeque<String>, initial_rows: Vec<Vec<Value>>,
    ) -> FrameSchemaManifestDto {
        let schema = columns.iter().map(|name| FrameFieldSchemaDto { name: name.clone(), dtype: dtype.to_string(), nullable: true }).collect();
        let mut state = StreamCursorState {
            columns, schema, format: format.to_string(), batch_size: clamp_frame_page_limit(Some(batch_size)),
            content_hash: vox_crypto::hex_encode(&vox_crypto::secure_hash(hash_input)),
            window_start: 0, retained_rows: VecDeque::new(), pending_csv_lines, total_emitted: 0, max_retained_rows: self.max_rows_per_stream,
        };
        for row in initial_rows { state.push_retained_row(row); }
        let manifest = state.manifest(stream_id);
        let mut guard = self.inner.lock();
        guard.0.retain(|id| id != stream_id);
        while guard.0.len() >= self.max_streams {
            if let Some(evicted) = guard.0.pop_front() { guard.1.remove(&evicted); }
        }
        guard.0.push_back(stream_id.to_string());
        guard.1.insert(stream_id.to_string(), state);
        manifest
    }
}

pub fn global_frame_stream_registry() -> &'static FrameStreamRegistry {
    static REGISTRY: OnceLock<FrameStreamRegistry> = OnceLock::new();
    REGISTRY.get_or_init(FrameStreamRegistry::default)
}

/// GET /api/v2/research/frame-chunks/{stream_id}?offset=&limit=
pub async fn get_research_frame_chunks(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(stream_id): Path<String>,
    Query(q): Query<FrameChunkQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, RESEARCH_SURFACES) {
        return e;
    }
    if let Err(msg) = validate_stream_id(&stream_id) {
        return err("invalid_stream_id", &msg);
    }
    let (offset, limit) = (q.offset.unwrap_or(0), clamp_frame_page_limit(q.limit));
    match global_frame_stream_registry().read_page(&stream_id, offset, limit) {
        Ok(page) => ok(page),
        Err(msg) if msg.starts_with("resync_required:") => err("resync_required", &msg),
        Err(msg) if msg.contains("not found") => err("not_found", &msg),
        Err(msg) => err("frame_stream_error", &msg),
    }
}

#[rustfmt::skip]
fn parse_csv_line(line: &str) -> Result<Vec<(String, bool)>, String> {
    let (mut fields, mut cur, mut in_q, mut quoted) = (Vec::new(), String::new(), false, false);
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_q {
            if ch == '"' {
                if chars.peek() == Some(&'"') { let _ = chars.next(); cur.push('"'); } else { in_q = false; }
            } else { cur.push(ch); }
        } else if ch == '"' && cur.is_empty() { in_q = true; quoted = true; }
        else if ch == ',' { fields.push((std::mem::take(&mut cur), quoted)); quoted = false; }
        else { cur.push(ch); }
    }
    if in_q { return Err("csv row has unterminated quote".to_string()); }
    fields.push((cur, quoted));
    Ok(fields)
}

#[rustfmt::skip]
fn csv_cell_to_value(raw: String, quoted: bool) -> Value {
    let t = raw.trim();
    if quoted { Value::String(raw) }
    else if t.is_empty() { Value::Null }
    else if t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("false") { Value::Bool(t.eq_ignore_ascii_case("true")) }
    else if let Ok(v) = t.parse::<i64>() { Value::Number(v.into()) }
    else if let Ok(v) = t.parse::<f64>() && let Some(n) = Number::from_f64(v) { Value::Number(n) }
    else { Value::String(raw) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer};

    #[test]
    #[rustfmt::skip]
    fn registry_paginates_evicts_and_enforces_backpressure() {
        let reg = FrameStreamRegistry::new(2, 50);
        let mut csv = String::from("id,name\n");
        for i in 0..120 { csv.push_str(&format!("{i},\"row-{i}\"\n")); }
        let manifest = reg.register_csv_stream("s1", &csv, 25).expect("register");
        assert_eq!((manifest.columns.len(), manifest.content_hash.len()), (2, 64));
        let p0 = reg.read_page("s1", 0, 25).expect("p0");
        assert_eq!((p0.rows.len(), p0.has_more, p0.total_rows), (25, true, None));
        let p3 = reg.read_page("s1", 75, 50).expect("p3");
        assert_eq!((p3.rows.len(), p3.has_more, p3.total_rows), (45, false, Some(120)));
        assert!(reg.read_page("s1", 0, 10).unwrap_err().starts_with("resync_required:"));
        reg.register_csv_stream("s2", "a\n1\n", 10).expect("s2");
        reg.register_csv_stream("s3", "a\n2\n", 10).expect("s3");
        assert!(reg.get_manifest("s1").is_err(), "oldest stream s1 should be evicted");
    }

    #[tokio::test]
    #[rustfmt::skip]
    async fn handler_serves_paginated_chunks_and_rejects_traversal_ids() {
        let (gs, _db) = gateway_with_memory_db().await;
        global_frame_stream_registry().register_csv_stream("test-stream-449", "col_a,col_b\n10,alpha\n20,beta\n30,gamma\n", 2).expect("register");
        let q = Query(FrameChunkQuery { offset: Some(0), limit: Some(2) });
        let resp = body(get_research_frame_chunks(State(gs.clone()), peer(), HeaderMap::new(), Path("test-stream-449".to_string()), q).await).await;
        assert_eq!((resp["data"]["stream_id"].as_str(), resp["data"]["rows"].as_array().map(Vec::len), resp["data"]["has_more"].as_bool()), (Some("test-stream-449"), Some(2), Some(true)));
        let bad = body(get_research_frame_chunks(State(gs), peer(), HeaderMap::new(), Path("../traversal".to_string()), Query(FrameChunkQuery::default())).await).await;
        assert_eq!(bad["error"]["code"].as_str(), Some("invalid_stream_id"));
        global_frame_stream_registry().remove_stream("test-stream-449");
    }
}
