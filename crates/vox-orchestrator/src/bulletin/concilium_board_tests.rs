use super::*;

#[tokio::test]
async fn channel_mention_wakes_only_workers_parked_on_that_channel() {
    let board = BulletinBoard::new(16);
    let d_ops = board.doorbell_for("ops-agent");
    let d_parser = board.doorbell_for("parser-agent");
    board.park_worker("ops-agent", "ops-agent", Some("#ops"));
    board.park_worker("parser-agent", "parser-agent", Some("#parser"));

    let _ = board.post_board_entry(
        "#ops",
        "watchdog",
        "checking @#ops",
        ComputeTier::InlineCpu,
        SwarmWorkspaceMode::Inherit,
        None,
        None,
        None,
        true,
    );

    tokio::time::timeout(vox_config::timeouts::D_100MS, d_ops.notified())
        .await
        .expect("#ops worker should wake");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), d_parser.notified())
            .await
            .is_err(),
        "@#ops mention must NOT wake worker parked on #parser"
    );
    let snap = board.concilium_snapshot();
    assert_eq!(snap.active_workers, 1);
    assert_eq!(snap.parked_workers, 1);
}

#[test]
fn read_board_without_since_seq_returns_latest_tail_ascending() {
    let board = BulletinBoard::new(16);
    for i in 1..=65 {
        let _ = board.post_board_entry(
            "#main",
            "author",
            &format!("msg {i}"),
            ComputeTier::InlineCpu,
            SwarmWorkspaceMode::Inherit,
            None,
            None,
            None,
            false,
        );
    }
    let tail = board.read_board(&SequenceBoardFilter::default());
    assert_eq!(tail.len(), 50);
    assert_eq!(tail.first().expect("first").seq, 16);
    assert_eq!(tail.last().expect("last").seq, 65);

    let paged = board.read_board(&SequenceBoardFilter {
        since_seq: Some(10),
        limit: Some(5),
        ..Default::default()
    });
    assert_eq!(
        paged.iter().map(|e| e.seq).collect::<Vec<_>>(),
        vec![11, 12, 13, 14, 15]
    );
}
