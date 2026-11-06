//! Arrow batch decode for `std.frame` `collect`.
//!
//! Wire fields only — the compiler builds `Decimal`, `Date`, `DateTime`, and
//! `Duration`. A null cell or a naive timestamp is `py-plan:` and builds no
//! value. Do not call [`crate::decode_column`] from here.

use std::io::Cursor;
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::{
    Date32Type, Decimal128Type, DurationMicrosecondType, DurationMillisecondType,
    DurationNanosecondType, DurationSecondType, Float32Type, Float64Type, Int8Type, Int16Type,
    Int32Type, Int64Type, TimestampMicrosecondType, TimestampMillisecondType,
    TimestampNanosecondType, TimestampSecondType, UInt8Type, UInt16Type, UInt32Type, UInt64Type,
};
use arrow_array::{Array, ArrayRef, BooleanArray, RecordBatch};
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;

/// Arrow time unit on the wire. The compiler converts with chrono.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalUnit {
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
}

/// One non-null cell. Nulls never appear here.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameCell {
    Bool(bool),
    Int(i64),
    Float(f64),
    Utf8(String),
    Binary(Vec<u8>),
    Decimal { coefficient: i128, scale: i8 },
    Date32(i32),
    Timestamp { value: i64, unit: TemporalUnit },
    Duration { value: i64, unit: TemporalUnit },
}

/// Decoded frame batch: named columns of wire cells.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedFrame {
    pub columns: Vec<(String, Vec<FrameCell>)>,
}

/// Decode an Arrow IPC stream body into wire columns.
///
/// `step` names the plan step for null-cell `py-plan:` errors (D-06).
pub fn decode_frame_columns(
    bytes: &[u8],
    max_body: u64,
    step: &str,
) -> Result<DecodedFrame, String> {
    let body_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if body_len > max_body {
        return Err("py: fetch exceeds cap".to_string());
    }
    let batches = read_batches(bytes)?;
    let schema = batches[0].schema();
    let mut columns: Vec<(String, Vec<FrameCell>)> = schema
        .fields()
        .iter()
        .map(|field| (field.name().clone(), Vec::new()))
        .collect();
    // A stream may carry several batches (polars splits some results); keep every row.
    for batch in &batches {
        for (index, (name, cells)) in columns.iter_mut().enumerate() {
            cells.extend(decode_one_column(batch.column(index), name, step)?);
        }
    }
    Ok(DecodedFrame { columns })
}

fn read_batches(bytes: &[u8]) -> Result<Vec<RecordBatch>, String> {
    let reader = StreamReader::try_new(Cursor::new(bytes), None)
        .map_err(|err| format!("py: arrow decode: {err}"))?;
    let batches = reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("py: arrow decode: {err}"))?;
    if batches.is_empty() {
        return Err("py: arrow decode: empty stream".to_string());
    }
    Ok(batches)
}

