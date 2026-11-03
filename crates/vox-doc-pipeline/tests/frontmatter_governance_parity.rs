use std::fs;
use std::path::Path;

#[test]
fn test_frontmatter_governance_parity() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let gov_path = repo_root.join("docs/src/contributors/documentation-governance.md");
    assert!(
        gov_path.exists(),
        "documentation-governance.md must exist at {}",
        gov_path.display()
    );

    let content = fs::read_to_string(&gov_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", gov_path.display(), e));

    // 1. Verify code_anchors, owner, review_cadence are documented
    assert!(
        content.contains("code_anchors"),
        "documentation-governance.md must document `code_anchors`"
    );
    assert!(
        content.contains("owner"),
        "documentation-governance.md must document `owner`"
    );
    assert!(
        content.contains("review_cadence"),
        "documentation-governance.md must document `review_cadence`"
    );

    // 2. Verify forbidden keys are documented
    assert!(
        content.contains("last_updated"),
        "documentation-governance.md must mention forbidden `last_updated`"
    );
    assert!(
        content.contains("last_verified"),
        "documentation-governance.md must mention forbidden `last_verified`"
    );
    assert!(
        content.contains("last_verified_commit"),
        "documentation-governance.md must mention forbidden `last_verified_commit`"
    );

    // 3. Verify ledger location is referenced
    assert!(
        content.contains("contracts/reports/docs-freshness/ledger.v1.json"),
        "documentation-governance.md must reference the ledger location"
    );
}
