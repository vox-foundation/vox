//! The run loop of the diagram-authoring eval: rounds of model calls through a [`Backend`], the
//! enforced spend cap, the append-only recording and the derived files.
//!
//! Per slot (A, then B): turn 0 for every task (VoxDiagram then Mermaid, task order), one Mermaid
//! batch, turn 1 for exactly the invalid (task, arm) pairs, a batch, turn 2. Both arms of a task and
//! turn share one seed. Every attempt is appended to `attempts.jsonl` as it happens, so a crash
//! leaves a valid partial recording; a stop line (`spend_cap`, `cost_unknown`, `oracle_failure`)
//! ends a run early. The derived files are then written exactly as the offline re-score derives them.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use vox_actor_runtime::llm::LlmChatMessage;

use super::oracle::{Judged, judge, oracle_map};
use super::prompt::{Template, parse_template, render_messages};
use super::render::render_tables;
use super::score::{check_recording, oracle_inputs, oracle_text};
use super::tally::tally;
use super::{
    Arm, Attempt, Line, MAX_REPAIR_TURNS, ModelSlot, OracleInput, OracleOutput, Rule, RunHeader,
    Stop, Task, parse_manifest, read_jsonl, write_jsonl,
};
use crate::commands::model::eval_corpus::{repair_seed, sample_seed};

/// The probe ledger in the dataset directory: one line per probe call, answered or not.
pub const PROBE_LEDGER: &str = "probe-spend.jsonl";
/// Highest spend cap a run may be given (user decision 2026-10-27); also the default.
pub const MAX_SPEND_CAP_USD: f64 = 25.0;
/// Transport retries of one call, on the same slot config.
pub const MAX_TRANSPORT_RETRIES: u32 = 3;

const SPEND_CAP: &str = "spend_cap";
const COST_UNKNOWN: &str = "cost_unknown";
const ORACLE_FAILURE: &str = "oracle_failure";

/// One model reply as the run records it.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub content: String,
    /// Provider-reported model id.
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    /// `None` when the facade could not price the call: the run stops, the cap cannot hold.
    pub cost_usd: Option<f64>,
    pub latency_ms: u64,
}

/// Where replies and Mermaid verdicts come from: the facade and Node in a live run, a stub in tests.
#[allow(async_fn_in_trait)] // reason: called only through generics inside this crate; no Send bound needed
pub trait Backend {
    /// One chat call on `slot`'s locked config; `Err` is a transport error.
    async fn chat(
        &mut self,
        slot: &str,
        messages: Vec<LlmChatMessage>,
        seed: u64,
    ) -> Result<Reply, String>;
    /// Judge an oracle-inputs JSONL text; returns the oracle's JSONL output verbatim.
    async fn mermaid_batch(&mut self, inputs_jsonl: &str) -> Result<String>;
}

/// One line of the probe ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbeRecord {
    pub time: String,
    pub candidate: String,
    pub ok: bool,
    pub error: Option<String>,
    pub resolved_model: Option<String>,
    pub cost_usd: Option<f64>,
}

/// Refuse a cap outside (0, 25] USD.
pub fn check_cap(cap: f64) -> Result<()> {
    ensure!(
        cap > 0.0 && cap <= MAX_SPEND_CAP_USD,
        "spend cap {cap} USD outside (0, {MAX_SPEND_CAP_USD}]"
    );
    Ok(())
}

