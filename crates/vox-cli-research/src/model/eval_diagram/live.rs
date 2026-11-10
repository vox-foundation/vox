//! The live half of the diagram-authoring eval: preconditions, unpinned model resolution with
//! probes, and the facade and Node backends. Nothing here runs in CI: the only entry is the ignored
//! test `eval_diagram_live::live_run`, which supplies every path and knob (no environment reads).
//!
//! Every precondition fails before the registry is read or any model is called. Models come only
//! from the registry's CodeGen decision under `CandidateScope::CloudOnly` (a scope, not a pin), and
//! every call goes through `vox_actor_runtime::llm::llm_chat` with the slot's one locked config.

use std::collections::BTreeMap;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig, llm_chat};
use vox_orchestrator::models::ranking::Exclusion;
use vox_orchestrator::models::{
    CandidateScope, ModelRegistry, ModelSelectionRequest, ModelSpec, ModelTier, ProviderType,
    SelectionIntent, SelectionReason, TaskCategory, decide, llm_config_for_spec,
};

pub use super::prereg::MERMAID_VERSION;
use super::prereg::{check_manifest, check_mermaid, check_report, check_sha, verify_freeze};
use super::run::{Backend, PROBE_LEDGER, ProbeRecord, Reply, RunConfig, RunOutcome};
use super::run::{check_cap, check_run_name, prior_spend, redact_error, refused, run_dataset};
use super::{ModelSlot, RunHeader};

const HUMAN: &str = "HUMAN ACTION REQUIRED";
/// Output budget, reasoning included. 4096 in the 2026-10-27 run, where reasoning models spent it
/// all thinking and wrote no answer on hard tasks; raised so the answer has room (user, 2026-10-27).
const MAX_TOKENS: u64 = 16384;
const SEED_BASE: u64 = 42;
const B_CANDIDATES: usize = 10;
const BATCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// Every path and knob of a live run, supplied by the ignored test.
#[derive(Debug, Clone)]
pub struct LiveConfig {
    pub repo_root: PathBuf,
    pub dataset_dir: PathBuf,
    pub report_path: PathBuf,
    pub run_name: String,
    /// The pre-registration commit (40 lowercase hex).
    pub freeze_commit: String,
    pub prereg_subject_prefix: String,
    pub max_spend_usd: f64,
}

/// Vendor organisation of a model id: the part before `/`, or the whole id.
#[must_use]
pub fn vendor_org(id: &str) -> &str {
    id.split_once('/').map_or(id, |(org, _)| org)
}

/// Only a registry score is a selection; an environment override or a configured premium alias is
/// a pin: refuse it.
pub fn refuse_override(reason: &SelectionReason) -> Result<()> {
    ensure!(
        matches!(reason, SelectionReason::Scored),
        "{HUMAN}: the selection is not a registry score ({reason}); unset the override or alias, the eval refuses pins"
    );
    Ok(())
}

/// Probe candidates in order, appending one ledger line per probe (answered or not) before the
/// next; returns the first that answers with a usable cost, and every probe. Refuses to probe
/// once the dataset's prior spend (runs and ledger) reaches `cap`. A provider refusal moves on to
/// the next candidate; any other probe without a usable cost may have been billed an unknown
/// amount, so probing stops for a human.
pub async fn probe_candidates<F, Fut>(
    dataset_dir: &Path,
    cap: f64,
    candidates: &[String],
    mut probe: F,
) -> Result<(Option<String>, Vec<ProbeRecord>)>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Reply, String>>,
{
    let mut records = Vec::new();
    for id in candidates {
        let spent = prior_spend(dataset_dir)?;
        ensure!(
            spent < cap,
            "{HUMAN}: spend {spent} USD has reached the cap {cap} USD; no further probe"
        );
        let reply = probe(id.clone()).await;
        let cost = reply
            .as_ref()
            .ok()
            .and_then(|r| r.cost_usd)
            .filter(|c| c.is_finite() && *c >= 0.0);
        let error = match &reply {
            Err(e) => Some(redact_error(e)),
            Ok(_) if cost.is_none() => Some("reply carried no usable cost".into()),
            Ok(_) => None,
        };
        let stop = error.as_deref().filter(|e| !refused(e)).map(String::from);
        let rec = ProbeRecord {
            time: chrono::Utc::now().to_rfc3339(),
            candidate: id.clone(),
            ok: error.is_none(),
            error,
            resolved_model: reply.as_ref().ok().map(|r| r.model.clone()),
            cost_usd: cost,
        };
        let mut ledger = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dataset_dir.join(PROBE_LEDGER))?;
        ledger.write_all(format!("{}\n", serde_json::to_string(&rec)?).as_bytes())?;
        ledger.flush()?;
        let ok = rec.ok;
        records.push(rec);
        if ok {
            return Ok((Some(id.clone()), records));
        }
        if let Some(e) = stop {
            bail!(
                "{HUMAN}: the probe of {id} may have been billed without a usable cost ({e}); the cap cannot hold, check the provider before probing again"
            );
        }
    }
    Ok((None, records))
}

