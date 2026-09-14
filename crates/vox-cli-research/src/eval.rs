use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use uuid::Uuid;
use vox_db::{DbConfig, ResearchEvalRunRecord, ResearchEvalSampleRecord, VoxDb, now_unix_ms};
use vox_search::context::SearchRuntimeContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenQueryItem {
    pub query: String,
    #[serde(default)]
    pub gold_answer: Option<String>,
    #[serde(default)]
    pub domain_mode: Option<String>,
    #[serde(default)]
    pub expected_sources: Option<Vec<String>>,
    #[serde(default)]
    pub is_adversarial: Option<bool>,
}

pub fn compute_token_recall(gold_answer: &str, model_answer: &str) -> f64 {
    vox_search::evaluation::calculate_recall_at_5(model_answer, gold_answer)
}

pub async fn evaluate_single_query_pipeline(
    run_id: &str,
    item: &GoldenQueryItem,
    ctx: &SearchRuntimeContext,
    db: Option<&VoxDb>,
    config: &vox_research_shim::research::ResearchConfig,
) -> anyhow::Result<ResearchEvalSampleRecord> {
    let start = std::time::Instant::now();
    let domain_mode = match item
        .domain_mode
        .as_deref()
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("shopping") => vox_research_shim::research::ResearchDomainMode::Shopping,
        Some("codegen") | Some("code_gen") => {
            vox_research_shim::research::ResearchDomainMode::CodeGen
        }
        _ => vox_research_shim::research::ResearchDomainMode::General,
    };
    let query_req = vox_research_shim::research::ResearchQuery {
        query: item.query.clone(),
        scope: vox_research_shim::research::ResearchScope::Both,
        max_sources: 10,
        persist_to_docs: false,
        verify_claims: true,
        site_scope: None,
        domain_mode,
    };

    let result =
        vox_research_shim::research::run_research_with_context(query_req, Some(ctx), db, config)
            .await?;

    let duration = start.elapsed().as_millis() as i64;
    let model_answer = result.answer;
    let evidence_snippets: Vec<String> = result.sources.iter().map(|s| s.snippet.clone()).collect();

    // 1. Calculate groundedness & citation precision from actual report
    let groundedness =
        vox_search::evaluation::calculate_groundedness(&model_answer, &evidence_snippets);
    let (citation_precision, citations_found, citations_supported) =
        if let Some(ref audit) = result.research_metadata.citation_audit {
            (
                audit.precision,
                audit.checked_citations,
                audit.supported_citations,
            )
        } else {
            let prec = citation_precision_from_answer(&model_answer, evidence_snippets.len());
            (prec, result.citations.len(), 0)
        };
    let abstained = answer_abstained(&model_answer, evidence_snippets.is_empty());
    let multi_hop_score = multi_hop_pipeline_score(
        &item.query,
        result.research_metadata.subquery_count,
        result.sources.len(),
    );

    // 2. Calculate recall against gold answer if provided
    let recall = item
        .gold_answer
        .as_ref()
        .map(|gold| compute_token_recall(gold, &model_answer));

    let quality_score = (groundedness + citation_precision + recall.unwrap_or(0.5)) / 3.0;

    let sample = ResearchEvalSampleRecord {
        run_id: run_id.to_string(),
        query: item.query.clone(),
        gold_answer: item.gold_answer.clone(),
        model_answer,
        recall_at_5: recall,
        groundedness: Some(groundedness),
        quality_score: Some(quality_score),
        latency_ms: Some(duration),
        evidence: serde_json::json!({
            "total_claims": result.research_metadata.claim_verdicts.len(),
            "verdicts": result.research_metadata.claim_verdicts.len(),
            "sources_count": result.sources.len(),
            "citation_precision": citation_precision,
            "abstained": abstained,
            "citations_found": citations_found,
            "citations_supported": citations_supported,
            "multi_hop_score": multi_hop_score,
        }),
        recorded_at_ms: now_unix_ms() as i64,
    };

    Ok(sample)
}