fn decode_one_column(column: &ArrayRef, name: &str, step: &str) -> Result<Vec<FrameCell>, String> {
    if column.as_struct_opt().is_some() {
        return Err(format!(
            "py-plan: unsupported frame column type struct in {name}"
        ));
    }
    if column.as_map_opt().is_some() {
        return Err(format!(
            "py-plan: unsupported frame column type map in {name}"
        ));
    }
    if column.as_union_opt().is_some() {
        return Err(format!(
            "py-plan: unsupported frame column type union in {name}"
        ));
    }

    if let Some(bools) = column.as_any().downcast_ref::<BooleanArray>() {
        reject_nulls(bools.null_count(), name, step)?;
        return Ok(bools
            .iter()
            .map(|v| FrameCell::Bool(v.expect("non-null")))
            .collect());
    }

    if let Some(arr) = column.as_primitive_opt::<Int8Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .values()
            .iter()
            .map(|v| FrameCell::Int(i64::from(*v)))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<Int16Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .values()
            .iter()
            .map(|v| FrameCell::Int(i64::from(*v)))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<Int32Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .values()
            .iter()
            .map(|v| FrameCell::Int(i64::from(*v)))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<Int64Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr.values().iter().map(|v| FrameCell::Int(*v)).collect());
    }
    if let Some(arr) = column.as_primitive_opt::<UInt8Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .values()
            .iter()
            .map(|v| FrameCell::Int(i64::from(*v)))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<UInt16Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .values()
            .iter()
            .map(|v| FrameCell::Int(i64::from(*v)))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<UInt32Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .values()
            .iter()
            .map(|v| FrameCell::Int(i64::from(*v)))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<UInt64Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        let mut cells = Vec::with_capacity(arr.len());
        for value in arr.values() {
            if *value > i64::MAX as u64 {
                return Err(format!("py: uint64 {value} does not fit i64"));
            }
            cells.push(FrameCell::Int(*value as i64));
        }
        return Ok(cells);
    }
    if let Some(arr) = column.as_primitive_opt::<Float32Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .values()
            .iter()
            .copied()
            .map(|v| FrameCell::Float(f64::from(v)))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<Float64Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr.values().iter().map(|v| FrameCell::Float(*v)).collect());
    }
    if let Some(arr) = column.as_string_opt::<i32>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .iter()
            .map(|v| FrameCell::Utf8(v.expect("non-null").to_string()))
            .collect());
    }
    if let Some(arr) = column.as_string_opt::<i64>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .iter()
            .map(|v| FrameCell::Utf8(v.expect("non-null").to_string()))
            .collect());
    }
    if let Some(arr) = column.as_binary_opt::<i32>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .iter()
            .map(|v| FrameCell::Binary(v.expect("non-null").to_vec()))
            .collect());
    }
    if let Some(arr) = column.as_binary_opt::<i64>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr
            .iter()
            .map(|v| FrameCell::Binary(v.expect("non-null").to_vec()))
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<Decimal128Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        let scale = arr.scale();
        return Ok(arr
            .values()
            .iter()
            .map(|v| FrameCell::Decimal {
                coefficient: *v,
                scale,
            })
            .collect());
    }
    if let Some(arr) = column.as_primitive_opt::<Date32Type>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(arr.values().iter().map(|v| FrameCell::Date32(*v)).collect());
    }
    if let Some(cells) = try_timestamp(column, name, step)? {
        return Ok(cells);
    }
    if let Some(cells) = try_duration(column, name, step)? {
        return Ok(cells);
    }
    Err(format!(
        "py: arrow decode: unsupported column {}",
        column.data_type()
    ))
}

fn try_timestamp(
    column: &ArrayRef,
    name: &str,
    step: &str,
) -> Result<Option<Vec<FrameCell>>, String> {
    if let Some(arr) = column.as_primitive_opt::<TimestampSecondType>() {
        return Ok(Some(timestamp_cells(
            arr,
            TemporalUnit::Second,
            name,
            step,
        )?));
    }
    if let Some(arr) = column.as_primitive_opt::<TimestampMillisecondType>() {
        return Ok(Some(timestamp_cells(
            arr,
            TemporalUnit::Millisecond,
            name,
            step,
        )?));
    }
    if let Some(arr) = column.as_primitive_opt::<TimestampMicrosecondType>() {
        return Ok(Some(timestamp_cells(
            arr,
            TemporalUnit::Microsecond,
            name,
            step,
        )?));
    }
    if let Some(arr) = column.as_primitive_opt::<TimestampNanosecondType>() {
        return Ok(Some(timestamp_cells(
            arr,
            TemporalUnit::Nanosecond,
            name,
            step,
        )?));
    }
    Ok(None)
}

fn timestamp_cells<T>(
    arr: &arrow_array::PrimitiveArray<T>,
    unit: TemporalUnit,
    name: &str,
    step: &str,
) -> Result<Vec<FrameCell>, String>
where
    T: arrow_array::types::ArrowTimestampType,
{
    if arr.timezone().is_none() {
        return Err(format!(
            "py-plan: {step} column {name} has a naive timestamp; use .replace_time_zone(\"UTC\")"
        ));
    }
    reject_nulls(arr.null_count(), name, step)?;
    Ok(arr
        .values()
        .iter()
        .map(|v| FrameCell::Timestamp { value: *v, unit })
        .collect())
}

