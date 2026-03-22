/// List Codex-bound MCP invocable names (namespace `invocable` in `names`).
pub async fn capability_list() -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let pairs = db
        .store()
        .list_names("invocable")
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "Codex invocable bindings (namespace `invocable`): {} entries",
        pairs.len()
    );
    println!("{:<48} hash (prefix)", "name");
    for (name, hash) in &pairs {
        let prefix: String = hash.chars().take(16).collect();
        let suffix = if hash.len() > 16 { "…" } else { "" };
        println!("{:<48} {}{}", name, prefix, suffix);
    }
    if pairs.is_empty() {
        println!("(none — run sync-invocables with an MCP invocables JSON array to populate)");
    }
    Ok(())
}

/// Ingest `mcp-invocables.json` (JSON array) into Codex CAS + `names`.
pub async fn sync_invocables(path: &std::path::Path) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let mut engine = vox_db::InvocableSyncEngine::new(&db);
    let count = engine
        .sync_from_file(path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Synced {} invocable(s) from {}", count, path.display());
    Ok(())
}

fn split_csv(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max_chars {
        trimmed.to_string()
    } else {
        let mut summary = trimmed.chars().take(max_chars).collect::<String>();
        summary.push_str("...");
        summary
    }
}

fn html_to_text_lossy(input: &str) -> String {
    let without_scripts = regex::Regex::new(r"(?is)<script.*?</script>|<style.*?</style>")
        .ok()
        .map(|re| re.replace_all(input, " ").into_owned())
        .unwrap_or_else(|| input.to_string());
    let without_tags = regex::Regex::new(r"(?is)<[^>]+>")
        .ok()
        .map(|re| re.replace_all(&without_scripts, " ").into_owned())
        .unwrap_or(without_scripts);
    let decoded = without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Show retrieval diagnostics (embeddings/graph/adaptive fusion state).
pub async fn retrieval_status() -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let diag = vox_db::retrieval_diagnostics(db.store()).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Retrieval diagnostics");
    println!("  Embeddings      : {}", diag.embeddings_count);
    println!("  KnowledgeNodes  : {}", diag.knowledge_nodes_count);
    println!("  KnowledgeEdges  : {}", diag.knowledge_edges_count);
    println!("  VectorWeight    : {}", diag.vector_weight);
    if let Some(ms) = diag.last_retrieval_latency_ms {
        println!("  LastLatencyMs   : {ms}");
    }
    println!("  ModeSplits      : {:?}", diag.retrieval_mode_splits);
    Ok(())
}

/// Fetch a URL and persist a normalized external research packet plus searchable document chunks.
#[allow(clippy::too_many_arguments)]
pub async fn research_ingest_url(
    vendor: &str,
    topic: &str,
    url: &str,
    title: Option<&str>,
    summary: Option<&str>,
    source_type: &str,
    area: Option<&str>,
    kb_id: Option<&str>,
    tags: Option<&str>,
    confidence: f64,
) -> anyhow::Result<()> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("failed to fetch {url}"))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("fetch failed for {url}: HTTP {status}");
    }

    let title = title
        .map(ToString::to_string)
        .unwrap_or_else(|| url.to_string());
    let plain_text = html_to_text_lossy(&body);
    let summary = summary
        .map(ToString::to_string)
        .unwrap_or_else(|| summarize_text(&plain_text, 320));
    let excerpt = summarize_text(&plain_text, 800);
    let packet = vox_db::ExternalResearchPacket {
        topic: topic.to_string(),
        vendor: vendor.to_string(),
        area: area.map(ToString::to_string),
        source_url: url.to_string(),
        source_type: source_type.to_string(),
        title,
        captured_at: chrono::Utc::now().to_rfc3339(),
        summary,
        raw_excerpt: excerpt,
        claims: vec![],
        tags: split_csv(tags),
        confidence,
        content_hash: String::new(),
        metadata: serde_json::json!({
            "http_status": status.as_u16(),
            "ingested_from": "vox codex research-ingest-url",
        }),
    };
    let kb_id = kb_id
        .map(ToString::to_string)
        .or_else(|| Some(format!("ecosystem/{vendor}")));
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<vox_db::ResearchIngestResult> {
        let mut req = vox_db::ResearchIngestRequest {
            packet,
            body: plain_text,
            kb_id,
            embeddings: vec![],
        };
        let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = db
            .ingest_research_document(&mut req)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.shutdown_for_drop();
        Ok(result)
    })
    .await
    .map_err(|e| anyhow::anyhow!("research ingest task failed: {e}"))??;

    println!("Research source persisted");
    let doc_id = result
        .document_id
        .map(|d| d.to_string())
        .unwrap_or_else(|| "—".to_string());
    println!("  Packet ID   : {}", result.packet_id);
    println!("  Document ID : {doc_id}");
    println!("  Chunks      : {}", result.chunk_ids.len());
    println!("  KB ID       : {}", result.kb_id.clone().unwrap_or_default());
    println!("  Hash        : {}", result.content_hash);
    Ok(())
}

