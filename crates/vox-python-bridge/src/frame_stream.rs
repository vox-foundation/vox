//! Bounded-RAM `std.frame` chunk streaming over Arrow IPC streams and CSV inputs (`STRM-03`).

use std::collections::VecDeque;
use std::io::{BufRead, Cursor, Read};

use arrow_array::cast::AsArray;
use arrow_array::types::*;
use arrow_array::{Array, ArrayRef, BooleanArray, RecordBatch};
use arrow_ipc::reader::StreamReader;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::BridgeError;

pub const MIN_FRAME_BATCH_SIZE: usize = 1;
pub const MAX_FRAME_BATCH_SIZE: usize = 5_000;
pub const DEFAULT_FRAME_BATCH_SIZE: usize = 250;

#[must_use]
pub fn clamp_frame_batch_size(requested: usize) -> usize {
    if requested == 0 {
        DEFAULT_FRAME_BATCH_SIZE
    } else {
        requested.clamp(MIN_FRAME_BATCH_SIZE, MAX_FRAME_BATCH_SIZE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameFieldSchema {
    pub name: String,
    pub dtype: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameSchemaManifest {
    pub columns: Vec<FrameFieldSchema>,
    pub format: String,
    pub batch_size: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameChunkPage {
    pub columns: Vec<String>,
    #[serde(default)]
    pub schema: Vec<FrameFieldSchema>,
    pub rows: Vec<Vec<Value>>,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
    pub total_rows: Option<usize>,
}

enum CursorBackend {
    ArrowIpc {
        reader: StreamReader<Box<dyn Read + Send>>,
        pending_batch: Option<(RecordBatch, usize)>,
    },
    Csv {
        reader: Box<dyn BufRead + Send>,
        line_number: usize,
    },
}

/// Incremental `O(batch_size)` cursor over an Arrow IPC stream or RFC-4180 CSV reader.
pub struct FrameChunkCursor {
    columns: Vec<String>,
    schema: Vec<FrameFieldSchema>,
    format: String,
    batch_size: usize,
    rows_emitted: usize,
    exhausted: bool,
    peeked_row: Option<Vec<Value>>,
    retained_window: VecDeque<(usize, Vec<Value>)>,
    max_retained_rows: usize,
    backend: CursorBackend,
}

impl FrameChunkCursor {
    fn new(
        schema: Vec<FrameFieldSchema>,
        format: &str,
        batch_size: usize,
        backend: CursorBackend,
    ) -> Self {
        let batch_size = clamp_frame_batch_size(batch_size);
        let columns = schema.iter().map(|c| c.name.clone()).collect();
        Self {
            columns,
            schema,
            format: format.to_string(),
            batch_size,
            rows_emitted: 0,
            exhausted: false,
            peeked_row: None,
            retained_window: VecDeque::new(),
            max_retained_rows: batch_size.saturating_mul(2).min(MAX_FRAME_BATCH_SIZE),
            backend,
        }
    }

    pub fn from_arrow_ipc_reader<R: Read + Send + 'static>(
        reader: R,
        batch_size: usize,
    ) -> Result<Self, BridgeError> {
        let boxed: Box<dyn Read + Send> = Box::new(reader);
        let sr = StreamReader::try_new(boxed, None)
            .map_err(|e| BridgeError::protocol(format!("py: arrow stream init: {e}")))?;
        let schema = sr
            .schema()
            .fields()
            .iter()
            .map(|f| FrameFieldSchema {
                name: f.name().clone(),
                dtype: f.data_type().to_string(),
                nullable: f.is_nullable(),
            })
            .collect();
        let backend = CursorBackend::ArrowIpc {
            reader: sr,
            pending_batch: None,
        };
        Ok(Self::new(schema, "arrow_ipc", batch_size, backend))
    }

    pub fn from_arrow_ipc_bytes(bytes: Vec<u8>, batch_size: usize) -> Result<Self, BridgeError> {
        Self::from_arrow_ipc_reader(Cursor::new(bytes), batch_size)
    }

    pub fn from_csv_reader<R: BufRead + Send + 'static>(
        mut reader: R,
        batch_size: usize,
    ) -> Result<Self, BridgeError> {
        let mut line_number = 0usize;
        let Some(raw_cols) = next_csv_fields(&mut reader, &mut line_number)? else {
            return Err(BridgeError::protocol("py: csv stream is empty"));
        };
        let schema = raw_cols
            .into_iter()
            .map(|(name, _)| FrameFieldSchema {
                name,
                dtype: "utf8".to_string(),
                nullable: true,
            })
            .collect();
        let backend = CursorBackend::Csv {
            reader: Box::new(reader),
            line_number,
        };
        Ok(Self::new(schema, "csv", batch_size, backend))
    }

    #[must_use]
    pub fn schema_manifest(&self) -> FrameSchemaManifest {
        FrameSchemaManifest {
            columns: self.schema.clone(),
            format: self.format.clone(),
            batch_size: self.batch_size,
        }
    }

    #[must_use]
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    #[must_use]
    pub fn rows_emitted(&self) -> usize {
        self.rows_emitted
    }

    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    pub fn next_chunk(&mut self) -> Result<Option<FrameChunkPage>, BridgeError> {
        if self.exhausted {
            return Ok(None);
        }
        let page = self.read_page(self.rows_emitted, self.batch_size)?;
        if page.rows.is_empty() {
            self.exhausted = true;
            return Ok(None);
        }
        Ok(Some(page))
    }

    pub fn read_page(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<FrameChunkPage, BridgeError> {
        let limit = clamp_frame_batch_size(limit);
        let target_end = offset.saturating_add(limit);
        while !self.exhausted && self.rows_emitted < target_end {
            let next_row = match self.peeked_row.take() {
                Some(r) => Some(r),
                None => self.read_backend_row()?,
            };
            let Some(row) = next_row else {
                self.exhausted = true;
                break;
            };
            let idx = self.rows_emitted;
            self.rows_emitted += 1;
            if self.max_retained_rows > 0 {
                if self.retained_window.len() >= self.max_retained_rows {
                    self.retained_window.pop_front();
                }
                self.retained_window.push_back((idx, row));
            }
        }
        if let Some(&(oldest_idx, _)) = self.retained_window.front()
            && offset < oldest_idx
        {
            return Err(BridgeError::protocol(format!(
                "py: frame_stream offset {offset} precedes retained window start {oldest_idx}"
            )));
        }
        let rows: Vec<Vec<Value>> = self
            .retained_window
            .iter()
            .filter(|(idx, _)| *idx >= offset && *idx < target_end)
            .map(|(_, row)| row.clone())
            .collect();
        let has_more = if self.rows_emitted > target_end {
            true
        } else {
            if self.peeked_row.is_none() {
                self.peeked_row = self.read_backend_row()?;
            }
            self.exhausted = self.peeked_row.is_none();
            !self.exhausted
        };
        Ok(FrameChunkPage {
            columns: self.columns.clone(),
            schema: self.schema.clone(),
            rows,
            offset,
            limit,
            has_more,
            total_rows: self.exhausted.then_some(self.rows_emitted),
        })
    }

    #[rustfmt::skip]
    fn read_backend_row(&mut self) -> Result<Option<Vec<Value>>, BridgeError> {
        let expected_cols = self.columns.len();
        match &mut self.backend {
            CursorBackend::ArrowIpc { reader, pending_batch } => loop {
                if let Some((batch, row_idx)) = pending_batch {
                    if *row_idx < batch.num_rows() {
                        let cur = *row_idx;
                        *row_idx += 1;
                        let row = (0..batch.num_columns()).map(|c| arrow_cell_to_json(batch.column(c), cur)).collect::<Result<Vec<_>, _>>()?;
                        if *row_idx >= batch.num_rows() { *pending_batch = None; }
                        return Ok(Some(row));
                    }
                    *pending_batch = None;
                }
                match reader.next() {
                    Some(Ok(b)) if b.num_rows() > 0 => *pending_batch = Some((b, 0)),
                    Some(Ok(_)) => {}
                    Some(Err(e)) => return Err(BridgeError::protocol(format!("py: arrow stream read: {e}"))),
                    None => return Ok(None),
                }
            },
            CursorBackend::Csv { reader, line_number } => {
                let Some(fields) = next_csv_fields(reader.as_mut(), line_number)? else { return Ok(None); };
                if fields.len() != expected_cols {
                    return Err(BridgeError::protocol(format!("py: csv line {line_number} has {} columns, expected {expected_cols}", fields.len())));
                }
                Ok(Some(fields.into_iter().map(|(t, q)| csv_cell_to_json(t, q)).collect()))
            }
        }
    }
}

fn next_csv_fields(
    reader: &mut dyn BufRead,
    line_number: &mut usize,
) -> Result<Option<Vec<(String, bool)>>, BridgeError> {
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        *line_number += 1;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if !trimmed.trim().is_empty() {
            return parse_csv_record(trimmed, *line_number).map(Some);
        }
    }
}

#[rustfmt::skip]
fn parse_csv_record(line: &str, line_number: usize) -> Result<Vec<(String, bool)>, BridgeError> {
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
    if in_q { return Err(BridgeError::protocol(format!("py: csv line {line_number} has unterminated quote"))); }
    fields.push((cur, quoted));
    Ok(fields)
}

#[rustfmt::skip]
fn csv_cell_to_json(raw: String, quoted: bool) -> Value {
    let t = raw.trim();
    if quoted { Value::String(raw) }
    else if t.is_empty() { Value::Null }
    else if t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("false") { Value::Bool(t.eq_ignore_ascii_case("true")) }
    else if let Ok(v) = t.parse::<i64>() { Value::Number(v.into()) }
    else if let Ok(v) = t.parse::<f64>() && let Some(n) = Number::from_f64(v) { Value::Number(n) }
    else { Value::String(raw) }
}

#[rustfmt::skip]
fn arrow_cell_to_json(col: &ArrayRef, row: usize) -> Result<Value, BridgeError> {
    if col.is_null(row) { return Ok(Value::Null); }
    if let Some(a) = col.as_any().downcast_ref::<BooleanArray>() { return Ok(Value::Bool(a.value(row))); }
    macro_rules! prim_i64 { ($($t:ty),+) => { $(if let Some(a) = col.as_primitive_opt::<$t>() { return Ok(Value::Number(i64::from(a.value(row)).into())); })+ }; }
    macro_rules! prim_u64 { ($($t:ty),+) => { $(if let Some(a) = col.as_primitive_opt::<$t>() { return Ok(Value::Number(u64::from(a.value(row)).into())); })+ }; }
    prim_i64!(Int8Type, Int16Type, Int32Type, Int64Type, Date32Type, TimestampSecondType, TimestampMillisecondType, TimestampMicrosecondType, TimestampNanosecondType, DurationSecondType, DurationMillisecondType, DurationMicrosecondType, DurationNanosecondType);
    prim_u64!(UInt8Type, UInt16Type, UInt32Type, UInt64Type);
    if let Some(a) = col.as_primitive_opt::<Float32Type>() { return Ok(Number::from_f64(f64::from(a.value(row))).map(Value::Number).unwrap_or(Value::Null)); }
    if let Some(a) = col.as_primitive_opt::<Float64Type>() { return Ok(Number::from_f64(a.value(row)).map(Value::Number).unwrap_or(Value::Null)); }
    if let Some(a) = col.as_string_opt::<i32>() { return Ok(Value::String(a.value(row).to_string())); }
    if let Some(a) = col.as_string_opt::<i64>() { return Ok(Value::String(a.value(row).to_string())); }
    if let Some(a) = col.as_primitive_opt::<Decimal128Type>() { return Ok(Value::String(format!("{}/10^{}", a.value(row), a.scale()))); }
    Err(BridgeError::protocol(format!("py: unsupported arrow column type {}", col.data_type())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode_i64_frame;

    #[test]
    fn csv_10k_rows_streams_in_250_row_chunks_with_exact_termination() {
        let mut csv = String::from("id,label,score\n");
        for i in 0..10_000 {
            csv.push_str(&format!("{i},\"item, #{i}\",{}\n", i * 2));
        }
        let mut cursor =
            FrameChunkCursor::from_csv_reader(Cursor::new(csv.into_bytes()), 250).expect("csv");
        let manifest = cursor.schema_manifest();
        assert_eq!(manifest.format, "csv");
        assert_eq!(manifest.batch_size, 250);
        assert_eq!(manifest.columns.len(), 3);
        let (mut chunks, mut total) = (0usize, 0usize);
        while let Some(page) = cursor.next_chunk().expect("chunk") {
            chunks += 1;
            total += page.rows.len();
            assert_eq!(page.rows.len(), 250);
            assert_eq!(page.has_more, chunks < 40);
            assert_eq!(page.total_rows, (chunks == 40).then_some(10_000));
        }
        assert_eq!((chunks, total), (40, 10_000));
        assert!(cursor.next_chunk().expect("eof").is_none());
    }

    #[test]
    fn arrow_ipc_10k_rows_streams_and_enforces_sliding_window_backpressure() {
        let ids: Vec<i64> = (0..10_000).collect();
        let vals: Vec<i64> = (0..10_000).map(|x| x * 10).collect();
        let bytes = encode_i64_frame(&[("id", &ids), ("val", &vals)]).expect("encode arrow");
        let mut cursor = FrameChunkCursor::from_arrow_ipc_bytes(bytes, 250).expect("arrow cursor");
        assert_eq!(cursor.schema_manifest().format, "arrow_ipc");
        for idx in 0..40 {
            let page = cursor.next_chunk().expect("chunk").expect("some page");
            assert_eq!(
                (page.offset, page.rows.len(), page.has_more),
                (idx * 250, 250, idx < 39)
            );
        }
        let evicted = cursor
            .read_page(0, 10)
            .expect_err("evicted window must fail");
        assert!(evicted.to_string().contains("precedes retained window"));
    }
}
