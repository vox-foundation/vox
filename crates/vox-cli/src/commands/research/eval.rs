use owo_colors::OwoColorize;
use std::path::PathBuf;

use uuid::Uuid;
use vox_db::{DbConfig, ResearchEvalRunRecord, ResearchEvalSampleRecord, VoxDb, now_unix_ms};
use vox_search::context::SearchRuntimeContext;
use vox_search::policy::SearchPolicy;

pub async fn run_eval(
    queries_path: Option<PathBuf>,
    _output_path: Option<PathBuf>,
    _concurrency: usize,
) -> anyhow::Result<()> {
    println!(
        "{} Initializing Research Evaluation Harness...",
        "INIT".blue().bold()
    );

    // 1. Establish database connection
    let db = VoxDb::connect(DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?).await?;
    let run_id = Uuid::new_v4().to_string();
    let start_at = now_unix_ms();

    // 2. Load Queries
    let queries = if let Some(path) = queries_path {
        load_queries_file(&path)?
    } else {
        default_golden_queries()
    };
    println!("{} Loaded {} queries", "INFO".blue(), queries.len());

    // 3. Execution Loop
    let mut results = Vec::new();
    let policy = SearchPolicy::default();
    let current_dir = std::env::current_dir()?;
    let ctx = SearchRuntimeContext::new(
        current_dir.clone(),
        Some(std::sync::Arc::new(db.clone())),
        current_dir.clone(),
        current_dir.join("memory.md"),
    );

    for query in &queries {
        println!("{} Evaluating: {}", "RUN".green(), query);
        let start = std::time::Instant::now();

        let plan = vox_db::heuristic_search_plan(query, false, None);
        let execution =
            vox_search::execution::execute_search_plan(&ctx, query, &plan, 5, &policy, None)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

        let duration = start.elapsed().as_millis() as i64;

        let model_answer = execution.web_lines.join("\n");
        let evidence_snippets = execution.web_lines.clone();

        // 3.1 Calculate Metrics
        let recall = Some(0.0); // No gold answer provided in default loop
        let groundedness =
            vox_search::evaluation::calculate_groundedness(&model_answer, &evidence_snippets);

        let sample = ResearchEvalSampleRecord {
            run_id: run_id.clone(),
            query: query.clone(),
            gold_answer: None,
            model_answer,
            recall_at_5: recall,
            groundedness: Some(groundedness),
            quality_score: Some(execution.evidence_quality),
            latency_ms: Some(duration),
            evidence: serde_json::to_value(&execution.web_lines)?,
            recorded_at_ms: now_unix_ms() as i64,
        };

        results.push(sample);
    }

    // 4. Summarize and Persist Run
    let avg_quality = results
        .iter()
        .map(|s| s.quality_score.unwrap_or(0.0))
        .sum::<f64>()
        / results.len() as f64;
    let avg_latency = results
        .iter()
        .map(|s| s.latency_ms.unwrap_or(0))
        .sum::<i64>() as f64
        / results.len() as f64;

    let run_record = ResearchEvalRunRecord {
        run_id,
        model_id: "localized-dispatcher-0.1".into(),
        config: serde_json::json!({ "policy_version": 1 }),
        metrics: serde_json::json!({
            "avg_quality": avg_quality,
            "avg_latency_ms": avg_latency,
            "total_samples": results.len(),
        }),
        latency_p50_ms: Some(avg_latency as i64),
        latency_p99_ms: None,
        tier_distribution: serde_json::json!({}),
        created_at_ms: start_at as i64,
    };

    db.record_research_eval_run(&run_record).await?;

    for sample in &results {
        db.record_research_eval_sample(sample).await?;
    }

    print_styled_summary(&run_record, avg_quality);

    Ok(())
}

fn load_queries_file(path: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let content = std::fs::read_to_string(path)?;
    let mut queries = Vec::new();
    for line in content.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(q) = json.get("query").and_then(|v| v.as_str()) {
                queries.push(q.to_string());
            }
        }
    }
    Ok(queries)
}

fn default_golden_queries() -> Vec<String> {
    vec![
        "What are the latest developments in Rust 2024 edition?".into(),
        "How do I configure SearXNG for private JSON output?".into(),
        "What is the current price of Ethereum in USD?".into(),
        "Vox Dei orchestrator architecture overview".into(),
    ]
}

fn print_styled_summary(run_record: &ResearchEvalRunRecord, avg_quality: f64) {
    println!(
        "\n{}",
        " RESEARCH EVALUATION COMPLETE ".on_blue().white().bold()
    );
    println!(
        "{:<15} {}",
        "Run ID:".dimmed(),
        run_record.run_id.bright_white()
    );
    println!(
        "{:<15} {}",
        "Model:".dimmed(),
        run_record.model_id.bright_white()
    );

    let quality_line = format!("{:.2}", avg_quality);
    let styled = if avg_quality > 0.8 {
        quality_line.green().bold().to_string()
    } else if avg_quality > 0.5 {
        quality_line.yellow().bold().to_string()
    } else {
        quality_line.red().bold().to_string()
    };
    println!("{:<15} {}", "Avg Quality:".dimmed(), styled);

    if let Some(lat) = run_record.latency_p50_ms {
        println!(
            "{:<15} {}ms",
            "P50 Latency:".dimmed(),
            lat.to_string().bright_cyan()
        );
    }

    println!("{}", "─".repeat(40).dimmed());
}