/// Extract title from markdown: frontmatter `title:` or first `# Heading`.
fn extract_md_title(content: &str) -> String {
    let content = content.trim();
    if let Some(rest) = content.strip_prefix("---") {
        let rest = rest.strip_prefix('\n').unwrap_or(rest);
        if let Some(idx) = rest.find("\n---") {
            let fm = &rest[..idx];
            for line in fm.lines() {
                if let Some(val) = line.trim().strip_prefix("title:") {
                    return val.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
    }
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            return heading.trim().to_string();
        }
    }
    "Untitled".to_string()
}

/// Ingest a local markdown file into Codex as an ecosystem research packet.
pub async fn research_ingest_file(
    vendor: &str,
    topic: &str,
    path: &std::path::Path,
    area: Option<&str>,
    kb_id: Option<&str>,
    tags: Option<&str>,
    confidence: f64,
) -> anyhow::Result<()> {
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let title = extract_md_title(&body);
    let summary = summarize_text(&body, 320);
    let excerpt = summarize_text(&body, 800);
    let source_url = format!(
        "file://{}",
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
    );
    let packet = vox_db::ExternalResearchPacket {
        topic: topic.to_string(),
        vendor: vendor.to_string(),
        area: area.map(ToString::to_string),
        source_url,
        source_type: "local_doc".to_string(),
        title,
        captured_at: chrono::Utc::now().to_rfc3339(),
        summary,
        raw_excerpt: excerpt,
        claims: vec![],
        tags: split_csv(tags),
        confidence,
        content_hash: String::new(),
        metadata: serde_json::json!({
            "ingested_from": "vox codex research-ingest-file",
            "file_path": path.display().to_string(),
        }),
    };
    let kb_id = kb_id
        .map(ToString::to_string)
        .or_else(|| Some(format!("ecosystem/{vendor}")));
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<vox_db::ResearchIngestResult> {
        let mut req = vox_db::ResearchIngestRequest {
            packet,
            body,
            kb_id,
            embeddings: vec![],
        };
        let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
        let result = db
            .ingest_research_document(&mut req)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.shutdown_for_drop();
        Ok(result)
    })
    .await
    .map_err(|e| anyhow::anyhow!("research ingest file task failed: {e}"))??;

    println!("Research document persisted");
    let doc_id = result
        .document_id
        .map(|d| d.to_string())
        .unwrap_or_else(|| "—".to_string());
    println!("  Packet ID   : {}", result.packet_id);
    println!("  Document ID : {doc_id}");
    println!("  Chunks      : {}", result.chunk_ids.len());
    println!("  KB ID       : {}", result.kb_id.clone().unwrap_or_default());
    println!("  Hash        : {}", result.content_hash);
    Ok(())
}

