//! Decode numerics protocol identity for MENS reasoning evaluation (D-24/D-25).
//!
//! # Protocol definitions
//!
//! - `f32`: NF4-quantised weights dequantised once and cached as f32, f32 matmuls,
//!   today's behavior.
//! - `bf16`: the same NF4 weights cached as bf16 for the transformer-block linears only,
//!   decode rows padded to `gemm_rows` so a sequence's text never depends on its batch-mates;
//!   embedding, norms, attention math, KV cache, LM head and logits stay f32.
//!
//! A later numerics change (fused kernels, a different row count) gets a new tag value,
//! never a redefinition. Results under different protocols are keyed and reported separately
//! and never paired.

/// Environment variable naming the decode protocol: `"f32"` (default) or `"bf16"`.
pub const PROTOCOL_ENV: &str = "VOX_MENS_DECODE_PROTOCOL";

/// Environment variable specifying the row count every bf16 decode GEMM is padded to.
pub const BF16_GEMM_ROWS_ENV: &str = "VOX_MENS_DECODE_BF16_GEMM_ROWS";

/// Decode numerics protocol identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeProtocol {
    /// NF4-quantised weights cached as f32, f32 matmuls (baseline).
    F32,
    /// NF4-quantised weights cached as bf16 for block linears, padded to `gemm_rows`.
    Bf16 { gemm_rows: usize },
}

impl DecodeProtocol {
    /// Parse the protocol from optional tag and gemm_rows string slices.
    pub fn parse(tag: Option<&str>, gemm_rows: Option<&str>) -> Result<Self, String> {
        let trimmed_tag = tag.map(str::trim);
        match trimmed_tag {
            None | Some("f32") => {
                if let Some(rows) = gemm_rows.map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    return Err(format!(
                        "decode protocol 'f32' does not accept a row count ({BF16_GEMM_ROWS_ENV}={rows:?}); \
                         the row count only applies to 'bf16'"
                    ));
                }
                Ok(Self::F32)
            }
            Some("bf16") => {
                let rows_str = gemm_rows.map(|s| s.trim()).filter(|s| !s.is_empty());
                let rows_val = match rows_str {
                    Some(s) => match s.parse::<usize>() {
                        Ok(n) if n > 0 => n,
                        _ => {
                            return Err(format!(
                                "{BF16_GEMM_ROWS_ENV} must be a positive integer, got {s:?}"
                            ));
                        }
                    },
                    None => {
                        return Err(format!(
                            "{BF16_GEMM_ROWS_ENV} is required when decode protocol is 'bf16'"
                        ));
                    }
                };
                Ok(Self::Bf16 {
                    gemm_rows: rows_val,
                })
            }
            Some(unknown) => Err(format!(
                "unknown decode protocol {unknown:?}; accepted values are 'f32' and 'bf16'"
            )),
        }
    }

    /// Read the protocol from process environment variables [`PROTOCOL_ENV`] and [`BF16_GEMM_ROWS_ENV`].
    pub fn from_env() -> Result<Self, String> {
        let tag_val = match std::env::var_os(PROTOCOL_ENV) {
            Some(v) => match v.into_string() {
                Ok(s) => Some(s),
                Err(_) => return Err(format!("{PROTOCOL_ENV} contains invalid UTF-8")),
            },
            None => None,
        };
        let rows_val = match std::env::var_os(BF16_GEMM_ROWS_ENV) {
            Some(v) => match v.into_string() {
                Ok(s) => Some(s),
                Err(_) => return Err(format!("{BF16_GEMM_ROWS_ENV} contains invalid UTF-8")),
            },
            None => None,
        };
        Self::parse(tag_val.as_deref(), rows_val.as_deref())
    }

    /// Static string tag (`"f32"` or `"bf16"`).
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::Bf16 { .. } => "bf16",
        }
    }

    /// Recover the protocol from a report's `decode` object.
    ///
    /// An absent object (`None` or `null`) corresponds to [`DecodeProtocol::F32`],
    /// since all reports written prior to Phase 337 were produced under the f32 protocol.
    pub fn from_report(decode: Option<&serde_json::Value>) -> Result<Self, String> {
        let val = match decode {
            None => return Ok(Self::F32),
            Some(v) if v.is_null() => return Ok(Self::F32),
            Some(v) => v,
        };
        let obj = val
            .as_object()
            .ok_or_else(|| format!("decode field in report must be an object, got {val}"))?;

        let tag = match obj.get("protocol") {
            Some(serde_json::Value::String(s)) => s.as_str(),
            Some(other) => {
                return Err(format!("decode.protocol must be a string, got {other}"));
            }
            None => {
                return Err("decode object missing required 'protocol' field".to_string());
            }
        };

        match tag {
            "f32" => Ok(Self::F32),
            "bf16" => {
                let rows = match obj.get("bf16_gemm_rows") {
                    Some(serde_json::Value::Number(n)) => match n.as_u64() {
                        Some(u) if u > 0 => u as usize,
                        _ => {
                            return Err(format!(
                                "{BF16_GEMM_ROWS_ENV} must be a positive integer in decode object, got {n}"
                            ));
                        }
                    },
                    Some(other) => {
                        return Err(format!(
                            "{BF16_GEMM_ROWS_ENV} must be an integer in decode object, got {other}"
                        ));
                    }
                    None => {
                        return Err(format!(
                            "decode object with protocol 'bf16' is missing {BF16_GEMM_ROWS_ENV} ('bf16_gemm_rows')"
                        ));
                    }
                };
                Ok(Self::Bf16 { gemm_rows: rows })
            }
            unknown => Err(format!(
                "unknown decode protocol {unknown:?}; accepted values are 'f32' and 'bf16'"
            )),
        }
    }
}

