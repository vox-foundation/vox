use std::sync::Arc;
use vox_search::tavily_budget::TavilySessionBudget;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_budget_spend_and_remaining_calculation() {
    let budget = TavilySessionBudget::new(1000);
    assert!(budget.try_consume(200));
    let (used, remaining) = budget.usage_and_remaining();
    assert_eq!(used, 200);
    assert_eq!(remaining, 800);
}

#[test]
fn test_budget_exhaustion_rejects_consumption() {
    let budget = TavilySessionBudget::new(10);
    assert!(budget.try_consume(10));
    assert!(
        !budget.try_consume(1),
        "Exhausted budget must reject consumption"
    );
    let (used, remaining) = budget.usage_and_remaining();
    assert_eq!(used, 10);
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn test_upstream_usage_sync_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/usage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "monthly_limit": 1000,
            "monthly_usage": 350
        })))
        .mount(&server)
        .await;

    let budget = TavilySessionBudget::new(1000);
    let (used, remaining) = budget
        .sync_with_upstream("dummy_key", Some(&server.uri()))
        .await
        .expect("sync");
    assert_eq!(used, 350);
    assert_eq!(remaining, 650);
}

#[tokio::test]
async fn test_budget_spend_persists_to_db() {
    let db = Arc::new(
        vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .expect("db"),
    );
    vox_search::tavily_budget::set_budget_db(db.clone());

    let budget = TavilySessionBudget::new(1000);
    assert!(budget.try_consume(150));

    // Yield to let async spawn execute
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let period = vox_search::tavily_budget::period_key();
    let record = db
        .get_quota_usage("tavily", &period)
        .await
        .expect("get quota")
        .expect("found record");
    assert_eq!(record.units_spent, 150);
    assert_eq!(record.units_limit, 1000);
}
