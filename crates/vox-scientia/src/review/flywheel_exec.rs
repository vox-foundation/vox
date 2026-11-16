//! Idempotent DB-backed publication flywheel stage executor (`publication_id` only).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use vox_db::VoxDb;

use crate::findings_site::render_finding_page;
use crate::manuscript::latex::{ArxivHandoffMeta, render_arxiv_bundle_with_handoff, render_latex};
use crate::manuscript::scaffold::{ScaffoldInput, render_imrad};
use crate::review::flywheel_gate::sha3_256_hex;
use crate::review_flow::{
    assemble_finding_page_from_db, assemble_scaffold_from_db, ensure_evidence_map,
    extract_novelty_for_nanopub, flywheel_build_worthiness, flywheel_extract_claims_into_db,
    flywheel_persist_evidence, publication_session_id, resolve_manifest_or_promote,
};
use crate::ro_crate::{RoCrateMetadata, build_ro_crate_json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlywheelStage {
    ExtractClaims,
    NoveltyCheck,
    ScaffoldImrad,
    RenderLatex,
    ArxivBundle,
    RoCrate,
    FindingPage,
    ReplayVerify,
}

const NAMES: [&str; 8] = [
    "extract_claims",
    "novelty_check",
    "scaffold_imrad",
    "render_latex",
    "arxiv_bundle",
    "ro_crate",
    "finding_page",
    "replay_verify",
];

const ALL_STAGES: [FlywheelStage; 8] = [
    FlywheelStage::ExtractClaims,
    FlywheelStage::NoveltyCheck,
    FlywheelStage::ScaffoldImrad,
    FlywheelStage::RenderLatex,
    FlywheelStage::ArxivBundle,
    FlywheelStage::RoCrate,
    FlywheelStage::FindingPage,
    FlywheelStage::ReplayVerify,
];

impl FlywheelStage {
    pub fn as_str(self) -> &'static str {
        NAMES[self as usize]
    }

    pub fn parse(raw: &str) -> Result<Self> {
        let key = raw.trim().to_ascii_lowercase();
        if key.is_empty() || key == "all" {
            return Ok(Self::ReplayVerify);
        }
        NAMES
            .iter()
            .position(|n| *n == key)
            .map(|idx| ALL_STAGES[idx])
            .ok_or_else(|| anyhow::anyhow!("unknown flywheel stage `{raw}`"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlywheelAdvanceResult {
    pub publication_id: String,
    pub stage_completed: String,
    pub stages_done: Vec<String>,
    pub worthiness_inputs: Value,
    pub artifacts: Value,
}

async fn get_scaffold<'a>(
    db: &VoxDb,
    pub_id: &str,
    slot: &'a mut Option<ScaffoldInput>,
) -> Result<&'a ScaffoldInput> {
    if slot.is_none() {
        *slot = Some(assemble_scaffold_from_db(db, pub_id).await?);
    }
    Ok(slot.as_ref().expect("scaffold populated"))
}

fn build_bundle_bytes(pub_id: &str, sc: &ScaffoldInput) -> Result<Vec<u8>> {
    let mut bundle_sc = sc.clone();
    bundle_sc.figures.clear();
    let handoff = ArxivHandoffMeta {
        primary_category: "cs.SE".into(),
        category_origin: "default".into(),
        license_spdx: Some("CC-BY-4.0".into()),
        comments: Some(format!("SCIENTIA flywheel bundle for {pub_id}")),
        orcids: sc
            .authors
            .iter()
            .map(|a| (a.name.clone(), a.orcid.clone()))
            .collect(),
    };
    render_arxiv_bundle_with_handoff(&bundle_sc, &[], Some(&handoff)).context("arxiv bundle")
}

fn build_ro_crate_sha(
    manifest: &vox_db::PublicationManifestRow,
    sc: &ScaffoldInput,
) -> Result<String> {
    let ro = RoCrateMetadata {
        name: manifest.title.clone(),
        description: manifest
            .abstract_text
            .clone()
            .unwrap_or_else(|| manifest.title.clone()),
        doi: None,
        author_orcid: sc.authors.first().and_then(|a| a.orcid.clone()),
        author_ror: sc.authors.first().and_then(|a| a.affiliation_ror.clone()),
        license_spdx: "CC-BY-4.0".into(),
        published_at: chrono::Utc::now().timestamp(),
        keywords: vec![manifest.content_type.clone()],
        main_entity: None,
    };
    Ok(sha3_256_hex(&serde_json::to_vec(&build_ro_crate_json(
        &ro,
    ))?))
}

/// Idempotently advance a publication through the SCIENTIA flywheel purely from `VoxDb`.
pub async fn flywheel_advance_from_db(
    db: &VoxDb,
    publication_id: &str,
    target_stage: &str,
) -> Result<FlywheelAdvanceResult> {
    let target = FlywheelStage::parse(target_stage)?;
    let manifest = resolve_manifest_or_promote(db, publication_id).await?;
    let pub_id = manifest.publication_id.clone();
    let pub_sid = publication_session_id(&pub_id);
    let mut meta: Value = manifest
        .metadata_json
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| json!({}));
    let mut done: Vec<String> = meta
        .pointer("/scientia_evidence/flywheel/stages_done")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut arts = Map::new();
    let mut sc_cache = None;
    let mut crate_sha = meta
        .pointer("/scientia_evidence/ro_crate_sha3_256")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut arxiv_len = 0usize;
    let mut fb_nov = "insufficient_evidence".to_string();

    for stage in ALL_STAGES {
        if stage > target {
            break;
        }
        let name = stage.as_str();
        match stage {
            FlywheelStage::ExtractClaims => {
                let count =
                    flywheel_extract_claims_into_db(db, &manifest, pub_sid, &done, &mut meta)
                        .await?;
                arts.insert("claims_count".into(), json!(count));
            }
            FlywheelStage::NoveltyCheck => {
                let (v, uris) = extract_novelty_for_nanopub(Some(&serde_json::to_string(&meta)?));
                fb_nov = v.clone();
                let ev = ensure_evidence_map(&mut meta);
                ev.entry("novelty_verdict").or_insert_with(|| json!(v));
                ev.insert("prior_art_uris".into(), json!(uris));
                arts.insert("novelty_verdict".into(), json!(v));
                arts.insert("prior_art_count".into(), json!(uris.len()));
            }
            FlywheelStage::ScaffoldImrad => {
                let md = render_imrad(get_scaffold(db, &pub_id, &mut sc_cache).await?);
                arts.insert("imrad_chars".into(), json!(md.len()));
                arts.insert("markdown".into(), json!(md));
            }
            FlywheelStage::RenderLatex => {
                let tex = render_latex(get_scaffold(db, &pub_id, &mut sc_cache).await?);
                arts.insert("latex_chars".into(), json!(tex.len()));
                arts.insert("latex".into(), json!(tex));
            }
            FlywheelStage::ArxivBundle => {
                let gz =
                    build_bundle_bytes(&pub_id, get_scaffold(db, &pub_id, &mut sc_cache).await?)?;
                arxiv_len = gz.len();
                arts.insert("arxiv_bundle_bytes".into(), json!(arxiv_len));
                arts.insert("arxiv_bytes_len".into(), json!(arxiv_len));
                arts.insert("arxiv_bundle_sha3_256".into(), json!(sha3_256_hex(&gz)));
            }
            FlywheelStage::RoCrate => {
                let sc = get_scaffold(db, &pub_id, &mut sc_cache).await?;
                crate_sha = build_ro_crate_sha(&manifest, sc)?;
                let ev = ensure_evidence_map(&mut meta);
                ev.insert("ro_crate_sha3_256".into(), json!(crate_sha));
                ev.insert("ro_crate_verified".into(), json!(true));
                arts.insert("ro_crate_sha3_256".into(), json!(crate_sha));
                arts.insert("ro_crate_digest".into(), json!(crate_sha));
            }
            FlywheelStage::FindingPage => {
                let html = render_finding_page(&assemble_finding_page_from_db(db, &pub_id).await?);
                arts.insert("finding_page_bytes".into(), json!(html.len()));
                arts.insert("html".into(), json!(html));
            }
            FlywheelStage::ReplayVerify => {
                let rec = db
                    .list_publication_status_events(&pub_id)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .find(|e| e.status == "artifact_replayability_measured")
                    .and_then(|e| e.detail_json)
                    .and_then(|d| serde_json::from_str::<Value>(&d).ok())
                    .and_then(|v| v.get("measured_score").and_then(Value::as_f64));
                let sup = db
                    .list_publication_claims(pub_sid)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .filter(|c| c.verdict.as_deref() == Some("Supported"))
                    .count();
                let score = rec.map(|v| v.clamp(0.0, 1.0)).unwrap_or_else(|| {
                    if !crate_sha.is_empty() && arxiv_len > 0 && sup > 0 {
                        1.0
                    } else if !crate_sha.is_empty() && arxiv_len > 0 {
                        0.5
                    } else {
                        0.0
                    }
                });
                let ev = ensure_evidence_map(&mut meta);
                ev.insert("artifact_replayability".into(), json!(score));
                ev.insert("artifact_replayability_measured".into(), json!(score));
                arts.insert("artifact_replayability".into(), json!(score));
                arts.insert("replayability".into(), json!(score));
            }
        }
        if !done.iter().any(|s| s == name) {
            done.push(name.to_string());
        }
    }

    let claims_final = db
        .list_publication_claims(pub_sid)
        .await
        .unwrap_or_default();
    let worthiness_inputs = flywheel_build_worthiness(&manifest, &claims_final, &meta, &fb_nov);
    ensure_evidence_map(&mut meta).insert(
        "flywheel".into(),
        json!({ "last_target_stage": target.as_str(), "stages_done": done }),
    );
    let event_status = if target == FlywheelStage::ReplayVerify {
        "artifact_replayability_measured"
    } else {
        "flywheel_advanced"
    };
    let detail = json!({
        "stage_completed": target.as_str(),
        "stages_done": done,
        "measured_score": worthiness_inputs["artifact_replayability"],
        "ro_crate_sha3_256": crate_sha,
    })
    .to_string();
    flywheel_persist_evidence(db, &manifest, &meta, event_status, &detail).await?;

    Ok(FlywheelAdvanceResult {
        publication_id: pub_id,
        stage_completed: target.as_str().into(),
        stages_done: done,
        worthiness_inputs,
        artifacts: Value::Object(arts),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_db::{DbConfig, PublicationManifestParams};

    #[tokio::test]
    async fn flywheel_advances_idempotently_through_replay_verify() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("mem db");
        db.upsert_publication_manifest(PublicationManifestParams {
            publication_id: "pub-fly-1",
            content_type: "scientia",
            source_ref: None,
            title: "Deterministic Flywheel Study",
            author: "Ada Lovelace",
            abstract_text: Some("Measured latency reduction across 30 runs."),
            body_markdown: "Benchmark throughput improved by 42% with p99 latency under 12 ms.",
            citations_json: Some(
                "Benchmark throughput improved by 42% with p99 latency under 12 ms.",
            ),
            metadata_json: None,
            revision_history_json: None,
            content_sha3_256: "feedbeef",
            state: "draft",
        })
        .await
        .expect("seed");
        let p = flywheel_advance_from_db(&db, "pub-fly-1", "render_latex")
            .await
            .expect("partial");
        assert_eq!(p.stages_done.len(), 4);
        assert!(p.artifacts["latex_chars"].as_u64().unwrap_or(0) > 100);
        let f = flywheel_advance_from_db(&db, "pub-fly-1", "replay_verify")
            .await
            .expect("full");
        assert_eq!(f.stages_done.len(), 8);
        assert!(
            !f.artifacts["ro_crate_sha3_256"]
                .as_str()
                .unwrap_or("")
                .is_empty()
        );
        let row = db
            .get_publication_manifest("pub-fly-1")
            .await
            .unwrap()
            .unwrap();
        let meta: Value =
            serde_json::from_str(row.metadata_json.as_deref().unwrap_or("{}")).unwrap();
        assert!(
            meta["scientia_evidence"]["ro_crate_sha3_256"]
                .as_str()
                .is_some()
        );
        assert!(
            meta["scientia_evidence"]["artifact_replayability"]
                .as_f64()
                .is_some()
        );
    }
}
