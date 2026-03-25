use subtle::ConstantTimeEq;

pub(super) fn populi_control_token_from_env() -> Option<String> {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshToken)
        .expose()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

/// Constant-time comparison when lengths match (avoids early return on length for the equal-length case).
pub(super) fn bearer_token_eq(expected: &str, presented: &str) -> bool {
    let a = expected.as_bytes();
    let b = presented.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}
