use serde_json::{Map, Value};

macro_rules! derived_tool_schema {
    ($t:ty) => {{
        let settings = schemars::generate::SchemaSettings::draft07().with(|s| {
            s.inline_subschemas = true;
        });
        let schema_generator = settings.into_generator();
        let root = schema_generator.into_root_schema_for::<$t>();
        let serde_json::Value::Object(mut map) =
            serde_json::to_value(&root).expect(concat!("schema_for ", stringify!($t)))
        else {
            panic!(concat!(
                "JsonSchema root must be an object: ",
                stringify!($t)
            ));
        };
        map.remove("$schema");
        map
    }};
}

fn parse_obj(s: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(s)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

#[inline(never)]
pub(super) fn schema_for_tool(name: &str) -> Option<Map<String, Value>> {
    let schema = match name {
        // ── Knowledge Bases (VoxKB) ──────────────────────────────────────────
        "vox_kb_search" => derived_tool_schema!(crate::kb_tools::library::KbSearchParams),
        "vox_kb_get" => derived_tool_schema!(crate::kb_tools::library::KbGetParams),
        "vox_kb_cite" => derived_tool_schema!(crate::kb_tools::library::KbCiteParams),
        "vox_kb_status" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_kb_reindex" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),

        // ── Unified news (syndication safety + templates) ───────────────────
        "vox_news_test_syndicate" => parse_obj(
            r#"{"type":"object","properties":{"content":{"type":"string","minLength":1,"description":"Markdown with YAML frontmatter"}},"required":["content"],"additionalProperties":false}"#,
        ),
        "vox_news_draft_research" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256,"description":"Filename stem for docs/news/drafts/{news_id}.md"},"title":{"type":"string","minLength":1},"author":{"type":"string","minLength":1},"abstract_text":{"type":"string"}},"required":["news_id","title","author","abstract_text"],"additionalProperties":false}"#,
        ),
        "vox_news_approve" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256},"approver":{"type":"string","minLength":1,"maxLength":256}},"required":["news_id","approver"],"additionalProperties":false}"#,
        ),
        "vox_news_approval_status" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256}},"required":["news_id"],"additionalProperties":false}"#,
        ),
        "vox_news_simulate_publish_gate" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256},"content":{"type":"string","minLength":1}},"required":["news_id","content"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_prepare" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"title":{"type":"string","minLength":1},"author":{"type":"string","minLength":1},"content":{"type":"string","minLength":1},"abstract_text":{"type":"string"},"citations_json":{"type":"object"},"scholarly_metadata":{"type":"object","description":"ScientificPublicationMetadata (authors, license_spdx, funding_statement, ...)","additionalProperties":true},"preflight":{"type":"boolean","description":"If true, run publication_preflight before upsert; fail on error-level findings."},"preflight_profile":{"type":"string","enum":["default","double_blind","metadata_complete","arxiv_assist"]},"scientia_evidence":{"type":"object","description":"Optional ScientiaEvidenceContext for metadata_json.scientia_evidence","additionalProperties":true},"discovery_intake_gate":{"type":"string","enum":["none","strong_signals_only","allow_review_suggested"],"description":"Optional scientia intake gate evaluated before upsert"}},"required":["publication_id","title","author","content"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_preflight" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"profile":{"type":"string","enum":["default","double_blind","metadata_complete","arxiv_assist"]},"with_worthiness":{"type":"boolean","description":"If true, attach conservative worthiness rubric output (repo default YAML)."}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_worthiness_evaluate" => parse_obj(
            r#"{"type":"object","properties":{"contract_yaml_relative":{"type":"string","minLength":1,"maxLength":512,"description":"Repo-relative path to worthiness YAML"},"with_live_trust":{"type":"boolean","description":"When true and VoxDb is attached, attach summarized trust_rollups for this repository to the result"},"metrics":{"type":"object","description":"WorthinessInputs","properties":{"red_line_violation_ids":{"type":"array","items":{"type":"string","minLength":1}},"repeated_unresolved_contradiction":{"type":"boolean"},"claim_evidence_coverage":{"type":"number"},"artifact_replayability":{"type":"number"},"before_after_pair_integrity":{"type":"number"},"metadata_completeness":{"type":"number"},"ai_disclosure_compliance":{"type":"number"},"epistemic":{"type":"number"},"reproducibility":{"type":"number"},"novelty":{"type":"number"},"reliability":{"type":"number"},"metadata_policy":{"type":"number"},"meaningful_advance":{"type":"boolean"}},"required":["claim_evidence_coverage","artifact_replayability","before_after_pair_integrity","metadata_completeness","ai_disclosure_compliance","epistemic","reproducibility","novelty","reliability","metadata_policy"],"additionalProperties":false}},"required":["metrics"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_approve" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"approver":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id","approver"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_submit_local" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"adapter":{"type":"string","minLength":1,"maxLength":64,"description":"Override scholarly adapter (zenodo, openreview, local_ledger, …) instead of VOX_SCHOLARLY_ADAPTER"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_status" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"with_worthiness":{"type":"boolean","description":"If true, enrich the embedded preflight report with worthiness scoring and hydrated scientia_evidence sidecars."}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_scholarly_remote_status" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"external_submission_id":{"type":"string","minLength":1,"maxLength":512}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_scholarly_remote_status_sync_all" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_scholarly_remote_status_sync_batch" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":500,"default":25},"iterations":{"type":"integer","minimum":1,"maximum":10000,"default":1},"interval_secs":{"type":"integer","minimum":0,"maximum":3600,"default":0},"max_runtime_secs":{"type":"integer","minimum":1,"maximum":86400,"description":"Wall-clock cap for loop mode"},"jitter_secs":{"type":"integer","minimum":0,"maximum":3600,"default":0,"description":"Optional extra sleep jitter bound (capped at interval)"}},"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_arxiv_handoff_record" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"stage":{"type":"string","enum":["staging_exported","operator_ack","bundle_validated","submitted","published"]},"operator":{"type":"string","maxLength":256},"note":{"type":"string","maxLength":4096},"arxiv_id":{"type":"string","maxLength":256}},"required":["publication_id","stage"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_scholarly_staging_export" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"output_dir":{"type":"string","minLength":1,"maxLength":4096,"description":"Directory to write staging files (created if needed)"},"venue":{"type":"string","minLength":1,"maxLength":64,"description":"zenodo | openreview | arxiv-assist"}},"required":["publication_id","output_dir","venue"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_external_jobs_due" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":500,"default":50}},"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_external_jobs_dead_letter" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":500,"default":50}},"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_external_jobs_replay" => parse_obj(
            r#"{"type":"object","properties":{"job_id":{"type":"integer","minimum":1}},"required":["job_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_external_jobs_tick" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":500,"default":10},"lock_ttl_ms":{"type":"integer","minimum":5000,"maximum":3600000,"default":120000},"lock_owner":{"type":"string","minLength":1,"maxLength":256},"iterations":{"type":"integer","minimum":1,"maximum":10000,"default":1},"interval_secs":{"type":"integer","minimum":0,"maximum":3600,"default":0},"max_runtime_secs":{"type":"integer","minimum":1,"maximum":86400},"jitter_secs":{"type":"integer","minimum":0,"maximum":3600,"default":0}},"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_scholarly_pipeline_run" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"preflight_profile":{"type":"string","enum":["default","double_blind","metadata_complete"]},"dry_run":{"type":"boolean","default":false},"staging_output_dir":{"type":"string","maxLength":4096},"venue":{"type":"string","maxLength":64,"description":"zenodo | openreview | arxiv-assist when staging_output_dir set"},"adapter":{"type":"string","maxLength":64},"json_compact":{"type":"boolean","default":false,"description":"Single-line JSON in tool result"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_external_pipeline_metrics" => parse_obj(
            r#"{"type":"object","properties":{"since_hours":{"type":"integer","minimum":0,"maximum":8760,"default":168,"description":"Window for attempts/snapshots/latencies/publication_attempts; 0 = all time"}},"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_media_list" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_media_delete" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"asset_ref":{"type":"string","minLength":1,"maxLength":2048}},"required":["publication_id","asset_ref"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_media_upsert" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"asset_ref":{"type":"string","minLength":1,"maxLength":2048},"media_type":{"type":"string","minLength":1,"maxLength":64},"storage_uri":{"type":"string","maxLength":4096},"status":{"type":"string","minLength":1,"maxLength":64},"metadata_json":{"type":"object","additionalProperties":true}},"required":["publication_id","asset_ref","media_type","status"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_route_simulate" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_publish" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"channels":{"type":"array","items":{"type":"string","minLength":1,"maxLength":64}},"dry_run":{"type":"boolean"},"json":{"type":"boolean"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_retry_failed" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"channel":{"type":"string","minLength":1,"maxLength":64},"dry_run":{"type":"boolean"},"json":{"type":"boolean"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_discovery_scan" => parse_obj(
            r#"{"type":"object","properties":{"content_type":{"type":"string","maxLength":64,"description":"Filter manifests (e.g. scientia)"},"state":{"type":"string","maxLength":64,"description":"Filter by manifest state"},"limit":{"type":"integer","minimum":1,"maximum":500,"default":50}},"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_discovery_explain" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_discovery_refresh_evidence" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_novelty_fetch" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"offline":{"type":"boolean"},"persist_metadata":{"type":"boolean"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_decision_explain" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"live_prior_art":{"type":"boolean"},"offline":{"type":"boolean"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_novelty_happy_path" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"offline":{"type":"boolean"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_assist_suggestions" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"use_llm":{"type":"boolean","description":"When false, return heuristic JSON only"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        _ => return None,
    };
    Some(schema)
}
