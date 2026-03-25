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
    let conn = &db.connection();

    let (query, headers, fields) = match domain {
        "endpoints" => (
            "SELECT endpoint_url, model_id, total_requests, hallucination_proxy_ewma, contradiction_ratio_ewma, infra_failure_ewma FROM endpoint_reliability ORDER BY hallucination_proxy_ewma DESC, infra_failure_ewma DESC LIMIT ?1",
            vec![
                "Endpoint",
                "Model",
                "Reqs",
                "Hallucina",
                "Contradic",
                "InfraFail",
            ],
            6,
        ),
        "skills" => (
            "SELECT skill_id, reliability, success_count, failure_count FROM skill_reliability ORDER BY reliability ASC LIMIT ?1",
            vec!["Skill ID", "Reliability", "Success", "Failure"],
            4,
        ),
        "workflows" => (
            "SELECT workflow_name, reliability, success_count, failure_count FROM workflow_reliability ORDER BY reliability ASC LIMIT ?1",
            vec!["Workflow", "Reliability", "Success", "Failure"],
            4,
        ),
        "repositories" => (
            "SELECT repository_id, reliability, success_count, failure_count FROM repository_reliability ORDER BY reliability ASC LIMIT ?1",
            vec!["Repository ID", "Reliability", "Success", "Failure"],
            4,
        ),
        _ => anyhow::bail!(
            "Unknown reliability domain '{}'. Use endpoints, skills, workflows, or repositories.",
            domain
        ),
    };

    let mut rows = conn.query(query, turso::params![limit]).await?;
    println!("Reliability stats for: {}", domain);

    for h in &headers {
        print!("{:<24} ", h);
    }
    println!();

    let mut count = 0;
    while let Some(row) = rows.next().await? {
        for i in 0..fields {
            let val_str = match row.get_value(i as usize) {
                Ok(turso::Value::Text(s)) => s,
                Ok(turso::Value::Integer(v)) => v.to_string(),
                Ok(turso::Value::Real(f)) => format!("{:.4}", f),
                _ => "-".to_string(),
            };
            print!("{:<24} ", summarize_text(&val_str, 22));
        }
        println!();
        count += 1;
    }

    if count == 0 {
        println!("(no records found)");
    }
    Ok(())
}

/// List reliability scores for execution agents.
pub async fn reliability_agents(limit: i64, min_score: Option<f64>) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let conn = &db.connection();

    let min = min_score.unwrap_or(0.0);
    let mut rows = conn.query(
        "SELECT agent_id, reliability, success_count, failure_count FROM agent_reliability WHERE reliability >= ?1 ORDER BY reliability DESC LIMIT ?2",
        turso::params![min, limit]
    ).await?;

    println!("Agent Reliability (min score: {:.2})", min);
    println!(
        "{:<40} {:<12} {:<10} {:<10}",
        "Agent ID", "Reliability", "Success", "Failure"
    );

    let mut count = 0;
    while let Some(row) = rows.next().await? {
        let aid: String = row.get(0)?;
        let rel: f64 = row.get(1)?;
        let succ: i64 = row.get(2)?;
        let fail: i64 = row.get(3)?;
        println!("{:<40} {:<12.4} {:<10} {:<10}", aid, rel, succ, fail);
        count += 1;
    }

    if count == 0 {
        println!("(no agents found matching criteria)");
    }
    Ok(())
}
