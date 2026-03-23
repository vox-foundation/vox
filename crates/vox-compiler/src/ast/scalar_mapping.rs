//! Vox **surface scalar** types (`int`, `float`, `str`, `bool`) and their lowering targets.
//!
//! Rust ([`vox-codegen-rust`]), TypeScript ([`vox-codegen-ts`]), and SQLite column affinities
//! must stay aligned; extend mappings here when adding scalars or target-specific overrides.

/// Built-in scalar type names as they appear in Vox source / HIR `Named` types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoxScalar {
    /// 64-bit integer (`i64` in Rust emit, `INTEGER` in SQLite).
    Int,
    /// IEEE double (`f64`, `REAL`).
    Float,
    /// UTF-8 string (`String`, `TEXT`).
    Str,
    /// Boolean (`bool`, stored as `INTEGER` 0/1 in SQLite).
    Bool,
}

impl VoxScalar {
    /// Parse a Vox scalar name; returns [`None`] for ADTs, `Id`, etc.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "str" => Some(Self::Str),
            "bool" => Some(Self::Bool),
            _ => None,
        }
    }

    /// Rust type used in generated structs, actors, and table rows.
    #[must_use]
    pub fn as_rust_type(self) -> &'static str {
        match self {
            Self::Int => "i64",
            Self::Float => "f64",
            Self::Str => "String",
            Self::Bool => "bool",
        }
    }

    /// TypeScript primitive or keyword for JSON / client stubs (both `int` and `float` → `number`).
    #[must_use]
    pub fn as_ts_primitive(self) -> &'static str {
        match self {
            Self::Int | Self::Float => "number",
            Self::Str => "string",
            Self::Bool => "boolean",
        }
    }

    /// SQLite column affinity for `@table` generation.
    #[must_use]
    pub fn as_sqlite_affinity(self) -> &'static str {
        match self {
            Self::Int => "INTEGER",
            Self::Float => "REAL",
            Self::Bool => "INTEGER",
            Self::Str => "TEXT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_only_known_scalars() {
        assert_eq!(VoxScalar::parse("int"), Some(VoxScalar::Int));
        assert!(VoxScalar::parse("Task").is_none());
    }

    #[test]
    fn rust_ts_sql_consistent_lengths() {
        for s in [
            VoxScalar::Int,
            VoxScalar::Float,
            VoxScalar::Str,
            VoxScalar::Bool,
        ] {
            assert!(!s.as_rust_type().is_empty());
            assert!(!s.as_ts_primitive().is_empty());
            assert!(!s.as_sqlite_affinity().is_empty());
        }
    }
}
