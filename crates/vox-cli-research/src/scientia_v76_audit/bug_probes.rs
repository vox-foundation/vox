//! Authentic pre-v7.6 live bug reproduction probes (`AUDT-01`).
//!
//! All probes execute hermetically in-process against `VoxDb::connect(DbConfig::Memory)`,
//! `tempfile::tempdir()`, and `ServerState::hermetic_stub` with zero network I/O and
//! zero dependence on host environment variables or developer home directories.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use vox_db::{DbConfig, VoxDb};
use vox_orchestrator::context_envelope::OperatingMode;
use vox_orchestrator::{
    AffinityGroupRegistry, ContextEnvelope, Orchestrator, OrchestratorConfig, SessionConfig,
    SessionManager,
};
use vox_orchestrator_mcp::chat_tools::build_system_prompt_with_skill;
use vox_orchestrator_mcp::chat_tools::chat::system_prompt_vars::{
    SystemPromptRuntimeVars, interpolate_system_prompt_variables,
};
use vox_orchestrator_mcp::server_state::ServerState;
use vox_repository::{RepoCapabilities, RepositoryContext};
use vox_scientia::claim_extractor::minicheck::MiniCheckVerifier;
use vox_scientia::inspect_bridge::novelty::{AtomicNoveltyScorer, NoveltyVerdict};

use super::cassettes::{CassetteClaimPair, CassetteNoveltyCase, round6};

/// Measured live bug reproduction indicators for `vox scientia v76-audit`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveBugReproductions {
    pub composer_research_scope_overwritten: bool,
    pub research_turn_waves_hardcoded_to_one: bool,
    pub none_to_novel_false_positive_rate: f64,
    pub artifact_json_wiped_on_doc_publish: bool,
    pub mock_verifier_fallback_rate: f64,
    pub contradiction_false_support_rate: f64,
    pub kv_cache_prefix_stability_ratio: f64,
    pub session_detail_ipc_payload_bytes: usize,
    pub gui_cli_db_spawn_count: usize,
}

/// Additional verification metrics derived from the hermetic claim-verifier probe.
#[derive(Debug, Clone, Copy)]
pub struct VerifierProbeMetrics {
    pub claim_precision: f64,
    pub contradiction_detection_recall: f64,
    pub contradiction_false_support_rate: f64,
    pub mock_verifier_fallback_rate: f64,
}

/// Run all live pre-v7.6 bug reproduction probes deterministically.
pub async fn collect_live_bug_reproductions(
    repo_root: &Path,
    claim_pairs: &[CassetteClaimPair],
    novelty_bundles: &[CassetteNoveltyCase],
) -> anyhow::Result<(LiveBugReproductions, VerifierProbeMetrics)> {
    let verifier_metrics = probe_verifier_behavior(claim_pairs).await?;
    let bugs = LiveBugReproductions {
        composer_research_scope_overwritten: probe_composer_scope_overwritten(repo_root)?,
        research_turn_waves_hardcoded_to_one: probe_research_turn_hardcoded(repo_root)?,
        none_to_novel_false_positive_rate: probe_none_to_novel_rate(novelty_bundles),
        artifact_json_wiped_on_doc_publish: probe_doc_publish_wipes_artifact().await?,
        mock_verifier_fallback_rate: verifier_metrics.mock_verifier_fallback_rate,
        contradiction_false_support_rate: verifier_metrics.contradiction_false_support_rate,
        kv_cache_prefix_stability_ratio: probe_kv_cache_prefix_stability().await?,
        session_detail_ipc_payload_bytes: probe_session_detail_ipc_payload_bytes().await?,
        gui_cli_db_spawn_count: probe_gui_cli_db_spawn_count(repo_root)?,
    };
    Ok((bugs, verifier_metrics))
}

