//! Uncompressed Arrow IPC stream for one f64 column.
//!
//! `arrow-ipc` default features do not enable lz4 or zstd. The column name is
//! `values`. Callers decode only a body the frame reader already admitted.

use std::io::Cursor;
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::Float64Type;
use arrow_array::{Array, ArrayRef, Float64Array, RecordBatch};
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;

/// Encode `values` as an Arrow IPC stream with one non-null f64 column.
pub fn encode_f64_column(values: &[f64]) -> Result<Vec<u8>, String> {
    let array: ArrayRef = Arc::new(Float64Array::from(values.to_vec()));
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

/// Decode the f64 column from an Arrow IPC stream body.
pub fn decode_f64_column(bytes: &[u8]) -> Result<Vec<f64>, String> {
    let mut reader = StreamReader::try_new(Cursor::new(bytes), None)
        .map_err(|err| format!("py: arrow decode: {err}"))?;
    let batch = reader
        .next()
        .transpose()
        .map_err(|err| format!("py: arrow decode: {err}"))?
        .ok_or_else(|| "py: arrow decode: empty stream".to_string())?;
    let column = batch
        .column(0)
        .as_primitive_opt::<Float64Type>()
        .ok_or_else(|| "py: arrow decode: expected an f64 column".to_string())?;
    if column.null_count() > 0 {
        return Err("py: arrow decode: f64 column contains nulls".to_string());
    }
    Ok(column.values().to_vec())
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
}