/// Spend already counted against the cap: every attempt cost under `runs/*/attempts.jsonl` plus
/// every cost in the probe ledger (a missing file counts 0, a null cost counts 0). The header's
/// probe list is provenance only and never summed here, so nothing counts twice.
pub fn prior_spend(dataset_dir: &Path) -> Result<f64> {
    let mut total = 0.0;
    match std::fs::read_dir(dataset_dir.join("runs")) {
        Ok(dir) => {
            for entry in dir {
                let path = entry?.path().join("attempts.jsonl");
                if !path.exists() {
                    continue;
                }
                let text = std::fs::read_to_string(&path)?;
                for line in read_jsonl::<Line>(&text).with_context(|| path.display().to_string())? {
                    if let Line::Attempt(a) = line {
                        total += a.cost_usd;
                    }
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("runs/"),
    }
    match std::fs::read_to_string(dataset_dir.join(PROBE_LEDGER)) {
        Ok(text) => {
            for p in read_jsonl::<ProbeRecord>(&text).context(PROBE_LEDGER)? {
                total += p.cost_usd.unwrap_or(0.0);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context(PROBE_LEDGER),
    }
    Ok(total)
}

/// What a run needs besides the backend and header; every path is a parameter.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub dataset_dir: PathBuf,
    pub run_name: String,
    pub max_spend_usd: f64,
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq)]
pub struct RunOutcome {
    pub run_dir: PathBuf,
    pub calls: usize,
    /// Cumulative spend counted against the cap (earlier runs and probes included).
    pub spent_usd: f64,
    pub stopped: Option<String>,
    /// Oracle errors and recording-check violations; any entry makes the run incomplete.
    pub problems: Vec<String>,
    pub complete: bool,
}

/// The frozen inputs of a dataset directory.
struct Dataset {
    rule: Rule,
    tasks: Vec<Task>,
    refs: Vec<OracleOutput>,
    template: Template,
    cards: [String; 2],
}

impl Dataset {
    fn load(dir: &Path) -> Result<Self> {
        let read = |p: PathBuf| {
            std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))
        };
        let cards = dir.join("cards");
        Ok(Self {
            rule: parse_manifest(&read(dir.join("manifest.v1.yaml"))?)?.decision_rule,
            tasks: read_jsonl(&read(dir.join("tasks.v1.jsonl"))?)?,
            refs: read_jsonl(&read(dir.join("reference-outputs.jsonl"))?)?,
            template: parse_template(&read(dir.join("prompt-template.txt"))?)?,
            cards: [
                read(cards.join("voxdiagram.txt"))?,
                read(cards.join("mermaid.txt"))?,
            ],
        })
    }

    fn card(&self, arm: Arm) -> &str {
        &self.cards[usize::from(arm == Arm::Mermaid)]
    }
}

struct Budget {
    spent: f64,
    cap: f64,
    calls: usize,
}

/// The append-only recording: every line is written and flushed before the run goes on.
struct Recorder {
    file: std::fs::File,
    lines: Vec<Line>,
}

impl Recorder {
    fn push(&mut self, line: Line) -> Result<()> {
        let mut text = serde_json::to_string(&line)?;
        text.push('\n');
        self.file.write_all(text.as_bytes())?;
        self.file.flush()?;
        self.lines.push(line);
        Ok(())
    }
}

/// Run every slot of `header.models` over the dataset; refuses an existing run directory and a cap
/// outside (0, 25] before any call.
pub async fn run_dataset<B: Backend>(
    backend: &mut B,
    cfg: &RunConfig,
    header: RunHeader,
) -> Result<RunOutcome> {
    check_cap(cfg.max_spend_usd)?;
    let run_dir = cfg.dataset_dir.join("runs").join(&cfg.run_name);
    ensure!(
        !run_dir.exists(),
        "run directory {} already exists; a run is never overwritten",
        run_dir.display()
    );
    let ds = Dataset::load(&cfg.dataset_dir)?;
    let mut budget = Budget {
        spent: prior_spend(&cfg.dataset_dir)?,
        cap: cfg.max_spend_usd,
        calls: 0,
    };
    std::fs::create_dir_all(&run_dir)?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(run_dir.join("attempts.jsonl"))?;
    let mut rec = Recorder {
        file,
        lines: Vec::new(),
    };
    let (slots, seed_base) = (header.models.clone(), header.seed_base);
    rec.push(Line::Run(header))?;
    let mut problems = Vec::new();
    let mut stopped = None;
    for slot in &slots {
        if let Some((reason, detail)) =
            run_slot(backend, &ds, slot, seed_base, &mut budget, &mut rec).await?
        {
            problems.extend(detail);
            stopped = Some(reason.to_string());
            break;
        }
    }
    if let Some(reason) = &stopped {
        rec.push(Line::Stop(Stop {
            reason: reason.clone(),
            spent_usd: budget.spent,
        }))?;
    }
    if let Err(e) = finish(backend, &ds, &run_dir, &rec.lines).await {
        problems.push(format!("{e:#}"));
    }
    Ok(RunOutcome {
        run_dir,
        calls: budget.calls,
        spent_usd: budget.spent,
        complete: stopped.is_none() && problems.is_empty(),
        stopped,
        problems,
    })
}

type StopReason = Option<(&'static str, Option<String>)>;

// ponytail: sequential calls keep the cap exact; add bounded concurrency with a spend reservation
// only if wall time matters.
async fn run_slot<B: Backend>(
    b: &mut B,
    ds: &Dataset,
    slot: &ModelSlot,
    seed_base: u64,
    budget: &mut Budget,
    rec: &mut Recorder,
) -> Result<StopReason> {
    let mut pending: Vec<(usize, Arm, Vec<(String, String)>)> = (0..ds.tasks.len())
        .flat_map(|i| Arm::BOTH.map(|arm| (i, arm, Vec::new())))
        .collect();
    for turn in 0..=MAX_REPAIR_TURNS {
        let mut round = Vec::with_capacity(pending.len());
        for (i, arm, history) in std::mem::take(&mut pending) {
            let t = &ds.tasks[i];
            let seed = match turn {
                0 => sample_seed(seed_base, &t.id, 0),
                _ => repair_seed(seed_base, &t.id, turn, 0),
            };
            let messages = render_messages(&ds.template, ds.card(arm), arm, &t.prompt, &history);
            let mut a = Attempt {
                task_id: t.id.clone(),
                arm,
                slot: slot.slot.clone(),
                turn,
                seed,
                requested_model: slot.requested_id.clone(),
                resolved_model: None,
                feedback: history.last().map(|(_, f)| f.clone()),
                raw_response: None,
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_usd: 0.0,
                latency_ms: 0,
                transport_error: None,
                retries: 0,
            };
            let Some(cost_known) = call(b, budget, &slot.slot, messages, &mut a).await else {
                return Ok(Some((SPEND_CAP, None)));
            };
            rec.push(Line::Attempt(a.clone()))?;
            if !cost_known {
                return Ok(Some((COST_UNKNOWN, None)));
            }
            round.push((i, arm, history, a));
        }
        if turn == MAX_REPAIR_TURNS || round.is_empty() {
            break;
        }
        let inputs: Vec<OracleInput> = round
            .iter()
            .filter(|r| r.1 == Arm::Mermaid)
            .filter_map(|(.., a)| {
                Some(OracleInput {
                    id: a.oracle_id(),
                    text: oracle_text(a)?,
                    expected: None,
                })
            })
            .collect();
        let outs: Vec<OracleOutput> = if inputs.is_empty() {
            Vec::new()
        } else {
            match batch(b, &write_jsonl(&inputs)?).await {
                Ok(o) => o,
                Err(e) => return Ok(Some((ORACLE_FAILURE, Some(format!("{e:#}"))))),
            }
        };
        let oracle = oracle_map(&outs);
        for (i, arm, mut history, a) in round {
            match judge(&a, &oracle) {
                Ok(Judged::Invalid(feedback)) => {
                    history.push((a.raw_response.clone().unwrap_or_default(), feedback));
                    pending.push((i, arm, history));
                }
                Ok(_) => {}
                Err(e) => return Ok(Some((ORACLE_FAILURE, Some(format!("{e:#}"))))),
            }
        }
    }
    Ok(None)
}

/// One model call: the cap is checked before it and before every transport retry (same slot,
/// at most [`MAX_TRANSPORT_RETRIES`]). `None` when the cap forbids the call; otherwise whether
/// the reply carried a usable cost (a transport failure carries none and costs nothing known).
async fn call<B: Backend>(
    b: &mut B,
    budget: &mut Budget,
    slot: &str,
    messages: Vec<LlmChatMessage>,
    a: &mut Attempt,
) -> Option<bool> {
    loop {
        if budget.spent >= budget.cap {
            return None;
        }
        budget.calls += 1;
        match b.chat(slot, messages.clone(), a.seed).await {
            Ok(r) => {
                let cost = r.cost_usd.filter(|c| c.is_finite() && *c >= 0.0);
                budget.spent += cost.unwrap_or(0.0);
                a.resolved_model = Some(r.model);
                a.raw_response = Some(r.content);
                a.prompt_tokens = r.prompt_tokens;
                a.completion_tokens = r.completion_tokens;
                a.cost_usd = cost.unwrap_or(0.0);
                a.latency_ms = r.latency_ms;
                return Some(cost.is_some());
            }
            Err(_) if a.retries < MAX_TRANSPORT_RETRIES => a.retries += 1,
            Err(e) => {
                a.transport_error = Some(e);
                return Some(true);
            }
        }
    }
}

async fn batch<B: Backend>(b: &mut B, inputs: &str) -> Result<Vec<OracleOutput>> {
    let out: Vec<OracleOutput> = read_jsonl(&b.mermaid_batch(inputs).await?)?;
    let want = read_jsonl::<OracleInput>(inputs)?.len();
    if out.len() != want {
        bail!(
            "Mermaid oracle returned {} lines for {want} inputs",
            out.len()
        );
    }
    Ok(out)
}

/// Derived files, exactly as the offline re-score derives them: oracle inputs, the oracle's own
/// output bytes, then (if the recording checks pass) tallies and tables.
async fn finish<B: Backend>(b: &mut B, ds: &Dataset, run_dir: &Path, lines: &[Line]) -> Result<()> {
    let inputs = oracle_inputs(&ds.tasks, &ds.refs, lines)?;
    std::fs::write(run_dir.join("oracle-inputs.jsonl"), &inputs)?;
    let raw = b
        .mermaid_batch(&inputs)
        .await
        .context("final Mermaid batch")?;
    std::fs::write(run_dir.join("oracle-outputs.jsonl"), &raw)?;
    let outs: Vec<OracleOutput> = read_jsonl(&raw)?;
    check_recording(&ds.tasks, lines, &outs).context("recording check")?;
    let t = tally(&ds.rule, &ds.tasks, lines, &outs)?;
    std::fs::write(
        run_dir.join("tallies.json"),
        serde_json::to_string_pretty(&t)? + "\n",
    )?;
    std::fs::write(run_dir.join("tables.md"), render_tables(&t))?;
    Ok(())
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
