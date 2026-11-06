//! `python.version()` for the tree-walking interpreter.
//!
//! The worker runs on a dedicated current-thread runtime so the child stays
//! alive and `block_on` is never called inside the caller's runtime.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::Interpreter;
use super::value::VoxValue;

/// Same text the CLI returns when `Vox.toml` has no `[python]` table.
pub const PY_ENV_MISSING: &str = "py-env: python not configured (add [python] to Vox.toml)";

/// `[python]` settings the CLI passes in after reading `Vox.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonProject {
    pub root: PathBuf,
    pub max_fetch_mb: u64,
    pub timeout: Duration,
}

enum Msg {
    Hello(std::sync::mpsc::SyncSender<Result<String, String>>),
    ExecutePlan {
        params: serde_json::Value,
        body: Vec<u8>,
        reply: std::sync::mpsc::SyncSender<Result<vox_python_bridge::Reply, String>>,
    },
}

/// Owns the worker thread. Dropping it closes the channel and drops the child.
pub(crate) struct Host {
    tx: tokio::sync::mpsc::UnboundedSender<Msg>,
}

impl Host {
    fn start(project: &PythonProject) -> Result<Self, String> {
        let config = bridge_config(project);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::Builder::new()
            .name("vox-python-bridge".into())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                rt.block_on(async move {
                    let bridge = vox_python_bridge::Bridge::new(config);
                    while let Some(msg) = rx.recv().await {
                        match msg {
                            Msg::Hello(reply) => {
                                let result = hello_version(&bridge).await;
                                let _ = reply.send(result);
                            }
                            Msg::ExecutePlan {
                                params,
                                body,
                                reply,
                            } => {
                                let result = execute_plan_call(&bridge, params, &body).await;
                                let _ = reply.send(result);
                            }
                        }
                    }
                });
            })
            .map_err(|err| format!("py: worker thread: {err}"))?;
        Ok(Self { tx })
    }
}