fn probe_composer_scope_overwritten(repo_root: &Path) -> anyhow::Result<bool> {
    let app_tsx = std::fs::read_to_string(repo_root.join("crates/vox-gui/ui/src/App.tsx"))?;
    let Some(start) = app_tsx.find("const submitFromComposer") else {
        return Ok(false);
    };
    let window = &app_tsx[start..(start + 900).min(app_tsx.len())];
    let overwrites_scope = window.contains("research_scope: researchMode ===")
        && !window.contains("p.research_scope ??");
    let overwrites_force = window.contains("force_research: researchMode ===")
        && !window.contains("p.force_research ??");
    Ok(window.contains("...p,") && (overwrites_scope || overwrites_force))
}

fn probe_research_turn_hardcoded(repo_root: &Path) -> anyhow::Result<bool> {
    let rel = "crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs";
    let src = std::fs::read_to_string(repo_root.join(rel))?;
    let Some(idx) = src
        .find("let rq = ResearchQuery {")
        .or_else(|| src.find("ResearchQuery {"))
    else {
        return Ok(false);
    };
    let block = &src[idx..(idx + 500).min(src.len())];
    Ok(block.contains("domain_mode: ResearchDomainMode::General") || block.contains("waves: 1,"))
}

fn probe_none_to_novel_rate(novelty_bundles: &[CassetteNoveltyCase]) -> f64 {
    let scorer = AtomicNoveltyScorer::default();
    let lex: Vec<_> = novelty_bundles
        .iter()
        .filter(|c| c.lexical_only_prior_art && !c.expected_novel)
        .collect();
    if lex.is_empty() {
        return 0.0;
    }
    let fp = lex
        .iter()
        .filter(|c| scorer.score(&c.bundle) == NoveltyVerdict::Novel)
        .count();
    round6(fp as f64 / lex.len() as f64)
}

async fn probe_doc_publish_wipes_artifact() -> anyhow::Result<bool> {
    let tmp = tempfile::tempdir()?;
    std::fs::create_dir_all(tmp.path().join("docs/src/architecture"))?;
    let db = VoxDb::connect(DbConfig::Memory).await?;
    let sid = db.create_research_session("sess-doc-wipe", "probe").await?;
    let rich = r#"{"plan":{"subqueries":["q1"]},"result":{"answer":"Grounded [1]"}}"#;
    db.store_research_artifact(sid, rich, "# Grounded Report")
        .await?;
    vox_orchestrator_mcp::gui_ops::research::write_research_doc(
        &db,
        tmp.path(),
        sid,
        "audit-doc-probe",
        "# Grounded Report",
    )
    .await
    .map_err(anyhow::Error::msg)?;
    let after = db
        .get_research_artifact(sid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("missing artifact after write_research_doc"))?;
    Ok(after.artifact_json.trim() == "{}")
}

async fn probe_verifier_behavior(
    claim_pairs: &[CassetteClaimPair],
) -> anyhow::Result<VerifierProbeMetrics> {
    let verifier = MiniCheckVerifier::from_env_with_lookup(|_| Err(std::env::VarError::NotPresent));
    let mock_verifier_fallback_rate = f64::from(u8::from(verifier.is_mock_backend()));
    let (mut sup_pred, mut true_sup, mut contra_tot, mut contra_det, mut contra_fp) =
        (0usize, 0usize, 0usize, 0usize, 0usize);

    for pair in claim_pairs {
        let out = verifier
            .verify_claim(&pair.claim, &pair.context)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let pred_contra = out.contradiction_score >= 0.6;
        let pred_sup = !pred_contra && !out.abstained && out.support_score >= 0.7;
        if pred_sup {
            sup_pred += 1;
            if pair.expected_verdict == "supported" {
                true_sup += 1;
            }
        }
        if pair.expected_verdict == "contradicted" {
            contra_tot += 1;
            if pred_contra {
                contra_det += 1;
            } else if pred_sup {
                contra_fp += 1;
            }
        }
    }
    let ratio = |num: usize, den: usize| {
        if den == 0 {
            0.0
        } else {
            round6(num as f64 / den as f64)
        }
    };
    Ok(VerifierProbeMetrics {
        claim_precision: ratio(true_sup, sup_pred),
        contradiction_detection_recall: ratio(contra_det, contra_tot),
        contradiction_false_support_rate: ratio(contra_fp, contra_tot),
        mock_verifier_fallback_rate,
    })
}

