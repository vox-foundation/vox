//! On-disk `std.array` rows joined to generated signatures and worker handlers.
//!
//! `workspace_root` matches `rust_ecosystem_support_parity_test.rs`.

use std::path::PathBuf;

use serde_yaml::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const D01_OPS: &[&str] = &[
    "from_list",
    "astype",
    "add",
    "sub",
    "mul",
    "div",
    "mean",
    "std",
    "sort",
    "to_list",
    "item",
    "shape",
];

#[test]
fn on_disk_expected_matches_behavior_cases() {
    let root = workspace_root();
    let yaml_path = root.join("contracts/python/ecosystem-support.yaml");
    let raw = std::fs::read_to_string(&yaml_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", yaml_path.display()));
    let doc: Value = serde_yaml::from_str(&raw)
        .unwrap_or_else(|err| panic!("parse {}: {err}", yaml_path.display()));
    let ops = doc.get("ops").and_then(Value::as_sequence).expect("ops");
    let names: Vec<&str> = ops
        .iter()
        .map(|op| op.get("name").and_then(Value::as_str).expect("op name"))
        .collect();
    assert_eq!(names, D01_OPS, "yaml ops must be the twelve D-01 names");
    for banned in ["linalg", "fft", "where", "slice"] {
        assert!(
            !names.contains(&banned),
            "{banned} is not a stats-slice node"
        );
    }

    let worker_path = root.join("crates/vox-python-bridge/worker/vox_bridge/array_ops.py");
    let worker = std::fs::read_to_string(&worker_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", worker_path.display()));
    let table = vox_compiler::python_array_support::python_array_ops();
    assert_eq!(table.len(), ops.len());

    for op in ops {
        let name = op.get("name").and_then(Value::as_str).expect("name");
        let signature = op
            .get("signature")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{name} missing signature"));
        let generated = table
            .iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("{name} missing from the generated table"));
        assert_eq!(generated.signature, signature, "{name} signature");
        assert_eq!(generated.handler, name, "{name} handler");
        let expected = op
            .get("expected")
            .unwrap_or_else(|| panic!("{name} missing expected"));
        assert!(
            expected.is_mapping(),
            "{name} expected must be a non-null object, got {expected:?}"
        );
        assert!(
            !generated.expected_yaml.trim().is_empty() && generated.expected_yaml.trim() != "null",
            "{name} generated expected is empty"
        );
        let def_at_column_0 = worker
            .lines()
            .any(|line| line.starts_with(&format!("def {name}(")));
        let handler_key = worker.contains(&format!("\"{name}\": {name}"));
        assert!(def_at_column_0, "{name} worker def must start at column 0");
        assert!(handler_key, "{name} must be a HANDLERS key");
    }

    let add = expected_of(ops, "add");
    assert_f64s(add.get("lhs").expect("add lhs"), &[1.0, 3.0]);
    assert_f64s(add.get("rhs").expect("add rhs"), &[1.0, 1.0]);
    assert_f64s(add.get("values").expect("add values"), &[2.0, 4.0]);

    let mul = expected_of(ops, "mul");
    assert_f64s(mul.get("lhs").expect("mul lhs"), &[2.0, 3.0]);
    assert_f64s(mul.get("rhs").expect("mul rhs"), &[4.0, 5.0]);
    assert_f64s(mul.get("values").expect("mul values"), &[8.0, 15.0]);

    let sort = expected_of(ops, "sort");
    assert_f64s(
        sort.get("input").expect("sort input"),
        &[3.0, f64::NAN, 1.0],
    );
    assert_f64s(
        sort.get("nulls_first").expect("nulls=first"),
        &[f64::NAN, 1.0, 3.0],
    );
    assert_f64s(
        sort.get("nulls_last").expect("nulls=last"),
        &[1.0, 3.0, f64::NAN],
    );

    let item = expected_of(ops, "item");
    assert_eq!(item.get("host").and_then(Value::as_str), Some("float"));
    assert_eq!(item.get("rank").and_then(Value::as_i64), Some(0));
    assert_f64s(item.get("input").expect("item input"), &[2.0]);
    assert_f64s(
        &Value::Sequence(vec![item.get("value").expect("item value").clone()]),
        &[2.0],
    );

    let shape = expected_of(ops, "shape");
    assert_eq!(shape.get("host").and_then(Value::as_str), Some("list_int"));
    assert_f64s(shape.get("input").expect("shape input"), &[1.0, 3.0]);
    assert_eq!(
        yaml_ints(shape.get("values").expect("shape values")),
        vec![2]
    );
}

/// A row counts only with a signature, a real worker handler, and a non-null
/// expected object. A comment that names an op is not a handler.
fn check_row(row: &Value, worker: &str) -> Result<(), String> {
    let name = row
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("row");

    let signature = row.get("signature").and_then(Value::as_str);
    if signature.is_none_or(|signature| signature.trim().is_empty()) {
        return Err(format!("{name} missing signature"));
    }

    let Some(handler) = row
        .get("handler")
        .and_then(Value::as_str)
        .filter(|handler| !handler.trim().is_empty())
    else {
        return Err(format!("{name} missing handler"));
    };
    if !worker_defines_handler(worker, handler) {
        return Err(format!(
            "{name} handler {handler} is not a column-0 def and HANDLERS key"
        ));
    }

    match row.get("expected") {
        None => Err(format!("{name} missing expected")),
        Some(expected) if expected.is_null() => Err(format!("{name} expected is null")),
        Some(expected) if !expected.is_mapping() => {
            Err(format!("{name} expected must be an object"))
        }
        Some(_) => Ok(()),
    }
}

/// Column-0 `def` plus a HANDLERS entry. Comment lines do not count.
fn worker_defines_handler(worker: &str, handler: &str) -> bool {
    let def_prefix = format!("def {handler}(");
    let key_needle = format!("\"{handler}\": {handler}");
    let mut has_def = false;
    let mut has_key = false;
    for line in worker.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if line.starts_with(&def_prefix) {
            has_def = true;
        }
        if let Some(code) = line.split('#').next()
            && code.contains(&key_needle)
        {
            has_key = true;
        }
    }
    has_def && has_key
}

