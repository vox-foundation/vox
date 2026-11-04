//! Arrow column decode for `std.array` materializers.
//!
//! `f32` widens with a numeric cast. A `uint64` above `i64::MAX` is an error
//! prefixed `py:`. A nested column is rank other than 1 and returns `py-plan:`.
//! Bytes longer than the fetch cap are rejected before the stream is read.

use std::io::Cursor;
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::{Float32Type, Float64Type, Int64Type, UInt64Type};
use arrow_array::{Array, ArrayRef, Float32Array, RecordBatch, UInt64Array};
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;

/// Host numbers from one Arrow column. Floats are `f64` after widening.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedColumn {
    Floats(Vec<f64>),
    Ints(Vec<i64>),
}

/// Encode one non-null `f32` column. The host widens it on decode.
pub fn encode_f32_column(values: &[f32]) -> Result<Vec<u8>, String> {
    let array: ArrayRef = Arc::new(Float32Array::from(values.to_vec()));
    write_batch(array)
}

/// Encode one non-null `uint64` column.
pub fn encode_u64_column(values: &[u64]) -> Result<Vec<u8>, String> {
    let array: ArrayRef = Arc::new(UInt64Array::from(values.to_vec()));
    write_batch(array)
}

/// Decode `bytes` when `bytes.len()` is within `max_body`.
///
/// Does not saturate a `uint64`. Does not `f64::from_bits` an `f32` pattern.
pub fn decode_column(bytes: &[u8], max_body: u64) -> Result<DecodedColumn, String> {
    let body_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if body_len > max_body {
        return Err("py: fetch exceeds cap".to_string());
    }
    let column = read_column(bytes)?;
    if column.data_type().is_nested() {
        return Err("py-plan: to_list requires rank 1".to_string());
    }
    if let Some(floats) = column.as_primitive_opt::<Float64Type>() {
        reject_nulls(floats.null_count())?;
        return Ok(DecodedColumn::Floats(floats.values().to_vec()));
    }
    if let Some(floats) = column.as_primitive_opt::<Float32Type>() {
        reject_nulls(floats.null_count())?;
        let widened = floats.values().iter().copied().map(f64::from).collect();
        return Ok(DecodedColumn::Floats(widened));
    }
    if let Some(ints) = column.as_primitive_opt::<Int64Type>() {
        reject_nulls(ints.null_count())?;
        return Ok(DecodedColumn::Ints(ints.values().to_vec()));
    }
    if let Some(ints) = column.as_primitive_opt::<UInt64Type>() {
        reject_nulls(ints.null_count())?;
        let mut host = Vec::with_capacity(ints.len());
        for value in ints.values() {
            if *value > i64::MAX as u64 {
                return Err(format!("py: uint64 {value} does not fit i64"));
            }
            host.push(*value as i64);
        }
        return Ok(DecodedColumn::Ints(host));
    }
    Err(format!(
        "py: arrow decode: unsupported column {}",
        column.data_type()
    ))
}

fn reject_nulls(null_count: usize) -> Result<(), String> {
    if null_count > 0 {
        Err("py: arrow decode: column contains nulls".to_string())
    } else {
        Ok(())
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
    fn decode_column_keeps_i64_min() {
        let bytes = crate::encode_i64_column(&[i64::MIN]).expect("encode");
        match decode_column(&bytes, u64::MAX).expect("decode") {
            DecodedColumn::Ints(values) => assert_eq!(values, vec![i64::MIN]),
            DecodedColumn::Floats(values) => panic!("i64::MIN came back as f64: {values:?}"),
        }
    }

    #[test]
    fn decode_column_rejects_uint64_past_i64() {
        let bytes = encode_u64_column(&[i64::MAX as u64 + 1]).expect("encode");
        let err = decode_column(&bytes, u64::MAX).expect_err("uint64 must not fit");
        assert!(err.starts_with("py:"), "{err}");
        assert!(err.contains("uint64"), "{err}");
        assert!(!err.starts_with("py-plan:"), "{err}");
    }

    #[test]
    fn decode_column_widens_f32() {
        let bytes = encode_f32_column(&[0.1f32]).expect("encode");
        let widened = f64::from(0.1f32);
        assert_ne!(widened, 0.1f64);
        match decode_column(&bytes, u64::MAX).expect("decode") {
            DecodedColumn::Floats(values) => assert_eq!(values, vec![widened]),
            DecodedColumn::Ints(values) => panic!("f32 came back as ints: {values:?}"),
        }
    }

    #[test]
    fn decode_column_rejects_over_cap_without_a_value() {
        let bytes = encode_f32_column(&[0.1f32]).expect("encode");
        let err = decode_column(&bytes, 0).expect_err("cap");
        assert_eq!(err, "py: fetch exceeds cap");
    }

    #[test]
    fn decode_column_rejects_nested_rank() {
        use arrow_array::ListArray;
        use arrow_array::types::Float64Type;

        let nested = ListArray::from_iter_primitive::<Float64Type, _, _>([Some(vec![
            Some(1.0_f64),
            Some(2.0),
        ])]);
        let array: ArrayRef = Arc::new(nested);
        let bytes = write_batch(array).expect("encode");
        let err = decode_column(&bytes, u64::MAX).expect_err("rank");
        assert!(err.starts_with("py-plan:"), "{err}");
    }

    #[test]
    fn decode_column_keeps_f64() {
        let bytes = crate::encode_f64_column(&[1.0, 3.0]).expect("encode");
        match decode_column(&bytes, u64::MAX).expect("decode") {
            DecodedColumn::Floats(values) => assert_eq!(values, vec![1.0, 3.0]),
            DecodedColumn::Ints(values) => panic!("f64 came back as ints: {values:?}"),
        }
    }
}
