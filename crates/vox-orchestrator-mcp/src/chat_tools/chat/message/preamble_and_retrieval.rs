use super::*;

pub(super) struct PreparedTurnContext {
    pub expanded_prompt: String,
    pub turn_user_prompt: String,
    pub explicit_search_result: Option<String>,
    pub deep_answer: Option<Result<String, String>>,
    pub retrieval_evidence: Option<crate::memory::RetrievalEvidenceEnvelope>,
    pub preamble_skipped_reason: Option<&'static str>,
    pub all_context_files: Vec<String>,
}

pub(super) async fn prepare_turn_context_and_retrieval(
    state: &ServerState,
    params: &ChatMessageParams,
    session_id: &str,
    expanded_prompt: String,
    mention_files: Vec<String>,
    mention_count: usize,
    intent: &super::super::research_intent::ResearchIntent,
    research_trace: &mut super::super::research_turn::ResearchTrace,
) -> PreparedTurnContext {
    let mut context_parts = Vec::new();

    if let Some(active_file) = &params.active_file {
        let line_info = params
            .active_line
            .map(|l| format!(" (line {l})"))
            .unwrap_or_default();
        context_parts.push(format!("[ACTIVE FILE]: {active_file}{line_info}"));
        // Phase 145 (SOTA-SLICE-02): When an active_file > 120 lines is open and
        // no explicit selection overrides it, automatically attach a compact AST
        // symbol skeleton so the model sees all file signatures with ~70% fewer tokens.
        if params.selected_text.as_ref().is_none_or(|s| s.is_empty()) {
            let trimmed_active = active_file.trim();
            if !trimmed_active.is_empty() && !trimmed_active.contains("..") {
                let candidate = state.repository.root.join(trimmed_active);
                if candidate.is_file()
                    && let Ok(src) = std::fs::read_to_string(&candidate)
                    && src.lines().count() > 120
                {
                    context_parts.push(super::super::mentions::extract_compact_symbol_skeleton(
                        trimmed_active,
                        &src,
                    ));
                }
            }
        }
    }

    if let Some(selected) = &params.selected_text
        && !selected.is_empty()
    {
        context_parts.push(format!("[SELECTED TEXT]:\n{selected}"));
    }

    if !params.diagnostics.is_empty() {
        let diag_str: Vec<String> = params
            .diagnostics
            .iter()
            .filter_map(|d| {
                let msg = d["message"].as_str()?;
                let line = d["line"].as_u64().unwrap_or(0);
                let sev = d["severity"].as_str().unwrap_or("error");
                Some(format!("  Line {line} [{sev}]: {msg}"))
            })
            .collect();
        if !diag_str.is_empty() {
            context_parts.push(format!(
                "[ACTIVE ERRORS/WARNINGS]:\n{}",
                diag_str.join("\n")
            ));
        }
    }

    if !params.open_files.is_empty() {
        context_parts.push(format!("[OPEN FILES]: {}", params.open_files.join(", ")));
    }

    // Check if explicit local knowledgebase search was requested
    let is_explicit_local_search = expanded_prompt.trim_start().starts_with("/research search")
        || expanded_prompt.trim_start().starts_with("/research-search")
        || expanded_prompt
            .trim_start()
            .starts_with("/research --search");

    let explicit_search_result = if is_explicit_local_search {
        let remainder = expanded_prompt
            .trim_start()
            .strip_prefix("/research search")
            .or_else(|| {
                expanded_prompt
                    .trim_start()
                    .strip_prefix("/research-search")
            })
            .or_else(|| {
                expanded_prompt
                    .trim_start()
                    .strip_prefix("/research --search")
            })
            .unwrap_or(&expanded_prompt)
            .trim();

        // Parse typed flags: --domain, --min-confidence, --verified-only, --limit
        let mut query_tokens = Vec::new();
        let mut domain = None;
        let mut min_confidence = None;
        let mut verified_only = None;
        let mut limit = Some(5);

        let parts = remainder.split_whitespace().collect::<Vec<_>>();
        let mut idx = 0;
        while idx < parts.len() {
            let p = parts[idx];
            if let Some(val) = p.strip_prefix("--domain=") {
                domain = Some(val.trim_matches('"').trim_matches('\'').to_string());
            } else if p == "--domain" && idx + 1 < parts.len() {
                idx += 1;
                domain = Some(parts[idx].trim_matches('"').trim_matches('\'').to_string());
            } else if let Some(val) = p.strip_prefix("--min-confidence=") {
                min_confidence = val.parse::<f64>().ok();
            } else if p == "--min-confidence" && idx + 1 < parts.len() {
                idx += 1;
                min_confidence = parts[idx].parse::<f64>().ok();
            } else if let Some(val) = p.strip_prefix("--limit=") {
                limit = val.parse::<usize>().ok();
            } else if p == "--limit" && idx + 1 < parts.len() {
                idx += 1;
                limit = parts[idx].parse::<usize>().ok();
            } else if p == "--verified-only" {
                verified_only = Some(true);
            } else {
                query_tokens.push(p);
            }
            idx += 1;
        }

        let clean_query = query_tokens
            .join(" ")
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_string();

        let search_params = crate::memory_tools::ResearchSearchParams {
            query: clean_query,
            limit,
            domain,
            min_confidence,
            verified_only,
        };
        Some(crate::memory_tools::research_search(state, search_params).await)
    } else {
        None
    };

    // 2b/2c. Unified autonomous retrieval injection:
    // Use the same retrieval pipeline as `vox_memory_search` with deterministic fallback
    // (hybrid -> BM25 -> lexical fallback), then append memory + knowledge snippets.
    let kb_mask = KbScopeMask::from_kb_scope(params.kb_scope.as_deref());
    let mut retrieval_evidence = None;
    let has_explicit_code_context = mention_count > 0
        || !params.context_files.is_empty()
        || params
            .selected_text
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
    let (run_auto_preamble, preamble_skipped_reason) = should_run_auto_chat_preamble(
        &params.prompt,
        params.kb_scope.as_deref(),
        has_explicit_code_context,
    );
    if explicit_search_result.is_none() && kb_mask.enabled {
        let retrieval_trace = params
            .trace_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                params
                    .correlation_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            });
        // Task 8d: when this turn's research intent is Quick or Deep, the
        // numbered research sources below are this turn's web evidence — the
        // preamble must not inject a second, unnumbered web block the
        // citation check can't verify. Its memory/KG/chunk/repo/KB legs are
        // unrelated to web evidence and stay on for every intent.
        let skip_preamble_web = intent.mode != super::super::research_intent::ResearchMode::None;
        if skip_preamble_web {
            research_trace.push(super::super::research_turn::StageRecord::new(
                "preamble",
                "skipped",
                None,
                "autonomous web retrieval skipped: numbered research sources are the web \
                 evidence this turn"
                    .to_string(),
                serde_json::json!({}),
            ));
        }
        if run_auto_preamble || skip_preamble_web {
            match tokio::time::timeout(
                std::time::Duration::from_millis(750),
                run_retrieval_bundle(
                    state,
                    &expanded_prompt,
                    RetrievalTriggerMode::AutoChatPreamble,
                    3,
                    retrieval_trace,
                    skip_preamble_web,
                ),
            )
            .await
            {
                Ok(Ok(bundle)) => {
                    if kb_mask.rrf && !bundle.rrf_fused_lines.is_empty() {
                        let snippets = bundle
                            .rrf_fused_lines
                            .iter()
                            .map(|h| format!("- {h}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        context_parts.push(format!(
                            "[AUTONOMOUS RESEARCH — RRF FUSION (tier: {})]:\n{snippets}",
                            bundle.evidence.retrieval_tier
                        ));
                    }
                    if kb_mask.memory && !bundle.memory_lines.is_empty() {
                        let snippets = bundle
                            .memory_lines
                            .iter()
                            .map(|h| format!("- {h}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        context_parts.push(format!(
                            "[AUTONOMOUS RESEARCH — MEMORY (tier: {})]:\n{snippets}",
                            bundle.evidence.retrieval_tier
                        ));
                    }
                    if kb_mask.knowledge && !bundle.knowledge_lines.is_empty() {
                        let formatted = bundle
                            .knowledge_lines
                            .iter()
                            .map(|n| format!("- {n}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        context_parts.push(format!(
                            "[AUTONOMOUS RESEARCH — KNOWLEDGE GRAPH]:\n{formatted}"
                        ));
                    }
                    if kb_mask.chunks && !bundle.chunk_lines.is_empty() {
                        let formatted = bundle
                            .chunk_lines
                            .iter()
                            .map(|c| format!("- {c}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        context_parts.push(format!(
                            "[AUTONOMOUS RESEARCH — DOCUMENT CHUNKS]:\n{formatted}"
                        ));
                    }
                    if kb_mask.repo && !bundle.repo_lines.is_empty() {
                        let formatted = bundle
                            .repo_lines
                            .iter()
                            .map(|c| format!("- {c}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        context_parts
                            .push(format!("[AUTONOMOUS RESEARCH — REPOSITORY]:\n{formatted}"));
                    }
                    retrieval_evidence = Some(bundle.evidence.clone());
                }
                Ok(Err(e)) => {
                    tracing::debug!(
                        target: "vox_mcp::autonomous_research",
                        error = %e,
                        "autonomous retrieval injection failed — continuing without injected context"
                    );
                }
                Err(_) => {
                    tracing::debug!(
                        target: "vox_mcp::autonomous_research",
                        "autonomous retrieval preamble exceeded 750ms latency budget — continuing without injected context"
                    );
                }
            }
        }
    }

    // Research (spec §4): intent was classified above (before the preamble);
    // run quick/deep now — every turn already has a trace.
    let mut deep_answer: Option<Result<String, String>> = None;
    if explicit_search_result.is_none() && vox_orchestrator::is_chat_research_enabled() {
        match intent.mode {
            super::super::research_intent::ResearchMode::None => {}
            super::super::research_intent::ResearchMode::Quick => {
                let block = super::super::research_turn::run_quick(state, research_trace).await;
                context_parts.push(block);
            }
            super::super::research_intent::ResearchMode::Deep => {
                deep_answer =
                    Some(super::super::research_turn::run_deep(state, research_trace).await);
            }
        }
    }
    // Slash commands: the model sees the question, not "/research …".
    let expanded_prompt =
        if intent.explicit && intent.mode != super::super::research_intent::ResearchMode::None {
            intent.query.clone()
        } else {
            expanded_prompt
        };

    // Hydrate persisted session context pins (`gui.session_context_pins.<session_id>`)
    // and combine with explicit `params.context_files` so pinned context reaches the LLM.
    let mut persisted_file_pins: Vec<String> = Vec::new();
    let mut persisted_other_pins: Vec<String> = Vec::new();
    if let Some(ref db) = state.db {
        let pref_key = format!("gui.session_context_pins.{session_id}");
        if let Ok(Some(raw)) = db.get_user_preference("local_user", &pref_key).await
            && let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&raw)
        {
            for item in arr {
                let kind = item.get("kind").and_then(|v| v.as_str()).unwrap_or("file");
                if let Some(val) = item
                    .get("value")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    if kind == "file" {
                        persisted_file_pins.push(val.to_string());
                    } else {
                        persisted_other_pins.push(format!("{kind}:{val}"));
                    }
                }
            }
        }
    }

    let all_context_files: Vec<String> = {
        let mut v = params.context_files.clone();
        v.extend(persisted_file_pins);
        v.extend(mention_files);
        v.dedup();
        v
    };

    if !all_context_files.is_empty() || !persisted_other_pins.is_empty() {
        let mut pinned_summary = all_context_files.clone();
        pinned_summary.extend(persisted_other_pins);
        context_parts.push(format!("[PINNED CONTEXT]: {}", pinned_summary.join(", ")));
    }

    if let Some(gov_hint) = build_governance_context_hint(
        params.mode.as_deref(),
        params.oversight_level.as_deref(),
        params.dry_run,
    ) {
        context_parts.push(gov_hint);
    }

    // Phase 147 (`SOTA-KVCACHE-01`): Keep `system_prompt` 100% invariant within a
    // calendar day by moving the per-second `session_ts` hint out of `system_prompt`
    // into turn-local `context_parts`, and removing the duplicate second copy of
    // `ANTI_LAZINESS_RIDER` (`build_system_prompt_with_skill` already embeds it once).
    let ctx_handle = state.orchestrator.context_handle();
    let session_ts =
        match crate::sync_poison::poison_rw_read(ctx_handle.read(), "orchestrator context") {
            Ok(guard) => guard
                .age_secs(&format!("chat_history:{session_id}"))
                .map(|a: u64| format!("Session last active: {a}s ago."))
                .unwrap_or_default(),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    tool = "vox_chat_message",
                    "context lock poisoned; skipping session age hint"
                );
                String::new()
            }
        };
    if !session_ts.is_empty() {
        context_parts.push(format!("[SESSION AGE]: {session_ts}"));
    }

    let turn_user_prompt = if context_parts.is_empty() {
        expanded_prompt.clone()
    } else {
        format!("{}\n\n{}", context_parts.join("\n"), expanded_prompt)
    };
    let mut runtime_vars = super::super::system_prompt_vars::build_live_runtime_vars(
        &state.repository.root,
        &super::super::super::ts_to_date_str(super::super::super::now_ts()),
        params.active_file.as_deref(),
    );
    runtime_vars.session_id = session_id.to_string();
    if let Some(model_ov) = params.model_override.as_deref().filter(|s| !s.is_empty()) {
        runtime_vars.model_id = model_ov.to_string();
    }
    if let Some(prof) = params
        .cognitive_profile
        .as_deref()
        .filter(|s| !s.is_empty())
    {
        runtime_vars.cognitive_profile = prof.to_string();
    }
    let expanded_prompt = vox_orchestrator::context_sanitizer::escape_user_prompt_special_tokens(
        &super::super::system_prompt_vars::interpolate_system_prompt_variables(
            &expanded_prompt,
            &runtime_vars,
        ),
    );
    let turn_user_prompt = vox_orchestrator::context_sanitizer::escape_user_prompt_special_tokens(
        &super::super::system_prompt_vars::interpolate_system_prompt_variables(
            &turn_user_prompt,
            &runtime_vars,
        ),
    );
    PreparedTurnContext {
        expanded_prompt,
        turn_user_prompt,
        explicit_search_result,
        deep_answer,
        retrieval_evidence,
        preamble_skipped_reason,
        all_context_files,
    }
}
