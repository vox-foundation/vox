/// Validate that `s` is a safe SQL identifier (table or column name).
///
/// Rejects empty strings, names longer than 64 bytes, names whose first
/// character is not ASCII alpha or `_`, and names containing characters
/// outside `[A-Za-z0-9_]`.  This prevents SQL identifier injection in
/// dynamic DDL/DML that cannot use bound parameters for identifiers.
pub fn validate_identifier(s: &str) -> Result<&str, &'static str> {
    if s.is_empty() {
        return Err("identifier must not be empty");
    }
    if s.len() > 64 {
        return Err("identifier too long (max 64 bytes)");
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err("identifier must start with ASCII letter or underscore");
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("identifier contains invalid characters (only [A-Za-z0-9_] allowed)");
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_names() {
        for ok in &["users", "_private", "col_1", "A", "table_name_64_chars_123456789012345678901234"] {
            assert!(validate_identifier(ok).is_ok(), "should accept: {ok}");
        }
    }

    #[test]
    fn rejects_injection_vectors() {
        for bad in &[
            "",
            "drop table users",
            "col; DROP TABLE users",
            "1bad",
            "col--comment",
            "col`injection`",
            "a".repeat(65).as_str(),
        ] {
            assert!(validate_identifier(bad).is_err(), "should reject: {bad}");
        }
    }
}
