use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use vox_search::parallel_exploration::{
    BranchResult, CancellationToken, ParallelExplorationCoordinator, ResearchBranch,
};

#[tokio::test]
async fn test_parallel_exploration_resilience_and_timeout() {
    let branches = vec![
        ResearchBranch {
            branch_id: "b1".to_string(),
            query: "fast".to_string(),
        },
        ResearchBranch {
            branch_id: "b2".to_string(),
            query: "slow".to_string(),
        },
    ];

    let results = ParallelExplorationCoordinator::execute_branches_resilient(
        branches,
        2,
        Duration::from_millis(50),
        None,
        |q| async move {
            if q == "slow" {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Ok(vec![format!("Result for {q}")])
        },
    )
    .await;

    assert_eq!(results.len(), 2);
    let fast_res = results.iter().find(|r| r.branch_id == "b1").unwrap();
    let slow_res = results.iter().find(|r| r.branch_id == "b2").unwrap();

    assert!(fast_res.success);
    assert_eq!(fast_res.snippets, vec!["Result for fast"]);
    assert!(!slow_res.success); // Timed out gracefully
    assert!(slow_res.snippets.is_empty());
}

#[tokio::test]
async fn test_parallel_exploration_cancellation() {
    let branches = vec![
        ResearchBranch {
            branch_id: "b1".to_string(),
            query: "branch1".to_string(),
        },
        ResearchBranch {
            branch_id: "b2".to_string(),
            query: "branch2".to_string(),
        },
    ];

    let token = CancellationToken::new();
    token.cancel();

    let executed_count = Arc::new(AtomicUsize::new(0));
    let executed_clone = executed_count.clone();

    let results: Vec<BranchResult> = ParallelExplorationCoordinator::execute_branches_resilient(
        branches,
        2,
        Duration::from_secs(1),
        Some(token),
        move |q| {
            let executed = executed_clone.clone();
            async move {
                executed.fetch_add(1, Ordering::SeqCst);
                Ok(vec![format!("Result for {q}")])
            }
        },
    )
    .await;

    assert_eq!(results.len(), 2);
    for res in &results {
        assert!(!res.success);
        assert!(res.snippets.is_empty());
    }
    // Branches should have exited immediately without calling the fetcher
    assert_eq!(executed_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_parallel_exploration_error_handling() {
    let branches = vec![
        ResearchBranch {
            branch_id: "b_ok".to_string(),
            query: "ok".to_string(),
        },
        ResearchBranch {
            branch_id: "b_err".to_string(),
            query: "err".to_string(),
        },
    ];

    let results = ParallelExplorationCoordinator::execute_branches_resilient(
        branches,
        2,
        Duration::from_secs(1),
        None,
        |q| async move {
            if q == "err" {
                anyhow::bail!("fetch failure");
            }
            Ok(vec![format!("Success for {q}")])
        },
    )
    .await;

    assert_eq!(results.len(), 2);
    let ok_res = results.iter().find(|r| r.branch_id == "b_ok").unwrap();
    let err_res = results.iter().find(|r| r.branch_id == "b_err").unwrap();

    assert!(ok_res.success);
    assert_eq!(ok_res.snippets, vec!["Success for ok"]);

    assert!(!err_res.success);
    assert!(err_res.snippets.is_empty());
}