fn build_hermetic_server_state(sessions_dir: PathBuf) -> anyhow::Result<ServerState> {
    let mut cfg = OrchestratorConfig::default();
    cfg.memory.memory_md_path = PathBuf::from("__vox_hermetic_audit_no_memory__.md");
    let orch = Arc::new(Orchestrator::with_groups(
        cfg.clone(),
        AffinityGroupRegistry::new(vec![]),
    ));
    let session_cfg = SessionConfig {
        persist: false,
        sessions_dir,
        ..SessionConfig::default()
    };
    let sm = SessionManager::new(session_cfg).map_err(|e| anyhow::anyhow!("{e}"))?;
    let repository = RepositoryContext {
        root: PathBuf::from("."),
        git_root: None,
        repository_id: "scientia-v76-audit".into(),
        origin_url: None,
        capabilities: RepoCapabilities {
            vox_project: false,
            cargo_workspace: false,
            cargo_package: false,
            node_workspace: false,
            python_project: false,
            go_module: false,
            git: false,
        },
        has_vox_agents_dir: false,
        vox_toml: None,
    };
    Ok(ServerState::hermetic_stub(
        cfg,
        repository,
        orch,
        Arc::new(Mutex::new(sm)),
        vox_skills::new_registry_arc(),
    ))
}

async fn probe_kv_cache_prefix_stability() -> anyhow::Result<f64> {
    let tmp = tempfile::tempdir()?;
    let state = build_hermetic_server_state(tmp.path().join("sessions"))?;
    let sid = "audit-kv-session";
    let env_key = vox_orchestrator::socrates::session_context_envelope_key(sid);
    let retrieval: vox_orchestrator::socrates::SessionRetrievalEnvelope =
        serde_json::from_str("{}")?;
    let turns = [
        (
            0u64,
            OperatingMode::Generation,
            "crates/vox-scientia/src/lib.rs",
        ),
        (
            25_000,
            OperatingMode::PlanExecution { execution_depth: 1 },
            "crates/vox-orchestrator-mcp/src/chat_tools/mod.rs",
        ),
        (
            85_000,
            OperatingMode::Verification {
                reason: Some("verify citation grounding".into()),
            },
            "crates/vox-cli-research/src/eval.rs",
        ),
        (
            40_000,
            OperatingMode::PlanExecution { execution_depth: 2 },
            "crates/vox-db/src/research_session_pages.rs",
        ),
        (
            10_000,
            OperatingMode::Generation,
            "crates/vox-gui/ui/src/App.tsx",
        ),
    ];

    let mut prompts = Vec::with_capacity(turns.len());
    for (debit_ms, mode, active_file) in turns {
        let bm = state.orchestrator.budget_manager_handle();
        vox_orchestrator::sync_lock::rw_read(&*bm).add_questioning_attention_debit_ms(debit_ms);
        let mut env =
            ContextEnvelope::from_session_retrieval("scientia-v76-audit", sid, &retrieval);
        env.created_at_unix_ms = 1_700_000_000_000;
        env.expires_at_unix_ms = Some(1_700_003_600_000);
        env.envelope_id = "env-fixed-audit".into();
        env.operating_mode = Some(mode);
        if let Ok(g) = state.orchestrator.context_store().read() {
            g.set(
                vox_orchestrator::AgentId(0),
                env_key.clone(),
                serde_json::to_string(&env)?,
                3600,
            );
        }
        let raw = build_system_prompt_with_skill(&state, Some(sid), None, None, &[]).await;
        let vars = SystemPromptRuntimeVars {
            current_datetime: "2026-11-15T00:00:00Z".into(),
            user_platform: "linux".into(),
            active_spoke: "vox-scientia".into(),
            session_id: sid.into(),
            active_file: active_file.into(),
            ..SystemPromptRuntimeVars::default()
        };
        prompts.push(interpolate_system_prompt_variables(&raw, &vars));
    }

    let sum: f64 = prompts
        .windows(2)
        .map(|w| {
            let (a, b) = (w[0].as_bytes(), w[1].as_bytes());
            let common = a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count();
            common as f64 / a.len().max(b.len()).max(1) as f64
        })
        .sum();
    Ok(round6(sum / (prompts.len() - 1) as f64))
}

