use crate::{ConflictId, OperationId, SnapshotId};

pub(super) fn _parse_snapshot_id_value(v: Option<&serde_json::Value>) -> Option<SnapshotId> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(SnapshotId(n));
    }
    let s = v.as_str()?;
    let raw = s.strip_prefix("S-").unwrap_or(s);
    raw.parse::<u64>().ok().map(SnapshotId)
}

pub(super) fn parse_operation_id_value(v: Option<&serde_json::Value>) -> Option<OperationId> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(OperationId(n));
    }
    let s = v.as_str()?;
    let raw = s.strip_prefix("OP-").unwrap_or(s);
    raw.parse::<u64>().ok().map(OperationId)
}

pub(super) fn parse_conflict_id_value(v: Option<&serde_json::Value>) -> Option<ConflictId> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(ConflictId(n));
    }
    let s = v.as_str()?;
    let raw = s.strip_prefix("C-").unwrap_or(s);
    raw.parse::<u64>().ok().map(ConflictId)
}

#[cfg(test)]
mod id_parse_tests {
    use super::{_parse_snapshot_id_value, parse_conflict_id_value, parse_operation_id_value};
    use crate::{ConflictId, OperationId, SnapshotId};
    use serde_json::json;

    #[test]
    fn snapshot_id_accepts_numeric_and_s_prefix() {
        assert_eq!(
            _parse_snapshot_id_value(Some(&json!(3))),
            Some(SnapshotId(3))
        );
        assert_eq!(
            _parse_snapshot_id_value(Some(&json!("S-000003"))),
            Some(SnapshotId(3))
        );
        assert_eq!(
            _parse_snapshot_id_value(Some(&json!("3"))),
            Some(SnapshotId(3))
        );
    }

    #[test]
    fn operation_id_accepts_numeric_and_op_prefix() {
        assert_eq!(
            parse_operation_id_value(Some(&json!(7))),
            Some(OperationId(7))
        );
        assert_eq!(
            parse_operation_id_value(Some(&json!("OP-000007"))),
            Some(OperationId(7))
        );
    }

    #[test]
    fn conflict_id_accepts_numeric_and_c_prefix() {
        assert_eq!(
            parse_conflict_id_value(Some(&json!(9))),
            Some(ConflictId(9))
        );
        assert_eq!(
            parse_conflict_id_value(Some(&json!("C-000009"))),
            Some(ConflictId(9))
        );
    }
}
