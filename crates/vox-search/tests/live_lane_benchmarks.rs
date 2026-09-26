// crates/vox-search/tests/live_lane_benchmarks.rs
use std::collections::HashSet;
use std::time::Instant;
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::{WebSearchDispatcher, WebSearchDispatcherExt};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "owner:search sunset:never live network benchmark requiring external search engine availability"]
async fn test_live_lane_comparative_benchmarks() {
    let queries = [
        "quantum computing fault tolerance surface code",
        "retrieval augmented generation reciprocal rank fusion",
    ];

    let mut policy = SearchPolicy::default();
    policy.wikipedia_fallback_enabled = true;
    policy.enable_openalex = true;
    policy.enable_arxiv = true;
    policy.fast_timeout_ms = 2500;
    policy.deep_timeout_ms = 15000;

    println!("\n=== LIVE LANE COMPARATIVE BENCHMARK SCOREBOARD ===");
    println!("| Query | Lane | Latency (ms) | Hits | Unique Domains | Engines |");
    println!("|---|---|---|---|---|---|");

    let dispatcher = WebSearchDispatcher::new();

    for query in queries {
        for lane in [ResearchLane::Fast, ResearchLane::Deep] {
            let start = Instant::now();
            let result = dispatcher.search_with_lane(query, lane, &policy).await;
            let elapsed_ms = start.elapsed().as_millis();

            match result {
                Ok(hits) => {
                    let mut domains = HashSet::new();
                    let mut engines = HashSet::new();
                    for hit in &hits {
                        for p in &hit.provenance {
                            if p.starts_with("engine:") {
                                engines.insert(p.replace("engine:", ""));
                            }
                        }
                        if let Ok(url) = url::Url::parse(&hit.path) {
                            if let Some(host) = url.host_str() {
                                domains.insert(host.to_string());
                            }
                        }
                    }

                    let engine_str = if engines.is_empty() {
                        "none".to_string()
                    } else {
                        engines.into_iter().collect::<Vec<_>>().join("+")
                    };

                    println!(
                        "| {:<35} | {:<5?} | {:<12} | {:<4} | {:<14} | {:<7} |",
                        query,
                        lane,
                        elapsed_ms,
                        hits.len(),
                        domains.len(),
                        engine_str
                    );
                }
                Err(err) => {
                    println!(
                        "| {:<35} | {:<5?} | {:<12} | ERR  | 0              | {} |",
                        query, lane, elapsed_ms, err
                    );
                }
            }
        }
    }
    println!("===================================================\n");
}