static PRINT_CAPTURE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static PRINT_LINES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Record `print` lines for a test. Libtest swallows `println!` before a pipe can see it.
pub fn start_print_capture() {
    if let Ok(mut lines) = PRINT_LINES.lock() {
        lines.clear();
    }
    PRINT_CAPTURE.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Stop recording and return the lines `print` emitted since [`start_print_capture`].
pub fn stop_print_capture() -> Vec<String> {
    PRINT_CAPTURE.store(false, std::sync::atomic::Ordering::SeqCst);
    PRINT_LINES
        .lock()
        .map(|mut lines| std::mem::take(&mut *lines))
        .unwrap_or_default()
}

pub(crate) fn note_print(line: &str) {
    if PRINT_CAPTURE.load(std::sync::atomic::Ordering::SeqCst)
        && let Ok(mut lines) = PRINT_LINES.lock()
    {
        lines.push(line.to_string());
    }
}

/// Record the project environment. Replaces any worker already started.
pub fn set_python_project(interp: &mut Interpreter, project: PythonProject) {
    interp.python_host = None;
    interp.python_project = Some(project);
}

#[cfg(test)]
pub(crate) fn worker_spawned(interp: &Interpreter) -> bool {
    interp.python_host.is_some()
}

pub(crate) fn call_version(interp: &mut Interpreter) -> VoxValue {
    match version_string(interp) {
        Ok(version) => VoxValue::Result(Ok(Box::new(VoxValue::Str(version.into())))),
        Err(err) => VoxValue::Result(Err(Box::new(VoxValue::Str(err.into())))),
    }
}

fn version_string(interp: &mut Interpreter) -> Result<String, String> {
    let Some(project) = interp.python_project.clone() else {
        return Err(PY_ENV_MISSING.to_string());
    };
    if interp.python_host.is_none() {
        interp.python_host = Some(Host::start(&project)?);
    }
    let host = interp
        .python_host
        .as_ref()
        .ok_or_else(|| "py: worker stopped".to_string())?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    host.tx
        .send(Msg::Hello(tx))
        .map_err(|_| "py: worker stopped".to_string())?;
    rx.recv_timeout(project.timeout)
        .map_err(|_| "py: worker timed out".to_string())?
}

/// One `execute_plan` on the worker this host already owns.
///
/// Does not construct a second [`vox_python_bridge::Bridge`].
pub(crate) fn execute_plan(
    interp: &mut Interpreter,
    params: serde_json::Value,
    body: Vec<u8>,
) -> Result<vox_python_bridge::Reply, String> {
    let Some(project) = interp.python_project.clone() else {
        return Err(PY_ENV_MISSING.to_string());
    };
    tracing::debug!(payload = %params, "array plan sent to execute_plan");
    if interp.python_host.is_none() {
        interp.python_host = Some(Host::start(&project)?);
    }
    let host = interp
        .python_host
        .as_ref()
        .ok_or_else(|| "py: worker stopped".to_string())?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    host.tx
        .send(Msg::ExecutePlan {
            params,
            body,
            reply: tx,
        })
        .map_err(|_| "py: worker stopped".to_string())?;
    let reply = rx
        .recv_timeout(project.timeout)
        .map_err(|_| "py: worker timed out".to_string())??;
    if std::env::var_os("VOX_BRIDGE_DEBUG_OPS").is_some_and(|value| value == "1") {
        eprintln!("execute_plan");
    }
    Ok(reply)
}

async fn execute_plan_call(
    bridge: &vox_python_bridge::Bridge,
    params: serde_json::Value,
    body: &[u8],
) -> Result<vox_python_bridge::Reply, String> {
    bridge
        .call("execute_plan", params, Some(body))
        .await
        .map_err(|err| err.to_string())
}

async fn hello_version(bridge: &vox_python_bridge::Bridge) -> Result<String, String> {
    match bridge.call("hello", serde_json::json!({}), None).await {
        Ok(reply) => reply
            .value
            .get("python")
            .and_then(|value| value.as_str())
            .filter(|version| !version.is_empty())
            .map(str::to_string)
            .ok_or_else(|| "py: hello reply missing python version".to_string()),
        Err(err) => Err(err.to_string()),
    }
}

fn bridge_config(project: &PythonProject) -> vox_python_bridge::BridgeConfig {
    let python = venv_python(&project.root)
        .map(std::ffi::OsString::from)
        .unwrap_or_else(|| "python3".into());
    let mut config = vox_python_bridge::BridgeConfig {
        worker_cmd: vec![python, "-m".into(), "vox_bridge".into()],
        max_fetch_mb: project.max_fetch_mb,
        timeout: project.timeout,
        ..Default::default()
    };
    config
        .env
        .push(("PYTHONPATH".into(), vox_python_bridge::worker_root().into()));
    config
}

/// `.vox/python/.venv` interpreter, when `vox py sync` has created one.
fn venv_python(root: &Path) -> Option<PathBuf> {
    let unix = root
        .join(".vox")
        .join("python")
        .join(".venv")
        .join("bin")
        .join("python");
    let windows = root
        .join(".vox")
        .join("python")
        .join(".venv")
        .join("Scripts")
        .join("python.exe");
    if unix.is_file() {
        Some(unix)
    } else if windows.is_file() {
        Some(windows)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        Interpreter, PY_ENV_MISSING, PythonProject, VoxValue, note_print, set_python_project,
        start_print_capture, stop_print_capture, worker_spawned,
    };
    use crate::eval::EvalError;
    use crate::eval::caps::CapabilitySet;
    use crate::typeck::{TypeckSeverity, typecheck_module};

    const RETURN_VERSION: &str = "@uses(python)\nfn main() {\n    return python.version()\n}\n";

    fn eval_main(
        src: &str,
        caps: CapabilitySet,
        project: Option<PythonProject>,
    ) -> Result<(VoxValue, Interpreter), EvalError> {
        let module = crate::parser::parse_script(crate::lexer::lex(src)).expect("parse");
        let hir = crate::hir::lower::lower_module(&module);
        let mut interp = Interpreter::new(1_000_000);
        interp.caps = caps;
        if let Some(project) = project {
            set_python_project(&mut interp, project);
        }
        interp.run_module(&hir)?;
        let value = interp.call("main", vec![])?;
        Ok((value, interp))
    }

    fn err_text(value: &VoxValue) -> String {
        match value {
            VoxValue::Result(Err(inner)) => match inner.as_ref() {
                VoxValue::Str(text) => text.to_string(),
                other => panic!("error payload {other:?}"),
            },
            other => panic!("expected Result error, got {other:?}"),
        }
    }

    #[test]
    fn python_bridge_golden_typechecks() {
        let src = include_str!("../../../../examples/golden/py_bridge_hello.vox");
        let module = crate::parser::parse_script(crate::lexer::lex(src)).expect("parse golden");
        let diags = typecheck_module(&module, src);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == TypeckSeverity::Error)
            .map(|d| d.message.clone())
            .collect();
        assert!(errors.is_empty(), "golden typecheck errors: {errors:?}");
    }

    #[test]
    fn python_bridge_missing_config_does_not_spawn() {
        let (value, interp) =
            eval_main(RETURN_VERSION, CapabilitySet::developer_default(), None).expect("eval");
        let text = err_text(&value);
        assert!(
            text.starts_with("py-env:"),
            "missing [python] must return py-env, got {text}"
        );
        assert_eq!(text, PY_ENV_MISSING);
        assert!(!worker_spawned(&interp));
    }

    #[test]
    fn python_bridge_missing_cap_does_not_spawn() {
        let src = "fn main() {\n    return python.version()\n}\n";
        let module = crate::parser::parse_script(crate::lexer::lex(src)).expect("parse");
        let hir = crate::hir::lower::lower_module(&module);
        let mut interp = Interpreter::new(1_000_000);
        interp.caps = CapabilitySet::parse("net:none").expect("caps");
        set_python_project(
            &mut interp,
            PythonProject {
                root: std::env::temp_dir(),
                max_fetch_mb: 1,
                timeout: Duration::from_secs(5),
            },
        );
        interp.run_module(&hir).expect("load");
        let err = interp.call("main", vec![]).expect_err("caps must deny");
        assert!(!worker_spawned(&interp));
        match err {
            EvalError::CapabilityDenied { ns, method } => {
                assert_eq!(ns, "py");
                assert_eq!(method, "version");
            }
            other => panic!("expected capability denied, got {other:?}"),
        }
    }

    #[test]
    fn python_bridge_version_from_worker() {
        let present = std::process::Command::new("python3")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !present {
            eprintln!("skip: python3 is absent");
            return;
        }
        let (value, interp) = eval_main(
            RETURN_VERSION,
            CapabilitySet::developer_default(),
            Some(PythonProject {
                root: std::env::temp_dir(),
                max_fetch_mb: 256,
                timeout: Duration::from_secs(30),
            }),
        )
        .expect("eval");
        match value {
            VoxValue::Result(Ok(inner)) => {
                let VoxValue::Str(version) = *inner else {
                    panic!("ok payload {inner:?}");
                };
                assert!(
                    version.contains('.'),
                    "expected a Python version, got {version}"
                );
            }
            other => panic!("expected Ok version, got {other:?}"),
        }
        assert!(worker_spawned(&interp));
    }

    #[test]
    fn execute_plan_missing_config_does_not_spawn() {
        let mut interp = Interpreter::new(1_000);
        let err = super::execute_plan(&mut interp, serde_json::json!({"ops": []}), Vec::new())
            .expect_err("missing project");
        assert!(err.starts_with("py-env:"), "{err}");
        assert!(!worker_spawned(&interp));
    }

    #[test]
    fn python_bridge_print_capture_round_trip() {
        start_print_capture();
        note_print("3.12.10");
        assert_eq!(stop_print_capture(), vec!["3.12.10".to_string()]);
        note_print("later");
        assert!(stop_print_capture().is_empty());
    }
}
