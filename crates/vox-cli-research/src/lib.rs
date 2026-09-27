use clap::Subcommand;
use vox_research_shim::research::types::{ResearchDomainMode, ResearchStage};
use vox_search::policy::ResearchLane;

pub mod eval;
pub mod infra;

#[derive(Subcommand)]
pub enum ResearchCmd {
    /// Start the SearXNG sidecar (requires Docker).
    Up,
    /// Stop the SearXNG sidecar.
    Down,
    /// Check the health of research backends (SearXNG, DDG, Tavily).
    Status,
    /// Run the orchestrator deep-research pipeline (`run_research`).
    Run {
        /// Topic / question tokens (join with spaces).
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// Emit JSON [`vox_research_shim::research::ResearchResult`] to stdout.
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Retrieval scope — `both` (default), `web`, or `local`.
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        max_sources: Option<usize>,
        #[arg(long, default_value_t = false)]
        verify_claims: bool,
        /// Restrict hits to this registrable domain (no scheme), e.g. `example.com`.
        #[arg(long)]
        site_scope: Option<String>,
        /// Create an async research session and return its id without running inline.
        #[arg(long = "async", default_value_t = false)]
        async_run: bool,
        /// Retrieval lane — `fast` (default; sends a condensed raw query, no LLM planner) or
        /// `deep` (LLM query decomposition into subqueries).
        #[arg(long)]
        lane: Option<String>,
        /// Research waves (1-5, default 1).
        #[arg(long)]
        waves: Option<usize>,
        /// Domain mode — `general` (default), `shopping`, or `codegen`.
        #[arg(long)]
        domain_mode: Option<String>,
    },
    /// Preview an editable research plan without executing retrieval.
    Preview {
        /// Topic / question tokens (join with spaces).
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// Emit JSON instead of Markdown.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// List recent persisted research sessions.
    History {
        /// Maximum sessions to show.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Show one persisted research session.
    Show {
        /// Numeric `scientia_research_sessions.id`.
        session_id: i64,
    },
    /// Watch a persisted research session until it reaches a terminal state.
    Watch { session_id: i64 },
    /// Print the latest persisted result/status for a research session.
    Result {
        session_id: i64,
        /// Output format: markdown or json.
        #[arg(long, default_value = "markdown")]
        format: String,
        /// Optional export path for the report.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Run the research evaluation harness against golden queries.
    Eval {
        /// Optional path to a golden query JSONL file.
        #[arg(long)]
        queries: Option<std::path::PathBuf>,
        /// Output path for the evaluation report.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        /// Number of parallel queries to run.
        #[arg(long, default_value_t = 4)]
        concurrency: usize,
    },
    /// Send a live query to the search providers and report status, latency and sample titles.
    Probe {
        /// Query tokens (join with spaces).
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// One provider (`searxng`, `tavily`, `openalex`, `arxiv`, `wikipedia`, `duckduckgo`); default: all keyed/optional ones.
        #[arg(long)]
        provider: Option<String>,
        /// Emit JSON instead of a table.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Flag a citation from a session as misleading (penalizes its domain in future retrieval).
    Flag {
        /// Numeric `scientia_research_sessions.id` the citation came from.
        session_id: i64,
        /// URL of the misleading source.
        #[arg(long)]
        url: String,
        /// `inelegant_code`, `fails_to_run`, `user_correction`, or `hallucinated_api`.
        #[arg(long, default_value = "inelegant_code")]
        defect: String,
        /// What was wrong / the correction.
        #[arg(long)]
        notes: Option<String>,
    },
    /// Publish a session's report to `docs/src/architecture/` with frontmatter.
    Publish {
        /// Numeric `scientia_research_sessions.id`.
        session_id: i64,
        /// File slug (default `session-<id>-research`); `-2026.md` is appended.
        #[arg(long)]
        slug: Option<String>,
    },
    /// Search past research artifacts via full-text search.
    Search {
        /// Topic / question tokens (join with spaces).
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// Maximum results to show.
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Emit JSON instead of text.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

pub async fn run(cmd: ResearchCmd) -> anyhow::Result<()> {
    match cmd {
        ResearchCmd::Up => infra::up().await,
        ResearchCmd::Down => infra::down().await,
        ResearchCmd::Status => infra::status().await,
        ResearchCmd::Run {
            query,
            json,
            scope,
            max_sources,
            verify_claims,
            site_scope,
            async_run,
            lane,
            waves,
            domain_mode,
        } => {
            let q = query.join(" ").trim().to_string();
            run_research_query(
                q,
                json,
                scope,
                max_sources,
                verify_claims,
                site_scope,
                async_run,
                lane,
                waves,
                domain_mode,
            )
            .await
        }
        ResearchCmd::Probe {
            query,
            provider,
            json,
        } => research_probe(query.join(" "), provider, json).await,
        ResearchCmd::Flag {
            session_id,
            url,
            defect,
            notes,
        } => research_flag(session_id, &url, &defect, notes).await,
        ResearchCmd::Publish { session_id, slug } => research_publish(session_id, slug).await,
        ResearchCmd::History { limit } => research_history(limit).await,
        ResearchCmd::Show { session_id } => research_show(session_id).await,
        ResearchCmd::Watch { session_id } => research_watch(session_id).await,
        ResearchCmd::Preview { query, json } => research_preview(query.join(" "), json).await,
        ResearchCmd::Result {
            session_id,
            format,
            output,
        } => research_result(session_id, &format, output).await,
        ResearchCmd::Search { query, limit, json } => {
            let q = query.join(" ").trim().to_string();
            research_search(q, limit, json).await
        }
        ResearchCmd::Eval {
            queries,
            output,
            concurrency,
        } => eval::run_eval(queries, output, concurrency).await,
    }
}

async fn connect_research_db() -> anyhow::Result<vox_db::VoxDb> {
    let cfg = vox_db::DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?;
    Ok(vox_db::VoxDb::connect(cfg).await?)
}

pub async fn research_history(limit: u32) -> anyhow::Result<()> {
    let db = connect_research_db().await?;
    let rows = db.list_recent_research_sessions(limit).await?;
    if rows.is_empty() {
        println!("No research sessions found.");
        return Ok(());
    }
    for row in rows {
        println!(
            "{}\t{}\t{}\t{}",
            row.id, row.status, row.started_at_ms, row.query_text
        );
    }
    Ok(())
}

pub async fn research_show(session_id: i64) -> anyhow::Result<()> {
    let db = connect_research_db().await?;
    let Some(row) = db.get_research_session(session_id).await? else {
        anyhow::bail!("research session {session_id} not found");
    };
    println!("session_id: {}", row.id);
    println!("session_key: {}", row.session_key);
    println!("status: {}", row.status);
    println!("started_at_ms: {}", row.started_at_ms);
    if let Some(finished) = row.finished_at_ms {
        println!("finished_at_ms: {finished}");
    }
    println!("query: {}", row.query_text);
    if let Some(artifact) = db.get_research_artifact(session_id).await? {
        println!("\n{}", artifact.report_markdown);
    } else {
        println!("\nNo durable research artifact found for this session.");
    }
    Ok(())
}

pub async fn research_preview(query: String, json: bool) -> anyhow::Result<()> {
    let preview = research_plan_preview(&query);
    if json {
        println!("{}", serde_json::to_string_pretty(&preview)?);
    } else {
        println!("# Research Plan Preview\n");
        println!("Query: {}\n", preview["query"].as_str().unwrap_or(""));
        println!("Editable: true\n");
        println!("Subqueries:");
        for subquery in preview["subqueries"].as_array().into_iter().flatten() {
            println!("- {}", subquery.as_str().unwrap_or(""));
        }
        println!("\nPolicy:");
        println!("- scope: {}", preview["scope"].as_str().unwrap_or("both"));
        println!(
            "- max_sources_per_subquery: {}",
            preview["max_sources_per_subquery"].as_u64().unwrap_or(10)
        );
    }
    Ok(())
}

pub async fn research_result(
    session_id: i64,
    format: &str,
    output: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let db = connect_research_db().await?;
    let Some(artifact) = db.get_research_artifact(session_id).await? else {
        anyhow::bail!("no durable research artifact found for session {session_id}");
    };
    let rendered = match format {
        "markdown" | "md" => artifact.report_markdown,
        "json" => artifact.artifact_json,
        other => anyhow::bail!("unsupported result format {other:?}: use markdown|json"),
    };
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, rendered)?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}

pub async fn research_search(query: String, limit: usize, json: bool) -> anyhow::Result<()> {
    let db = connect_research_db().await?;
    let hits = db.search_research_artifacts(&query, limit).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&hits)?);
    } else if hits.is_empty() {
        println!("No research artifacts found matching: {query}");
    } else {
        println!("Found {} research artifact(s):", hits.len());
        for hit in hits {
            println!(
                "- [Session {}] {}\n  Snippet: {}",
                hit.session_id, hit.query_text, hit.snippet
            );
        }
    }
    Ok(())
}

fn research_plan_preview(query: &str) -> serde_json::Value {
    let trimmed = query.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut subqueries = vec![trimmed.clone()];
    if trimmed.contains("compare") {
        subqueries.push(format!("{trimmed} comparison criteria"));
        subqueries.push(format!("{trimmed} independent source corroboration"));
    } else if trimmed.contains("trace") || trimmed.contains("lineage") {
        subqueries.push(format!("{trimmed} timeline primary sources"));
        subqueries.push(format!("{trimmed} current state"));
    } else {
        subqueries.push(format!("{trimmed} primary sources"));
        subqueries.push(format!("{trimmed} recent independent analysis"));
    }
    serde_json::json!({
        "schema_version": 1,
        "editable": true,
        "query": trimmed,
        "scope": "both",
        "max_sources_per_subquery": 10,
        "subqueries": subqueries,
        "progress_states": ResearchStage::ORDERED.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        "free_baseline": {
            "required_paid_services": false,
            "optional_tavily_when_user_configured": true
        }
    })
}

fn format_probe_report(results: &[vox_search::probe::ProviderProbeResult]) -> String {
    let mut out = String::new();
    for r in results {
        let mark = if r.success { "ok  " } else { "FAIL" };
        out.push_str(&format!(
            "{mark} {:<11} http={:<3} {:>5}ms hits={}\n",
            r.provider, r.http_status, r.latency_ms, r.hit_count
        ));
        for title in r.sample_titles.iter().take(3) {
            out.push_str(&format!("       - {title}\n"));
        }
        if let Some(e) = &r.error_message {
            out.push_str(&format!("       error: {e}\n"));
        }
        if let Some(tip) = &r.remediation_tip {
            out.push_str(&format!("       fix:   {tip}\n"));
        }
    }
    out
}

pub async fn research_probe(
    query: String,
    provider: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    use vox_search::probe::{probe_all_search_providers, probe_search_provider};
    let results = match provider {
        Some(p) => vec![probe_search_provider(p, query).await],
        None => match probe_all_search_providers(query).await {
            Ok(all) => all.into_iter().map(Ok).collect(),
            Err(e) => vec![Err(e)],
        },
    }
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .map_err(anyhow::Error::msg)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        print!("{}", format_probe_report(&results));
    }
    Ok(())
}

fn build_misguidance_params(
    session_id: i64,
    query_text: &str,
    url: &str,
    defect: &str,
    notes: Option<String>,
) -> anyhow::Result<vox_db::RecordMisguidanceParams> {
    Ok(vox_db::RecordMisguidanceParams {
        session_id: Some(session_id),
        defect_class: defect.parse().map_err(anyhow::Error::msg)?,
        culprit_url: Some(url.to_string()),
        culprit_domain: vox_research_shim::research::distillation::extract_registrable_domain(url),
        claim_id: None,
        research_query: query_text.to_string(),
        misleading_excerpt: None,
        generated_code_snippet: None,
        failure_diagnostic: None,
        correction_diff: notes,
        reporter: vox_db::MisguidanceReporter::User,
        // Same penalty the GUI's flag modal records (`ResearchView.tsx`).
        domain_penalty: 0.2,
    })
}

pub async fn research_flag(
    session_id: i64,
    url: &str,
    defect: &str,
    notes: Option<String>,
) -> anyhow::Result<()> {
    let db = connect_research_db().await?;
    let Some(row) = db.get_research_session(session_id).await? else {
        anyhow::bail!("research session {session_id} not found");
    };
    let params = build_misguidance_params(session_id, &row.query_text, url, defect, notes)?;
    let id = db.record_research_misguidance(&params).await?;
    println!(
        "recorded misguidance event {id} against {}",
        params.culprit_domain
    );
    Ok(())
}

// vox:defactored-from vox-gui 2026-09-20 (`save_research_doc` / `publish_research_doc` slug rules + frontmatter)
/// Build `(filename, markdown)` for a published research report. Never touches
/// `research-index.md` (retired — the Starlight sidebar derives from frontmatter).
fn render_published_doc(
    query_text: &str,
    session_id: i64,
    slug: Option<&str>,
    report_markdown: &str,
) -> anyhow::Result<(String, String)> {
    let slug = slug
        .map(str::to_string)
        .unwrap_or_else(|| format!("session-{session_id}-research"));
    if slug.contains('/') || slug.contains('\\') || slug.contains("..") || slug.starts_with('.') {
        anyhow::bail!("invalid slug {slug:?}: path characters are not permitted");
    }
    let filename = if slug.ends_with("-2026.md") {
        slug
    } else {
        format!("{slug}-2026.md")
    };
    let topic: String = query_text.split_whitespace().collect::<Vec<_>>().join(" ");
    let short: String = topic.chars().take(80).collect();
    let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let doc = format!(
        "---\ntitle: \"Research: {}\"\ndescription: \"Deep-research report for: {}\"\ncategory: \"Architecture SSOTs\"\nstatus: \"research\"\n---\n\n{report_markdown}\n",
        escape(&short),
        escape(&topic.chars().take(200).collect::<String>()),
    );
    Ok((filename, doc))
}

pub async fn research_publish(session_id: i64, slug: Option<String>) -> anyhow::Result<()> {
    let db = connect_research_db().await?;
    let Some(row) = db.get_research_session(session_id).await? else {
        anyhow::bail!("research session {session_id} not found");
    };
    let Some(artifact) = db.get_research_artifact(session_id).await? else {
        anyhow::bail!("no durable research artifact found for session {session_id}");
    };
    let (filename, doc) = render_published_doc(
        &row.query_text,
        session_id,
        slug.as_deref(),
        &artifact.report_markdown,
    )?;
    let root = vox_repository::discover_repository_or_fallback(&std::env::current_dir()?).root;
    let path = root.join("docs/src/architecture").join(&filename);
    vox_db::research_doc_io::atomic_write_secure(&path, doc.as_bytes())?;
    println!("published {}", path.display());
    Ok(())
}

pub async fn research_watch(session_id: i64) -> anyhow::Result<()> {
    let db = connect_research_db().await?;
    loop {
        let Some(row) = db.get_research_session(session_id).await? else {
            anyhow::bail!("research session {session_id} not found");
        };
        println!("{}\t{}\t{}", row.id, row.status, row.query_text);
        if matches!(row.status.as_str(), "completed" | "failed" | "orphaned") {
            return Ok(());
        }
        tokio::time::sleep(vox_config::timeouts::D_3S).await;
    }
}

/// Catalog handler anchor for `research.run` (`contracts/operations/catalog.v1.yaml`).
#[allow(clippy::too_many_arguments)]
pub async fn run_research_query(
    query: String,
    json: bool,
    scope: Option<String>,
    max_sources: Option<usize>,
    verify_claims: bool,
    site_scope: Option<String>,
    async_run: bool,
    lane: Option<String>,
    waves: Option<usize>,
    domain_mode: Option<String>,
) -> anyhow::Result<()> {
    use std::sync::Arc;
    use vox_db::{DbConfig, VoxDb};
    use vox_repository::discover_repository_or_fallback;
    use vox_research_shim::research::{
        ResearchConfig, ResearchQuery, ResearchScope, run_research_with_context,
    };
    use vox_search::SearchRuntimeContext;

    if query.is_empty() {
        anyhow::bail!("research run: query must not be empty");
    }

    let scope_label = scope.as_deref().unwrap_or("both").trim();
    let scope = match scope_label.to_ascii_lowercase().as_str() {
        "both" => ResearchScope::Both,
        "local" => ResearchScope::Local,
        "web" => ResearchScope::Web,
        other => anyhow::bail!("invalid scope {other:?}: use web|local|both"),
    };

    let rq = ResearchQuery {
        query,
        scope,
        max_sources: max_sources.unwrap_or(10).clamp(1, 50),
        persist_to_docs: false,
        verify_claims,
        site_scope,
        domain_mode: match domain_mode.as_deref() {
            Some(m) => m.parse().map_err(anyhow::Error::msg)?,
            None => Default::default(),
        },
        waves: waves.unwrap_or(1).clamp(1, 5),
        lane: match lane.as_deref() {
            Some(l) => l.parse().map_err(anyhow::Error::msg)?,
            None => Default::default(),
        },
    };

    if async_run {
        return run_research_async_via_daemon(&rq, json).await;
    }

    let config = ResearchConfig::default();
    let cwd = std::env::current_dir()?;
    let repo_ctx = discover_repository_or_fallback(&cwd);
    let mem = vox_orchestrator::MemoryConfig::default();
    let db = match DbConfig::resolve_canonical() {
        Ok(cfg) => VoxDb::connect(cfg).await.ok().map(Arc::new),
        Err(_) => None,
    };
    let ctx = SearchRuntimeContext::new(
        repo_ctx.root,
        db.clone(),
        cwd.join(&mem.log_dir),
        cwd.join(&mem.memory_md_path),
    );
    let result = run_research_with_context(rq, Some(&ctx), db.as_deref(), &config).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.answer);
        if !result.sources.is_empty() {
            println!("\nSources:");
            for h in &result.sources {
                println!("- {} — {}", h.title, h.url);
            }
        }
        println!(
            "\n(routing_tier={:?}, sources={}, quality_score={})",
            result.research_metadata.routing_tier,
            result.sources.len(),
            result.research_metadata.quality_score
        );
    }

    Ok(())
}

