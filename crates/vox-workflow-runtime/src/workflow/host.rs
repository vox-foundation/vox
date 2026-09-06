//! The `!Send` bridge between the tree-walking interpreter and the async runner.

use anyhow::Context;
use serde_json::Value;
use std::mem::ManuallyDrop;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use vox_compiler::eval::Interpreter;
use vox_compiler::eval::activity_hook::{HostCall, HostDecision as InterpDecision};
use vox_compiler::eval::value::VoxValue;
use vox_compiler::hir::HirModule;
use vox_compiler::hir::nodes::DurabilityKind;

const WORKFLOW_STEP_LIMIT: usize = 10_000_000;

/// A request emitted by the interpreter for the async workflow runner.
#[derive(Debug, Clone)]
pub enum HostRequest {
    /// Begins an activity call.
    Begin {
        /// Activity name.
        name: String,
        /// Journal-JSON activity arguments.
        args: Vec<Value>,
        /// Optional journal-JSON activity options.
        options: Option<Value>,
    },
    /// Ends an executed activity call.
    End {
        /// Activity name.
        name: String,
        /// Journal-JSON activity result.
        result: Value,
        /// Whether activity execution failed.
        failed: bool,
    },
    /// Checks a version patch.
    Patch {
        /// Stable patch identifier.
        change_id: String,
        /// Minimum supported patch version.
        min: u32,
        /// Maximum supported patch version.
        max: u32,
    },
    /// Waits until a deadline.
    Wait {
        /// Unix deadline in milliseconds.
        deadline_ms: i64,
    },
    /// Waits for a named signal.
    WaitSignal {
        /// Signal key.
        key: String,
    },
}

/// A runner decision returned to the interpreter.
#[derive(Debug, Clone)]
pub enum HostDecision {
    /// Replays a journal-JSON value without executing the activity.
    Replay(Value),
    /// Executes the activity body.
    Execute,
    /// Parks workflow execution with a reason.
    Park(String),
    /// Acknowledges a one-way request.
    Ack,
}

/// The terminal result of the interpreter thread.
#[derive(Debug, Clone)]
pub enum HostOutcome {
    /// Workflow completed with a journal-JSON value.
    Completed(Value),
    /// Workflow parked with a reason.
    Parked(String),
    /// Workflow failed with a diagnostic.
    Failed(String),
}