/// Known OpenClaw primary sources for refresh (OC126).
const OPENCLAW_REFRESH_URLS: &[(&str, &str)] = &[
    (
        "architecture",
        "https://openclawlab.com/en/docs/concepts/system-architecture/",
    ),
    (
        "gateway",
        "https://openclawlab.com/en/docs/gateway/protocol/",
    ),
    ("remote", "https://openclawlab.com/en/docs/gateway/remote/"),
    (
        "failover",
        "https://openclawlab.com/en/docs/providers/failover/",
    ),
    (
        "openresponses",
        "https://openclawlab.com/en/docs/gateway/openresponses-http-api/",
    ),
];

/// Context-engineering research docs to ingest into ecosystem/context-engineering.
const CONTEXT_ENGINEERING_FILES: &[(&str, &str)] = &[
    (
        "context_engineering",
        "docs/src/research/res-context-engineering-2025-2026.md",
    ),
    (
        "context_rot",
        "docs/src/research/res-context-rot-long-window.md",
    ),
    (
        "prompt_caching",
        "docs/src/research/res-prompt-caching-observability.md",
    ),
    (
        "mcp_a2a",
        "docs/src/research/res-mcp-a2a-context-transfer.md",
    ),
    (
        "populi_policy",
        "docs/src/research/res-populi-context-policy.md",
    ),
    (
        "capability_map",
        "docs/src/research/res-context-capability-map.md",
    ),
];

/// Planning-mode research docs to ingest into ecosystem/planning-mode.
const PLANNING_MODE_FILES: &[(&str, &str)] = &[
    (
        "agentic_coding",
        "docs/src/research/res-planning-mode-agentic-coding-2026.md",
    ),
    (
        "capability_map",
        "docs/src/research/res-planning-mode-capability-map.md",
    ),
];

/// Re-fetch OpenClaw primary sources or ingest context-engineering docs; flags drift when content changes.
pub async fn research_refresh(vendor: &str, dry_run: bool) -> anyhow::Result<()> {
    if vendor == "openclaw" {
        if dry_run {
            println!(
                "Would refresh {} OpenClaw sources:",
                OPENCLAW_REFRESH_URLS.len()
            );
            for (topic, url) in OPENCLAW_REFRESH_URLS {
                println!("  {}: {}", topic, url);
            }
            return Ok(());
        }
        for (topic, url) in OPENCLAW_REFRESH_URLS {
            eprintln!("Fetching {}...", topic);
            research_ingest_url(
                vendor,
                topic,
                url,
                None,
                None,
                "official",
                Some(topic),
                Some("ecosystem/openclaw"),
                Some("refresh"),
                0.9,
            )
            .await?;
        }
        println!(
            "Refreshed {} OpenClaw sources. Run quarterly per codex-baas.md.",
            OPENCLAW_REFRESH_URLS.len()
        );
        return Ok(());
    }
    if vendor == "context_engineering" {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if dry_run {
            println!(
                "Would ingest {} context-engineering docs:",
                CONTEXT_ENGINEERING_FILES.len()
            );
            for (topic, rel) in CONTEXT_ENGINEERING_FILES {
                let p = base.join(rel);
                println!(
                    "  {}: {} ({})",
                    topic,
                    rel,
                    if p.exists() { "exists" } else { "missing" }
                );
            }
            return Ok(());
        }
        for (topic, rel) in CONTEXT_ENGINEERING_FILES {
            let path = base.join(rel);
            if !path.exists() {
                eprintln!("Skipping {} (not found at {})", topic, path.display());
                continue;
            }
            eprintln!("Ingesting {}...", topic);
            research_ingest_file(
                vendor,
                topic,
                &path,
                Some(topic),
                Some("ecosystem/context-engineering"),
                Some("context,memory,retrieval"),
                0.95,
            )
            .await?;
        }
        // Persist capability-map rows for context-engineering gaps
        research_refresh_context_capability_map().await?;
        println!(
            "Ingested context-engineering docs and capability map into ecosystem/context-engineering."
        );
        return Ok(());
    }
    if vendor == "planning_mode" {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if dry_run {
            println!(
                "Would ingest {} planning-mode docs:",
                PLANNING_MODE_FILES.len()
            );
            for (topic, rel) in PLANNING_MODE_FILES {
                let p = base.join(rel);
                println!(
                    "  {}: {} ({})",
                    topic,
                    rel,
                    if p.exists() { "exists" } else { "missing" }
                );
            }
            return Ok(());
        }
        for (topic, rel) in PLANNING_MODE_FILES {
            let path = base.join(rel);
            if !path.exists() {
                eprintln!("Skipping {} (not found at {})", topic, path.display());
                continue;
            }
            eprintln!("Ingesting {}...", topic);
            research_ingest_file(
                vendor,
                topic,
                &path,
                Some(topic),
                Some("ecosystem/planning-mode"),
                Some("planning,agentic,evidence,verification,schema,persistence"),
                0.95,
            )
            .await?;
        }
        println!("Ingested planning-mode docs into ecosystem/planning-mode.");
        return Ok(());
    }
    anyhow::bail!(
        "research-refresh supports vendor=openclaw, vendor=context_engineering, or vendor=planning_mode"
    );
}

