use vox_search::corroboration::{CorroboratingHit, count_corroboration};

#[test]
fn test_uncorroborated_single_domain_does_not_inflate_to_two() {
    let hits = vec![
        CorroboratingHit {
            url: "https://single-source.com/page1".into(),
            supports_claim: true,
        },
        CorroboratingHit {
            url: "https://single-source.com/page2".into(),
            supports_claim: true,
        },
    ];
    let count = count_corroboration("claim-1", &hits).count();
    assert_eq!(
        count, 1,
        "multiple pages on same domain must equal 1 corroboration"
    );
}