pub async fn run_eval(
    queries_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
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
    let queries = if let Some(path) = &queries_path {
        load_queries_file(path)?
    } else {
        default_golden_queries()
    };
    println!("{} Loaded {} queries", "INFO".blue(), queries.len());

    // 3. Execution Loop
    let mut results = Vec::new();
    let current_dir = std::env::current_dir()?;
    let ctx = SearchRuntimeContext::new(
        current_dir.clone(),
        Some(std::sync::Arc::new(db.clone())),
        current_dir.clone(),
        current_dir.join("memory.md"),
    );
    let config = vox_research_shim::research::ResearchConfig::default();

    for item in &queries {
        println!("{} Evaluating: {}", "RUN".green(), item.query);
        match evaluate_single_query_pipeline(&run_id, item, &ctx, Some(&db), &config).await {
            Ok(sample) => results.push(sample),
            Err(e) => {
                eprintln!(
                    "{} Failed evaluating {}: {}",
                    "WARN".yellow(),
                    item.query,
                    e
                );
            }
        }
    }

    if results.is_empty() {
        anyhow::bail!("no queries were successfully evaluated");
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
    let avg_citation_precision = average_evidence_metric(&results, "citation_precision");
    let abstention_rate = average_bool_metric(&results, "abstained");
    let avg_multi_hop_score = average_evidence_metric(&results, "multi_hop_score");

    let run_record = ResearchEvalRunRecord {
        run_id,
        model_id: "vox-deep-research-pipeline".into(),
        config: serde_json::json!({ "policy_version": 2 }),
        metrics: serde_json::json!({
            "avg_quality": avg_quality,
            "avg_latency_ms": avg_latency,
            "total_samples": results.len(),
            "citation_precision": avg_citation_precision,
            "abstention_rate": abstention_rate,
            "multi_hop_completion": avg_multi_hop_score,
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

    let report = serde_json::json!({
        "schema_version": 1,
        "run_id": run_record.run_id,
        "suite": if queries_path.is_some() { "custom" } else { "local" },
        "total_samples": results.len(),
        "metrics": {
            "citation_precision": avg_citation_precision,
            "citation_recall": results.iter().map(|s| s.recall_at_5.unwrap_or(0.0)).sum::<f64>() / results.len() as f64,
            "answer_factuality": avg_quality,
            "abstention_rate": abstention_rate,
            "multi_hop_completion": avg_multi_hop_score,
            "avg_latency_ms": avg_latency
        }
    });

    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
    }

    print_styled_summary(&run_record, avg_quality);

    Ok(())
}

fn citation_precision_from_answer(answer: &str, evidence_count: usize) -> f64 {
    if evidence_count == 0 {
        return if answer_abstained(answer, true) {
            1.0
        } else {
            0.0
        };
    }
    let mut cited = 0usize;
    let mut supported = 0usize;
    for token in answer.split(|c: char| c.is_whitespace() || c == ',' || c == '.') {
        let Some(inner) = token.strip_prefix('[').and_then(|t| t.strip_suffix(']')) else {
            continue;
        };
        if let Ok(n) = inner.parse::<usize>() {
            cited += 1;
            if (1..=evidence_count).contains(&n) {
                supported += 1;
            }
        }
    }
    if cited == 0 {
        0.0
    } else {
        supported as f64 / cited as f64
    }
}

fn answer_abstained(answer: &str, no_evidence: bool) -> bool {
    let lower = answer.to_ascii_lowercase();
    no_evidence
        || lower.contains("insufficient evidence")
        || lower.contains("not enough evidence")
        || lower.contains("no external sources were found")
}

fn multi_hop_pipeline_score(query: &str, subquery_count: usize, sources_count: usize) -> f64 {
    let multi_hop_query = query.contains("compare")
        || query.contains("trace")
        || query.contains("then")
        || query.contains("FRAMES-style")
        || query.contains("BrowseComp-style");
    if !multi_hop_query {
        return 1.0;
    }
    let subquery_score = (subquery_count as f64 / 2.0).min(1.0);
    let source_score = (sources_count as f64 / 3.0).min(1.0);
    ((subquery_score + source_score) / 2.0).clamp(0.0, 1.0)
}

#[cfg(test)]
fn multi_hop_completion_score(
    query: &str,
    backend_mix: &[vox_db::SearchBackend],
    web_lines: &[String],
) -> f64 {
    let multi_hop_query = query.contains("compare")
        || query.contains("trace")
        || query.contains("then")
        || query.contains("FRAMES-style")
        || query.contains("BrowseComp-style");
    if !multi_hop_query {
        return 1.0;
    }
    let backend_score = (backend_mix.len() as f64 / 2.0).min(1.0);
    let source_score = (web_lines.len() as f64 / 3.0).min(1.0);
    ((backend_score + source_score) / 2.0).clamp(0.0, 1.0)
}

fn average_evidence_metric(samples: &[ResearchEvalSampleRecord], key: &str) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples
        .iter()
        .map(|sample| {
            sample
                .evidence
                .get(key)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        })
        .sum::<f64>()
        / samples.len() as f64
}

fn average_bool_metric(samples: &[ResearchEvalSampleRecord], key: &str) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples
        .iter()
        .filter(|sample| {
            sample
                .evidence
                .get(key)
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .count() as f64
        / samples.len() as f64
}

fn load_queries_file(path: &Path) -> anyhow::Result<Vec<GoldenQueryItem>> {
    let content = std::fs::read_to_string(path)?;
    if let Ok(items) = serde_json::from_str::<Vec<GoldenQueryItem>>(&content) {
        return Ok(items);
    }
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Ok(item) = serde_json::from_str::<GoldenQueryItem>(trimmed) {
            items.push(item);
        } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(q) = val.get("query").and_then(|v| v.as_str()) {
                items.push(GoldenQueryItem {
                    query: q.to_string(),
                    gold_answer: val
                        .get("gold_answer")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    domain_mode: val
                        .get("domain_mode")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string),
                    expected_sources: val
                        .get("expected_sources")
                        .and_then(|v| serde_json::from_value(v.clone()).ok()),
                    is_adversarial: val.get("is_adversarial").and_then(|v| v.as_bool()),
                });
            }
        } else {
            items.push(GoldenQueryItem {
                query: trimmed.to_string(),
                gold_answer: None,
                domain_mode: None,
                expected_sources: None,
                is_adversarial: None,
            });
        }
    }
    Ok(items)
}