fn try_duration(
    column: &ArrayRef,
    name: &str,
    step: &str,
) -> Result<Option<Vec<FrameCell>>, String> {
    if let Some(arr) = column.as_primitive_opt::<DurationSecondType>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(Some(
            arr.values()
                .iter()
                .map(|v| FrameCell::Duration {
                    value: *v,
                    unit: TemporalUnit::Second,
                })
                .collect(),
        ));
    }
    if let Some(arr) = column.as_primitive_opt::<DurationMillisecondType>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(Some(
            arr.values()
                .iter()
                .map(|v| FrameCell::Duration {
                    value: *v,
                    unit: TemporalUnit::Millisecond,
                })
                .collect(),
        ));
    }
    if let Some(arr) = column.as_primitive_opt::<DurationMicrosecondType>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(Some(
            arr.values()
                .iter()
                .map(|v| FrameCell::Duration {
                    value: *v,
                    unit: TemporalUnit::Microsecond,
                })
                .collect(),
        ));
    }
    if let Some(arr) = column.as_primitive_opt::<DurationNanosecondType>() {
        reject_nulls(arr.null_count(), name, step)?;
        return Ok(Some(
            arr.values()
                .iter()
                .map(|v| FrameCell::Duration {
                    value: *v,
                    unit: TemporalUnit::Nanosecond,
                })
                .collect(),
        ));
    }
    Ok(None)
}

fn reject_nulls(null_count: usize, name: &str, step: &str) -> Result<(), String> {
    if null_count > 0 {
        Err(format!(
            "py-plan: {step} must drop or fill nulls in column {name}"
        ))
    } else {
        Ok(())
    }
}

/// Encode named `i64` columns as an Arrow IPC stream (collect fixtures).
pub fn encode_i64_frame(columns: &[(&str, &[i64])]) -> Result<Vec<u8>, String> {
    use arrow_array::Int64Array;
    let cols: Vec<(&str, ArrayRef)> = columns
        .iter()
        .map(|(name, values)| {
            (
                *name,
                Arc::new(Int64Array::from(values.to_vec())) as ArrayRef,
            )
        })
        .collect();
    write_batch(cols)
}

/// Encode one nullable `i64` column (null-cell error fixtures).
pub fn encode_nullable_i64_frame(name: &str, values: &[Option<i64>]) -> Result<Vec<u8>, String> {
    use arrow_array::Int64Array;
    write_batch(vec![(
        name,
        Arc::new(Int64Array::from(values.to_vec())) as ArrayRef,
    )])
}

/// Encode the typed collect fixture: ints, decimal128, UTC timestamp, date32, duration.
///
/// All columns are padded to `revenue.len()` so the batch is rectangular.
pub fn encode_typed_frame_fixture(
    revenue: &[i64],
    decimal_coefficient: i128,
    decimal_scale: i8,
    timestamp_us_utc: i64,
    date32_days: i32,
    duration_us: i64,
) -> Result<Vec<u8>, String> {
    use arrow_array::{
        Date32Array, Decimal128Array, DurationMicrosecondArray, Int64Array,
        TimestampMicrosecondArray,
    };
    let n = revenue.len();
    let amount = Decimal128Array::from(vec![decimal_coefficient; n])
        .with_precision_and_scale(38, decimal_scale)
        .map_err(|err| format!("py: arrow encode: {err}"))?;
    let when = TimestampMicrosecondArray::from(vec![timestamp_us_utc; n]).with_timezone_utc();
    let days = Date32Array::from(vec![date32_days; n]);
    let elapsed = DurationMicrosecondArray::from(vec![duration_us; n]);
    write_batch(vec![
        (
            "revenue",
            Arc::new(Int64Array::from(revenue.to_vec())) as ArrayRef,
        ),
        ("price", Arc::new(amount) as ArrayRef),
        ("when", Arc::new(when) as ArrayRef),
        ("day", Arc::new(days) as ArrayRef),
        ("elapsed", Arc::new(elapsed) as ArrayRef),
    ])
}