/// Map a [`ResearchScope`] to its `web|local|both` wire label for `research.run`.
fn research_scope_label(scope: &vox_research_shim::research::ResearchScope) -> &'static str {
    use vox_research_shim::research::ResearchScope;
    match scope {
        ResearchScope::Web => "web",
        ResearchScope::Local => "local",
        ResearchScope::Both => "both",
    }
}

/// Build the `research.run` JSON params from a [`ResearchQuery`].
fn research_run_daemon_params(
    rq: &vox_research_shim::research::ResearchQuery,
) -> serde_json::Value {
    serde_json::json!({
        "query": rq.query,
        "scope": research_scope_label(&rq.scope),
        "max_sources": rq.max_sources,
        "verify_claims": rq.verify_claims,
        "site_scope": rq.site_scope,
        "waves": rq.waves,
        "domain_mode": match rq.domain_mode {
            ResearchDomainMode::General => "general",
            ResearchDomainMode::Shopping => "shopping",
            ResearchDomainMode::CodeGen => "codegen",
        },
        "lane": match rq.lane {
            ResearchLane::Fast => "fast",
            ResearchLane::Deep => "deep",
        },
    })
}

/// Submit an async research run to the **persistent** orchestrator daemon
/// (`vox-orchestrator-d`) over its TCP transport, which actually executes the
/// pipeline to a terminal status in the background. Returns immediately with the
/// `{session_id, task_id, status}` envelope so `research watch` can poll.
///
/// This is the real fire-and-forget path: the work runs inside the long-lived
/// daemon process, never in this ephemeral CLI process (which exits right after
/// printing). A bare `tokio::spawn` here would be killed on exit — see the task
/// design notes. The daemon's `research.run` handler (MCP `ExtraDispatch`) owns
/// the spawn and advances the session to `completed`/`failed`.
async fn run_research_async_via_daemon(
    rq: &vox_research_shim::research::ResearchQuery,
    json: bool,
) -> anyhow::Result<()> {
    use vox_foundation::protocol::dei_method;
    use vox_orchestrator::orch_daemon::{OrchDaemonClient, is_stdio_transport};

    let socket = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxOrchestratorDaemonSocket)
        .expose()
        .map(str::to_string)
        .filter(|s| !s.trim().is_empty());

    let Some(socket) = socket else {
        anyhow::bail!(
            "research run --async needs a running orchestrator daemon, but \
             VOX_ORCHESTRATOR_DAEMON_SOCKET is unset.\n\
             Start the daemon (e.g. `VOX_ORCHESTRATOR_DAEMON_SOCKET=127.0.0.1:9745 vox-orchestrator-d`) \
             and re-run, or drop --async to run the pipeline inline in this process."
        );
    };

    // Only the long-lived TCP daemon can host the background pipeline run; a
    // per-call stdio daemon would exit (killing the spawned task) the moment
    // this CLI closes the connection. Refuse stdio rather than silently dropping
    // the work on the floor.
    if is_stdio_transport(&socket) {
        anyhow::bail!(
            "research run --async requires a TCP orchestrator daemon (e.g. 127.0.0.1:9745), \
             but VOX_ORCHESTRATOR_DAEMON_SOCKET is set to stdio. A stdio daemon is per-call and \
             cannot run the pipeline in the background. Point it at a TCP socket, or drop --async."
        );
    }

    let client = OrchDaemonClient::new(socket.clone());
    let value = client
        .call(dei_method::RESEARCH_RUN, research_run_daemon_params(rq))
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "failed to submit async research to orchestrator daemon at {socket}: {e}\n\
                 Is `vox-orchestrator-d` running and bound to that socket?"
            )
        })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        let session_id = value.get("session_id").and_then(|v| v.as_i64());
        let status = value
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("running");
        match session_id {
            Some(id) => println!(
                "research run submitted (session_id={id}, status={status}). \
                 Track it with `vox research watch {id}`."
            ),
            None => println!("{value}"),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_query(
        lane: ResearchLane,
        domain_mode: ResearchDomainMode,
        waves: usize,
    ) -> serde_json::Value {
        use vox_research_shim::research::{ResearchQuery, ResearchScope};
        research_run_daemon_params(&ResearchQuery {
            query: "q".into(),
            scope: ResearchScope::Web,
            max_sources: 10,
            persist_to_docs: false,
            verify_claims: false,
            site_scope: None,
            domain_mode,
            waves,
            lane,
        })
    }

    #[test]
    fn daemon_params_carry_lane_waves_and_domain_mode() {
        let v = run_query(ResearchLane::Deep, ResearchDomainMode::CodeGen, 3);
        assert_eq!(v["lane"], "deep");
        assert_eq!(v["waves"], 3);
        assert_eq!(v["domain_mode"], "codegen");
    }

    #[test]
    fn probe_report_lists_status_titles_and_remediation() {
        let ok = vox_search::probe::ProviderProbeResult {
            provider: "wikipedia".into(),
            http_status: 200,
            latency_ms: 42,
            success: true,
            hit_count: 2,
            sample_titles: vec!["Tucson".into()],
            error_message: None,
            remediation_tip: None,
        };
        let bad = vox_search::probe::ProviderProbeResult {
            provider: "searxng".into(),
            http_status: 0,
            latency_ms: 0,
            success: false,
            hit_count: 0,
            sample_titles: vec![],
            error_message: Some("SearXNG URL is not configured".into()),
            remediation_tip: Some("Set VOX_SEARCH_SEARXNG_URL".into()),
        };
        let report = format_probe_report(&[ok, bad]);
        assert!(report.contains("ok   wikipedia"), "{report}");
        assert!(report.contains("- Tucson"), "{report}");
        assert!(report.contains("FAIL searxng"), "{report}");
        assert!(
            report.contains("fix:   Set VOX_SEARCH_SEARXNG_URL"),
            "{report}"
        );
    }

    #[test]
    fn misguidance_params_derive_domain_and_reject_bad_defect() {
        let p = build_misguidance_params(
            9,
            "tucson events",
            "https://www.example.com/a/b?x=1",
            "hallucinated_api",
            Some("wrong".into()),
        )
        .expect("valid params");
        assert_eq!(p.culprit_domain, "example.com");
        assert_eq!(p.session_id, Some(9));
        assert_eq!(p.correction_diff.as_deref(), Some("wrong"));
        assert!(build_misguidance_params(9, "q", "https://a.io", "nonsense", None).is_err());
    }

    #[test]
    fn published_doc_has_frontmatter_and_safe_slug() {
        let (name, doc) =
            render_published_doc("Tucson \"tech\" events", 3, None, "# Body").expect("renders");
        assert_eq!(name, "session-3-research-2026.md");
        assert!(doc.starts_with("---\ntitle: \"Research: Tucson \\\"tech\\\" events\"\n"));
        assert!(doc.contains("category: \"Architecture SSOTs\""));
        assert!(doc.contains("status: \"research\""));
        assert!(doc.ends_with("# Body\n"));
        let (kept, _) = render_published_doc("q", 3, Some("x-2026.md"), "b").unwrap();
        assert_eq!(kept, "x-2026.md");
        for bad in ["../evil", "a/b", ".hidden", "a\\b"] {
            assert!(
                render_published_doc("q", 3, Some(bad), "b").is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn research_plan_preview_is_editable_and_free_baseline() {
        let preview = research_plan_preview("compare local RAG and Gemini Deep Research");

        assert_eq!(preview["editable"].as_bool(), Some(true));
        assert_eq!(
            preview["free_baseline"]["required_paid_services"].as_bool(),
            Some(false)
        );
        assert!(preview["subqueries"].as_array().expect("subqueries").len() >= 3);
    }
}
