use super::helpers::summarize_text;

/// Show telemetry metrics for research sessions.
pub async fn research_metrics(session_id: i64, metric_type: Option<&str>) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let sid = session_id.to_string();
    let mt = metric_type.unwrap_or("");
    let metrics = db
        .list_research_metrics_by_type(mt, &sid, 500)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if metrics.is_empty() {
        println!("(no research metrics for session {session_id})");
    } else {
        println!("Research metrics (session {session_id})");
        for (mtype, value, meta) in metrics {
            print!("  - {mtype}: {value}");
            if let Some(m) = meta {
                print!("  metadata: {m}");
            }
            println!();
        }
    }
    Ok(())
}

/// List reliability scores for LLM endpoints, skills, workflows, or repositories.
pub async fn reliability_list(domain: &str, limit: i64) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;

    println!("Reliability stats for: {}", domain);

    match domain {
        "endpoints" => {
            let headers = vec![
                "Endpoint",
                "Model",
                "Reqs",
                "Hallucina",
                "Contradic",
                "InfraFail",
            ];
            let entries = db
                .list_endpoint_reliability(limit)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            for h in &headers {
                print!("{:<24} ", h);
            }
            println!();
            let mut count = 0;
            for e in entries {
                print!(
                    "{:<24} ",
                    summarize_text(&e.endpoint_url, 22)
                );
                print!("{:<24} ", summarize_text(&e.model_id, 22));
                print!("{:<24} ", e.total_requests);
                print!("{:<24.4} ", e.hallucination_proxy_ewma);
                print!("{:<24.4} ", e.contradiction_ratio_ewma);
                print!("{:<24.4} ", e.infra_failure_ewma);
                println!();
                count += 1;
            }
            if count == 0 {
                println!("(no records found)");
            }
        }
        "skills" => {
            let headers = vec!["Skill ID", "Reliability", "Success", "Failure"];
            let rows = db
                .list_skill_reliability_worst_first(limit)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            for h in &headers {
                print!("{:<24} ", h);
            }
            println!();
            let mut count = 0;
            for (id, rel, succ, fail) in rows {
                print!("{:<24} ", summarize_text(&id, 22));
                print!("{:<24.4} ", rel);
                print!("{:<24} ", succ);
                print!("{:<24} ", fail);
                println!();
                count += 1;
            }
            if count == 0 {
                println!("(no records found)");
            }
        }
        "workflows" => {
            let headers = vec!["Workflow", "Reliability", "Success", "Failure"];
            let rows = db
                .list_workflow_reliability_worst_first(limit)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            for h in &headers {
                print!("{:<24} ", h);
            }
            println!();
            let mut count = 0;
            for (id, rel, succ, fail) in rows {
                print!("{:<24} ", summarize_text(&id, 22));
                print!("{:<24.4} ", rel);
                print!("{:<24} ", succ);
                print!("{:<24} ", fail);
                println!();
                count += 1;
            }
            if count == 0 {
                println!("(no records found)");
            }
        }
        "repositories" => {
            let headers = vec!["Repository ID", "Reliability", "Success", "Failure"];
            let rows = db
                .list_repository_reliability_worst_first(limit)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            for h in &headers {
                print!("{:<24} ", h);
            }
            println!();
            let mut count = 0;
            for (id, rel, succ, fail) in rows {
                print!("{:<24} ", summarize_text(&id, 22));
                print!("{:<24.4} ", rel);
                print!("{:<24} ", succ);
                print!("{:<24} ", fail);
                println!();
                count += 1;
            }
            if count == 0 {
                println!("(no records found)");
            }
        }
        _ => anyhow::bail!(
            "Unknown reliability domain '{}'. Use endpoints, skills, workflows, or repositories.",
            domain
        ),
    }
    Ok(())
}

/// List reliability scores for execution agents.
pub async fn reliability_agents(limit: i64, min_score: Option<f64>) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;

    let min = min_score.unwrap_or(0.0);
    let rows = db
        .list_agent_reliability_above(min, limit)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("Agent Reliability (min score: {:.2})", min);
    println!(
        "{:<40} {:<12} {:<10} {:<10}",
        "Agent ID", "Reliability", "Success", "Failure"
    );

    let mut count = 0;
    for (aid, rel, succ, fail) in rows {
        println!("{:<40} {:<12.4} {:<10} {:<10}", aid, rel, succ, fail);
        count += 1;
    }

    if count == 0 {
        println!("(no agents found matching criteria)");
    }
    Ok(())
}