/// Persist context-engineering capability-map rows (external guidance vs Vox state).
async fn research_refresh_context_capability_map() -> anyhow::Result<()> {
    let rows: Vec<(&str, &str, &str, &str, &str, &str, &str)> = vec![
        (
            "working_memory",
            "Active prompt, current files, recent messages, tool outputs",
            "MCP chat_tools, Dei context_window",
            "partial",
            "vox_ahead",
            "Reuse in Populi generate",
            "crates/vox-mcp/src/tools/chat_tools.rs, crates/vox-dei/src/memory/context_window.rs",
        ),
        (
            "episodic_memory",
            "Snapshots, conversation edges, version linkage",
            "Arca/Codex accessors; generate graph-aware mode",
            "partial",
            "parity",
            "Populi generate --context-mode graph-aware --conversation-id N",
            "crates/vox-arca/src/store_ext.rs, crates/vox-cli/src/commands/ai/generate.rs",
        ),
        (
            "semantic_memory",
            "Schema digest, KB results, module signatures",
            "SchemaDigest, vox-codex retrieval",
            "partial",
            "parity",
            "Augment Populi retrieval",
            "crates/vox-codex/src/schema_digest.rs",
        ),
        (
            "procedural_memory",
            "Playbooks, prompt templates, routing policy",
            "Prompt templates in training artifacts",
            "partial",
            "parity",
            "First-class runtime field",
            "crates/vox-cli/src/commands/ai/",
        ),
        (
            "context_editing",
            "Stale tool-result clearing before summarization",
            "MCP has some; Populi none",
            "partial",
            "vox_ahead",
            "Extract shared, wire Populi",
            "crates/vox-mcp/src/tools/chat_tools.rs",
        ),
        (
            "prompt_caching",
            "Stable prefixes, versioned blocks",
            "None",
            "missing",
            "external_ahead",
            "Defer; prep layout",
            "crates/vox-arca/src/store_ext.rs",
        ),
        (
            "a2a_handoff_durability",
            "Structured briefing, snapshot linkage",
            "store_a2a_snapshot in handoff; vox_a2a_snapshots",
            "partial",
            "parity",
            "Wire vox_a2a_edges for handoff chains",
            "crates/vox-dei/src/coordination/handoff.rs",
        ),
        (
            "observability",
            "Token, latency, truncation, retrieval telemetry",
            "Minimal",
            "partial",
            "external_ahead",
            "Instrument LLM spans",
            "crates/vox-dei/, crates/vox-mcp/",
        ),
        (
            "evals",
            "Targeted context evals (compaction, handoff, goal retention)",
            "Eval harness exists; no context evals",
            "missing",
            "external_ahead",
            "Add context evals",
            "crates/vox-eval/",
        ),
    ];
    for (area, ext_guidance, vox_evidence, status, direction, action, paths) in rows {
        research_map_add(
            "context_engineering",
            "capability_gap",
            area,
            ext_guidance,
            vox_evidence,
            status,
            direction,
            action,
            Some(paths),
        )
        .await?;
    }
    Ok(())
}