#[test]
fn synthetic_row_missing_signature_handler_or_expected_is_err() {
    let root = workspace_root();
    let yaml_path = root.join("contracts/python/ecosystem-support.yaml");
    let before = std::fs::read(&yaml_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", yaml_path.display()));
    let doc: Value = serde_yaml::from_slice(&before)
        .unwrap_or_else(|err| panic!("parse {}: {err}", yaml_path.display()));
    let ops = doc.get("ops").and_then(Value::as_sequence).expect("ops");
    let mean = ops
        .iter()
        .find(|op| op.get("name").and_then(Value::as_str) == Some("mean"))
        .expect("mean row")
        .clone();
    let worker_path = root.join("crates/vox-python-bridge/worker/vox_bridge/array_ops.py");
    let worker = std::fs::read_to_string(&worker_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", worker_path.display()));

    let mut dropped_signature = mean.clone();
    dropped_signature
        .as_mapping_mut()
        .expect("row")
        .remove("signature");
    assert!(
        check_row(&dropped_signature, &worker).is_err(),
        "dropping signature must return Err"
    );

    let mut dropped_handler = mean.clone();
    dropped_handler
        .as_mapping_mut()
        .expect("row")
        .remove("handler");
    assert!(
        check_row(&dropped_handler, &worker).is_err(),
        "dropping handler must return Err"
    );

    let mut unknown_handler = mean.clone();
    unknown_handler.as_mapping_mut().expect("row").insert(
        Value::String("handler".into()),
        Value::String("no_such_op".into()),
    );
    assert!(
        check_row(&unknown_handler, &worker).is_err(),
        "a handler that is not a column-0 def and not a HANDLERS key must return Err"
    );

    // A comment that mentions the op is not a column-0 def and not a HANDLERS key.
    let comment_only = "# def ghost(\n# \"ghost\": ghost\n";
    let mut comment_row = mean.clone();
    let comment_map = comment_row.as_mapping_mut().expect("row");
    comment_map.insert(Value::String("name".into()), Value::String("ghost".into()));
    comment_map.insert(
        Value::String("handler".into()),
        Value::String("ghost".into()),
    );
    assert!(
        check_row(&comment_row, comment_only).is_err(),
        "a comment that mentions an op name is not a handler"
    );

    let mut dropped_expected = mean.clone();
    dropped_expected
        .as_mapping_mut()
        .expect("row")
        .remove("expected");
    assert!(
        check_row(&dropped_expected, &worker).is_err(),
        "dropping expected must return Err"
    );

    let mut null_expected = mean.clone();
    null_expected
        .as_mapping_mut()
        .expect("row")
        .insert(Value::String("expected".into()), Value::Null);
    assert!(
        check_row(&null_expected, &worker).is_err(),
        "a null expected must return Err"
    );

    assert!(
        check_row(&mean, &worker).is_ok(),
        "a complete mean row against the worker must return Ok"
    );

    let after = std::fs::read(&yaml_path)
        .unwrap_or_else(|err| panic!("reread {}: {err}", yaml_path.display()));
    assert_eq!(before, after, "synthetic cases must not rewrite the yaml");
}

fn expected_of<'a>(ops: &'a [Value], name: &str) -> &'a Value {
    ops.iter()
        .find(|op| op.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|op| op.get("expected"))
        .unwrap_or_else(|| panic!("{name} expected"))
}

fn assert_f64s(value: &Value, expect: &[f64]) {
    let got = f64s(value);
    assert_eq!(got.len(), expect.len(), "{value:?} vs {expect:?}");
    for (got, expect) in got.iter().zip(expect) {
        if expect.is_nan() {
            assert!(got.is_nan(), "expected NaN, got {got} in {value:?}");
        } else {
            assert_eq!(got, expect, "{value:?}");
        }
    }
}

fn f64s(value: &Value) -> Vec<f64> {
    value
        .as_sequence()
        .unwrap_or_else(|| panic!("expected a list, got {value:?}"))
        .iter()
        .map(|item| {
            item.as_f64()
                .unwrap_or_else(|| panic!("expected a float, got {item:?}"))
        })
        .collect()
}

fn yaml_ints(value: &Value) -> Vec<i64> {
    value
        .as_sequence()
        .unwrap_or_else(|| panic!("expected a list, got {value:?}"))
        .iter()
        .map(|item| {
            item.as_i64()
                .or_else(|| item.as_f64().map(|number| number as i64))
                .unwrap_or_else(|| panic!("expected an int, got {item:?}"))
        })
        .collect()
}
