use crate::hir::nodes::HirStmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum VoxValue {
    Int(i64),
    Float(f64),
    /// Exact fixed-point decimal (`dec` literals), backed by `rust_decimal` so
    /// interpreter arithmetic matches the Rust-codegen (`--mode script`) path:
    /// `0.1dec + 0.2dec` is exactly `0.3dec`, not an IEEE-754 approximation.
    Decimal(rust_decimal::Decimal),
    /// A compiled regular expression (`std.regex.compile`). Holds the parsed
    /// `regex::Regex` so `re.find/matches/find_all` run without recompiling,
    /// matching the Rust-codegen `VoxRegex` path.
    Regex(regex::Regex),
    /// A regex match: capture groups by index (group 0 = whole match). A `None`
    /// slot means that group did not participate. `m.group(i)` → `Option[str]`.
    Match(Vec<core::option::Option<String>>),
    Str(String),
    Bool(bool),
    /// Copy-on-write list payload. `Rc` makes `VoxValue::clone()` O(1) (a refcount
    /// bump) so pass-by-value is cheap; in-place mutation uses [`Rc::make_mut`],
    /// which clones once iff the payload is aliased — preserving Vox's value
    /// semantics. Construct via [`VoxValue::list`].
    List(Rc<Vec<VoxValue>>),
    /// Copy-on-write object payload. See [`VoxValue::List`]. Construct via
    /// [`VoxValue::object`].
    Object(Rc<Vec<(String, VoxValue)>>),
    /// Copy-on-write tuple payload. See [`VoxValue::List`]. Construct via
    /// [`VoxValue::tuple`].
    Tuple(Rc<Vec<VoxValue>>),
    Null,
    Fn {
        params: Vec<String>,
        /// `Rc`-shared closure body so cloning a function value (every closure
        /// capture and every recursive call) is an O(1) refcount bump rather
        /// than a deep clone of the HIR statement list.
        body: Rc<Vec<HirStmt>>,
        env: crate::eval::env::Scope,
        /// Function name, used to build the auto-snapshot label for
        /// `@versioned` functions; empty (`""`) for anonymous lambdas.
        name: String,
        /// `@versioned`/`@tracked` — when true, the interpreter records one
        /// `repo.snapshot()` checkpoint on this function's successful return.
        is_versioned: bool,
        /// `@traced` — when true, the interpreter opens a `tracing` span for
        /// the duration of this function call.
        is_traced: bool,
    },
    Option(core::option::Option<Box<VoxValue>>),
    /// `Result[T, E]`. The Err side carries a real `VoxValue` (was `String`) so
    /// typed errors — `Error(MyAdt)` as well as `Error("string")` — survive at
    /// runtime, matching the two-parameter `Ty::Result`. A string error boxes to
    /// `VoxValue::Str` (see `err_str`), preserving the historical behavior.
    Result(core::result::Result<Box<VoxValue>, Box<VoxValue>>),
    /// An ADT variant constructor callable (not yet applied). Created by `run_module`.
    Constructor(String),
    /// An applied ADT variant value, e.g. `Applied(10, 0)`.
    Tagged {
        name: String,
        fields: Vec<VoxValue>,
    },
    // Sentinel for control flow
    _Return(Box<VoxValue>),
    _Break,
    _Continue,
    /// Builtin-method panic sentinel — produced by `unwrap()` on `None` /
    /// `Err`, `expect(msg)` on `None` / `Err`, `unwrap_err()` on `Ok`, and
    /// the rare other "this is a programmer error, halt loudly" cases.
    /// Caught by the method-call handler in
    /// [`crate::eval::expr`] and converted to
    /// [`crate::eval::EvalError::AssertionFailed`].
    ///
    /// This sentinel exists because `call_builtin_method` returns
    /// `Option<VoxValue>`, where `None` means "method not found". To
    /// signal "method ran but the program should halt", we need an
    /// in-band marker. Without it, `unwrap` on `None` would either
    /// silently return `Null` (the prior behavior — a footgun) or
    /// surface as the misleading "Method unwrap not found" diagnostic.
    /// See `docs/src/architecture/vox-stdlib-gap-audit-2026-05-23.md` §10.4
    /// for the design discussion.
    _Panic(String),
}