/// Owns the dedicated interpreter thread and its request/response channels.
pub struct WorkflowHost {
    req_rx: UnboundedReceiver<HostRequest>,
    resp_tx: ManuallyDrop<UnboundedSender<HostDecision>>,
    handle: Option<std::thread::JoinHandle<HostOutcome>>,
    #[cfg(test)]
    thread_finished: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl WorkflowHost {
    /// Spawns a workflow interpreter on a dedicated OS thread.
    pub fn spawn(hir: HirModule, workflow_name: &str, args: Vec<Value>) -> anyhow::Result<Self> {
        let activity_names: std::collections::HashSet<String> = hir
            .functions
            .iter()
            .filter(|f| f.durability == Some(DurabilityKind::Activity))
            .map(|f| f.name.clone())
            .collect();
        let workflow_names: std::collections::HashSet<String> = hir
            .functions
            .iter()
            .filter(|f| f.durability == Some(DurabilityKind::Workflow))
            .map(|f| f.name.clone())
            .collect();
        if !workflow_names.contains(workflow_name) {
            anyhow::bail!(
                "workflow `{workflow_name}` not found in HIR; declare it with the `workflow` keyword"
            );
        }

        let (req_tx, req_rx) = unbounded_channel::<HostRequest>();
        let (resp_tx, resp_rx) = unbounded_channel::<HostDecision>();
        let name = workflow_name.to_string();
        #[cfg(test)]
        let thread_finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        #[cfg(test)]
        let thread_finished_on_thread = std::sync::Arc::clone(&thread_finished);

        let handle = std::thread::Builder::new()
            .name(format!("vox-workflow-{name}"))
            .spawn(move || {
                let outcome = run_on_thread(
                    hir,
                    name,
                    args,
                    activity_names,
                    workflow_names,
                    req_tx,
                    resp_rx,
                );
                #[cfg(test)]
                thread_finished_on_thread.store(true, std::sync::atomic::Ordering::Release);
                outcome
            })
            .context("spawning the workflow interpreter thread")?;

        Ok(Self {
            req_rx,
            resp_tx: ManuallyDrop::new(resp_tx),
            handle: Some(handle),
            #[cfg(test)]
            thread_finished,
        })
    }

    /// Receives the next interpreter request, or `None` after completion.
    pub async fn next(&mut self) -> Option<HostRequest> {
        self.req_rx.recv().await
    }

    /// Sends a runner decision to the waiting interpreter.
    pub fn respond(&self, decision: HostDecision) -> anyhow::Result<()> {
        self.resp_tx
            .send(decision)
            .map_err(|_| anyhow::anyhow!("workflow interpreter thread hung up"))
    }

    /// Joins the interpreter thread and returns its terminal outcome.
    pub async fn finish(mut self) -> anyhow::Result<HostOutcome> {
        let handle = self
            .handle
            .take()
            .ok_or_else(|| anyhow::anyhow!("workflow host already joined"))?;
        tokio::task::spawn_blocking(move || handle.join())
            .await
            .context("joining the workflow interpreter thread")?
            .map_err(|_| anyhow::anyhow!("workflow interpreter thread panicked"))
    }
}

impl Drop for WorkflowHost {
    fn drop(&mut self) {
        let _ = self.resp_tx.send(HostDecision::Park("host-dropped".into()));
        // SAFETY: `resp_tx` is dropped exactly once here, and no later code accesses it.
        #[allow(unsafe_code)]
        unsafe {
            ManuallyDrop::drop(&mut self.resp_tx);
        }
        if let Some(handle) = self.handle.take() {
            let _ = std::thread::Builder::new()
                .name("vox-workflow-reap".into())
                .spawn(move || {
                    let _ = handle.join();
                });
        }
    }
}

fn run_on_thread(
    hir: HirModule,
    workflow_name: String,
    args: Vec<Value>,
    activity_names: std::collections::HashSet<String>,
    workflow_names: std::collections::HashSet<String>,
    req_tx: UnboundedSender<HostRequest>,
    mut resp_rx: UnboundedReceiver<HostDecision>,
) -> HostOutcome {
    let mut interp = Interpreter::new(WORKFLOW_STEP_LIMIT);
    if let Err(e) = interp.run_module(&hir) {
        return HostOutcome::Failed(format!("loading module: {e:?}"));
    }
    interp.activity_names = activity_names;
    interp.workflow_names = workflow_names;
    interp.activity_hook = Some(Box::new(move |call| {
        let request = match &call {
            HostCall::Begin {
                name,
                args,
                options,
            } => HostRequest::Begin {
                name: name.clone(),
                args: args.iter().map(VoxValue::to_journal_json).collect(),
                options: options.as_ref().map(VoxValue::to_journal_json),
            },
            HostCall::End {
                name,
                result,
                failed,
            } => HostRequest::End {
                name: name.clone(),
                result: result.to_journal_json(),
                failed: *failed,
            },
            HostCall::Patch {
                change_id,
                min,
                max,
            } => HostRequest::Patch {
                change_id: change_id.clone(),
                min: *min,
                max: *max,
            },
            HostCall::Wait { deadline_ms } => HostRequest::Wait {
                deadline_ms: *deadline_ms,
            },
            HostCall::WaitSignal { key } => HostRequest::WaitSignal { key: key.clone() },
        };
        if req_tx.send(request).is_err() {
            return Err(vox_compiler::eval::EvalError::Panic(
                "durable workflow runner hung up".into(),
            ));
        }
        let Some(decision) = resp_rx.blocking_recv() else {
            return Err(vox_compiler::eval::EvalError::Panic(
                "durable workflow runner dropped the response channel".into(),
            ));
        };
        Ok(match decision {
            HostDecision::Replay(v) => InterpDecision::Replay(
                VoxValue::from_journal_json(&v)
                    .map_err(|e| vox_compiler::eval::EvalError::Panic(e.message))?,
            ),
            HostDecision::Execute => InterpDecision::Execute,
            HostDecision::Park(r) => InterpDecision::Park(r),
            HostDecision::Ack => InterpDecision::Ack,
        })
    }));

    let vox_args = match args
        .iter()
        .map(VoxValue::from_journal_json)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(e) => return HostOutcome::Failed(e.message),
    };
    match interp.call(&workflow_name, vox_args) {
        Ok(value) => HostOutcome::Completed(value.to_journal_json()),
        Err(vox_compiler::eval::EvalError::WorkflowParked(reason)) => HostOutcome::Parked(reason),
        Err(e) => HostOutcome::Failed(format!("{e:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vox_compiler::hir::lower_module;
    use vox_compiler::lexer::cursor::lex;
    use vox_compiler::parser::parse;

    const SRC: &str = r#"
activity charge(amount: int) to int { return amount * 2 }
workflow wf(amount: int) to int {
    let a = charge(amount)
    return a
}
"#;

    fn hir_of(src: &str) -> vox_compiler::hir::HirModule {
        lower_module(&parse(lex(src)).expect("parses"))
    }

    #[tokio::test]
    async fn host_executes_the_body_and_returns_the_real_value() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(21)]).expect("spawns");
        let mut begins = 0;
        while let Some(req) = host.next().await {
            match req {
                HostRequest::Begin { name, args, .. } => {
                    begins += 1;
                    assert_eq!(name, "charge");
                    assert_eq!(args, vec![json!(21)]);
                    host.respond(HostDecision::Execute).expect("respond");
                }
                HostRequest::End { result, failed, .. } => {
                    assert_eq!(result, json!(42));
                    assert!(!failed);
                    host.respond(HostDecision::Ack).expect("respond");
                }
                other => panic!("unexpected request {other:?}"),
            }
        }
        assert_eq!(begins, 1);
        match host.finish().await.expect("joins") {
            HostOutcome::Completed(v) => assert_eq!(v, json!(42)),
            other => panic!("expected Completed(42), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn host_replay_does_not_execute_the_body() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(21)]).expect("spawns");
        let mut ends = 0;
        while let Some(req) = host.next().await {
            match req {
                HostRequest::Begin { .. } => {
                    host.respond(HostDecision::Replay(json!(999)))
                        .expect("respond");
                }
                HostRequest::End { .. } => {
                    ends += 1;
                    host.respond(HostDecision::Ack).expect("respond");
                }
                other => {
                    host.respond(HostDecision::Ack).expect("respond");
                    panic!("unexpected {other:?}");
                }
            }
        }
        assert_eq!(ends, 0);
        match host.finish().await.expect("joins") {
            HostOutcome::Completed(v) => assert_eq!(v, json!(999)),
            other => panic!("expected Completed(999), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn host_park_surfaces_as_parked_outcome() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(1)]).expect("spawns");
        while let Some(req) = host.next().await {
            if let HostRequest::Begin { .. } = req {
                host.respond(HostDecision::Park("timer".into()))
                    .expect("respond");
            } else {
                host.respond(HostDecision::Ack).expect("respond");
            }
        }
        match host.finish().await.expect("joins") {
            HostOutcome::Parked(reason) => assert_eq!(reason, "timer"),
            other => panic!("expected Parked, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn host_reports_a_missing_workflow_as_an_error() {
        let err = WorkflowHost::spawn(hir_of(SRC), "no_such_workflow", vec![]);
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn drop_unblocks_a_waiting_thread() {
        let mut host = WorkflowHost::spawn(hir_of(SRC), "wf", vec![json!(1)]).expect("spawns");
        let first = host.next().await;
        assert!(matches!(first, Some(HostRequest::Begin { .. })));
        let thread_finished = std::sync::Arc::clone(&host.thread_finished);
        drop(host);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !thread_finished.load(std::sync::atomic::Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("Drop must unblock and reap the interpreter thread");
    }
}
