//! Ensures Ludus cutover creates extended `gamify_*` tables on a fresh baseline DB.

use vox_db::VoxDb;

#[tokio::test]
async fn ludus_cutover_creates_policy_snapshots_and_dedupe() {
    let db = VoxDb::open_memory().await.expect("open");
    let mut rows = db
        .connection()
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('gamify_policy_snapshots', 'gamify_processed_events', 'gamify_hint_telemetry', 'gamify_collegium') ORDER BY name",
            (),
        )
        .await
        .expect("q");
    let mut names = Vec::new();
    while let Some(row) = rows.next().await.expect("next") {
        names.push(row.get::<String>(0).expect("name"));
    }
    assert_eq!(
        names,
        vec![
            "gamify_collegium",
            "gamify_hint_telemetry",
            "gamify_policy_snapshots",
            "gamify_processed_events",
        ]
    );
}