async fn probe_session_detail_ipc_payload_bytes() -> anyhow::Result<usize> {
    let tmp = tempfile::tempdir()?;
    let mut state = build_hermetic_server_state(tmp.path().join("sessions"))?;
    let db = VoxDb::connect(DbConfig::Memory).await?;
    let sid = db.create_research_session("sess-ipc-50x100", "q").await?;

    let citations: Vec<serde_json::Value> = (1..=50)
        .map(|i| {
            serde_json::json!({
                "source_id": i,
                "url": format!("https://scholar.example.org/work/{i:03}"),
                "title": format!("Benchmarking Multi-Hop Epistemic Retrieval Study #{i:03}"),
                "snippet": format!("Source #{i:03} demonstrates empirical grounding across distributed scholarly corpora with reproducible citation spans and calibrated confidence intervals."),
                "confidence": 0.91
            })
        })
        .collect();
    let claim_verdicts: Vec<serde_json::Value> = (1..=100u64)
        .map(|i| {
            serde_json::json!({
                "claim": {
                    "claim_id": i,
                    "text": format!("Atomic claim #{i:03}: multi-wave retrieval improves bridge-entity recall by {:.1}% across heterogeneous scholarly sources.", 12.0 + (i % 15) as f64)
                },
                "verdict": if i % 5 == 0 { "contested" } else { "supported" },
                "confidence": 0.88,
                "evidence_spans": [{
                    "source_id": ((i - 1) % 50) + 1,
                    "text": format!("Supporting span for claim #{i:03} from primary benchmark table."),
                    "span_type": "supporting"
                }],
                "resample_stability": 0.94
            })
        })
        .collect();
    let subqueries = ["subquery-1", "subquery-2", "subquery-3"];
    let artifact = serde_json::json!({
        "plan": { "subqueries": subqueries },
        "result": {
            "answer": "Comprehensive multi-wave synthesis grounded across 50 scholarly sources [1][2][3].",
            "citations": citations,
            "research_metadata": {
                "routing_tier": "deep", "confidence": 0.89, "source_count": 50,
                "claim_verdicts": claim_verdicts, "citation_audit": { "precision": 0.96 },
                "corroboration_counts": [[1, 3], [2, 2]], "wave_count": 3, "wave_stability": 0.92,
                "quality_score": 91, "synthesis_model": "scientia-audit-fixture",
                "subqueries": subqueries
            }
        },
        "report_markdown": "# Deep Research Audit Fixture\n\nGrounded synthesis across 50 sources and 100 claims."
    });
    let artifact_str = serde_json::to_string(&artifact)?;
    db.store_research_artifact(sid, &artifact_str, "# Deep Research Audit Fixture")
        .await?;
    state.db = Some(Arc::new(db));
    let resp = vox_orchestrator_mcp::gui_ops::inproc::api_v2_get(
        &state,
        &format!("/research/sessions/{sid}"),
    )
    .await
    .map_err(anyhow::Error::msg)?;
    Ok(serde_json::to_vec(&resp)?.len())
}

fn probe_gui_cli_db_spawn_count(repo_root: &Path) -> anyhow::Result<usize> {
    let mut count = 0usize;
    for rel in [
        "crates/vox-gui/ui/src/components/surfaces/Publications",
        "crates/vox-gui/ui/src/components/surfaces/Scientia",
    ] {
        let dir = repo_root.join(rel);
        if !dir.is_dir() {
            continue;
        }
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|d| d.path()))
            .filter(|p| {
                let s = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                p.is_file()
                    && (s.ends_with(".ts") || s.ends_with(".tsx"))
                    && !s.ends_with(".test.ts")
                    && !s.ends_with(".test.tsx")
            })
            .collect();
        entries.sort();
        for file in entries {
            let text = std::fs::read_to_string(&file)?;
            count += text.matches("'execute_command'").count();
            count += text.matches("\"execute_command\"").count();
        }
    }
    Ok(count)
}
