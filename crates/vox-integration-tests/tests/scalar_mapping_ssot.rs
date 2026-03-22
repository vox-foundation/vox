//! Regression guard: Vox scalars must map consistently for Rust / TS / SQLite emit.

use vox_ast::scalar_mapping::VoxScalar;

#[test]
fn vox_scalar_rust_sqlite_pairs() {
    let cases = [
        (VoxScalar::Int, "i64", "INTEGER"),
        (VoxScalar::Float, "f64", "REAL"),
        (VoxScalar::Str, "String", "TEXT"),
        (VoxScalar::Bool, "bool", "INTEGER"),
    ];
    for (s, rust, sql) in cases {
        assert_eq!(s.as_rust_type(), rust, "rust type for {rust}");
        assert_eq!(s.as_sqlite_affinity(), sql, "sqlite for {rust}");
    }
}

#[test]
fn vox_scalar_ts_primitives_are_stable() {
    assert_eq!(VoxScalar::Int.as_ts_primitive(), "number");
    assert_eq!(VoxScalar::Float.as_ts_primitive(), "number");
    assert_eq!(VoxScalar::Str.as_ts_primitive(), "string");
    assert_eq!(VoxScalar::Bool.as_ts_primitive(), "boolean");
}