/// Generation decode settings for reasoning evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodeSettings {
    pub protocol: DecodeProtocol,
    pub max_tokens: usize,
    pub temperature: f64,
    pub seed: u64,
    pub thinking: bool,
}

impl Default for DecodeSettings {
    fn default() -> Self {
        Self {
            protocol: DecodeProtocol::F32,
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        }
    }
}

impl DecodeSettings {
    /// Protocol fields to append to the per-item checkpoint inputs key.
    ///
    /// For [`DecodeProtocol::F32`], this returns empty (preserving all existing
    /// checkpoint key hashes byte-for-byte). For [`DecodeProtocol::Bf16`],
    /// it returns `[("decode_protocol", "bf16"), ("bf16_gemm_rows", N)]` in that order.
    pub fn checkpoint_protocol_fields(&self) -> Vec<(&'static str, serde_json::Value)> {
        match self.protocol {
            DecodeProtocol::F32 => Vec::new(),
            DecodeProtocol::Bf16 { gemm_rows } => vec![
                (
                    "decode_protocol",
                    serde_json::Value::String("bf16".to_string()),
                ),
                (
                    "bf16_gemm_rows",
                    serde_json::Value::Number(serde_json::Number::from(gemm_rows)),
                ),
            ],
        }
    }

    /// Serialise the decode settings for inclusion in collateral reports.
    ///
    /// Keys appear in fixed order: `protocol`, `bf16_gemm_rows` (only for bf16),
    /// `max_tokens`, `temperature`, `seed`, `thinking`. Batch width is never included.
    pub fn report_object(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert(
            "protocol".to_string(),
            serde_json::Value::String(self.protocol.tag().to_string()),
        );
        if let DecodeProtocol::Bf16 { gemm_rows } = self.protocol {
            map.insert(
                "bf16_gemm_rows".to_string(),
                serde_json::Value::Number(serde_json::Number::from(gemm_rows)),
            );
        }
        map.insert(
            "max_tokens".to_string(),
            serde_json::Value::Number(serde_json::Number::from(self.max_tokens)),
        );
        map.insert(
            "temperature".to_string(),
            serde_json::Number::from_f64(self.temperature)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "seed".to_string(),
            serde_json::Value::Number(serde_json::Number::from(self.seed)),
        );
        map.insert(
            "thinking".to_string(),
            serde_json::Value::Bool(self.thinking),
        );
        serde_json::Value::Object(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_to_f32() {
        assert_eq!(
            DecodeProtocol::parse(None, None).unwrap(),
            DecodeProtocol::F32
        );
        assert_eq!(
            DecodeProtocol::parse(Some("f32"), None).unwrap(),
            DecodeProtocol::F32
        );
    }

    #[test]
    fn parse_bf16_with_positive_gemm_rows() {
        assert_eq!(
            DecodeProtocol::parse(Some("bf16"), Some("32")).unwrap(),
            DecodeProtocol::Bf16 { gemm_rows: 32 }
        );
    }

    #[test]
    fn parse_bf16_missing_or_invalid_gemm_rows_errors() {
        let err_none = DecodeProtocol::parse(Some("bf16"), None).unwrap_err();
        assert!(
            err_none.contains(BF16_GEMM_ROWS_ENV),
            "expected {BF16_GEMM_ROWS_ENV} in: {err_none}"
        );

        let err_zero = DecodeProtocol::parse(Some("bf16"), Some("0")).unwrap_err();
        assert!(
            err_zero.contains(BF16_GEMM_ROWS_ENV),
            "expected {BF16_GEMM_ROWS_ENV} in: {err_zero}"
        );

        let err_bad = DecodeProtocol::parse(Some("bf16"), Some("x")).unwrap_err();
        assert!(
            err_bad.contains(BF16_GEMM_ROWS_ENV),
            "expected {BF16_GEMM_ROWS_ENV} in: {err_bad}"
        );
    }

    #[test]
    fn parse_f32_with_gemm_rows_errors() {
        let err = DecodeProtocol::parse(Some("f32"), Some("32")).unwrap_err();
        assert!(
            err.contains("row count only applies to 'bf16'")
                || err.contains("only applies to 'bf16'"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_unknown_tag_errors_naming_f32_and_bf16() {
        let err_empty = DecodeProtocol::parse(Some(""), None).unwrap_err();
        assert!(
            err_empty.contains("f32") && err_empty.contains("bf16"),
            "got: {err_empty}"
        );

        let err_fp16 = DecodeProtocol::parse(Some("fp16"), None).unwrap_err();
        assert!(
            err_fp16.contains("f32") && err_fp16.contains("bf16"),
            "got: {err_fp16}"
        );
    }

    #[test]
    fn checkpoint_protocol_fields_empty_for_f32_and_present_for_bf16() {
        let f32_settings = DecodeSettings {
            protocol: DecodeProtocol::F32,
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        assert!(f32_settings.checkpoint_protocol_fields().is_empty());

        let bf16_settings = DecodeSettings {
            protocol: DecodeProtocol::Bf16 { gemm_rows: 32 },
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let fields = bf16_settings.checkpoint_protocol_fields();
        assert_eq!(fields.len(), 2);
        assert_eq!(
            fields[0],
            (
                "decode_protocol",
                serde_json::Value::String("bf16".to_string())
            )
        );
        assert_eq!(
            fields[1],
            (
                "bf16_gemm_rows",
                serde_json::Value::Number(serde_json::Number::from(32))
            )
        );
    }

    #[test]
    fn report_object_exact_serialization() {
        let f32_settings = DecodeSettings {
            protocol: DecodeProtocol::F32,
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let f32_obj = f32_settings.report_object();
        let serialized = serde_json::to_string(&f32_obj).unwrap();
        assert_eq!(
            serialized,
            r#"{"protocol":"f32","max_tokens":2048,"temperature":0.0,"seed":0,"thinking":true}"#
        );
        assert!(!serialized.contains("batch"));

        let bf16_settings = DecodeSettings {
            protocol: DecodeProtocol::Bf16 { gemm_rows: 32 },
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let bf16_obj = bf16_settings.report_object();
        let bf16_serialized = serde_json::to_string(&bf16_obj).unwrap();
        assert_eq!(
            bf16_serialized,
            r#"{"protocol":"bf16","bf16_gemm_rows":32,"max_tokens":2048,"temperature":0.0,"seed":0,"thinking":true}"#
        );
        assert!(!bf16_serialized.contains("batch"));
    }

    #[test]
    fn from_report_round_trips_and_handles_absent() {
        assert_eq!(
            DecodeProtocol::from_report(None).unwrap(),
            DecodeProtocol::F32
        );
        assert_eq!(
            DecodeProtocol::from_report(Some(&serde_json::Value::Null)).unwrap(),
            DecodeProtocol::F32
        );

        let f32_settings = DecodeSettings {
            protocol: DecodeProtocol::F32,
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let f32_obj = f32_settings.report_object();
        assert_eq!(
            DecodeProtocol::from_report(Some(&f32_obj)).unwrap(),
            DecodeProtocol::F32
        );

        let bf16_settings = DecodeSettings {
            protocol: DecodeProtocol::Bf16 { gemm_rows: 32 },
            max_tokens: 2048,
            temperature: 0.0,
            seed: 0,
            thinking: true,
        };
        let bf16_obj = bf16_settings.report_object();
        assert_eq!(
            DecodeProtocol::from_report(Some(&bf16_obj)).unwrap(),
            DecodeProtocol::Bf16 { gemm_rows: 32 }
        );

        let unknown_obj = serde_json::json!({
            "protocol": "fp16",
            "max_tokens": 2048,
        });
        let err = DecodeProtocol::from_report(Some(&unknown_obj)).unwrap_err();
        assert!(err.contains("f32") && err.contains("bf16"), "got: {err}");
    }
}
