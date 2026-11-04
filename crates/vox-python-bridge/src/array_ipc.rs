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
    let mut buf = Vec::new();
    let mut writer = StreamWriter::try_new(&mut buf, batch.schema().as_ref())
        .map_err(|err| format!("py: arrow encode: {err}"))?;
    writer
        .write(&batch)
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
