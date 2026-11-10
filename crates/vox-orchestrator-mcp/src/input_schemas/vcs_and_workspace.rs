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
        // ── Compiler / workspace ─────────────────────────────────────────────
        "vox_check" | "vox_validate_file" | "vox_compiler::ast_inspect" => {
            derived_tool_schema!(crate::params::ValidateFileParams)
        }
        "vox_validate_source" => {
            derived_tool_schema!(crate::params::ValidateSourceParams)
        }
        "vox_validate_mermaid" => parse_obj(
            r#"{"type":"object","properties":{"code":{"type":"string","description":"Standalone Mermaid diagram source code"},"markdown":{"type":"string","description":"Markdown document containing fenced mermaid code blocks"}},"additionalProperties":false}"#,
        ),
        "vox_run_tests" => derived_tool_schema!(crate::params::RunTestsParams),
        "vox_build_crate" | "vox_lint_crate" | "vox_coverage_report" => {
            derived_tool_schema!(crate::params::OptionalCrateNameParams)
        }

        // ── File affinity ────────────────────────────────────────────────────
        "vox_check_file_owner" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_my_files" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0}},"required":["agent_id"],"additionalProperties":false}"#,
        ),
        "vox_claim_file" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0},"path":{"type":"string"}},"required":["agent_id","path"],"additionalProperties":false}"#,
        ),
        "vox_transfer_file" => parse_obj(
            r#"{"type":"object","properties":{"from_agent":{"type":"integer","minimum":0},"to_agent":{"type":"integer","minimum":0},"path":{"type":"string"}},"required":["from_agent","to_agent","path"],"additionalProperties":false}"#,
        ),

        // ── Git & repo index ─────────────────────────────────────────────────
        "vox_git_log" => parse_obj(
            r#"{"type":"object","properties":{"max_commits":{"type":"integer"}},"additionalProperties":false}"#,
        ),
        "vox_git_diff" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"}},"additionalProperties":false}"#,
        ),
        "vox_git_status" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_git_blame" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_repo_index_status" | "vox_repo_index_refresh" => {
            parse_obj(r#"{"type":"object","additionalProperties":false}"#)
        }
        #[cfg(feature = "gui-visual-review")]
        "vox_visus_audit" => parse_obj(
            r#"{"type":"object","properties":{"url":{"type":"string","description":"Target URL for visual audit"}},"required":["url"],"additionalProperties":false}"#,
        ),
        #[cfg(feature = "gui-visual-review")]
        "vox_visus_baseline" => parse_obj(
            r#"{"type":"object","properties":{"url":{"type":"string"}},"required":["url"],"additionalProperties":false}"#,
        ),
        "vox_visual_rag_query" => {
            derived_tool_schema!(crate::params::VoxVisualRagQueryParams)
        }
        "vox_repo_status" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_gui_components"
        | "vox_gui_tokens"
        | "vox_gui_rules"
        | "vox_agy_doctor"
        | "vox_credentials_status"
        | "vox_agy_ledger_digest" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_agy_delegate" => parse_obj(
            r#"{
            "type": "object",
            "required": ["task"],
            "properties": {
                "task": { "type": "string", "description": "Exact, zero-ambiguity spec (paths + target symbols)." },
                "model": { "type": "string", "description": "Optional agy model DISPLAY NAME (not a slug), e.g. \"Gemini 3.5 Flash (Low)\". Omit for the default (Gemini 3.5 Flash High)." },
                "timeout_secs": { "type": "integer", "default": 900, "description": "Hard kill after this many seconds." }
            }
        }"#,
        ),
        "vox_agy_delegate_batch" => parse_obj(
            r#"{
            "type": "object",
            "required": ["tasks"],
            "properties": {
                "tasks": { "type": "array", "items": { "type": "string" }, "description": "File-disjoint, self-contained specs; one worker + worktree per task." },
                "max_concurrency": { "type": "integer", "default": 3, "description": "Parallel workers (clamped to 8)." },
                "timeout_secs": { "type": "integer", "default": 900 }
            }
        }"#,
        ),
        "vox_agy_pipeline" => parse_obj(
            r#"{
            "type": "object",
            "required": ["task"],
            "properties": {
                "task": { "type": "string", "description": "Exact, zero-ambiguity launch statement (paths + target symbols), hardened with ledger §B lessons + the digest's top categories." },
                "model": { "type": "string", "description": "Optional agy model DISPLAY NAME (not a slug)." },
                "timeout_secs": { "type": "integer", "default": 900, "description": "Hard kill for the agy delegation AND each gate." },
                "gates": {
                    "type": "array",
                    "description": "Verification gates run inside the jail. Scope to the touched crate and set env.CARGO_TARGET_DIR to the main target. Empty means outcome 'partial' (unverified).",
                    "items": {
                        "type": "object",
                        "required": ["name", "program"],
                        "properties": {
                            "name": { "type": "string" },
                            "program": { "type": "string" },
                            "args": { "type": "array", "items": { "type": "string" } },
                            "env": { "type": "object", "additionalProperties": { "type": "string" } }
                        }
                    }
                }
            }
        }"#,
        ),
        "vox_agy_review" => parse_obj(
            r#"{
            "type": "object",
            "required": ["ledger_id", "verdict"],
            "properties": {
                "ledger_id": { "type": "string", "description": "The AGH id returned by vox_agy_pipeline." },
                "verdict": { "type": "string", "enum": ["approve", "approve-with-followups", "request-changes"] },
                "categories": { "type": "array", "items": { "type": "string" }, "description": "Stable §B vocabulary, e.g. hallucinated-api, scope-creep." },
                "findings": { "type": "string" },
                "lessons": { "type": "array", "items": { "type": "string" }, "description": "1-3 prompt-hardening lessons fed to the flywheel." }
            }
        }"#,
        ),
        "vox_validate_vuv" => parse_obj(
            r#"{"type":"object","properties":{"source":{"type":"string","minLength":1,"description":"Vox/VUV source to validate against the compile-time GUI guarantees (contrast, layer-occlusion, a11y, structural web-IR). No files are written."}},"required":["source"],"additionalProperties":false}"#,
        ),
        "vox_validate_diagram" => parse_obj(
            r#"{"type":"object","properties":{"source":{"type":"string","minLength":1,"description":"VoxDiagram source (graph or sequence). At most 65536 bytes."},"from_mermaid":{"type":"boolean","default":false,"description":"Treat source as one Mermaid block; return the converted VoxDiagram or the by-line unsupported list."},"emit":{"type":"string","enum":["svg","mermaid"],"description":"Also return the SVG or Mermaid rendering of a valid diagram."}},"required":["source"],"additionalProperties":false}"#,
        ),
        "vox_search_status" => parse_obj(
            r#"{"type":"object","properties":{"corpus":{"type":"string","description":"Corpus id from contracts/retrieval/graphify-corpora.v1.yaml; omit for all corpora"}},"additionalProperties":false}"#,
        ),
        "vox_search_structural" => parse_obj(
            r#"{"type":"object","properties":{"corpus":{"type":"string","description":"Corpus id from contracts/retrieval/graphify-corpora.v1.yaml; omit for default corpus"},"query":{"type":"string","minLength":1,"description":"Lexical search query matched against node labels"},"limit":{"type":"integer","minimum":1,"description":"Maximum hits to return (default 10)"},"persist":{"type":"boolean","default":true,"description":"When true (default), upsert hits into knowledge_nodes for future agent recall. Pass false for ephemeral searches. Recall consumers must compare metadata.git_sha against HEAD to detect stale hits."}},"required":["query"],"additionalProperties":false}"#,
        ),
        "vox_search_atlas" => parse_obj(
            r#"{"type":"object","properties":{"corpus":{"type":"string","description":"Corpus id; omit for default"},"level":{"type":"string","enum":["crate","module","file","symbol"],"description":"LOD level of the slice"},"focus":{"type":"string","description":"Node id to center the slice on"},"branch_tip":{"type":"string","description":"Ref name for provenance header"},"kinds":{"type":"array","items":{"type":"string"},"description":"Restrict to node kinds"},"limit":{"type":"integer","minimum":1,"description":"Max nodes returned (hard-capped by the atlas budget)"}},"required":["level"],"additionalProperties":false}"#,
        ),
        "vox_search_neighbors" => parse_obj(
            r#"{"type":"object","properties":{"corpus":{"type":"string","description":"Corpus id; omit for default"},"seeds":{"type":"array","items":{"type":"string"},"minItems":1,"description":"Seed node IDs to BFS-expand from"},"max_depth":{"type":"integer","minimum":1,"maximum":5,"description":"BFS hop limit (default 2)"},"limit":{"type":"integer","minimum":1,"description":"Max hits returned (default 20)"},"direction":{"type":"string","enum":["in","out","both"],"description":"in=callers, out=callees, both=undirected (default)"}},"required":["seeds"],"additionalProperties":false}"#,
        ),
        "vox_search_path" => parse_obj(
            r#"{"type":"object","properties":{"corpus":{"type":"string","description":"Corpus id; omit for default"},"from":{"type":"string","description":"Source node ID"},"to":{"type":"string","description":"Destination node ID"},"direction":{"type":"string","enum":["in","out","both"],"description":"in=callers, out=callees, both=undirected (default)"}},"required":["from","to"],"additionalProperties":false}"#,
        ),
        "vox_search_callers" | "vox_search_callees" => parse_obj(
            r#"{"type":"object","properties":{"corpus":{"type":"string","description":"Corpus id; omit for default"},"seeds":{"type":"array","items":{"type":"string"},"minItems":1,"description":"Symbol node IDs to find callers/callees of"},"max_depth":{"type":"integer","minimum":1,"maximum":5,"description":"Transitive hop limit (default 2)"},"limit":{"type":"integer","minimum":1,"description":"Max hits returned (default 20)"}},"required":["seeds"],"additionalProperties":false}"#,
        ),
        "vox_search_compare" => parse_obj(
            r#"{"type":"object","properties":{"corpus_a":{"type":"string","description":"First corpus id to compare"},"corpus_b":{"type":"string","description":"Second corpus id to compare"}},"required":["corpus_a","corpus_b"],"additionalProperties":false}"#,
        ),
        "vox_capability_explain" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string","minLength":1,"maxLength":256,"description":"capability id, CLI path, MCP tool, GUI view key, crate or skill"}},"required":["query"],"additionalProperties":false}"#,
        ),
        "vox_capability_features" => parse_obj(
            r#"{"type":"object","properties":{"domain":{"type":"string","minLength":1,"maxLength":64,"description":"domain id (omit to list every domain with counts)"}},"additionalProperties":false}"#,
        ),
        "vox_search_history" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string","enum":["log","focus","forgotten","search","timeline","brief"],"description":"View: log (a file's changes through rename/split/merge lineage), focus (churn per area), forgotten (dormant high-fan-in areas), search (commit text, paths, symbols), timeline (top areas per bucket), brief (one line)"},"path":{"type":"string","description":"log: repo-relative file path"},"text":{"type":"string","description":"search: terms"},"since_days":{"type":"integer","minimum":1,"description":"focus (default 30) / timeline (default 90)"},"min_age_days":{"type":"integer","minimum":0,"description":"forgotten: default 60"},"bucket_days":{"type":"integer","minimum":1,"description":"timeline: default 7"},"by":{"type":"string","enum":["crate","dir"],"description":"Area grouping (default crate)"},"include_mechanical":{"type":"boolean","description":"Include fmt/generated/import-only changes (default false)"},"limit":{"type":"integer","minimum":1,"description":"Max rows (default 20)"}},"required":["query"],"additionalProperties":false}"#,
        ),
        "vox_search_rebuild" => parse_obj(
            r#"{"type":"object","properties":{"corpus":{"type":"string","description":"Corpus id to rebuild; omit for the registry default_corpus_id. WRITE/mutating: regenerates the on-disk AST code graph and snapshots the previous one."}},"additionalProperties":false}"#,
        ),
        "vox_search_set_ttl" => parse_obj(
            r#"{"type":"object","properties":{"ttl_days":{"type":"integer","minimum":1,"maximum":3650,"description":"Staleness TTL in days for Vox Search corpora. WRITE/mutating: edits the tracked contract contracts/retrieval/vox-graph-corpora.v1.yaml, which the CLI and the CI freshness gate also read. This leaves an uncommitted working-tree change: the user must commit it before the CLI and CI see the new value on other checkouts. The VOX_GRAPHIFY_TTL_DAYS env var still takes precedence at runtime."}},"required":["ttl_days"],"additionalProperties":false}"#,
        ),
        "vox_project_init" => parse_obj(
            r#"{"type":"object","properties":{"project_name":{"type":"string","minLength":1,"description":"Project / package name"},"package_kind":{"type":"string","description":"e.g. application, skill, agent, workflow, chatbot, library"},"template":{"type":"string","description":"Optional application template: chatbot, dashboard, api"},"target_subdir":{"type":"string","description":"Repo-relative directory for the scaffold (no `..`); default is workspace root"}},"required":["project_name"],"additionalProperties":false}"#,
        ),
        "vox_repo_catalog_list" | "vox_repo_catalog_refresh" => {
            parse_obj(r#"{"type":"object","additionalProperties":false}"#)
        }
        "vox_repo_query_text" => derived_tool_schema!(vox_repository::QueryTextParams),
        "vox_repo_query_file" => derived_tool_schema!(vox_repository::QueryFileParams),
        "vox_repo_query_history" => derived_tool_schema!(vox_repository::QueryHistoryParams),
        "vox_conflict_diff" => parse_obj(
            r#"{"type":"object","properties":{"conflict_id":{"description":"Conflict id as number or C-XXXXXX string","oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^C-[0-9]{6}$"}]}},"required":["conflict_id"],"additionalProperties":false}"#,
        ),
        "vox_snapshot_diff" => parse_obj(
            r#"{"type":"object","properties":{"before":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^S-[0-9]{6}$"}]},"after":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^S-[0-9]{6}$"}]}},"additionalProperties":false}"#,
        ),
        "vox_snapshot_restore" => parse_obj(
            r#"{"type":"object","properties":{"snapshot_id":{"type":"string","pattern":"^S-[0-9]{6}$"}},"required":["snapshot_id"],"additionalProperties":false}"#,
        ),
        "vox_undo" => parse_obj(
            r#"{"type":"object","properties":{"operation_id":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^OP-[0-9]{6}$"}]}},"required":["operation_id"],"additionalProperties":false}"#,
        ),
        "vox_redo" => parse_obj(
            r#"{"type":"object","properties":{"operation_id":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^OP-[0-9]{6}$"}]}},"required":["operation_id"],"additionalProperties":false}"#,
        ),
        "vox_resolve_conflict" => parse_obj(
            r#"{"type":"object","properties":{"conflict_id":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^C-[0-9]{6}$"}]},"strategy":{"type":"string","enum":["take_left","take_right","defer"]},"defer_to_agent":{"type":"integer","minimum":0}},"required":["conflict_id"],"additionalProperties":false}"#,
        ),

        // ── VCS (pass-through args; shape varies by tool) ───────────────────
        "vox_snapshot_list"
        | "vox_oplog"
        | "vox_conflicts"
        | "vox_workspace_create"
        | "vox_workspace_merge"
        | "vox_workspace_status"
        | "vox_change_create"
        | "vox_change_log"
        | "vox_vcs_status" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"Tool-specific JSON; see vcs_tools handlers."}"#,
        ),

        // ── DB digest tools ──────────────────────────────────────────────────
        "vox_db_schema" | "vox_db_relationships" | "vox_db_data_flow" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }
        "vox_db_sample_data" | "vox_db_explain_query" | "vox_db_suggest_query" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }
        "vox_journey_canonical_steps" => parse_obj(
            r#"{"type":"object","properties":{"journey_id":{"type":"string","maxLength":512,"description":"Canonical journey id; defaults to canonical_journey.v1.greenfield_vox_mens_devloop"}},"additionalProperties":false}"#,
        ),

        "vox_db_research_session_upsert" => parse_obj(
            r#"{"type":"object","properties":{"session_key":{"type":"string"},"title":{"type":"string"},"status":{"type":"string"},"repository_id":{"type":"string"},"config_json":{"type":"object"},"summary_json":{"type":"object"}},"required":["session_key"],"additionalProperties":false}"#,
        ),
        "vox_db_conversation_version_append" => parse_obj(
            r#"{"type":"object","properties":{"conversation_id":{"type":"integer"},"version_index":{"type":"integer"},"label":{"type":"string"},"snapshot_json":{"type":"object"}},"required":["conversation_id","version_index"],"additionalProperties":false}"#,
        ),
        "vox_db_conversation_edge_insert" => parse_obj(
            r#"{"type":"object","properties":{"from_conversation_id":{"type":"integer"},"to_conversation_id":{"type":"integer"},"edge_kind":{"type":"string"},"weight":{"type":"number"},"metadata_json":{"type":"object"}},"required":["from_conversation_id","to_conversation_id"],"additionalProperties":false}"#,
        ),
        "vox_db_topic_evolution_append" => parse_obj(
            r#"{"type":"object","properties":{"topic_id":{"type":"integer"},"event_kind":{"type":"string"},"prior_label":{"type":"string"},"new_label":{"type":"string"},"detail_json":{"type":"object"}},"required":["topic_id","event_kind"],"additionalProperties":false}"#,
        ),
        "vox_db_research_metric_linked" => parse_obj(
            r#"{"type":"object","properties":{"session_key":{"type":"string"},"metric_type":{"type":"string"},"metric_value":{"type":"number"},"metadata_json":{"type":["string","object","null"]},"repository_id":{"type":"string"}},"required":["session_key","metric_type"],"additionalProperties":false}"#,
        ),
        "vox_db_trust_rollups" => parse_obj(
            r#"{"type":"object","properties":{"entity_type":{"type":"string","maxLength":128},"dimension":{"type":"string","maxLength":128},"domain":{"type":"string","maxLength":256},"repository_id":{"type":"string","maxLength":512},"repository_id_default_workspace":{"type":"boolean","description":"When true, filter rollups to the MCP workspace repository_id"},"limit":{"type":"integer","minimum":1,"maximum":10000}},"additionalProperties":false}"#,
        ),
        "vox_db_trust_summary" => parse_obj(
            r#"{"type":"object","properties":{"entity_type":{"type":"string","maxLength":128},"dimension":{"type":"string","maxLength":128},"domain":{"type":"string","maxLength":256},"repository_id":{"type":"string","maxLength":512},"repository_id_default_workspace":{"type":"boolean","description":"When true, filter trust rollups to the MCP workspace repository_id"},"group_by":{"type":"string","enum":["dimension","domain","entity_type","dimension_domain","entity_dimension"]},"limit_groups":{"type":"integer","minimum":1,"maximum":500}},"additionalProperties":false}"#,
        ),
        "vox_db_trust_drift" => parse_obj(
            r#"{"type":"object","properties":{"entity_type":{"type":"string","maxLength":128},"dimension":{"type":"string","maxLength":128},"window_ms":{"type":"integer","minimum":60000,"maximum":2592000000,"description":"Recent vs prior window length (ms); default 86400000"}},"additionalProperties":false}"#,
        ),
        "vox_db_trust_propagate" => parse_obj(
            r#"{"type":"object","properties":{"dimension":{"type":"string","minLength":1,"maxLength":128},"repository_id":{"type":"string","maxLength":512},"repository_id_default_workspace":{"type":"boolean","description":"When true, scope model rollups to workspace repository_id"},"damping":{"type":"number","minimum":0.0,"maximum":1.0},"iterations":{"type":"integer","minimum":1,"maximum":256},"persist":{"type":"boolean","description":"Write {dimension}_propagated trust observations"}},"required":["dimension"],"additionalProperties":false}"#,
        ),

        // ── Codegen ──────────────────────────────────────────────────────────
        "vox_generate_code" => parse_obj(
            r#"{"type":"object","properties":{"prompt":{"type":"string","minLength":1,"description":"Natural-language description of the `.vox` code to generate"},"validate":{"type":"boolean","description":"When true (default), run `validate_document_with_hir` and bounded repair retries"},"max_retries":{"type":"integer","minimum":0,"maximum":5,"description":"Max HIR repair attempts (clamped to speech policy cap)"},"session_id":{"type":"string","description":"Optional session id for model routing affinity"},"journey_id":{"type":"string","description":"Optional stable id for this generate request (joins routing + telemetry; server-generated if omitted)"},"output_surface_mode":{"type":"string","enum":["raw_code_only","fenced_transport"],"description":"How the model wraps `.vox` surface text"},"output_path":{"type":"string","description":"Optional workspace-relative path to write validated `.vox` text; `meta.file_outcomes` reports bytes and relative path"},"vcs_agent_id":{"type":"integer","minimum":0,"description":"When set with `output_path`, capture a filesystem snapshot for this agent after the write; `meta.file_outcomes.post_write_snapshot_id` is set"}},"required":["prompt"],"additionalProperties":false}"#,
        ),

        _ => return None,
    };
    Some(schema)
}