/// One `llm_chat` call; the only LLM call path of this eval.
async fn chat_once(config: LlmConfig, messages: Vec<LlmChatMessage>) -> Result<Reply, String> {
    let started = std::time::Instant::now();
    let outcome = llm_chat(&vox_actor_runtime::ActivityOptions::new(), messages, config).await;
    let r = match outcome {
        vox_actor_runtime::ActivityResult::Ok(inner) => inner?,
        vox_actor_runtime::ActivityResult::Failed(e) => return Err(e.to_string()),
        vox_actor_runtime::ActivityResult::Cancelled => return Err("activity cancelled".into()),
    };
    Ok(Reply {
        content: r.content,
        model: r.model,
        prompt_tokens: r.prompt_tokens.into(),
        completion_tokens: r.completion_tokens.into(),
        cost_usd: r.cost_usd,
        latency_ms: started.elapsed().as_millis() as u64,
        finish_reason: r.finish_reason,
        reasoning_tokens: r.reasoning_tokens.into(),
    })
}

/// The locked config of a slot: temperature 0, [`MAX_TOKENS`], no tools; the seed is set per call.
fn slot_config(spec: &ModelSpec) -> LlmConfig {
    let mut c = llm_config_for_spec(spec, TaskCategory::CodeGen);
    c.temperature = Some(0.0);
    c.max_tokens = Some(MAX_TOKENS);
    c.telemetry_task_category = Some("eval-diagram".into());
    c
}

/// The live backend: `llm_chat` with each slot's locked config, the Node Mermaid batch oracle.
pub struct LiveBackend {
    repo_root: PathBuf,
    configs: BTreeMap<String, LlmConfig>,
}

impl Backend for LiveBackend {
    async fn chat(
        &mut self,
        slot: &str,
        messages: Vec<LlmChatMessage>,
        seed: u64,
    ) -> Result<Reply, String> {
        let mut config = self.configs.get(slot).ok_or("unknown slot")?.clone();
        config.seed = Some(seed);
        chat_once(config, messages).await
    }

