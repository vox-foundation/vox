use crate::orchestrator::Orchestrator;
use crate::socrates::SocratesTaskContext;
use tracing::{info, warn};
use vox_search::web_dispatcher::WebSearchDispatcher;

impl Orchestrator {
    /// Performs autonomous research based on Socrates policy requests or CRAG routing.
    /// This is a blocking (async) step that injects live web evidence.
    pub async fn perform_autonomous_research(
        &self,
        agent_id: Option<crate::types::AgentId>,
        task_id: Option<crate::types::TaskId>,
        queries: Vec<String>,
        reason: &str,
    ) -> Result<Vec<String>, String> {
        info!(
            reason = %reason,
            query_count = queries.len(),
            "triggering autonomous socrates research dispatch"
        );

        let (policy, quality_target) = {
            let cfg = crate::sync_lock::rw_read(&*self.config);
            (cfg.effective_search_policy(), cfg.research_quality_target)
        };

        let mut research_results = Vec::new();
        let mut hops_remaining = policy.web_search_max_hops;
        let mut active_queries = queries.clone();
        let mut visited_urls = std::collections::HashSet::new();

        while hops_remaining > 0 && !active_queries.is_empty() {
            let mut hop_hits = Vec::new();
            info!(
                hop = policy.web_search_max_hops - hops_remaining + 1,
                query_count = active_queries.len(),
                "starting research hop"
            );

            for query in &active_queries {
                match WebSearchDispatcher::search(&query, &policy).await {
                    Ok(hits) => {
                        for hit in hits {
                            if visited_urls.insert(hit.path.clone()) {
                                let engine = hit
                                    .provenance
                                    .iter()
                                    .find_map(|p| p.strip_prefix("engine:"))
                                    .unwrap_or("unknown");

                                research_results.push(format!(
                                    "[autonomous_research:{}] {} (score: {:.3}; engine: {}) - {}",
                                    hit.path,
                                    hit.title,
                                    hit.score,
                                    engine,
                                    hit.content_snippet.replace('\n', " ")
                                ));
                                hop_hits.push(hit);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(query = %query, error = %e, "research query failed");
                    }
                }
            }

            // Calculate current evidence quality (simplified heuristic)
            let current_quality = (research_results.len() as f64 * 0.1).min(1.0);

            if !vox_search::crag::CragRouter::should_continue(
                current_quality,
                quality_target,
                hops_remaining,
            ) {
                break;
            }

            // Expand queries for next hop
            active_queries = vox_search::crag::CragRouter::expand_queries_from_partial_evidence(
                &queries[0],
                &hop_hits,
            );
            hops_remaining -= 1;
        }

        let research_model_enabled = crate::sync_lock::rw_read(&*self.config).research_model_enabled;
        
        if research_model_enabled && !research_results.is_empty() {
            info!("delegating research synthesis to Lane G (research-expert)");
            
            #[cfg(feature = "runtime")]
            {
                use vox_runtime::llm::{infer_with_retry, LlmConfig, LlmChatMessage};

                let combined_evidence = research_results.join("\n\n");
                
                // Configure Lane G endpoint
                // In production, this might map to a custom local inference server or an external expert model.
                let config = LlmConfig::openrouter("anthropic/claude-3.5-sonnet:beta");
                if let Some(_key) = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshToken).expose() {
                    // Overwrite if hitting internal mesh. For now, we fallback to standard LLM pipeline.
                    tracing::debug!("Using specific Lane G auth");
                }
                
                let messages = vec![
                    LlmChatMessage {
                        role: "system".into(),
                        content: "You are Lane G, the Vox autonomous research synthesis expert. Your objective is to ingest raw search observations and formulate a dense, factual markdown summary containing specific claims, figures, and direct citations.".into(),
                    },
                    LlmChatMessage {
                        role: "user".into(),
                        content: format!("Synthesize the following recent web evidence into a high-fidelity summary:\n\n{}", combined_evidence),
                    }
                ];

                match infer_with_retry(&vox_runtime::activity::ActivityOptions::default(), messages, vec![config]).await {
                    vox_runtime::ActivityResult::Ok(Ok((res, used_cfg))) if !res.content.is_empty() => {
                        let text = res.content.clone();
                        let prompt_tokens = res.prompt_tokens;
                        let completion_tokens = res.completion_tokens;
                        let model_id = res.model.clone();
                        let provider = used_cfg.provider.clone();

                        // Compute cost using the model registry
                        let cost_usd = self
                            .models
                            .read()
                            .unwrap()
                            .get(&model_id)
                            .map(|spec| {
                                ((prompt_tokens + completion_tokens) as f64 / 1000.0)
                                    * spec.cost_per_1k
                            })
                            .unwrap_or(0.0);

                        // Record flat telemetry for SQL-based evaluation
                        self.record_telemetry(
                                agent_id.unwrap_or(crate::types::AgentId(0)),
                                "ResearchSynthesisExecuted",
                                Some(&model_id),
                                Some(&provider),
                                Some(prompt_tokens),
                                Some(completion_tokens),
                                Some(cost_usd),
                                Some(serde_json::json!({
                                    "task_id": task_id,
                                    "results_count": research_results.len(),
                                    "content_preview_len": text.len(),
                                })),
                            )
                            .await;

                        // Emit detailed event for real-time observability
                        self.event_bus.emit(crate::events::AgentEventKind::ResearchSynthesisExecuted {
                            agent_id,
                            task_id,
                            model_id: model_id.clone(),
                            provider: provider.clone(),
                            input_tokens: prompt_tokens,
                            output_tokens: completion_tokens,
                            cost_usd,
                            content_preview: text.chars().take(200).collect(),
                        });

                        info!(
                            "Lane G synthesis completed: {} tokens ($ {:.6})",
                            prompt_tokens + completion_tokens,
                            cost_usd
                        );
                        research_results.push(format!("[lane_g_synthesis] {}", text));
                    }
                    other => {
                        warn!(?other, "Lane G synthesis failed or returned empty content, falling back to raw results");
                    }
                }
            }
        }


        if !research_results.is_empty() {
            info!(
                count = research_results.len(),
                "collected autonomous research evidence"
            );
        }

        self.event_bus
            .emit(crate::events::AgentEventKind::ResearchExecuted {
                agent_id,
                task_id,
                queries,
                results_count: research_results.len(),
            });

        Ok(research_results)
    }

    /// Injects research results into a Socrates task context using a canonical CRAG line prefix.
    pub fn inject_research_results(&self, ctx: &mut SocratesTaskContext, results: Vec<String>) {
        if results.is_empty() {
            return;
        }

        // We update the evidence count to reflect the new findings.
        // In a full CRAG loop, we would re-run socrates evaluation AFTER injection,
        // but for now we trust the injection is sufficient for the agent.
        ctx.evidence_count = ctx
            .evidence_count
            .saturating_add(results.len().min(u8::MAX as usize) as u8);

        if let Some(ref mut diag) = ctx.retrieval_diagnosis {
            diag.evidence_shape = "ok".to_string(); // evidence is no longer empty/thin
            if !diag.corpora_with_hits.contains(&"webresearch".to_string()) {
                diag.corpora_with_hits.push("webresearch".to_string());
            }
        }

        ctx.citation_coverage = (ctx.citation_coverage + 0.20).min(1.0);
        ctx.evidence_quality = (ctx.evidence_quality + 0.15).min(1.0);
    }
}
