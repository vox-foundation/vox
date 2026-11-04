//! Uncompressed Arrow IPC stream for one f64 column.
//!
//! `arrow-ipc` default features do not enable lz4 or zstd. The column name is
//! `values`. Callers decode only a body the frame reader already admitted.

use std::io::Cursor;
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::{Float64Type, Int64Type};
use arrow_array::{Array, ArrayRef, Float64Array, Int64Array, RecordBatch};
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;

/// Encode `values` as an Arrow IPC stream with one non-null f64 column.
pub fn encode_f64_column(values: &[f64]) -> Result<Vec<u8>, String> {
    let array: ArrayRef = Arc::new(Float64Array::from(values.to_vec()));
    write_batch(array)
}

/// One decoded host column. Ints stay `i64` so `i64::MAX` is not an `f64`.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedHost {
    F64(Vec<f64>),
    I64(Vec<i64>),
}

/// Encode `values` as an Arrow IPC stream with one non-null i64 column.
pub fn encode_i64_column(values: &[i64]) -> Result<Vec<u8>, String> {
    let array: ArrayRef = Arc::new(Int64Array::from(values.to_vec()));
    write_batch(array)
}

/// One host column in an `execute_plan` body.
#[derive(Debug, Clone, Copy)]
pub enum HostColumnEncode<'a> {
    F64(&'a [f64]),
    I64(&'a [i64]),
}

impl HostColumnEncode<'_> {
    fn len(self) -> usize {
        match self {
            Self::F64(values) => values.len(),
            Self::I64(values) => values.len(),
        }
    }

    fn array(self) -> ArrayRef {
        match self {
            Self::F64(values) => Arc::new(Float64Array::from(values.to_vec())),
            Self::I64(values) => Arc::new(Int64Array::from(values.to_vec())),
        }
    }
}

/// Encode every host column.
///
/// Equal lengths share one record batch. Unequal lengths are a `VXHC` frame
/// of one-column streams so a shorter column is not padded onto a longer one.
pub fn encode_host_columns(columns: &[HostColumnEncode<'_>]) -> Result<Vec<u8>, String> {
    if columns.is_empty() {
        return Err("py: arrow encode: no host column".to_string());
    }
    if columns.len() == 1 {
        return match columns[0] {
            HostColumnEncode::F64(values) => encode_f64_column(values),
            HostColumnEncode::I64(values) => encode_i64_column(values),
        };
    }
    let same_len = columns
        .windows(2)
        .all(|pair| pair[0].len() == pair[1].len());
    if same_len {
        let owned: Vec<(String, ArrayRef)> = columns
            .iter()
            .enumerate()
            .map(|(index, column)| (format!("c{index}"), column.array()))
            .collect();
        let batch = RecordBatch::try_from_iter(
            owned
                .iter()
                .map(|(name, array)| (name.as_str(), array.clone())),
        )
        .map_err(|err| format!("py: arrow encode: {err}"))?;
        return write_record_batch(&batch);
    }
    let mut framed = Vec::from(&b"VXHC"[..]);
    push_u32(&mut framed, columns.len())?;
    for (index, column) in columns.iter().enumerate() {
        let name = format!("c{index}");
        let batch = RecordBatch::try_from_iter([(name.as_str(), column.array())])
            .map_err(|err| format!("py: arrow encode: {err}"))?;
        let bytes = write_record_batch(&batch)?;
        push_u32(&mut framed, bytes.len())?;
        framed.extend_from_slice(&bytes);
    }
    Ok(framed)
}

fn push_u32(buf: &mut Vec<u8>, value: usize) -> Result<(), String> {
    let value = u32::try_from(value)
        .map_err(|_| "py: arrow encode: column frame exceeds u32".to_string())?;
    buf.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

/// Decode an f64 or i64 column from an Arrow IPC stream body.
pub fn decode_host_column(bytes: &[u8]) -> Result<DecodedHost, String> {
    let column = read_column(bytes)?;
    if let Some(floats) = column.as_primitive_opt::<Float64Type>() {
        if floats.null_count() > 0 {
            return Err("py: arrow decode: f64 column contains nulls".to_string());
        }
        return Ok(DecodedHost::F64(floats.values().to_vec()));
    }
    if let Some(ints) = column.as_primitive_opt::<Int64Type>() {
        if ints.null_count() > 0 {
            return Err("py: arrow decode: i64 column contains nulls".to_string());
        }
        return Ok(DecodedHost::I64(ints.values().to_vec()));
    }
    Err("py: arrow decode: expected an f64 or i64 column".to_string())
}

/// Decode the f64 column from an Arrow IPC stream body.
pub fn decode_f64_column(bytes: &[u8]) -> Result<Vec<f64>, String> {
    match decode_host_column(bytes)? {
        DecodedHost::F64(values) => Ok(values),
        DecodedHost::I64(_) => Err("py: arrow decode: expected an f64 column".to_string()),
    }
}

fn write_batch(array: ArrayRef) -> Result<Vec<u8>, String> {
    let batch = RecordBatch::try_from_iter([("values", array)])
        .map_err(|err| format!("py: arrow encode: {err}"))?;
    write_record_batch(&batch)
}

fn write_record_batch(batch: &RecordBatch) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut writer = StreamWriter::try_new(&mut buf, batch.schema().as_ref())
        .map_err(|err| format!("py: arrow encode: {err}"))?;
    writer
        .write(batch)
        .map_err(|err| format!("py: arrow encode: {err}"))?;
    writer
        .finish()
        .map_err(|err| format!("py: arrow encode: {err}"))?;
    Ok(buf)
}

fn read_column(bytes: &[u8]) -> Result<ArrayRef, String> {
    let mut reader = StreamReader::try_new(Cursor::new(bytes), None)
        .map_err(|err| format!("py: arrow decode: {err}"))?;
    let batch = reader
        .next()
        .transpose()
        .map_err(|err| format!("py: arrow decode: {err}"))?
        .ok_or_else(|| "py: arrow decode: empty stream".to_string())?;
    Ok(batch.column(0).clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_column_roundtrip() {
        let bytes = encode_f64_column(&[1.0, 3.0]).expect("encode");
        let values = decode_f64_column(&bytes).expect("decode");
        assert_eq!(values, vec![1.0, 3.0]);
    }

    #[test]
    fn i64_column_roundtrip_keeps_max() {
        let max = 9_223_372_036_854_775_807_i64;
        let bytes = encode_i64_column(&[max]).expect("encode");
        match decode_host_column(&bytes).expect("decode") {
            DecodedHost::I64(values) => assert_eq!(values, vec![max]),
            DecodedHost::F64(values) => panic!("i64 column came back as f64: {values:?}"),
        }
    }
}