/// List normalized external research packets stored in Codex.
pub async fn research_list(vendor: Option<&str>, topic: Option<&str>, limit: i64) -> anyhow::Result<()> {
    let vendor = vendor.map(ToString::to_string);
    let topic = topic.map(ToString::to_string);
    let rows =
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<vox_db::ExternalResearchPacket>> {
            let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
            let rows = db
                .list_research_packets(vendor.as_deref(), topic.as_deref(), limit)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            db.shutdown_for_drop();
            Ok(rows)
        })
        .await
        .map_err(|e| anyhow::anyhow!("research list task failed: {e}"))??;
    if rows.is_empty() {
        println!("(no research packets)");
        return Ok(());
    }
    println!("{:<16} {:<24} {:<40} Conf", "Vendor", "Topic", "Title");
    for row in rows {
        println!(
            "{:<16} {:<24} {:<40} {:.2}",
            row.vendor, row.topic, row.title, row.confidence
        );
    }
    Ok(())
}

/// Persist one competitor capability-map row into Codex.
#[allow(clippy::too_many_arguments)]
pub async fn research_map_add(
    vendor: &str,
    topic: &str,
    area: &str,
    openclaw_capability: &str,
    vox_evidence: &str,
    status: &str,
    advantage_direction: &str,
    recommended_action: &str,
    linked_paths: Option<&str>,
) -> anyhow::Result<()> {
    let rec = vox_db::CapabilityMapRecord {
        topic: topic.to_string(),
        vendor: vendor.to_string(),
        area: area.to_string(),
        openclaw_capability: openclaw_capability.to_string(),
        vox_evidence: vox_evidence.to_string(),
        status: status.to_string(),
        advantage_direction: advantage_direction.to_string(),
        recommended_action: recommended_action.to_string(),
        linked_paths: split_csv(linked_paths),
        metadata: serde_json::json!({
            "created_from": "vox codex research-map-add",
        }),
    };
    let id = tokio::task::spawn_blocking(move || -> anyhow::Result<i64> {
        let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
        let id = db
            .store_capability_map_record(&rec)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.shutdown_for_drop();
        Ok(id)
    })
    .await
    .map_err(|e| anyhow::anyhow!("research map add task failed: {e}"))??;
    println!("Capability map row persisted: {id}");
    Ok(())
}

/// List capability-map rows stored in Codex.
pub async fn research_map_list(
    vendor: Option<&str>,
    topic: Option<&str>,
    limit: i64,
) -> anyhow::Result<()> {
    let vendor = vendor.map(ToString::to_string);
    let topic = topic.map(ToString::to_string);
    let rows =
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<vox_db::CapabilityMapRecord>> {
            let db = vox_db::VoxDb::connect_default_sync().map_err(|e| anyhow::anyhow!("{e}"))?;
            let rows = db
                .list_capability_map_records(vendor.as_deref(), topic.as_deref(), limit)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            db.shutdown_for_drop();
            Ok(rows)
        })
        .await
        .map_err(|e| anyhow::anyhow!("research map list task failed: {e}"))??;
    if rows.is_empty() {
        println!("(no capability-map rows)");
        return Ok(());
    }
    for row in rows {
        println!("- [{}] {} / {}", row.vendor, row.topic, row.area);
        println!("  capability : {}", row.openclaw_capability);
        println!("  status     : {}", row.status);
        println!("  direction  : {}", row.advantage_direction);
        println!("  action     : {}", row.recommended_action);
        println!("  evidence   : {}", row.vox_evidence);
        if !row.linked_paths.is_empty() {
            println!("  links      : {}", row.linked_paths.join(", "));
        }
    }
    Ok(())
}

/// Show telemetry metrics for research sessions.
pub async fn research_metrics(session_id: i64, metric_type: Option<&str>) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let sid = session_id.to_string();
    let mt = metric_type.unwrap_or("");
    let metrics = db
        .store()
        .list_research_metrics(&sid, mt)
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