fn write_batch(columns: Vec<(&str, ArrayRef)>) -> Result<Vec<u8>, String> {
    let batch =
        RecordBatch::try_from_iter(columns).map_err(|err| format!("py: arrow encode: {err}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::builder::{ArrayBuilder, UnionBuilder};
    use arrow_array::types::Int32Type;
    use arrow_array::{
        Date32Array, Decimal128Array, DurationMicrosecondArray, Int64Array, MapArray, StringArray,
        StructArray, TimestampMicrosecondArray, UInt8Array, UInt16Array, UInt32Array, UInt64Array,
        UnionArray,
    };

    /// polars' `to_arrow()` can split a result across record batches (a group_by
    /// did for py_frame_report); every batch's rows must come back, in order.
    #[test]
    fn multi_batch_stream_decodes_every_row() {
        let schema = RecordBatch::try_from_iter(vec![(
            "revenue",
            Arc::new(Int64Array::from(vec![0_i64])) as ArrayRef,
        )])
        .expect("schema batch")
        .schema();
        let mut buf = Vec::new();
        let mut writer = StreamWriter::try_new(&mut buf, schema.as_ref()).expect("writer");
        for chunk in [vec![10_i64], vec![5_i64, 7]] {
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(Int64Array::from(chunk)) as ArrayRef],
            )
            .expect("batch");
            writer.write(&batch).expect("write");
        }
        writer.finish().expect("finish");
        drop(writer);
        let decoded = decode_frame_columns(&buf, u64::MAX, "collect").expect("decode");
        assert_eq!(
            decoded.columns,
            vec![(
                "revenue".to_string(),
                vec![FrameCell::Int(10), FrameCell::Int(5), FrameCell::Int(7)]
            )]
        );
    }

    #[test]
    fn frame_column_decode() {
        // decimal128, zoned timestamp, date32, duration
        let amount = Decimal128Array::from(vec![1999_i128])
            .with_precision_and_scale(10, 2)
            .expect("decimal meta");
        let when =
            TimestampMicrosecondArray::from(vec![1_700_000_000_000_000_i64]).with_timezone_utc();
        let day = Date32Array::from(vec![19_723_i32]);
        let elapsed = DurationMicrosecondArray::from(vec![3_600_000_000_i64]);
        let body = write_batch(vec![
            ("amount", Arc::new(amount) as ArrayRef),
            ("when", Arc::new(when) as ArrayRef),
            ("day", Arc::new(day) as ArrayRef),
            ("elapsed", Arc::new(elapsed) as ArrayRef),
        ])
        .expect("encode typed");
        let decoded = decode_frame_columns(&body, u64::MAX, "scan_csv").expect("decode typed");
        assert_eq!(decoded.columns.len(), 4);
        assert_eq!(
            decoded.columns[0].1,
            vec![FrameCell::Decimal {
                coefficient: 1999,
                scale: 2
            }]
        );
        assert_eq!(
            decoded.columns[1].1,
            vec![FrameCell::Timestamp {
                value: 1_700_000_000_000_000,
                unit: TemporalUnit::Microsecond
            }]
        );
        assert_eq!(decoded.columns[2].1, vec![FrameCell::Date32(19_723)]);
        assert_eq!(
            decoded.columns[3].1,
            vec![FrameCell::Duration {
                value: 3_600_000_000,
                unit: TemporalUnit::Microsecond
            }]
        );

        // null slot → py-plan:, no value
        let with_null = Int64Array::from(vec![Some(1), None]);
        let null_body =
            write_batch(vec![("revenue", Arc::new(with_null) as ArrayRef)]).expect("encode null");
        let null_err =
            decode_frame_columns(&null_body, u64::MAX, "scan_csv").expect_err("null must fail");
        assert!(
            null_err.starts_with("py-plan:"),
            "null must be py-plan:, got {null_err}"
        );
        assert!(
            null_err.contains("revenue"),
            "null must name the column, got {null_err}"
        );
        assert!(
            null_err.contains("scan_csv"),
            "null must name the step, got {null_err}"
        );

        // naive timestamp → fix-it substring, no value
        let naive = TimestampMicrosecondArray::from(vec![1_i64]);
        let naive_body =
            write_batch(vec![("when", Arc::new(naive) as ArrayRef)]).expect("encode naive");
        let naive_err =
            decode_frame_columns(&naive_body, u64::MAX, "collect").expect_err("naive must fail");
        assert!(
            naive_err.starts_with("py-plan:"),
            "naive must be py-plan:, got {naive_err}"
        );
        assert!(
            naive_err.contains(".replace_time_zone(\"UTC\")"),
            "naive must include fix-it, got {naive_err}"
        );

        // struct / map / union build no value
        let struct_col = StructArray::try_from(vec![(
            "inner",
            Arc::new(Int64Array::from(vec![1])) as ArrayRef,
        )])
        .expect("struct");
        let struct_body =
            write_batch(vec![("nested", Arc::new(struct_col) as ArrayRef)]).expect("struct");
        let struct_err =
            decode_frame_columns(&struct_body, u64::MAX, "collect").expect_err("struct");
        assert!(!struct_err.is_empty(), "struct must not decode to a value");

        let map_col = MapArray::from_vec_of_maps::<StringArray, Int64Array, _, _>(
            vec![Some(vec![("k".to_string(), Some(1_i64))])],
            false,
        );
        let map_body = write_batch(vec![("m", Arc::new(map_col) as ArrayRef)]).expect("map");
        let map_err = decode_frame_columns(&map_body, u64::MAX, "collect").expect_err("map");
        assert!(!map_err.is_empty(), "map must not decode to a value");

        let mut union_builder = UnionBuilder::new_dense();
        union_builder
            .append::<Int32Type>("a", 1)
            .expect("union append");
        let union_ref = union_builder.finish();
        let union_col = union_ref
            .as_any()
            .downcast_ref::<UnionArray>()
            .expect("union array");
        let union_body =
            write_batch(vec![("u", Arc::new(union_col.clone()) as ArrayRef)]).expect("union");
        let union_err = decode_frame_columns(&union_body, u64::MAX, "collect").expect_err("union");
        assert!(!union_err.is_empty(), "union must not decode to a value");

        // uint8–uint32 widen to i64; uint64 above i64::MAX is py: overflow
        let u8s = UInt8Array::from(vec![255_u8]);
        let u16s = UInt16Array::from(vec![65_535_u16]);
        let u32s = UInt32Array::from(vec![4_000_000_000_u32]);
        let wide_body = write_batch(vec![
            ("a", Arc::new(u8s) as ArrayRef),
            ("b", Arc::new(u16s) as ArrayRef),
            ("c", Arc::new(u32s) as ArrayRef),
        ])
        .expect("encode uints");
        let wide = decode_frame_columns(&wide_body, u64::MAX, "collect").expect("widen");
        assert_eq!(wide.columns[0].1, vec![FrameCell::Int(255)]);
        assert_eq!(wide.columns[1].1, vec![FrameCell::Int(65_535)]);
        assert_eq!(wide.columns[2].1, vec![FrameCell::Int(4_000_000_000)]);

        let overflow = UInt64Array::from(vec![i64::MAX as u64 + 1]);
        let overflow_body =
            write_batch(vec![("big", Arc::new(overflow) as ArrayRef)]).expect("encode u64");
        let overflow_err =
            decode_frame_columns(&overflow_body, u64::MAX, "collect").expect_err("overflow");
        assert!(
            overflow_err.starts_with("py:"),
            "uint64 overflow must be py:, got {overflow_err}"
        );
        assert!(
            overflow_err.contains("uint64"),
            "overflow must mention uint64, got {overflow_err}"
        );
        assert!(
            !overflow_err.starts_with("py-plan:"),
            "overflow must not be py-plan:, got {overflow_err}"
        );

        // fetch cap
        let tiny = write_batch(vec![("x", Arc::new(Int64Array::from(vec![1])) as ArrayRef)])
            .expect("tiny");
        let cap_err = decode_frame_columns(&tiny, 0, "collect").expect_err("cap");
        assert_eq!(cap_err, "py: fetch exceeds cap");
    }
}