    async fn mermaid_batch(&mut self, inputs_jsonl: &str) -> Result<String> {
        use tokio::io::AsyncWriteExt;
        let script = ["docs-astro", "tests", "lib", "mermaid-batch.mjs"]
            .iter()
            .fold(self.repo_root.clone(), |p, s| p.join(s));
        let mut child = tokio::process::Command::new("node")
            .arg(&script)
            .current_dir(&self.repo_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("spawn node")?;
        let mut stdin = child.stdin.take().context("node stdin")?;
        let input = inputs_jsonl.to_owned();
        let writer = tokio::spawn(async move { stdin.write_all(input.as_bytes()).await });
        let out = tokio::time::timeout(BATCH_TIMEOUT, child.wait_with_output())
            .await
            .context("Mermaid batch killed after 600 s")??;
        let wrote = writer.await?;
        ensure!(
            out.status.success(),
            "Mermaid batch exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
        wrote.context("write node stdin")?;
        Ok(String::from_utf8(out.stdout)?)
    }
}

fn rel(root: &Path, path: &Path) -> Result<String> {
    let r = path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?;
    Ok(r.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

/// Every precondition, in order, before any registry read or model call; returns the pinned hashes.
fn preconditions(cfg: &LiveConfig) -> Result<BTreeMap<String, String>> {
    check_cap(cfg.max_spend_usd)?;
    let f = cfg.freeze_commit.as_str();
    check_sha(f)?;
    let git = |args: &[&str]| vox_git::read_only(&cfg.repo_root, args).map_err(|e| e.to_string());
    let subject = git(&["log", "-1", "--format=%s", f]);
    let (ancestry, changed) = match &subject {
        Err(_) => (String::new(), String::new()),
        Ok(_) => {
            let ds = rel(&cfg.repo_root, &cfg.dataset_dir)?;
            let report = rel(&cfg.repo_root, &cfg.report_path)?;
            let range = format!("{f}^..HEAD");
            let (runs, ledger) = (
                format!(":(exclude){ds}/runs"),
                format!(":(exclude){ds}/{PROBE_LEDGER}"),
            );
            let diff = ["diff", "--name-only", f, "--", &ds, &runs, &ledger, &report];
            (
                git(&["log", "--format=%H", &range]).map_err(anyhow::Error::msg)?,
                git(&diff).map_err(anyhow::Error::msg)?,
            )
        }
    };
    let prefix = cfg.prereg_subject_prefix.as_str();
    verify_freeze(
        f,
        prefix,
        subject.as_deref().map_err(String::as_str),
        &ancestry,
        &changed,
    )?;
    let read =
        |p: &Path| std::fs::read_to_string(p).with_context(|| format!("read {}", p.display()));
    let hashes = check_manifest(
        &read(&cfg.dataset_dir.join("manifest.v1.yaml"))?,
        &cfg.dataset_dir,
    )?;
    check_report(&read(&cfg.report_path)?)?;
    let pkg = ["docs-astro", "node_modules", "mermaid", "package.json"]
        .iter()
        .fold(cfg.repo_root.clone(), |p, s| p.join(s));
    check_mermaid(&read(&pkg)?)?;
    check_run_name(&cfg.run_name)?;
    let run_dir = cfg.dataset_dir.join("runs").join(&cfg.run_name);
    ensure!(
        !run_dir.exists(),
        "run directory {} already exists",
        run_dir.display()
    );
    let spent = prior_spend(&cfg.dataset_dir)?;
    ensure!(
        spent < cfg.max_spend_usd,
        "spend {spent} USD already reaches the cap {} USD",
        cfg.max_spend_usd
    );
    Ok(hashes)
}

fn is_local(p: &ProviderType) -> bool {
    matches!(
        p,
        ProviderType::Ollama | ProviderType::VoxLocal | ProviderType::PopuliMesh
    )
}

/// Probe `ids` in order with each model's locked config; the first answering one becomes `slot`.
async fn resolve_slot(
    cfg: &LiveConfig,
    registry: &ModelRegistry,
    slot: &str,
    ids: &[String],
    probes: &mut Vec<ProbeRecord>,
) -> Result<Option<(ModelSlot, LlmConfig)>> {
    let configs: BTreeMap<String, LlmConfig> = ids
        .iter()
        .filter_map(|id| registry.get(id).map(|s| (id.clone(), slot_config(&s))))
        .collect();
    let ids: Vec<String> = ids
        .iter()
        .filter(|i| configs.contains_key(*i))
        .cloned()
        .collect();
    let ping = vec![LlmChatMessage {
        role: "user".into(),
        content: "Reply with the single word OK.".into(),
        ..Default::default()
    }];
    let (hit, recs) = probe_candidates(&cfg.dataset_dir, cfg.max_spend_usd, &ids, |id| {
        chat_once(configs[&id].clone(), ping.clone())
    })
    .await?;
    probes.extend(recs);
    Ok(hit.map(|id| {
        let c = configs[&id].clone();
        let s = ModelSlot {
            slot: slot.into(),
            family: vendor_org(&id).into(),
            provider: c.provider.clone(),
            requested_id: id,
        };
        (s, c)
    }))
}

/// The live run: preconditions, selection (CloudOnly, env override refused), probes, then the run.
pub async fn run_live(cfg: &LiveConfig) -> Result<RunOutcome> {
    let hashes = preconditions(cfg).context(format!("{HUMAN}: live-run precondition failed"))?;
    let registry = ModelRegistry::from_cache();
    let intent = SelectionIntent::for_task(TaskCategory::CodeGen);
    let mut req = ModelSelectionRequest::from_intent(intent.clone());
    req.candidate_scope = CandidateScope::CloudOnly;
    let d = decide(&req, &registry)
        .with_context(|| format!("{HUMAN}: no cloud model is eligible for CodeGen; configure a provider key through Clavis"))?;
    refuse_override(&d.outcome.reason)?;
    let a_ids: Vec<String> = std::iter::once(d.selected_model.clone())
        .chain(d.alternatives.iter().cloned())
        .collect();
    let mut probes = Vec::new();
    let Some((a, a_cfg)) = resolve_slot(cfg, &registry, "A", &a_ids, &mut probes).await? else {
        bail!(
            "{HUMAN}: no cloud model answered the probe; probes: {}",
            serde_json::to_string(&probes)?
        );
    };
    let org_a = a.family.clone();
    let ranked = registry.rank_with_filter(
        intent.task,
        intent.complexity,
        intent.axes.to_cost_preference(),
        intent.allow_free_in_performance_mode,
        |m| {
            if is_local(&m.provider_type) || vendor_org(&m.id) == org_a {
                Some(Exclusion::Filtered)
            } else if intent.axes.intelligence < 50 && m.capabilities.tier == ModelTier::Elite {
                Some(Exclusion::FlagshipExcludedByMode)
            } else {
                None
            }
        },
        None,
    );
    let b_ids: Vec<String> = ranked
        .ranked
        .iter()
        .take(B_CANDIDATES)
        .map(|r| r.spec.id.clone())
        .collect();
    let b = resolve_slot(cfg, &registry, "B", &b_ids, &mut probes).await?;
    let mut configs = BTreeMap::from([("A".to_string(), a_cfg)]);
    let mut models = vec![a];
    if let Some((s, c)) = b {
        configs.insert("B".into(), c);
        models.push(s);
    }
    let card_hashes = ["voxdiagram", "mermaid"]
        .map(|arm| {
            (
                arm.to_string(),
                hashes
                    .get(&format!("cards/{arm}.txt"))
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .into();
    let header = RunHeader {
        schema_version: 1,
        synthetic: false,
        freeze_commit: cfg.freeze_commit.clone(),
        tasks_hash: hashes.get("tasks.v1.jsonl").cloned().unwrap_or_default(),
        card_hashes,
        prompt_template_hash: hashes
            .get("prompt-template.txt")
            .cloned()
            .unwrap_or_default(),
        intent: "codegen".into(),
        scope: "cloud_only".into(),
        selection: serde_json::json!({
            "selected": d.selected_model,
            "reason": d.outcome.reason.to_string(),
            "alternatives": d.alternatives,
        }),
        models,
        temperature: 0.0,
        seed_base: SEED_BASE,
        max_tokens: MAX_TOKENS as u32,
        max_spend_usd: cfg.max_spend_usd,
        mermaid_version: MERMAID_VERSION.into(),
        vox_version: env!("CARGO_PKG_VERSION").into(),
        probes: serde_json::to_value(&probes)?,
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    let mut backend = LiveBackend {
        repo_root: cfg.repo_root.clone(),
        configs,
    };
    let run = RunConfig {
        dataset_dir: cfg.dataset_dir.clone(),
        run_name: cfg.run_name.clone(),
        max_spend_usd: cfg.max_spend_usd,
    };
    run_dataset(&mut backend, &run, header).await
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
