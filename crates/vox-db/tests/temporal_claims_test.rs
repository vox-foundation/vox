use vox_db::temporal_claims::{is_claim_valid_at_version, normalize_semver};

#[test]
fn test_normalize_semver_loose_formats() {
    assert_eq!(normalize_semver("v1.25.0").unwrap().to_string(), "1.25.0");
    assert_eq!(normalize_semver("1.0").unwrap().to_string(), "1.0.0");
    assert_eq!(normalize_semver("2").unwrap().to_string(), "2.0.0");
    assert_eq!(normalize_semver("= 1.2.3").unwrap().to_string(), "1.2.3");
    assert_eq!(
        normalize_semver("v2.1.0-alpha.1").unwrap().to_string(),
        "2.1.0-alpha.1"
    );
    assert!(normalize_semver("invalid-semver").is_none());
}

#[test]
fn test_is_claim_valid_at_version_interval_semantics() {
    assert!(is_claim_valid_at_version(Some("1.0.0"), None, None, "v1.25.0").unwrap());
    assert!(!is_claim_valid_at_version(Some("1.0.0"), None, None, "0.2.22").unwrap());

    // Half-open interval [0.2.0, 1.0.0)
    assert!(is_claim_valid_at_version(Some("0.2.0"), Some("1.0.0"), None, "0.9.0").unwrap());
    assert!(!is_claim_valid_at_version(Some("0.2.0"), Some("1.0.0"), None, "1.0.0").unwrap());
    assert!(!is_claim_valid_at_version(Some("0.2.0"), Some("1.0.0"), None, "0.1.9").unwrap());

    // Version requirement match
    assert!(is_claim_valid_at_version(None, None, Some("^1.2"), "1.2.5").unwrap());
    assert!(!is_claim_valid_at_version(None, None, Some("^1.2"), "2.0.0").unwrap());

    // Invalid target version returns Err
    assert!(is_claim_valid_at_version(None, None, None, "not-a-version").is_err());
}