fn default_golden_queries() -> Vec<GoldenQueryItem> {
    let queries = [
        "What are the latest developments in Rust 2024 edition?",
        "How do I configure SearXNG for private JSON output?",
        "What is the current price of Ethereum in USD?",
        "Vox Dei orchestrator architecture overview",
        // Deep-research-style multi-hop prompts (evidence spread across sources)
        "Compare MLX vs CUDA for on-device LLM fine-tuning: hardware requirements, tooling, and community adoption in 2025.",
        "Trace the lineage from ReAct agents to modern web-browsing research assistants; name key papers and vendor products.",
        "Summarize EU AI Act transparency obligations for general-purpose AI models and cite primary regulator sources.",
        "Where does the Vox research pipeline create Codex sessions?",
        "Which Vox module converts local search execution rows into research hits?",
        "How does Vox choose local Mens versus OpenRouter for research LLM calls?",
        "What contract file declares MCP research tools?",
        "Which CLI command shows persisted research sessions?",
        "What DB table stores Scientia research sessions?",
        "Which verifier verdicts are supported by Vox research?",
        "What is the fallback behavior when claim extraction has no LLM?",
        "Which search context fields are required for local retrieval?",
        "How are research cache keys normalized?",
        "FRAMES-style: identify the component that plans subqueries, then name the metric recorded after planning.",
        "FRAMES-style: find the local retrieval bridge and describe how repo hits are cited.",
        "FRAMES-style: compare synchronous research run with async session status reporting.",
        "BrowseComp-style: find primary documentation for OpenRouter chat completions and summarize endpoint shape.",
        "BrowseComp-style: find current Ollama OpenAI compatibility notes and cite the local endpoint.",
        "BrowseComp-style: find Tavily search API result fields relevant to citations.",
        "BrowseComp-style: find SearXNG JSON output configuration guidance.",
        "BrowseComp-style: find CRAG prior art and identify its correction trigger.",
        "BrowseComp-style: find citation precision evaluation approaches for web QA.",
        "BrowseComp-style: find Gemini Deep Research public product behavior and compare async expectations.",
        "BrowseComp-style: find OpenClaw research assistant claims and cite product docs.",
        "BrowseComp-style: find MiniCheck claim verification model details.",
        "BrowseComp-style: find CoVE self-verification paper and summarize its loop.",
    ];
    queries
        .iter()
        .map(|q| GoldenQueryItem {
            query: (*q).to_string(),
            gold_answer: None,
            domain_mode: None,
            expected_sources: None,
            is_adversarial: None,
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_precision_counts_only_existing_sources() {
        let precision = citation_precision_from_answer("Claim [1] and unsupported [4].", 2);
        assert_eq!(precision, 0.5);
    }

    #[test]
    fn abstention_counts_empty_evidence_as_valid_abstention() {
        assert!(answer_abstained("No external sources were found.", true));
        assert!(!answer_abstained("The answer is well-supported.", false));
    }

    #[test]
    fn multi_hop_queries_require_backend_and_source_coverage() {
        let score = multi_hop_completion_score(
            "FRAMES-style: find one thing then compare another",
            &[
                vox_db::SearchBackend::MemoryBm25,
                vox_db::SearchBackend::Web,
            ],
            &[
                "source 1".to_string(),
                "source 2".to_string(),
                "source 3".to_string(),
            ],
        );

        assert_eq!(score, 1.0);
    }

    #[test]
    fn token_recall_computes_overlap() {
        let recall = compute_token_recall(
            "Rust memory safety ownership borrowing",
            "Rust memory safety is guaranteed by ownership.",
        );
        assert!(recall > 0.0);
    }
}