/// Why a journal envelope could not be decoded.
#[derive(Debug, Clone)]
pub struct JournalDecodeError {
    /// Human-readable reason (unknown kind, malformed fields, unjournalable).
    pub message: String,
}

impl std::fmt::Display for JournalDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "journal decode error: {}", self.message)
    }
}

impl std::error::Error for JournalDecodeError {}

impl VoxValue {
    /// Build a CoW [`VoxValue::List`] from an owned `Vec`.
    #[inline]
    pub fn list(items: Vec<VoxValue>) -> Self {
        VoxValue::List(Rc::new(items))
    }
    /// Build a CoW [`VoxValue::Object`] from owned fields.
    #[inline]
    pub fn object(fields: Vec<(String, VoxValue)>) -> Self {
        VoxValue::Object(Rc::new(fields))
    }
    /// Build a CoW [`VoxValue::Tuple`] from an owned `Vec`.
    #[inline]
    pub fn tuple(items: Vec<VoxValue>) -> Self {
        VoxValue::Tuple(Rc::new(items))
    }

    /// Encode this value for the durable workflow journal.
    ///
    /// Unlike `json.encode` (`eval::builtins::vox_to_json`, deliberately
    /// lossy), this encoding is round-trippable. Objects that contain a
    /// `"__vox"` key are escaped. Unjournalable kinds (`Fn`, `Regex`,
    /// sentinels) encode as `{"__vox":"Unjournalable",...}` so decode fails
    /// loudly rather than substituting `Null`.
    #[must_use]
    pub fn to_journal_json(&self) -> serde_json::Value {
        use serde_json::{Value as J, json};
        match self {
            VoxValue::Int(n) => json!(n),
            VoxValue::Float(f) => json!(f),
            VoxValue::Str(s) => json!(s),
            VoxValue::Bool(b) => json!(b),
            VoxValue::Null => J::Null,
            VoxValue::Decimal(d) => json!({"__vox": "Decimal", "value": d.to_string()}),
            VoxValue::List(items) => {
                J::Array(items.iter().map(VoxValue::to_journal_json).collect())
            }
            VoxValue::Tuple(items) => json!({
                "__vox": "Tuple",
                "items": items.iter().map(VoxValue::to_journal_json).collect::<Vec<_>>(),
            }),
            VoxValue::Object(fields) => {
                let escaped = fields.iter().any(|(k, _)| k == "__vox");
                let map: serde_json::Map<String, J> = fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_journal_json()))
                    .collect();
                if escaped {
                    json!({"__vox": "Object", "fields": J::Object(map)})
                } else {
                    J::Object(map)
                }
            }
            VoxValue::Option(Some(v)) => {
                json!({"__vox": "Some", "value": v.to_journal_json()})
            }
            VoxValue::Option(None) => json!({"__vox": "None"}),
            VoxValue::Result(Ok(v)) => json!({"__vox": "Ok", "value": v.to_journal_json()}),
            VoxValue::Result(Err(e)) => json!({"__vox": "Err", "value": e.to_journal_json()}),
            VoxValue::Tagged { name, fields } => json!({
                "__vox": "Tagged",
                "name": name,
                "fields": fields.iter().map(VoxValue::to_journal_json).collect::<Vec<_>>(),
            }),
            other => json!({"__vox": "Unjournalable", "kind": format!("{other:?}")}),
        }
    }

    /// Decode a value produced by [`VoxValue::to_journal_json`].
    ///
    /// Unknown or `Unjournalable` envelopes return [`Err`] — silent `Null`
    /// is replay corruption.
    pub fn from_journal_json(value: &serde_json::Value) -> Result<VoxValue, JournalDecodeError> {
        use serde_json::Value as J;
        let fail = |message: String| Err(JournalDecodeError { message });
        match value {
            J::Null => Ok(VoxValue::Null),
            J::Bool(b) => Ok(VoxValue::Bool(*b)),
            J::Number(n) => n
                .as_i64()
                .map(VoxValue::Int)
                .or_else(|| n.as_f64().map(VoxValue::Float))
                .ok_or_else(|| JournalDecodeError {
                    message: format!("unrepresentable number: {n}"),
                }),
            J::String(s) => Ok(VoxValue::Str(s.clone())),
            J::Array(items) => Ok(VoxValue::list(
                items
                    .iter()
                    .map(VoxValue::from_journal_json)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            J::Object(map) => match map.get("__vox").and_then(J::as_str) {
                Some("Decimal") => map
                    .get("value")
                    .and_then(J::as_str)
                    .and_then(|s| s.parse().ok())
                    .map(VoxValue::Decimal)
                    .ok_or_else(|| JournalDecodeError {
                        message: "malformed Decimal envelope".into(),
                    }),
                Some("Tuple") => {
                    let items = map.get("items").and_then(J::as_array).ok_or_else(|| {
                        JournalDecodeError {
                            message: "malformed Tuple envelope".into(),
                        }
                    })?;
                    Ok(VoxValue::tuple(
                        items
                            .iter()
                            .map(VoxValue::from_journal_json)
                            .collect::<Result<Vec<_>, _>>()?,
                    ))
                }
                Some("Object") => match map.get("fields") {
                    Some(J::Object(inner)) => Ok(VoxValue::object(
                        inner
                            .iter()
                            .map(|(k, v)| Ok((k.clone(), VoxValue::from_journal_json(v)?)))
                            .collect::<Result<Vec<_>, JournalDecodeError>>()?,
                    )),
                    _ => fail("malformed Object envelope".into()),
                },
                Some("Some") => Ok(VoxValue::Option(Some(Box::new(
                    VoxValue::from_journal_json(map.get("value").unwrap_or(&J::Null))?,
                )))),
                Some("None") => Ok(VoxValue::Option(None)),
                Some("Ok") => Ok(VoxValue::Result(Ok(Box::new(VoxValue::from_journal_json(
                    map.get("value").unwrap_or(&J::Null),
                )?)))),
                Some("Err") => Ok(VoxValue::Result(Err(Box::new(
                    VoxValue::from_journal_json(map.get("value").unwrap_or(&J::Null))?,
                )))),
                Some("Tagged") => Ok(VoxValue::Tagged {
                    name: map
                        .get("name")
                        .and_then(J::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    fields: map
                        .get("fields")
                        .and_then(J::as_array)
                        .ok_or_else(|| JournalDecodeError {
                            message: "malformed Tagged envelope".into(),
                        })?
                        .iter()
                        .map(VoxValue::from_journal_json)
                        .collect::<Result<Vec<_>, _>>()?,
                }),
                Some("Unjournalable") => fail(format!(
                    "unjournalable value: {}",
                    map.get("kind").and_then(J::as_str).unwrap_or("?")
                )),
                Some(other) => fail(format!("unknown journal envelope kind: {other}")),
                None => Ok(VoxValue::object(
                    map.iter()
                        .map(|(k, v)| Ok((k.clone(), VoxValue::from_journal_json(v)?)))
                        .collect::<Result<Vec<_>, JournalDecodeError>>()?,
                )),
            },
        }
    }
}

impl PartialEq for VoxValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            // Cross-numeric value equality, consistent with mixed Int/Float
            // arithmetic promotion — `1 is 1.0` is true.
            (Self::Int(a), Self::Float(b)) | (Self::Float(b), Self::Int(a)) => (*a as f64) == *b,
            // rust_decimal's Eq compares numeric value regardless of trailing
            // zeros, so `8.2500dec == 8.25dec` is true.
            (Self::Decimal(a), Self::Decimal(b)) => a == b,
            (Self::Regex(a), Self::Regex(b)) => a.as_str() == b.as_str(),
            (Self::Match(a), Self::Match(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => a == b,
            (Self::Tuple(a), Self::Tuple(b)) => a == b,
            (Self::Null, Self::Null) => true,
            (Self::Option(a), Self::Option(b)) => a == b,
            (Self::Result(a), Self::Result(b)) => a == b,
            (Self::Constructor(a), Self::Constructor(b)) => a == b,
            (
                Self::Tagged {
                    name: na,
                    fields: fa,
                },
                Self::Tagged {
                    name: nb,
                    fields: fb,
                },
            ) => na == nb && fa == fb,
            _ => false,
        }
    }
}

/// Box a string error message as the `VoxValue::Str` carried in a `Result` Err
/// slot. Used by stdlib builtins that produce string-valued errors, so they keep
/// their historical behavior under the widened `Result(_, Box<VoxValue>)` shape.
pub(crate) fn err_str(s: String) -> Box<VoxValue> {
    Box::new(VoxValue::Str(s))
}

#[cfg(test)]
mod tests {
    use super::VoxValue;

    /// The CoW constructors wrap their owned input in the expected variant.
    #[test]
    fn constructors_wrap_the_expected_variants() {
        assert!(matches!(VoxValue::list(vec![]), VoxValue::List(_)));
        assert!(matches!(VoxValue::object(vec![]), VoxValue::Object(_)));
        assert!(matches!(VoxValue::tuple(vec![]), VoxValue::Tuple(_)));
    }

    /// Cloning a constructed value shares the payload (O(1)) yet compares equal.
    #[test]
    fn list_clone_is_value_equal() {
        let a = VoxValue::list(vec![VoxValue::Int(1), VoxValue::Int(2)]);
        let b = a.clone();
        assert_eq!(a, b);
    }
}

#[cfg(test)]
mod journal_json_tests {
    use super::*;

    fn roundtrip(v: VoxValue) -> VoxValue {
        VoxValue::from_journal_json(&v.to_journal_json()).expect("roundtrip")
    }

    #[test]
    fn result_ok_survives_roundtrip() {
        let v = VoxValue::Result(Ok(Box::new(VoxValue::Str("tx_5".into()))));
        match roundtrip(v) {
            VoxValue::Result(Ok(inner)) => {
                assert!(matches!(*inner, VoxValue::Str(ref s) if s == "tx_5"))
            }
            other => panic!("expected Result(Ok(Str)), got {other:?}"),
        }
    }

    #[test]
    fn result_err_survives_roundtrip() {
        let v = VoxValue::Result(Err(Box::new(VoxValue::Str("declined".into()))));
        match roundtrip(v) {
            VoxValue::Result(Err(inner)) => {
                assert!(matches!(*inner, VoxValue::Str(ref s) if s == "declined"))
            }
            other => panic!("expected Result(Err(Str)), got {other:?}"),
        }
    }

    #[test]
    fn option_none_is_distinguishable_from_null() {
        assert!(matches!(
            roundtrip(VoxValue::Option(None)),
            VoxValue::Option(None)
        ));
        assert!(matches!(roundtrip(VoxValue::Null), VoxValue::Null));
    }

    #[test]
    fn tagged_adt_survives_roundtrip() {
        let v = VoxValue::Tagged {
            name: "Declined".into(),
            fields: vec![VoxValue::Int(402)],
        };
        match roundtrip(v) {
            VoxValue::Tagged { name, fields } => {
                assert_eq!(name, "Declined");
                assert!(matches!(fields.as_slice(), [VoxValue::Int(402)]));
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
    }

    #[test]
    fn user_object_with_vox_key_is_escaped() {
        let v = VoxValue::object(vec![("__vox".into(), VoxValue::Str("Ok".into()))]);
        match roundtrip(v) {
            VoxValue::Object(fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].0, "__vox");
            }
            other => panic!("expected Object, got {other:?}"),
        }
    }

    #[test]
    fn plain_scalars_and_lists_roundtrip() {
        assert!(matches!(roundtrip(VoxValue::Int(7)), VoxValue::Int(7)));
        assert!(matches!(
            roundtrip(VoxValue::Bool(true)),
            VoxValue::Bool(true)
        ));
        match roundtrip(VoxValue::list(vec![VoxValue::Int(1), VoxValue::Int(2)])) {
            VoxValue::List(items) => assert_eq!(items.len(), 2),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn unknown_envelope_is_an_error() {
        let raw = serde_json::json!({"__vox": "FromTheFuture", "value": 1});
        let err = VoxValue::from_journal_json(&raw).expect_err("unknown envelope must fail");
        assert!(
            err.message.contains("FromTheFuture"),
            "error must name the unknown kind; got {err}"
        );
    }

    #[test]
    fn unjournalable_kind_is_an_error_on_decode() {
        let raw = serde_json::json!({"__vox": "Unjournalable", "kind": "Fn"});
        assert!(VoxValue::from_journal_json(&raw).is_err());
    }
}
